use crate::shell::{detect_shell, ShellKind};
use anyhow::{Context, Result};
use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

const START_MARKER: &str = "# >>> awsp shell integration >>>";
const END_MARKER: &str = "# <<< awsp shell integration <<<";
const LEGACY_START_MARKER: &str = "# >>> awsp init >>>";
const LEGACY_END_MARKER: &str = "# <<< awsp init <<<";

pub struct ShellIntegrationPlan {
    shell: ShellKind,
    script_path: PathBuf,
    rc_paths: Vec<PathBuf>,
}

pub struct AppliedShellIntegration {
    pub script_path: PathBuf,
    pub rc_paths: Vec<PathBuf>,
}

impl ShellIntegrationPlan {
    pub fn for_shell(shell: ShellKind) -> Result<Self> {
        Ok(Self {
            shell,
            script_path: integration_script_path()?,
            rc_paths: rc_paths(shell)?,
        })
    }

    pub fn for_current_shell() -> Result<Option<Self>> {
        let Some(shell) = detect_shell() else {
            return Ok(None);
        };
        Ok(Some(Self::for_shell(shell)?))
    }

    pub fn shell(&self) -> ShellKind {
        self.shell
    }

    pub fn script_path(&self) -> &Path {
        &self.script_path
    }

    pub fn display_rc_paths(&self) -> String {
        display_paths(&self.rc_paths)
    }

    pub fn is_installed(&self) -> Result<bool> {
        integration_is_installed(&self.rc_paths, &self.script_path)
    }

    pub fn apply(&self) -> Result<AppliedShellIntegration> {
        write_integration_script(&self.script_path, self.shell)?;
        for path in &self.rc_paths {
            install_rc_hook(path)?;
        }
        Ok(AppliedShellIntegration {
            script_path: self.script_path.clone(),
            rc_paths: self.rc_paths.clone(),
        })
    }
}

pub fn integration_script_path() -> Result<PathBuf> {
    let home = env::var("HOME").context("HOME is not set")?;
    Ok(Path::new(&home)
        .join(".config")
        .join("awsp")
        .join("shell")
        .join("awsp.sh"))
}

pub fn integration_is_installed_for_current_shell() -> Result<bool> {
    let Some(plan) = ShellIntegrationPlan::for_current_shell()? else {
        return Ok(false);
    };
    plan.is_installed()
}

fn write_integration_script(path: &Path, shell: ShellKind) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    fs::write(path, crate::shell::init_script(shell))
        .with_context(|| format!("failed to write {}", path.display()))
}

fn install_rc_hook(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let block = rc_block();
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => {
            return Err(error).with_context(|| format!("failed to read {}", path.display()))
        }
    };

    if content.contains(START_MARKER) {
        let updated = replace_marked_block(&content, START_MARKER, END_MARKER, &block)
            .unwrap_or_else(|| content.clone());
        fs::write(path, updated).with_context(|| format!("failed to write {}", path.display()))?;
        return Ok(());
    }

    if content.contains(LEGACY_START_MARKER) {
        let updated =
            replace_marked_block(&content, LEGACY_START_MARKER, LEGACY_END_MARKER, &block)
                .unwrap_or_else(|| content.clone());
        fs::write(path, updated).with_context(|| format!("failed to write {}", path.display()))?;
        return Ok(());
    }

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("failed to open {}", path.display()))?;
    writeln!(file, "\n{block}").with_context(|| format!("failed to write {}", path.display()))?;

    Ok(())
}

fn integration_is_installed(paths: &[PathBuf], script_path: &Path) -> Result<bool> {
    if !script_path.exists() {
        return Ok(false);
    }

    for path in paths {
        match fs::read_to_string(path) {
            Ok(content) if content.contains(START_MARKER) => {}
            Ok(_) => return Ok(false),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(error) => {
                return Err(error).with_context(|| format!("failed to read {}", path.display()))
            }
        }
    }
    Ok(true)
}

fn rc_block() -> String {
    format!(
        r#"{START_MARKER}
if [ -r "$HOME/.config/awsp/shell/awsp.sh" ]; then
  . "$HOME/.config/awsp/shell/awsp.sh"
fi
{END_MARKER}"#
    )
}

fn replace_marked_block(
    content: &str,
    start_marker: &str,
    end_marker: &str,
    replacement: &str,
) -> Option<String> {
    let start = content.find(start_marker)?;
    let end = content[start..].find(end_marker)? + start + end_marker.len();
    let mut updated = String::new();
    updated.push_str(&content[..start]);
    updated.push_str(replacement);
    updated.push_str(&content[end..]);
    Some(updated)
}

fn rc_paths(shell: ShellKind) -> Result<Vec<PathBuf>> {
    let home = env::var("HOME").context("HOME is not set")?;
    let home = Path::new(&home);
    Ok(match shell {
        ShellKind::Bash => bash_rc_paths_for_home(home),
        ShellKind::Zsh => zsh_rc_paths_for_home(home, env::var_os("ZDOTDIR").map(PathBuf::from)),
    })
}

#[cfg(test)]
fn rc_paths_for_home(shell: ShellKind, home: &Path) -> Vec<PathBuf> {
    match shell {
        ShellKind::Bash => bash_rc_paths_for_home(home),
        ShellKind::Zsh => zsh_rc_paths_for_home(home, None),
    }
}

fn bash_rc_paths_for_home(home: &Path) -> Vec<PathBuf> {
    vec![home.join(".bashrc"), bash_login_rc_path(home)]
}

fn bash_login_rc_path(home: &Path) -> PathBuf {
    for file_name in [".bash_profile", ".bash_login", ".profile"] {
        let path = home.join(file_name);
        if path.exists() {
            return path;
        }
    }
    home.join(".bash_profile")
}

fn zsh_rc_paths_for_home(home: &Path, zdotdir_env: Option<PathBuf>) -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Some(zdotdir) = zdotdir_env {
        push_unique_path(&mut dirs, expand_zdotdir_path(&zdotdir, home));
    }

    if let Some(zdotdir) = parse_zdotdir_from_zshenv(home) {
        push_unique_path(&mut dirs, zdotdir);
    }

    let xdg_zsh_dir = home.join(".config").join("zsh");
    if xdg_zsh_dir.is_dir() {
        push_unique_path(&mut dirs, xdg_zsh_dir);
    }

    push_unique_path(&mut dirs, home.to_path_buf());

    let mut paths = Vec::new();
    for dir in dirs {
        push_unique_path(&mut paths, dir.join(".zshrc"));
        push_unique_path(&mut paths, dir.join(".zprofile"));
    }
    paths
}

fn parse_zdotdir_from_zshenv(home: &Path) -> Option<PathBuf> {
    let content = fs::read_to_string(home.join(".zshenv")).ok()?;
    for line in content.lines() {
        let line = line.split('#').next().unwrap_or_default().trim();
        let line = line.strip_prefix("export ").unwrap_or(line).trim();
        let line = line.strip_prefix("typeset -gx ").unwrap_or(line).trim();
        let Some(value) = line.strip_prefix("ZDOTDIR=") else {
            continue;
        };
        let value = value
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .trim_end_matches(';');
        if value.is_empty() {
            continue;
        }
        return Some(expand_zdotdir_value(value, home));
    }
    None
}

fn expand_zdotdir_path(path: &Path, home: &Path) -> PathBuf {
    if path.is_absolute() {
        return path.to_path_buf();
    }

    let value = path.to_string_lossy();
    expand_zdotdir_value(&value, home)
}

fn expand_zdotdir_value(value: &str, home: &Path) -> PathBuf {
    let value = value.trim().trim_matches('"').trim_matches('\'');

    if value == "$HOME" || value == "${HOME}" || value == "~" {
        return home.to_path_buf();
    }
    if let Some(rest) = value.strip_prefix("$HOME/") {
        return home.join(rest);
    }
    if let Some(rest) = value.strip_prefix("${HOME}/") {
        return home.join(rest);
    }
    if let Some(rest) = value.strip_prefix("~/") {
        return home.join(rest);
    }

    let path = PathBuf::from(value);
    if path.is_absolute() {
        path
    } else {
        home.join(path)
    }
}

fn push_unique_path(paths: &mut Vec<PathBuf>, path: PathBuf) {
    if !paths.iter().any(|existing| existing == &path) {
        paths.push(path);
    }
}

fn display_paths(paths: &[PathBuf]) -> String {
    paths
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(" and ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_home(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = env::temp_dir().join(format!("awsp-{name}-{}-{nanos}", std::process::id()));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn rc_block_sources_static_script_without_eval_init() {
        let block = rc_block();
        assert!(block.contains(". \"$HOME/.config/awsp/shell/awsp.sh\""));
        assert!(!block.contains("awsp init"));
        assert!(!block.contains("eval"));
    }

    #[test]
    fn replaces_legacy_eval_block() {
        let content =
            "before\n# >>> awsp init >>>\neval \"$(awsp init zsh)\"\n# <<< awsp init <<<\nafter\n";
        let replacement = rc_block();
        let updated = replace_marked_block(
            content,
            LEGACY_START_MARKER,
            LEGACY_END_MARKER,
            &replacement,
        )
        .unwrap();

        assert!(updated.contains(START_MARKER));
        assert!(updated.contains("before"));
        assert!(updated.contains("after"));
        assert!(!updated.contains("eval \"$(awsp init zsh)\""));
    }

    #[test]
    fn bash_setup_targets_bashrc_and_existing_login_file() {
        let home = temp_home("bash-existing-login");
        fs::write(home.join(".profile"), "# existing profile\n").unwrap();

        let paths = rc_paths_for_home(ShellKind::Bash, &home);

        assert_eq!(paths, vec![home.join(".bashrc"), home.join(".profile")]);
        fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn bash_setup_creates_bash_profile_when_no_login_file_exists() {
        let home = temp_home("bash-new-login");

        let paths = rc_paths_for_home(ShellKind::Bash, &home);

        assert_eq!(
            paths,
            vec![home.join(".bashrc"), home.join(".bash_profile")]
        );
        fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn zsh_setup_targets_zshrc_and_zprofile_by_default() {
        let home = temp_home("zsh-default");

        let paths = rc_paths_for_home(ShellKind::Zsh, &home);

        assert_eq!(paths, vec![home.join(".zshrc"), home.join(".zprofile")]);
        fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn zsh_setup_targets_exported_zdotdir_before_home() {
        let home = temp_home("zsh-env-zdotdir");
        let zdotdir = home.join(".config").join("zsh");

        let paths = zsh_rc_paths_for_home(&home, Some(zdotdir.clone()));

        assert_eq!(
            paths,
            vec![
                zdotdir.join(".zshrc"),
                zdotdir.join(".zprofile"),
                home.join(".zshrc"),
                home.join(".zprofile"),
            ]
        );
        fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn zsh_setup_reads_simple_zdotdir_from_zshenv() {
        let home = temp_home("zsh-zshenv-zdotdir");
        fs::write(
            home.join(".zshenv"),
            "export ZDOTDIR=\"$HOME/.config/zsh\"\n",
        )
        .unwrap();
        let zdotdir = home.join(".config").join("zsh");

        let paths = rc_paths_for_home(ShellKind::Zsh, &home);

        assert_eq!(
            paths,
            vec![
                zdotdir.join(".zshrc"),
                zdotdir.join(".zprofile"),
                home.join(".zshrc"),
                home.join(".zprofile"),
            ]
        );
        fs::remove_dir_all(home).unwrap();
    }
}

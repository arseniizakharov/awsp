use crate::prompt_yes_no;
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

pub fn maybe_install_for_plain_entrypoint() -> Result<()> {
    let Some(shell) = detect_shell() else {
        return Ok(());
    };

    let rc_path = rc_path(shell)?;
    if integration_is_installed(&rc_path, &integration_script_path()?)? {
        return Ok(());
    }

    let question = format!(
        "awsp shell integration is not installed. Install a static hook into {}? [Y/n] ",
        rc_path.display()
    );

    if !prompt_yes_no(&question, true)? {
        return Ok(());
    }

    install_shell_integration(shell)?;
    let script_path = integration_script_path()?;
    eprintln!(
        "Installed awsp shell integration: {} sources {}.",
        rc_path.display(),
        script_path.display()
    );
    eprintln!(
        "This process cannot modify its parent shell. Restart the shell or run: source {}",
        script_path.display()
    );

    Ok(())
}

pub fn install_shell_integration(shell: ShellKind) -> Result<()> {
    let script_path = integration_script_path()?;
    write_integration_script(&script_path, shell)?;
    install_rc_hook(&rc_path(shell)?)
}

pub fn integration_script_path() -> Result<PathBuf> {
    let home = env::var("HOME").context("HOME is not set")?;
    Ok(Path::new(&home)
        .join(".config")
        .join("awsp")
        .join("shell")
        .join("awsp.sh"))
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

fn integration_is_installed(path: &Path, script_path: &Path) -> Result<bool> {
    match fs::read_to_string(path) {
        Ok(content) => Ok(content.contains(START_MARKER) && script_path.exists()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error).with_context(|| format!("failed to read {}", path.display())),
    }
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

fn rc_path(shell: ShellKind) -> Result<PathBuf> {
    let home = env::var("HOME").context("HOME is not set")?;
    let file_name = match shell {
        ShellKind::Bash => ".bashrc",
        ShellKind::Zsh => ".zshrc",
    };
    Ok(Path::new(&home).join(file_name))
}

#[cfg(test)]
mod tests {
    use super::*;

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
}

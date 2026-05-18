use crate::prompt_yes_no;
use crate::shell::{detect_shell, ShellKind};
use anyhow::{Context, Result};
use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

const START_MARKER: &str = "# >>> awsp init >>>";
const END_MARKER: &str = "# <<< awsp init <<<";

pub fn maybe_install_for_plain_entrypoint() -> Result<()> {
    let Some(shell) = detect_shell() else {
        return Ok(());
    };

    let rc_path = rc_path(shell)?;
    if integration_is_installed(&rc_path)? {
        return Ok(());
    }

    let question = format!(
        "awsp shell integration is not installed. Install it into {}? [Y/n] ",
        rc_path.display()
    );

    if !prompt_yes_no(&question, true)? {
        return Ok(());
    }

    append_integration(&rc_path, shell)?;
    eprintln!(
        "Installed awsp shell integration into {}.",
        rc_path.display()
    );
    eprintln!(
        "This process cannot modify its parent shell. Restart the shell or run: source {}",
        rc_path.display()
    );

    Ok(())
}

fn append_integration(path: &Path, shell: ShellKind) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("failed to open {}", path.display()))?;

    writeln!(
        file,
        "\n{START_MARKER}\neval \"$(awsp init {})\"\n{END_MARKER}",
        shell.as_str()
    )
    .with_context(|| format!("failed to write {}", path.display()))?;

    Ok(())
}

fn integration_is_installed(path: &Path) -> Result<bool> {
    match fs::read_to_string(path) {
        Ok(content) => Ok(content.contains(START_MARKER)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error).with_context(|| format!("failed to read {}", path.display())),
    }
}

fn rc_path(shell: ShellKind) -> Result<PathBuf> {
    let home = env::var("HOME").context("HOME is not set")?;
    let file_name = match shell {
        ShellKind::Bash => ".bashrc",
        ShellKind::Zsh => ".zshrc",
    };
    Ok(Path::new(&home).join(file_name))
}

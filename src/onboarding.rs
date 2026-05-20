use crate::prompt;
use crate::shell_integration::ShellIntegrationPlan;
use anyhow::Result;

pub fn maybe_install_for_plain_entrypoint() -> Result<()> {
    let Some(plan) = ShellIntegrationPlan::for_current_shell()? else {
        return Ok(());
    };

    if plan.is_installed()? {
        return Ok(());
    }

    let question = format!(
        "awsp shell integration is not installed. Install a static hook into {}? [Y/n] ",
        plan.display_rc_paths()
    );

    if !prompt::yes_no(&question, true)? {
        return Ok(());
    }

    let applied = plan.apply()?;
    eprintln!(
        "Installed awsp shell integration: {} source {}.",
        plan.display_rc_paths(),
        applied.script_path.display()
    );
    eprintln!(
        "This process cannot modify its parent shell. Restart the shell or run: source {}",
        applied.script_path.display()
    );

    Ok(())
}

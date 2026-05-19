use anyhow::{bail, Context, Result};
use std::fs::OpenOptions;
use std::process::{Command, Stdio};

const AWS_CLI_MISSING_MESSAGE: &str = "\
AWS CLI is required for this command, but `aws` was not found in PATH.

Install AWS CLI v2, for example:
  brew install awscli

If AWS CLI is already installed, make sure `aws` is available in PATH and run:
  awsp doctor";

pub fn is_available() -> bool {
    Command::new("aws")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

pub fn missing_cli_message() -> &'static str {
    AWS_CLI_MISSING_MESSAGE
}

fn ensure_available() -> Result<()> {
    if is_available() {
        return Ok(());
    }

    bail!("{AWS_CLI_MISSING_MESSAGE}");
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AwsOutput {
    Inherit,
    UserTerminal,
}

pub fn login_profile(profile: &str, output: AwsOutput) -> Result<()> {
    ensure_available()?;

    let status = Command::new("aws")
        .args(["sso", "login", "--profile", profile])
        .env("AWS_PAGER", "")
        .stdin(Stdio::inherit())
        .stdout(user_stdout(output))
        .stderr(Stdio::inherit())
        .status()
        .with_context(|| "failed to run aws sso login")?;

    if !status.success() {
        bail!("aws sso login failed for profile {profile}");
    }

    Ok(())
}

fn user_stdout(output: AwsOutput) -> Stdio {
    match output {
        AwsOutput::Inherit => Stdio::inherit(),
        AwsOutput::UserTerminal => OpenOptions::new()
            .write(true)
            .open("/dev/tty")
            .map(Stdio::from)
            .or_else(|_| {
                OpenOptions::new()
                    .write(true)
                    .open("/dev/stderr")
                    .map(Stdio::from)
            })
            .unwrap_or_else(|_| Stdio::null()),
    }
}

pub fn login_session(session: &str) -> Result<()> {
    ensure_available()?;

    let status = Command::new("aws")
        .args(["sso", "login", "--sso-session", session])
        .env("AWS_PAGER", "")
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .with_context(|| "failed to run aws sso login")?;

    if !status.success() {
        bail!("aws sso login failed for SSO session {session}");
    }

    Ok(())
}

pub fn logout() -> Result<()> {
    ensure_available()?;

    let status = Command::new("aws")
        .args(["sso", "logout"])
        .env("AWS_PAGER", "")
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .with_context(|| "failed to run aws sso logout")?;

    if !status.success() {
        bail!("aws sso logout failed");
    }

    Ok(())
}

pub fn whoami(profile: Option<&str>) -> Result<()> {
    ensure_available()?;

    let mut command = Command::new("aws");
    command
        .args(["sts", "get-caller-identity", "--no-cli-pager"])
        .env("AWS_PAGER", "")
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    if let Some(profile) = profile {
        command.args(["--profile", profile]);
    }

    let status = command
        .status()
        .with_context(|| "failed to run aws sts get-caller-identity")?;

    if !status.success() {
        bail!("aws sts get-caller-identity failed");
    }

    Ok(())
}

pub fn verify(profile: &str) -> Result<String> {
    ensure_available()?;

    let output = Command::new("aws")
        .args([
            "sts",
            "get-caller-identity",
            "--profile",
            profile,
            "--output",
            "json",
            "--no-cli-pager",
        ])
        .env("AWS_PAGER", "")
        .output()
        .with_context(|| "failed to run aws sts get-caller-identity")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("aws sts get-caller-identity failed for {profile}: {stderr}");
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_cli_message_names_dependency_and_fix() {
        let message = missing_cli_message();
        assert!(message.contains("AWS CLI is required"));
        assert!(message.contains("brew install awscli"));
        assert!(message.contains("awsp doctor"));
    }
}

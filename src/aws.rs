use anyhow::{bail, Context, Result};
use std::fs::OpenOptions;
use std::process::{Command, Stdio};

pub fn is_available() -> bool {
    Command::new("aws")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AwsOutput {
    Inherit,
    UserTerminal,
}

pub fn login_profile(profile: &str, output: AwsOutput) -> Result<()> {
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

pub fn whoami(profile: Option<&str>) -> Result<()> {
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

use crate::aws_config::SsoProfile;
use crate::cache::LoginStatus;
use anyhow::{bail, Context, Result};
use std::io::Write;
use std::process::{Command, Stdio};

pub fn is_available() -> bool {
    Command::new("fzf")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

pub fn select_profile(
    profiles: &[SsoProfile],
    statuses: &[LoginStatus],
    current_profile: Option<&str>,
) -> Result<String> {
    if profiles.is_empty() {
        bail!("no complete AWS SSO profiles found");
    }

    if !is_available() {
        bail!("fzf is required for interactive profile selection; install fzf or run awsp use <profile>");
    }

    let mut rows = String::new();
    for (index, profile) in profiles.iter().enumerate() {
        let marker = if Some(profile.name.as_str()) == current_profile {
            "*"
        } else {
            ""
        };
        let status = statuses.get(index).copied().unwrap_or(LoginStatus::Unknown);
        rows.push_str(&format!(
            "{index}\t{marker}\t{}\t{}\t{}\t{status}\n",
            profile.name,
            profile.role_name,
            profile.region.label(),
        ));
    }

    let mut command = Command::new("fzf");
    command
        .args([
            "--delimiter",
            "\t",
            "--with-nth",
            "2..",
            "--nth",
            "3..",
            "--prompt",
            "awsp> ",
            "--header",
            "* current profile | region ending in * is inherited from [default]",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());

    if let Some(current_profile) = current_profile {
        command.args(["--query", current_profile]);
    }

    let mut child = command.spawn().with_context(|| "failed to start fzf")?;
    {
        let mut stdin = child.stdin.take().context("failed to open fzf stdin")?;
        stdin
            .write_all(rows.as_bytes())
            .with_context(|| "failed to write profiles to fzf")?;
    }

    let output = child
        .wait_with_output()
        .with_context(|| "failed to wait for fzf")?;

    if !output.status.success() {
        bail!("profile selection cancelled");
    }

    let selected = String::from_utf8_lossy(&output.stdout);
    let Some(index) = selected
        .split('\t')
        .next()
        .and_then(|value| value.trim().parse::<usize>().ok())
    else {
        bail!("fzf returned an invalid selection");
    };

    profiles
        .get(index)
        .map(|profile| profile.name.clone())
        .context("fzf returned an out-of-range selection")
}

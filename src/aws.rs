use crate::aws_config::SsoProfile;
use anyhow::{bail, Context, Result};
use std::fs::OpenOptions;
use std::process::{Command, Stdio};

const AWS_CLI_MISSING_MESSAGE: &str = "\
AWS CLI is required for this command, but `aws` was not found in PATH.

Install AWS CLI v2, for example:
  brew install awscli

If AWS CLI is already installed, make sure `aws` is available in PATH and run:
  awsp doctor";

// Profiles are always selected explicitly (--profile/--sso-session/raw
// credentials). Ambient AWS_PROFILE may point at a profile the user already
// deleted from ~/.aws/config, and the AWS CLI fails on it even for commands
// that do not need a profile, so child processes must not inherit it.
fn aws_command() -> Command {
    let mut command = Command::new("aws");
    command
        .env("AWS_PAGER", "")
        .env_remove("AWS_PROFILE")
        .env_remove("AWS_DEFAULT_PROFILE");
    command
}

pub fn is_available() -> bool {
    aws_command()
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SsoRoleAccess {
    Available,
    LoginExpired { message: String },
    AssignmentMissing { message: String },
    UnknownFailure { message: String },
}

pub fn login_profile(profile: &str, output: AwsOutput) -> Result<()> {
    ensure_available()?;

    let status = aws_command()
        .args(["sso", "login", "--profile", profile])
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

pub fn sso_role_access(profile: &SsoProfile, access_token: &str) -> Result<SsoRoleAccess> {
    ensure_available()?;

    let output = aws_command()
        .args([
            "sso",
            "get-role-credentials",
            "--account-id",
            &profile.account_id,
            "--role-name",
            &profile.role_name,
            "--access-token",
            access_token,
            "--region",
            &profile.sso_region,
            "--output",
            "json",
            "--no-cli-pager",
        ])
        .stdin(Stdio::null())
        .output()
        .with_context(|| "failed to run aws sso get-role-credentials")?;

    if output.status.success() {
        return Ok(SsoRoleAccess::Available);
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    Ok(classify_sso_role_access_error(&stderr))
}

fn classify_sso_role_access_error(stderr: &str) -> SsoRoleAccess {
    let lower = stderr.to_ascii_lowercase();
    if lower.contains("unauthorizedexception")
        || lower.contains("session token not found")
        || lower.contains("invalid token")
        || lower.contains("token has expired")
        || lower.contains("invalidgrant")
    {
        return SsoRoleAccess::LoginExpired {
            message: stderr.to_string(),
        };
    }

    if lower.contains("forbiddenexception")
        || lower.contains("resourcenotfoundexception")
        || lower.contains("no access")
        || lower.contains("not assigned")
        || lower.contains("role cannot be found")
        || lower.contains("account and role")
    {
        return SsoRoleAccess::AssignmentMissing {
            message: stderr.to_string(),
        };
    }

    SsoRoleAccess::UnknownFailure {
        message: stderr.to_string(),
    }
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

    let status = aws_command()
        .args(["sso", "login", "--sso-session", session])
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

    let status = aws_command()
        .args(["sso", "logout"])
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

    let output = aws_command()
        .args([
            "sts",
            "get-caller-identity",
            "--profile",
            profile,
            "--output",
            "json",
            "--no-cli-pager",
        ])
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
    use crate::aws_config::{RegionDisplay, SsoProfile};
    use crate::test_support::{env_lock, EnvGuard};

    fn install_profile_sensitive_aws_mock() -> Result<tempfile::TempDir> {
        let tempdir = tempfile::tempdir()?;
        let aws_path = tempdir.path().join("aws");
        std::fs::write(
            &aws_path,
            r#"#!/bin/sh
if [ -n "$AWS_PROFILE" ] || [ -n "$AWS_DEFAULT_PROFILE" ]; then
  echo "aws: [ERROR]: The config profile (${AWS_PROFILE:-$AWS_DEFAULT_PROFILE}) could not be found" >&2
  exit 1
fi
exit 0
"#,
        )?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = std::fs::metadata(&aws_path)?.permissions();
            permissions.set_mode(0o755);
            std::fs::set_permissions(&aws_path, permissions)?;
        }

        let mut paths = vec![tempdir.path().to_path_buf()];
        if let Some(path) = std::env::var_os("PATH") {
            paths.extend(std::env::split_paths(&path));
        }
        std::env::set_var("PATH", std::env::join_paths(paths)?);
        Ok(tempdir)
    }

    #[test]
    fn logout_ignores_deleted_active_profile() -> Result<()> {
        let _lock = env_lock();
        let _guard = EnvGuard::capture(&["PATH", "AWS_PROFILE", "AWS_DEFAULT_PROFILE"]);
        let _mock = install_profile_sensitive_aws_mock()?;
        std::env::set_var("AWS_PROFILE", "ghost");
        std::env::set_var("AWS_DEFAULT_PROFILE", "ghost");

        logout()
    }

    #[test]
    fn sso_role_access_ignores_deleted_active_profile() -> Result<()> {
        let _lock = env_lock();
        let _guard = EnvGuard::capture(&["PATH", "AWS_PROFILE", "AWS_DEFAULT_PROFILE"]);
        let _mock = install_profile_sensitive_aws_mock()?;
        std::env::set_var("AWS_PROFILE", "ghost");
        std::env::set_var("AWS_DEFAULT_PROFILE", "ghost");

        let profile = SsoProfile {
            name: "mg-tt-prod".to_string(),
            sso_session: Some("corp".to_string()),
            sso_start_url: "https://example.awsapps.com/start".to_string(),
            sso_region: "us-east-1".to_string(),
            account_id: "111122223333".to_string(),
            role_name: "Admin".to_string(),
            region: RegionDisplay::Unset,
        };

        let access = sso_role_access(&profile, "token")?;
        assert_eq!(access, SsoRoleAccess::Available);
        Ok(())
    }

    #[test]
    fn login_session_ignores_deleted_active_profile() -> Result<()> {
        let _lock = env_lock();
        let _guard = EnvGuard::capture(&["PATH", "AWS_PROFILE", "AWS_DEFAULT_PROFILE"]);
        let _mock = install_profile_sensitive_aws_mock()?;
        std::env::set_var("AWS_PROFILE", "ghost");
        std::env::set_var("AWS_DEFAULT_PROFILE", "ghost");

        login_session("corp")
    }

    #[test]
    fn missing_cli_message_names_dependency_and_fix() {
        let message = missing_cli_message();
        assert!(message.contains("AWS CLI is required"));
        assert!(message.contains("brew install awscli"));
        assert!(message.contains("awsp doctor"));
    }

    #[test]
    fn classifies_sso_access_errors() {
        assert!(matches!(
            classify_sso_role_access_error(
                "An error occurred (UnauthorizedException) when calling the GetRoleCredentials operation: Session token not found or invalid"
            ),
            SsoRoleAccess::LoginExpired { .. }
        ));
        assert!(matches!(
            classify_sso_role_access_error(
                "An error occurred (ForbiddenException) when calling the GetRoleCredentials operation: No access"
            ),
            SsoRoleAccess::AssignmentMissing { .. }
        ));
        assert!(matches!(
            classify_sso_role_access_error("network melted"),
            SsoRoleAccess::UnknownFailure { .. }
        ));
    }
}

use clap::ValueEnum;
use std::env;
use std::path::Path;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ShellKind {
    Bash,
    Zsh,
}

impl ShellKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Bash => "bash",
            Self::Zsh => "zsh",
        }
    }
}

pub fn detect_shell() -> Option<ShellKind> {
    let shell = env::var("SHELL").ok()?;
    let name = Path::new(&shell).file_name()?.to_str()?;
    match name {
        "bash" => Some(ShellKind::Bash),
        "zsh" => Some(ShellKind::Zsh),
        _ => None,
    }
}

pub fn init_script(shell: ShellKind) -> String {
    let shell_name = shell.as_str();
    format!(
        r#"# awsp shell integration for {shell_name}
if [ -z "${{AWSP_SESSION_ID:-}}" ]; then
  export AWSP_SESSION_ID="$(command awsp new-session-id)"
fi

__awsp_restore="$(command awsp __shell restore 2>/dev/null)"
if [ $? -eq 0 ] && [ -n "$__awsp_restore" ]; then
  eval "$__awsp_restore"
fi
unset __awsp_restore

awsp() {{
  case "${{1-}}" in
    ""|use|off|restore)
      local __awsp_output
      local __awsp_status
      __awsp_output="$(command awsp __shell "$@")"
      __awsp_status=$?
      if [ $__awsp_status -eq 0 ]; then
        eval "$__awsp_output"
      else
        return $__awsp_status
      fi
      ;;
    *)
      command awsp "$@"
      ;;
  esac
}}
"#
    )
}

pub fn activation_code(profile: &str, session_id: Option<&str>) -> String {
    let mut lines = Vec::new();
    if let Some(session_id) = session_id {
        lines.push(format!("export AWSP_SESSION_ID={}", quote(session_id)));
    }
    lines.push("unset AWS_ACCESS_KEY_ID".to_string());
    lines.push("unset AWS_SECRET_ACCESS_KEY".to_string());
    lines.push("unset AWS_SESSION_TOKEN".to_string());
    lines.push("unset AWS_SESSION_EXPIRATION".to_string());
    lines.push(format!("export AWS_PROFILE={}", quote(profile)));
    lines.push("export AWS_SDK_LOAD_CONFIG='1'".to_string());
    lines.join("\n")
}

pub fn off_code(session_id: Option<&str>) -> String {
    let mut lines = Vec::new();
    if let Some(session_id) = session_id {
        lines.push(format!("export AWSP_SESSION_ID={}", quote(session_id)));
    }
    lines.push("unset AWS_PROFILE".to_string());
    lines.push("unset AWS_ACCESS_KEY_ID".to_string());
    lines.push("unset AWS_SECRET_ACCESS_KEY".to_string());
    lines.push("unset AWS_SESSION_TOKEN".to_string());
    lines.push("unset AWS_SESSION_EXPIRATION".to_string());
    lines.push("export AWS_SDK_LOAD_CONFIG='1'".to_string());
    lines.join("\n")
}

fn quote(value: &str) -> String {
    let escaped = value.replace('\'', r#"'\''"#);
    format!("'{escaped}'")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_single_quotes_for_shell() {
        assert_eq!(quote("abc'def"), "'abc'\\''def'");
    }

    #[test]
    fn activation_exports_expected_variables() {
        let code = activation_code("prod", Some("session-1"));
        assert!(code.contains("export AWSP_SESSION_ID='session-1'"));
        assert!(code.contains("export AWS_PROFILE='prod'"));
        assert!(code.contains("export AWS_SDK_LOAD_CONFIG='1'"));
        assert!(code.contains("unset AWS_ACCESS_KEY_ID"));
    }
}

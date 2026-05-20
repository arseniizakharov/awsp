use crate::aws_config::{SsoInventory, SsoProfile};
use crate::cache::{CacheStatus, LoginStatus};
use crate::palette;
use crate::{shell, shell_integration};
use crossterm::queue;
use crossterm::style::{Attribute, Print, ResetColor, SetAttribute, SetForegroundColor};
use serde::Serialize;
use std::io::{self, Write};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMode {
    Human,
    Shell,
}

pub fn shell_code(code: &str) {
    println!("{code}");
}

pub fn switch_success(old_profile: Option<&str>, profile: &SsoProfile, status: &CacheStatus) {
    let mut stderr = io::stderr();
    queue!(
        stderr,
        Print("  "),
        SetForegroundColor(palette::GREEN),
        SetAttribute(Attribute::Bold),
        Print("✓  "),
        SetAttribute(Attribute::Reset),
        SetForegroundColor(palette::DIM),
        Print("switched  "),
        SetForegroundColor(palette::FG),
        SetAttribute(Attribute::Bold),
        Print(old_profile.unwrap_or("none")),
        SetAttribute(Attribute::Reset),
        SetForegroundColor(palette::DIM),
        Print("  →  "),
        SetForegroundColor(palette::FG),
        SetAttribute(Attribute::Bold),
        Print(&profile.name),
        SetAttribute(Attribute::Reset),
        ResetColor,
        Print("\n     "),
        SetForegroundColor(palette::MUTED),
        Print(format!(
            "{} · {} · {} · session {}",
            profile.account_id,
            profile.region.label(),
            profile.role_name,
            status.label()
        )),
        ResetColor,
        Print("\n")
    )
    .ok();
    stderr.flush().ok();
}

pub fn inactive_activation(profile_name: &str) {
    eprintln!("Selected {profile_name}.");
    eprintln!(
        "Shell integration is not active in this process, so AWS_PROFILE was not exported here."
    );
    inactive_shell_integration_guidance();
}

pub fn inactive_off() {
    eprintln!("Cleared active AWS profile for this awsp session.");
    eprintln!(
        "Shell integration is not active in this process, so AWS_PROFILE was not unset here."
    );
    inactive_shell_integration_guidance();
}

pub fn profile_table(inventory: &SsoInventory, current: Option<&str>, statuses: &[CacheStatus]) {
    let mut stderr = io::stderr();
    queue!(
        stderr,
        Print("  "),
        SetForegroundColor(palette::MUTED),
        SetAttribute(Attribute::Bold),
        Print(format!("{:<26}", "PROFILE")),
        Print(format!("{:<15}", "ACCOUNT")),
        Print(format!("{:<18}", "REGION")),
        Print("ROLE"),
        SetAttribute(Attribute::Reset),
        ResetColor,
        Print("\n  "),
        SetForegroundColor(palette::DIM),
        Print("─".repeat(76)),
        ResetColor,
        Print("\n")
    )
    .ok();

    for (index, profile) in inventory.profiles().iter().enumerate() {
        let selected = Some(profile.name.as_str()) == current;
        let marker = if selected { "● " } else { "  " };
        let _status = statuses
            .get(index)
            .cloned()
            .unwrap_or_else(CacheStatus::unknown);
        queue!(
            stderr,
            SetForegroundColor(if selected {
                palette::GREEN
            } else {
                palette::DIM
            }),
            Print(marker),
            SetForegroundColor(if selected {
                palette::GREEN
            } else {
                palette::FG
            }),
            Print(format!("{:<24}", profile.name)),
            SetForegroundColor(palette::MUTED),
            Print(format!("{:<15}", profile.account_id)),
            SetForegroundColor(palette::CYAN),
            Print(format!("{:<18}", profile.region.label())),
            SetForegroundColor(palette::PURPLE),
            Print(&profile.role_name),
            ResetColor,
            Print("\n")
        )
        .ok();
    }
    stderr.flush().ok();
}

pub fn status(profile: &SsoProfile, status: &CacheStatus) {
    let mut stderr = io::stderr();
    queue!(
        stderr,
        SetForegroundColor(palette::MINT),
        Print("●"),
        SetForegroundColor(palette::FG),
        SetAttribute(Attribute::Bold),
        Print(format!(" {} ", profile.name)),
        SetAttribute(Attribute::Reset),
        SetForegroundColor(palette::DIM),
        Print("  ·  "),
        SetForegroundColor(palette::CYAN),
        Print(profile.region.label()),
        SetForegroundColor(palette::DIM),
        Print("  ·  "),
        SetForegroundColor(palette::PURPLE),
        Print(&profile.role_name),
        SetForegroundColor(palette::DIM),
        Print("  ·  "),
        SetForegroundColor(status_color(status)),
        SetAttribute(if status.state == LoginStatus::Expired {
            Attribute::Bold
        } else {
            Attribute::Reset
        }),
        Print(status.label()),
        SetAttribute(Attribute::Reset),
        ResetColor,
        Print("\n  "),
        SetForegroundColor(palette::DIM),
        Print("└─ "),
        SetForegroundColor(palette::MUTED),
        Print(format!(
            "{} · {}",
            profile.account_id,
            profile
                .sso_start_url
                .trim_start_matches("https://")
                .trim_end_matches('/')
        )),
        ResetColor,
        Print("\n")
    )
    .ok();
    stderr.flush().ok();
}

pub fn status_json(profile: &SsoProfile, status: &CacheStatus) {
    #[derive(Serialize)]
    struct StatusJson<'a> {
        profile: &'a str,
        account: &'a str,
        region: String,
        role: &'a str,
        sso_start_url: &'a str,
        session_state: &'a str,
        expires_in_seconds: Option<i64>,
    }

    let state = match status.state {
        LoginStatus::Valid => "valid",
        LoginStatus::Expired => "expired",
        LoginStatus::Unknown => "unknown",
    };
    let payload = StatusJson {
        profile: &profile.name,
        account: &profile.account_id,
        region: profile.region.label(),
        role: &profile.role_name,
        sso_start_url: &profile.sso_start_url,
        session_state: state,
        expires_in_seconds: status.expires_in_seconds(),
    };
    println!("{}", serde_json::to_string(&payload).unwrap_or_default());
}

pub fn ambiguous(fragment: &str, matches: &[SsoProfile]) {
    let mut stderr = io::stderr();
    queue!(
        stderr,
        Print("  "),
        SetForegroundColor(palette::DIM),
        Print(format!("matches {} profiles:\n", matches.len()))
    )
    .ok();
    for (index, profile) in matches.iter().take(9).enumerate() {
        queue!(
            stderr,
            Print("  "),
            SetForegroundColor(palette::PINK),
            SetAttribute(Attribute::Bold),
            Print(format!("{} ", index + 1)),
            SetAttribute(Attribute::Reset),
            SetForegroundColor(palette::FG),
            SetAttribute(Attribute::Bold),
            Print(format!("{:<20}", profile.name)),
            SetAttribute(Attribute::Reset),
            SetForegroundColor(palette::MUTED),
            Print(format!("{:<14}", profile.account_id)),
            SetForegroundColor(palette::CYAN),
            Print(format!("{:<11}", profile.region.label())),
            SetForegroundColor(palette::PURPLE),
            Print(&profile.role_name),
            ResetColor,
            Print("\n")
        )
        .ok();
    }
    queue!(
        stderr,
        Print("\n  "),
        SetForegroundColor(palette::DIM),
        Print("pick "),
        SetForegroundColor(palette::PINK),
        SetAttribute(Attribute::Bold),
        Print(format!("1-{}", matches.len().min(9))),
        SetAttribute(Attribute::Reset),
        SetForegroundColor(palette::DIM),
        Print(" · or refine: "),
        SetForegroundColor(palette::FG),
        Print(format!("awsp {fragment}_")),
        ResetColor,
        Print("\n")
    )
    .ok();
    stderr.flush().ok();
}

pub fn did_you_mean(fragment: &str, suggestions: &[SsoProfile]) {
    let mut stderr = io::stderr();
    queue!(
        stderr,
        Print("  "),
        SetForegroundColor(palette::RED),
        SetAttribute(Attribute::Bold),
        Print("✗ "),
        SetAttribute(Attribute::Reset),
        Print("no profile named "),
        SetForegroundColor(palette::RED),
        Print(fragment),
        ResetColor,
        Print("\n")
    )
    .ok();

    if let Some(first) = suggestions.first() {
        queue!(
            stderr,
            SetForegroundColor(palette::DIM),
            Print("    did you mean    "),
            SetForegroundColor(palette::PINK),
            SetAttribute(Attribute::Bold),
            SetAttribute(Attribute::Underlined),
            Print(&first.name),
            SetAttribute(Attribute::NoUnderline),
            SetAttribute(Attribute::Reset),
            SetForegroundColor(palette::DIM),
            Print("  ?\n")
        )
        .ok();
    }

    if suggestions.len() > 1 {
        queue!(
            stderr,
            SetForegroundColor(palette::DIM),
            Print("       or           ")
        )
        .ok();
        for (index, suggestion) in suggestions.iter().skip(1).enumerate() {
            if index > 0 {
                queue!(stderr, SetForegroundColor(palette::DIM), Print(",  ")).ok();
            }
            queue!(
                stderr,
                SetForegroundColor(palette::BLUE),
                Print(&suggestion.name)
            )
            .ok();
        }
        queue!(stderr, Print("\n")).ok();
    }

    queue!(
        stderr,
        Print("\n  "),
        SetForegroundColor(palette::DIM),
        Print("→ "),
        SetForegroundColor(palette::MUTED),
        Print("run "),
        SetForegroundColor(palette::FG),
        SetAttribute(Attribute::Bold),
        Print("awsp"),
        SetAttribute(Attribute::Reset),
        SetForegroundColor(palette::MUTED),
        Print(" with no args for the interactive picker"),
        ResetColor,
        Print("\n")
    )
    .ok();
    stderr.flush().ok();
}

pub fn device_flow_start(profile: &SsoProfile, status: &CacheStatus) {
    let mut stderr = io::stderr();
    let session = profile.sso_session.as_deref().unwrap_or(&profile.name);
    queue!(
        stderr,
        Print("  "),
        SetForegroundColor(palette::RED),
        SetAttribute(Attribute::Bold),
        Print("!  "),
        SetAttribute(Attribute::Reset),
        SetForegroundColor(palette::FG),
        Print(format!("SSO session for {session} {}", status.label())),
        ResetColor,
        Print("\n  "),
        SetForegroundColor(palette::DIM),
        Print("→  "),
        SetForegroundColor(palette::FG),
        Print("Launching AWS SSO login…"),
        ResetColor,
        Print("\n")
    )
    .ok();
    stderr.flush().ok();
}

pub fn inactive_shell_integration_guidance() {
    match shell_integration::integration_is_installed_for_current_shell() {
        Ok(true) => match shell_integration::integration_script_path() {
            Ok(path) => eprintln!(
                "Restart the shell or run: source {}",
                shell::quote(&path.display().to_string())
            ),
            Err(_) => eprintln!("Restart the shell or source the awsp shell integration."),
        },
        _ => eprintln!("Run awsp setup zsh or awsp setup bash once, then restart the shell."),
    }
}

fn status_color(status: &CacheStatus) -> crossterm::style::Color {
    match status.expires_in_seconds() {
        Some(seconds) if seconds < 0 => palette::RED,
        Some(seconds) if seconds < 3600 => palette::YELLOW,
        Some(_) => palette::GREEN,
        None => palette::MUTED,
    }
}

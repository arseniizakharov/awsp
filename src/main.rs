mod aws;
mod aws_config;
mod cache;
mod fzf;
mod onboarding;
mod shell;
mod state;

use anyhow::{bail, Context, Result};
use aws_config::{AwsConfig, SsoProfile};
use cache::LoginStatus;
use clap::{Parser, Subcommand};
use shell::ShellKind;
use std::collections::BTreeSet;
use std::env;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};

#[derive(Debug, Parser)]
#[command(
    name = "awsp",
    version,
    about = "Switch AWS SSO profiles across shell sessions."
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Print zsh/bash shell integration.
    Init {
        /// Shell to initialize. Autodetects from SHELL when omitted.
        shell: Option<ShellKind>,
    },
    /// Generate a new awsp shell-session id.
    NewSessionId,
    /// Restore the saved profile for the current AWSP_SESSION_ID.
    Restore {
        /// Print shell code instead of human output.
        #[arg(long)]
        shell: bool,
    },
    /// List complete AWS SSO profiles.
    List,
    /// Select and activate an AWS SSO profile.
    Use {
        /// Exact AWS profile name. Omit to choose with fzf.
        profile: Option<String>,
    },
    /// Log in to an AWS SSO profile.
    Login {
        /// Exact AWS profile name. Omit to choose with fzf.
        profile: Option<String>,
    },
    /// Log in to a named modern sso-session.
    LoginSession {
        /// Name from an [sso-session name] section.
        session: String,
    },
    /// Clear the active AWS profile from this shell session.
    Off,
    /// Show the current local awsp/AWS profile state.
    Current,
    /// Verify the active identity through AWS STS.
    Whoami {
        /// Exact AWS profile name. Defaults to AWS_PROFILE.
        profile: Option<String>,
    },
    /// Show local SSO cache status.
    Status {
        /// Exact AWS profile name. Omit to show all profiles unless --verify is used.
        profile: Option<String>,
        /// Verify through AWS STS.
        #[arg(long)]
        verify: bool,
    },
    /// Diagnose local dependencies and AWS config.
    Doctor,
    /// Internal shell integration entrypoint.
    #[command(name = "__shell", hide = true)]
    Shell {
        #[command(subcommand)]
        command: Option<ShellCommand>,
    },
}

#[derive(Debug, Subcommand)]
enum ShellCommand {
    Use { profile: Option<String> },
    Off,
    Restore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputMode {
    Human,
    Shell,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("awsp: {error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        None => {
            onboarding::maybe_install_for_plain_entrypoint()?;
            activate_profile(None, OutputMode::Human)
        }
        Some(Command::Init { shell }) => {
            let shell = shell
                .or_else(shell::detect_shell)
                .context("could not autodetect shell; pass zsh or bash")?;
            if !fzf::is_available() {
                eprintln!("awsp: warning: fzf is required for interactive profile selection");
            }
            print!("{}", shell::init_script(shell));
            Ok(())
        }
        Some(Command::NewSessionId) => {
            println!("{}", state::new_session_id());
            Ok(())
        }
        Some(Command::Restore { shell }) => restore(if shell {
            OutputMode::Shell
        } else {
            OutputMode::Human
        }),
        Some(Command::List) => list_profiles(),
        Some(Command::Use { profile }) => activate_profile(profile, OutputMode::Human),
        Some(Command::Login { profile }) => login_profile(profile),
        Some(Command::LoginSession { session }) => login_session(&session),
        Some(Command::Off) => turn_off(OutputMode::Human),
        Some(Command::Current) => current(),
        Some(Command::Whoami { profile }) => whoami(profile),
        Some(Command::Status { profile, verify }) => status(profile, verify),
        Some(Command::Doctor) => doctor(),
        Some(Command::Shell { command }) => match command {
            None => activate_profile(None, OutputMode::Shell),
            Some(ShellCommand::Use { profile }) => activate_profile(profile, OutputMode::Shell),
            Some(ShellCommand::Off) => turn_off(OutputMode::Shell),
            Some(ShellCommand::Restore) => restore(OutputMode::Shell),
        },
    }
}

fn activate_profile(profile_name: Option<String>, mode: OutputMode) -> Result<()> {
    let config = AwsConfig::load_from_env()?;
    let current = active_profile_name();
    let selected_name = match profile_name {
        Some(profile_name) => profile_name,
        None => select_profile(&config, current.as_deref())?,
    };

    let profile = config.require_profile(&selected_name)?.clone();
    let status = cache::status_for_profile(&profile);

    if status != LoginStatus::Valid {
        if should_login(&profile, status)? {
            let aws_output = match mode {
                OutputMode::Human => aws::AwsOutput::Inherit,
                OutputMode::Shell => aws::AwsOutput::UserTerminal,
            };
            aws::login_profile(&profile.name, aws_output)?;
        } else if status == LoginStatus::Expired {
            bail!("login declined; profile {} was not activated", profile.name);
        }
    }

    let session_id = ensure_session_id();
    state::set_session_profile(&session_id, &profile.name)?;

    match mode {
        OutputMode::Shell => {
            println!(
                "{}",
                shell::activation_code(&profile.name, Some(&session_id))
            );
        }
        OutputMode::Human => {
            eprintln!("Selected {}.", profile.name);
            eprintln!(
                "Shell integration is not active in this process, so AWS_PROFILE was not exported here."
            );
            eprintln!("Run eval \"$(awsp init zsh)\" or eval \"$(awsp init bash)\" in this shell, or restart after installing the rc hook.");
        }
    }

    Ok(())
}

fn login_profile(profile_name: Option<String>) -> Result<()> {
    let config = AwsConfig::load_from_env()?;
    let current = active_profile_name();
    let selected_name = match profile_name {
        Some(profile_name) => profile_name,
        None => select_profile(&config, current.as_deref())?,
    };
    let profile = config.require_profile(&selected_name)?;
    aws::login_profile(&profile.name, aws::AwsOutput::Inherit)
}

fn login_session(session: &str) -> Result<()> {
    let config = AwsConfig::load_from_env()?;
    let _ = config.require_session(session)?;
    aws::login_session(session)
}

fn select_profile(config: &AwsConfig, current: Option<&str>) -> Result<String> {
    let statuses = config
        .profiles
        .iter()
        .map(cache::status_for_profile)
        .collect::<Vec<_>>();
    fzf::select_profile(&config.profiles, &statuses, current)
}

fn should_login(profile: &SsoProfile, status: LoginStatus) -> Result<bool> {
    let question = format!(
        "SSO session for {} is {status}. Log in now? [Y/n] ",
        profile.name
    );
    prompt_yes_no(&question, true)
}

fn restore(mode: OutputMode) -> Result<()> {
    let Some(session_id) = state::current_session_id() else {
        if mode == OutputMode::Human {
            println!("No AWSP_SESSION_ID is set.");
        }
        return Ok(());
    };

    let Some(profile) = state::get_session_profile(&session_id)? else {
        if mode == OutputMode::Human {
            println!("No saved AWS profile for this AWSP_SESSION_ID.");
        }
        return Ok(());
    };

    match mode {
        OutputMode::Shell => println!("{}", shell::activation_code(&profile, None)),
        OutputMode::Human => println!("{profile}"),
    }

    Ok(())
}

fn turn_off(mode: OutputMode) -> Result<()> {
    let session_id = ensure_session_id();
    state::clear_session_profile(&session_id)?;

    match mode {
        OutputMode::Shell => println!("{}", shell::off_code(Some(&session_id))),
        OutputMode::Human => {
            eprintln!("Cleared active AWS profile for this awsp session.");
            eprintln!(
                "Shell integration is not active in this process, so AWS_PROFILE was not unset here."
            );
        }
    }

    Ok(())
}

fn list_profiles() -> Result<()> {
    let config = AwsConfig::load_from_env()?;
    let current = active_profile_name();
    print_profile_table(&config, current.as_deref());
    Ok(())
}

fn current() -> Result<()> {
    let env_profile = env::var("AWS_PROFILE")
        .ok()
        .filter(|value| !value.trim().is_empty());
    let session_id = state::current_session_id();
    let state_profile = match session_id.as_deref() {
        Some(session_id) => state::get_session_profile(session_id)?,
        None => None,
    };

    println!("AWS_PROFILE={}", env_profile.as_deref().unwrap_or("unset"));
    println!(
        "AWSP_SESSION_ID={}",
        session_id.as_deref().unwrap_or("unset")
    );
    println!(
        "state_profile={}",
        state_profile.as_deref().unwrap_or("unset")
    );
    println!(
        "AWS_SDK_LOAD_CONFIG={}",
        env::var("AWS_SDK_LOAD_CONFIG").unwrap_or_else(|_| "unset".to_string())
    );

    Ok(())
}

fn whoami(profile: Option<String>) -> Result<()> {
    let profile = profile.or_else(active_profile_name);
    aws::whoami(profile.as_deref())
}

fn status(profile_name: Option<String>, verify: bool) -> Result<()> {
    let config = AwsConfig::load_from_env()?;

    if verify {
        let profile_name = profile_name
            .or_else(active_profile_name)
            .context("--verify requires a profile argument or active AWS_PROFILE")?;
        let profile = config.require_profile(&profile_name)?;
        let identity = aws::verify(&profile.name)?;
        println!("{} verified", profile.name);
        if !identity.is_empty() {
            println!("{identity}");
        }
        return Ok(());
    }

    if let Some(profile_name) = profile_name {
        let profile = config.require_profile(&profile_name)?;
        println!("{}\t{}", profile.name, cache::status_for_profile(profile));
        return Ok(());
    }

    let current = active_profile_name();
    print_profile_table(&config, current.as_deref());
    Ok(())
}

fn doctor() -> Result<()> {
    println!("awsp doctor");
    println!(
        "aws cli: {}",
        if aws::is_available() { "ok" } else { "missing" }
    );
    println!(
        "fzf: {}",
        if fzf::is_available() { "ok" } else { "missing" }
    );
    println!("state: {}", state::state_path()?.display());

    match AwsConfig::load_from_env() {
        Ok(config) => {
            println!("aws config: {}", config.path.display());
            println!("complete SSO profiles: {}", config.profiles.len());
            println!("sso sessions: {}", config.sso_sessions.len());
            println!(
                "modern SSO profiles: {}",
                config
                    .profiles
                    .iter()
                    .filter(|profile| profile.sso_session.is_some())
                    .count()
            );
            println!(
                "accounts: {}",
                config
                    .profiles
                    .iter()
                    .map(|profile| profile.account_id.as_str())
                    .collect::<BTreeSet<_>>()
                    .len()
            );
            if config.diagnostics.is_empty() {
                println!("config diagnostics: none");
            } else {
                println!("config diagnostics:");
                for diagnostic in config.diagnostics {
                    println!("  {}: {}", diagnostic.subject, diagnostic.message);
                }
            }
        }
        Err(error) => {
            println!("aws config: error: {error:#}");
        }
    }

    Ok(())
}

fn print_profile_table(config: &AwsConfig, current: Option<&str>) {
    println!(
        "{:<2} {:<30} {:<24} {:<18} {:<8}",
        "", "profile", "role", "region", "status"
    );
    for profile in &config.profiles {
        let marker = if Some(profile.name.as_str()) == current {
            "*"
        } else {
            ""
        };
        println!(
            "{:<2} {:<30} {:<24} {:<18} {:<8}",
            marker,
            profile.name,
            profile.role_name,
            profile.region.label(),
            cache::status_for_profile(profile)
        );
    }
    println!("* current profile; region ending in * is inherited from [default]");
}

fn active_profile_name() -> Option<String> {
    env::var("AWS_PROFILE")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn ensure_session_id() -> String {
    state::current_session_id().unwrap_or_else(state::new_session_id)
}

fn prompt_yes_no(question: &str, default_yes: bool) -> Result<bool> {
    if let Ok(tty) = OpenOptions::new().read(true).write(true).open("/dev/tty") {
        return prompt_yes_no_on_tty(tty, question, default_yes);
    }

    eprint!("{question}");
    std::io::stderr().flush().ok();
    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .context("failed to read prompt response")?;
    Ok(parse_yes_no(&input, default_yes))
}

fn prompt_yes_no_on_tty(mut tty: std::fs::File, question: &str, default_yes: bool) -> Result<bool> {
    write!(tty, "{question}").context("failed to write prompt")?;
    tty.flush().context("failed to flush prompt")?;
    let mut reader = BufReader::new(tty.try_clone().context("failed to clone tty")?);
    let mut input = String::new();
    reader
        .read_line(&mut input)
        .context("failed to read prompt response")?;
    Ok(parse_yes_no(&input, default_yes))
}

fn parse_yes_no(input: &str, default_yes: bool) -> bool {
    let value = input.trim().to_ascii_lowercase();
    if value.is_empty() {
        return default_yes;
    }
    matches!(value.as_str(), "y" | "yes")
}

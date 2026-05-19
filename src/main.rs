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
    about = "Switch AWS SSO profiles across shell sessions.",
    after_help = "Quick start:\n  awsp                         Pick an SSO profile and activate it\n  awsp setup zsh               Install shell integration once\n  awsp status                  Show local SSO cache status\n  awsp profiles                List complete SSO profiles"
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
    /// Install static zsh/bash shell integration.
    Setup {
        /// Shell to set up. Autodetects from SHELL when omitted.
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
    #[command(visible_alias = "profiles")]
    List,
    /// Select and activate an AWS SSO profile.
    #[command(visible_alias = "activate")]
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
    #[command(visible_alias = "clear")]
    Off,
    /// Run a command with a specific AWS profile.
    Exec {
        /// Exact AWS profile name.
        profile: String,
        /// Command and arguments to execute.
        #[arg(last = true, required = true)]
        command: Vec<String>,
    },
    /// Clear AWS CLI SSO sessions.
    Logout {
        /// Required because AWS CLI SSO logout clears every cached SSO session.
        #[arg(long)]
        all: bool,
    },
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
    #[command(alias = "activate")]
    Use {
        profile: Option<String>,
    },
    #[command(alias = "clear")]
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
            require_shell_function_for_activation("awsp")?;
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
        Some(Command::Setup { shell }) => setup_shell(shell),
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
        Some(Command::Use { profile }) => {
            require_shell_function_for_activation("awsp use")?;
            activate_profile(profile, OutputMode::Human)
        }
        Some(Command::Login { profile }) => login_profile(profile),
        Some(Command::LoginSession { session }) => login_session(&session),
        Some(Command::Off) => {
            require_shell_function_for_activation("awsp off")?;
            turn_off(OutputMode::Human)
        }
        Some(Command::Exec { profile, command }) => exec_profile(&profile, command),
        Some(Command::Logout { all }) => logout(all),
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
            print_inactive_shell_integration_guidance();
        }
    }

    Ok(())
}

fn setup_shell(shell: Option<ShellKind>) -> Result<()> {
    let shell = shell
        .or_else(shell::detect_shell)
        .context("could not autodetect shell; pass zsh or bash")?;
    let rc_paths = onboarding::install_shell_integration(shell)?;
    let script_path = onboarding::integration_script_path()?;

    eprintln!("Installed awsp shell integration for {}.", shell.as_str());
    eprintln!("New shells will source {}.", script_path.display());
    eprintln!("Updated shell startup files:");
    for path in rc_paths {
        eprintln!("  {}", path.display());
    }
    eprintln!(
        "To enable it in the current shell, run: source {}",
        shell::quote(&script_path.display().to_string())
    );
    eprintln!("Until then, `awsp` resolves to the binary and cannot export AWS_PROFILE.");
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

fn exec_profile(profile_name: &str, command: Vec<String>) -> Result<()> {
    if command.is_empty() {
        bail!("no command specified");
    }

    let config = AwsConfig::load_from_env()?;
    let profile = config.require_profile(profile_name)?.clone();
    let status = cache::status_for_profile(&profile);

    if status != LoginStatus::Valid {
        if should_login(&profile, status)? {
            aws::login_profile(&profile.name, aws::AwsOutput::Inherit)?;
        } else if status == LoginStatus::Expired {
            bail!("login declined; command was not run");
        }
    }

    let status = std::process::Command::new(&command[0])
        .args(&command[1..])
        .env("AWS_PROFILE", &profile.name)
        .env("AWS_SDK_LOAD_CONFIG", "1")
        .env_remove("AWS_ACCESS_KEY_ID")
        .env_remove("AWS_SECRET_ACCESS_KEY")
        .env_remove("AWS_SESSION_TOKEN")
        .env_remove("AWS_SESSION_EXPIRATION")
        .status()
        .with_context(|| format!("failed to execute {}", command[0]))?;

    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }

    Ok(())
}

fn logout(all: bool) -> Result<()> {
    if !all {
        bail!("AWS CLI SSO logout clears every cached SSO session; rerun with awsp logout --all");
    }

    aws::logout()?;
    state::clear_all()?;
    eprintln!("Cleared all AWS CLI SSO sessions and awsp state.");
    Ok(())
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
            print_inactive_shell_integration_guidance();
        }
    }

    Ok(())
}

fn require_shell_function_for_activation(command: &str) -> Result<()> {
    let script_path = onboarding::integration_script_path().ok();
    let script_command = script_path
        .as_ref()
        .map(|path| format!("source {}", shell::quote(&path.display().to_string())))
        .unwrap_or_else(|| "source ~/.config/awsp/shell/awsp.sh".to_string());

    if matches!(
        onboarding::integration_is_installed_for_current_shell(),
        Ok(true)
    ) {
        bail!(
            "shell integration is installed, but this terminal has not loaded the awsp function.\n\
             `{command}` must run through the shell function so it can export AWS_PROFILE in your current shell.\n\n\
             Run:\n  {script_command}\n\n\
             Then verify:\n  type awsp\n\n\
             Expected: awsp is a shell function"
        );
    }

    let setup_command = shell::detect_shell()
        .map(|shell| format!("awsp setup {}", shell.as_str()))
        .unwrap_or_else(|| "awsp setup zsh".to_string());

    bail!(
        "shell integration is not active.\n\
         `{command}` must run through the awsp shell function so it can export AWS_PROFILE in your current shell.\n\n\
         Run:\n  {setup_command}\n  {script_command}\n\n\
         Then verify:\n  type awsp\n\n\
         Expected: awsp is a shell function"
    );
}

fn print_inactive_shell_integration_guidance() {
    match onboarding::integration_is_installed_for_current_shell() {
        Ok(true) => match onboarding::integration_script_path() {
            Ok(path) => eprintln!(
                "Restart the shell or run: source {}",
                shell::quote(&path.display().to_string())
            ),
            Err(_) => eprintln!("Restart the shell or source the awsp shell integration."),
        },
        _ => eprintln!("Run awsp setup zsh or awsp setup bash once, then restart the shell."),
    }
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
    if aws::is_available() {
        println!("aws cli: ok");
    } else {
        println!("aws cli: missing");
        println!("  {}", aws::missing_cli_message().replace('\n', "\n  "));
    }
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

mod activation;
mod aws;
mod aws_config;
mod cache;
mod onboarding;
mod output;
mod palette;
mod picker;
mod picker_model;
mod prompt;
mod shell;
mod shell_integration;
mod state;

use anyhow::{bail, Context, Result};
use aws_config::SsoInventory;
use clap::{Parser, Subcommand};
use output::OutputMode;
use picker::PickerView;
use shell::ShellKind;
use std::env;

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
        /// Exact AWS profile name. Omit to choose interactively.
        profile: Option<String>,
    },
    /// Log in to an AWS SSO profile.
    Login {
        /// Exact AWS profile name. Omit to choose interactively.
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
        /// Emit status as one JSON object on stdout.
        #[arg(long)]
        json: bool,
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
    Table,
    Query {
        fragment: String,
    },
    #[command(alias = "activate")]
    Use {
        profile: Option<String>,
    },
    #[command(alias = "clear")]
    Off,
    Restore,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("awsp: {error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    if let Some(result) = run_raw_entrypoint()? {
        return result;
    }

    let cli = Cli::parse();

    match cli.command {
        None => {
            onboarding::maybe_install_for_plain_entrypoint()?;
            require_shell_function_for_activation("awsp")?;
            activation::activate_profile(None, OutputMode::Human)
        }
        Some(Command::Init { shell }) => {
            let shell = shell
                .or_else(shell::detect_shell)
                .context("could not autodetect shell; pass zsh or bash")?;
            print!("{}", shell::init_script(shell));
            Ok(())
        }
        Some(Command::Setup { shell }) => setup_shell(shell),
        Some(Command::NewSessionId) => {
            println!("{}", state::new_session_id());
            Ok(())
        }
        Some(Command::Restore { shell }) => activation::restore(if shell {
            OutputMode::Shell
        } else {
            OutputMode::Human
        }),
        Some(Command::List) => list_profiles(),
        Some(Command::Use { profile }) => {
            require_shell_function_for_activation("awsp use")?;
            activation::activate_profile(profile, OutputMode::Human)
        }
        Some(Command::Login { profile }) => activation::login_profile(profile),
        Some(Command::LoginSession { session }) => login_session(&session),
        Some(Command::Off) => {
            require_shell_function_for_activation("awsp off")?;
            activation::turn_off(OutputMode::Human)
        }
        Some(Command::Exec { profile, command }) => activation::exec_profile(&profile, command),
        Some(Command::Logout { all }) => logout(all),
        Some(Command::Current) => current(),
        Some(Command::Whoami { profile }) => whoami(profile),
        Some(Command::Status {
            profile,
            verify,
            json,
        }) => status(profile, verify, json),
        Some(Command::Doctor) => doctor(),
        Some(Command::Shell { command }) => match command {
            None => activation::activate_profile(None, OutputMode::Shell),
            Some(ShellCommand::Table) => {
                activation::activate_with_picker(OutputMode::Shell, PickerView::Table)
            }
            Some(ShellCommand::Query { fragment }) => {
                activation::activate_query(&fragment, OutputMode::Shell)
            }
            Some(ShellCommand::Use { profile }) => {
                activation::activate_profile(profile, OutputMode::Shell)
            }
            Some(ShellCommand::Off) => activation::turn_off(OutputMode::Shell),
            Some(ShellCommand::Restore) => activation::restore(OutputMode::Shell),
        },
    }
}

fn run_raw_entrypoint() -> Result<Option<Result<()>>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let Some(first) = args.first() else {
        return Ok(None);
    };

    if first == "--table" {
        return Ok(Some(activation::activate_with_picker(
            OutputMode::Human,
            PickerView::Table,
        )));
    }

    if first == "--emit-env" {
        let result = match args.get(1) {
            Some(fragment) => activation::activate_query(fragment, OutputMode::Shell),
            None => activation::activate_with_picker(OutputMode::Shell, PickerView::Numbered),
        };
        return Ok(Some(result));
    }

    if args.len() == 1 && is_direct_fragment(first) {
        return Ok(Some(activation::activate_query(first, OutputMode::Human)));
    }

    Ok(None)
}

fn is_direct_fragment(value: &str) -> bool {
    !value.starts_with('-')
        && !matches!(
            value,
            "init"
                | "setup"
                | "new-session-id"
                | "restore"
                | "list"
                | "profiles"
                | "use"
                | "activate"
                | "login"
                | "login-session"
                | "off"
                | "clear"
                | "exec"
                | "logout"
                | "current"
                | "whoami"
                | "status"
                | "doctor"
                | "__shell"
        )
}

fn setup_shell(shell: Option<ShellKind>) -> Result<()> {
    let shell = shell
        .or_else(shell::detect_shell)
        .context("could not autodetect shell; pass zsh or bash")?;
    let plan = shell_integration::ShellIntegrationPlan::for_shell(shell)?;
    let applied = plan.apply()?;

    eprintln!(
        "Installed awsp shell integration for {}.",
        plan.shell().as_str()
    );
    eprintln!("New shells will source {}.", plan.script_path().display());
    eprintln!("Updated shell startup files:");
    for path in applied.rc_paths {
        eprintln!("  {}", path.display());
    }
    eprintln!(
        "To enable it in the current shell, run: source {}",
        shell::quote(&applied.script_path.display().to_string())
    );
    eprintln!("Until then, `awsp` resolves to the binary and cannot export AWS_PROFILE.");
    Ok(())
}

fn login_session(session: &str) -> Result<()> {
    let inventory = SsoInventory::load_from_env()?;
    let _ = inventory.require_session(session)?;
    aws::login_session(session)
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

fn require_shell_function_for_activation(command: &str) -> Result<()> {
    let script_path = shell_integration::integration_script_path().ok();
    let script_command = script_path
        .as_ref()
        .map(|path| format!("source {}", shell::quote(&path.display().to_string())))
        .unwrap_or_else(|| "source ~/.config/awsp/shell/awsp.sh".to_string());

    if matches!(
        shell_integration::integration_is_installed_for_current_shell(),
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

fn list_profiles() -> Result<()> {
    let inventory = SsoInventory::load_from_env()?;
    if inventory.profiles().is_empty() {
        picker::bail_no_profiles();
    }
    let current = activation::active_profile_name();
    let statuses = activation::statuses_for_profiles(&inventory);
    output::profile_table(&inventory, current.as_deref(), &statuses);
    Ok(())
}

fn current() -> Result<()> {
    let env_profile = activation::active_profile_name();
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
    let profile = profile.or_else(activation::active_profile_name);
    aws::whoami(profile.as_deref())
}

fn status(profile_name: Option<String>, verify: bool, json: bool) -> Result<()> {
    let inventory = SsoInventory::load_from_env()?;
    if inventory.profiles().is_empty() {
        picker::bail_no_profiles();
    }

    if verify {
        let profile_name = profile_name
            .or_else(activation::active_profile_name)
            .context("--verify requires a profile argument or active AWS_PROFILE")?;
        let profile = inventory.require_profile(&profile_name)?;
        let identity = aws::verify(&profile.name)?;
        println!("{} verified", profile.name);
        if !identity.is_empty() {
            println!("{identity}");
        }
        return Ok(());
    }

    if let Some(profile_name) = profile_name {
        let profile = inventory.require_profile(&profile_name)?;
        let status = cache::cache_status_for_profile(profile);
        if json {
            output::status_json(profile, &status);
        } else {
            output::status(profile, &status);
        }
        return Ok(());
    }

    if let Some((profile, profile_status)) = activation::active_profile(&inventory)? {
        if json {
            output::status_json(profile, &profile_status);
        } else {
            output::status(profile, &profile_status);
        }
        return Ok(());
    }

    let current = activation::active_profile_name();
    let statuses = activation::statuses_for_profiles(&inventory);
    output::profile_table(&inventory, current.as_deref(), &statuses);
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
    println!("picker: builtin");
    println!("state: {}", state::state_path()?.display());

    match SsoInventory::load_from_env() {
        Ok(inventory) => {
            println!("aws config: {}", inventory.path().display());
            println!("complete SSO profiles: {}", inventory.profiles().len());
            println!("sso sessions: {}", inventory.sso_session_count());
            println!("modern SSO profiles: {}", inventory.modern_profile_count());
            println!("accounts: {}", inventory.account_count());
            if inventory.diagnostics().is_empty() {
                println!("config diagnostics: none");
            } else {
                println!("config diagnostics:");
                for diagnostic in inventory.diagnostics() {
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

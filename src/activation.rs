use crate::aws;
use crate::aws_config::{SsoInventory, SsoProfile};
use crate::cache::{self, CacheStatus, LoginStatus};
use crate::elevation::{self, ElevationOptions, ElevationOutcome};
use crate::output::{self, OutputMode};
use crate::picker::{self, PickerSelection, PickerView};
use crate::prompt;
use crate::shell;
use crate::state;
use anyhow::{bail, Context, Result};
use crossterm::event::{self, Event, KeyCode};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use std::env;

pub fn activate_profile(profile_name: Option<String>, mode: OutputMode) -> Result<()> {
    activate_profile_with_options(profile_name, mode, ElevationOptions::default())
}

pub fn activate_profile_with_options(
    profile_name: Option<String>,
    mode: OutputMode,
    elevation_options: ElevationOptions,
) -> Result<()> {
    match profile_name {
        Some(profile_name) => activate_exact_with_options(&profile_name, mode, elevation_options),
        None => activate_with_picker_with_options(mode, PickerView::Numbered, elevation_options),
    }
}

pub fn activate_with_picker(mode: OutputMode, view: PickerView) -> Result<()> {
    activate_with_picker_with_options(mode, view, ElevationOptions::default())
}

fn activate_with_picker_with_options(
    mode: OutputMode,
    view: PickerView,
    elevation_options: ElevationOptions,
) -> Result<()> {
    let inventory = SsoInventory::load_from_env()?;
    let current = active_profile_name_for_inventory(&inventory);
    let selection = select_profile(&inventory, current.as_deref(), view)?;
    let force_login = matches!(selection, PickerSelection::Relogin(_));
    let selected_name = match selection {
        PickerSelection::Profile(profile) | PickerSelection::Relogin(profile) => profile,
    };
    switch_to_profile(
        &inventory,
        &selected_name,
        mode,
        force_login,
        &elevation_options,
    )
}

pub fn activate_query(fragment: &str, mode: OutputMode) -> Result<()> {
    let inventory = SsoInventory::load_from_env()?;
    let selected_name = resolve_query_interactively(&inventory, fragment)?;
    switch_to_profile(
        &inventory,
        &selected_name,
        mode,
        false,
        &ElevationOptions::default(),
    )
}

fn activate_exact_with_options(
    profile_name: &str,
    mode: OutputMode,
    elevation_options: ElevationOptions,
) -> Result<()> {
    let inventory = SsoInventory::load_from_env()?;
    switch_to_profile(&inventory, profile_name, mode, false, &elevation_options)
}

pub fn request_elevation(profile_name: &str, options: ElevationOptions) -> Result<()> {
    let inventory = SsoInventory::load_from_env()?;
    let profile = inventory.require_profile(profile_name)?.clone();
    match elevation::request_access(&profile, &options)? {
        ElevationOutcome::Submitted { id, status } => {
            eprintln!("TEAM request submitted: {id} ({status})");
            Ok(())
        }
        ElevationOutcome::ExistingPending { id, status } => {
            eprintln!("TEAM request already exists: {id} ({status})");
            Ok(())
        }
        ElevationOutcome::NotConfigured => {
            bail!("TEAM request submission is not configured")
        }
        ElevationOutcome::Declined => {
            bail!("TEAM request declined")
        }
    }
}

pub fn login_profile(profile_name: Option<String>) -> Result<()> {
    let inventory = SsoInventory::load_from_env()?;
    let current = active_profile_name_for_inventory(&inventory);
    let selected_name = match profile_name {
        Some(profile_name) => profile_name,
        None => match select_profile(&inventory, current.as_deref(), PickerView::Numbered)? {
            PickerSelection::Profile(profile) | PickerSelection::Relogin(profile) => profile,
        },
    };
    let profile = inventory.require_profile(&selected_name)?;
    aws::login_profile(&profile.name, aws::AwsOutput::Inherit)
}

pub fn exec_profile(profile_name: &str, command: Vec<String>) -> Result<()> {
    if command.is_empty() {
        bail!("no command specified");
    }

    let inventory = SsoInventory::load_from_env()?;
    let profile = inventory.require_profile(profile_name)?.clone();

    if !ensure_login_for_exec(&profile)? {
        bail!("login declined; command was not run");
    }
    if !ensure_assignment_or_request(&profile, OutputMode::Human, &ElevationOptions::default())? {
        bail!("profile access is not active; command was not run");
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

pub fn restore(mode: OutputMode) -> Result<()> {
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
        OutputMode::Shell => output::shell_code(&shell::activation_code(&profile, None)),
        OutputMode::Human => println!("{profile}"),
    }

    Ok(())
}

pub fn turn_off(mode: OutputMode) -> Result<()> {
    let session_id = ensure_session_id();
    state::clear_session_profile(&session_id)?;

    match mode {
        OutputMode::Shell => output::shell_code(&shell::off_code(Some(&session_id))),
        OutputMode::Human => output::inactive_off(),
    }

    Ok(())
}

pub fn active_profile_name() -> Option<String> {
    env::var("AWS_PROFILE")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub fn active_profile_name_for_inventory(inventory: &SsoInventory) -> Option<String> {
    active_profile_name().or_else(|| inventory.default_profile_name().map(str::to_string))
}

pub fn statuses_for_profiles(inventory: &SsoInventory) -> Vec<CacheStatus> {
    inventory
        .profiles()
        .iter()
        .map(cache::cache_status_for_profile)
        .collect()
}

pub fn active_profile(inventory: &SsoInventory) -> Result<Option<(&SsoProfile, CacheStatus)>> {
    let Some(name) = active_profile_name_for_inventory(inventory) else {
        return Ok(None);
    };
    let Some(profile) = inventory.find_profile(&name) else {
        return Ok(None);
    };
    Ok(Some((profile, cache::cache_status_for_profile(profile))))
}

fn switch_to_profile(
    inventory: &SsoInventory,
    profile_name: &str,
    mode: OutputMode,
    force_login: bool,
    elevation_options: &ElevationOptions,
) -> Result<()> {
    let old_profile = active_profile_name_for_inventory(inventory);
    let profile = inventory.require_profile(profile_name)?.clone();
    ensure_login_for_activation(&profile, mode, force_login)?;
    if !ensure_assignment_or_request(&profile, mode, elevation_options)? {
        return Ok(());
    }

    let session_id = ensure_session_id();
    state::set_session_profile(&session_id, &profile.name)?;
    let status = cache::cache_status_for_profile(&profile);

    match mode {
        OutputMode::Shell => {
            output::switch_success(old_profile.as_deref(), &profile, &status);
            output::shell_code(&shell::activation_code_for_profile(
                &profile,
                Some(&session_id),
            ));
        }
        OutputMode::Human => output::inactive_activation(&profile.name),
    }

    Ok(())
}

fn ensure_login_for_activation(
    profile: &SsoProfile,
    mode: OutputMode,
    force_login: bool,
) -> Result<()> {
    let status = cache::cache_status_for_profile(profile);
    if status.state == LoginStatus::Valid && !force_login {
        return Ok(());
    }

    if !force_login && !should_login(profile, status.state)? {
        if status.state == LoginStatus::Expired {
            bail!("login declined; profile {} was not activated", profile.name);
        }
        return Ok(());
    }

    output::device_flow_start(profile, &status);
    let aws_output = match mode {
        OutputMode::Human => aws::AwsOutput::Inherit,
        OutputMode::Shell => aws::AwsOutput::UserTerminal,
    };
    aws::login_profile(&profile.name, aws_output)
}

fn ensure_assignment_or_request(
    profile: &SsoProfile,
    mode: OutputMode,
    elevation_options: &ElevationOptions,
) -> Result<bool> {
    let Some(token) = cache::access_token_for_profile(profile) else {
        return Ok(true);
    };

    match aws::sso_role_access(profile, &token.token)? {
        aws::SsoRoleAccess::Available => Ok(true),
        aws::SsoRoleAccess::LoginExpired { .. } => {
            output::device_flow_start(profile, &cache::cache_status_for_profile(profile));
            let aws_output = match mode {
                OutputMode::Human => aws::AwsOutput::Inherit,
                OutputMode::Shell => aws::AwsOutput::UserTerminal,
            };
            aws::login_profile(&profile.name, aws_output)?;
            let Some(token) = cache::access_token_for_profile(profile) else {
                return Ok(true);
            };
            match aws::sso_role_access(profile, &token.token)? {
                aws::SsoRoleAccess::Available => Ok(true),
                aws::SsoRoleAccess::AssignmentMissing { message } => {
                    request_missing_assignment(profile, elevation_options, Some(&message))
                }
                aws::SsoRoleAccess::LoginExpired { message }
                | aws::SsoRoleAccess::UnknownFailure { message } => {
                    bail!(
                        "could not verify SSO role access for {}: {message}",
                        profile.name
                    )
                }
            }
        }
        aws::SsoRoleAccess::AssignmentMissing { message } => {
            request_missing_assignment(profile, elevation_options, Some(&message))
        }
        aws::SsoRoleAccess::UnknownFailure { message } => {
            bail!(
                "could not verify SSO role access for {}: {message}",
                profile.name
            )
        }
    }
}

fn request_missing_assignment(
    profile: &SsoProfile,
    elevation_options: &ElevationOptions,
    reason: Option<&str>,
) -> Result<bool> {
    eprintln!(
        "  {} is not currently assigned in IAM Identity Center.",
        profile.name
    );
    if let Some(reason) = reason.filter(|reason| !reason.trim().is_empty()) {
        eprintln!("  AWS SSO said: {}", reason.trim());
    }

    match elevation::request_access(profile, elevation_options)? {
        ElevationOutcome::Submitted { id, status } => {
            eprintln!("  TEAM request submitted: {id} ({status}).");
            eprintln!(
                "  Activate {} again after access becomes active.",
                profile.name
            );
            Ok(false)
        }
        ElevationOutcome::ExistingPending { id, status } => {
            eprintln!("  TEAM request already pending: {id} ({status}).");
            eprintln!(
                "  Activate {} again after access becomes active.",
                profile.name
            );
            Ok(false)
        }
        ElevationOutcome::NotConfigured => {
            bail!("TEAM request submission is not configured")
        }
        ElevationOutcome::Declined => {
            bail!(
                "TEAM request declined; profile {} was not activated",
                profile.name
            )
        }
    }
}

fn ensure_login_for_exec(profile: &SsoProfile) -> Result<bool> {
    let status = cache::cache_status_for_profile(profile);
    if status.state == LoginStatus::Valid {
        return Ok(true);
    }

    if !should_login(profile, status.state)? {
        return Ok(status.state != LoginStatus::Expired);
    }

    output::device_flow_start(profile, &status);
    aws::login_profile(&profile.name, aws::AwsOutput::Inherit)?;
    Ok(true)
}

fn select_profile(
    inventory: &SsoInventory,
    current: Option<&str>,
    view: PickerView,
) -> Result<PickerSelection> {
    let statuses = statuses_for_profiles(inventory);
    picker::select_profile(inventory.profiles(), &statuses, current, view)
}

fn should_login(profile: &SsoProfile, status: LoginStatus) -> Result<bool> {
    let question = format!(
        "SSO session for {} is {status}. Log in now? [Y/n] ",
        profile.name
    );
    prompt::yes_no(&question, true)
}

fn resolve_query_interactively(inventory: &SsoInventory, fragment: &str) -> Result<String> {
    if inventory.profiles().is_empty() {
        picker::bail_no_profiles();
    }

    let mut query = fragment.to_string();
    loop {
        match resolve_query(inventory, &query) {
            QueryResolution::One(profile) => return Ok(profile.name.clone()),
            QueryResolution::Many(matches) => {
                output::ambiguous(&query, &matches);
                query = read_ambiguous_choice(&query, &matches)?;
            }
            QueryResolution::None => {
                let suggestions = suggestions(inventory, &query);
                output::did_you_mean(&query, &suggestions);
                std::process::exit(1);
            }
        }
    }
}

fn read_ambiguous_choice(query: &str, matches: &[SsoProfile]) -> Result<String> {
    enable_raw_mode().context("failed to enable terminal raw mode")?;
    let result = (|| loop {
        let Event::Key(key) = event::read().context("failed to read terminal input")? else {
            continue;
        };
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => std::process::exit(130),
            KeyCode::Char(value @ '1'..='9') => {
                let index = value as usize - '1' as usize;
                if let Some(profile) = matches.get(index) {
                    return Ok(profile.name.clone());
                }
            }
            KeyCode::Enter => {
                if let Some(profile) = matches.first() {
                    return Ok(profile.name.clone());
                }
            }
            KeyCode::Char(value) if !value.is_control() => {
                return Ok(format!("{query}{value}"));
            }
            _ => {}
        }
    })();
    disable_raw_mode().ok();
    result
}

enum QueryResolution {
    One(SsoProfile),
    Many(Vec<SsoProfile>),
    None,
}

fn resolve_query(inventory: &SsoInventory, query: &str) -> QueryResolution {
    if let Some(profile) = inventory.find_profile(query) {
        return QueryResolution::One(profile.clone());
    }

    let needle = query.to_ascii_lowercase();
    let matches = inventory
        .profiles()
        .iter()
        .filter(|profile| profile.name.to_ascii_lowercase().contains(&needle))
        .cloned()
        .collect::<Vec<_>>();

    match matches.len() {
        0 => QueryResolution::None,
        1 => QueryResolution::One(matches[0].clone()),
        _ => QueryResolution::Many(matches),
    }
}

fn suggestions(inventory: &SsoInventory, query: &str) -> Vec<SsoProfile> {
    let threshold = usize::max(2, query.len() / 3);
    let mut suggestions = inventory
        .profiles()
        .iter()
        .map(|profile| (levenshtein(query, &profile.name), profile.clone()))
        .filter(|(distance, _)| *distance <= threshold)
        .collect::<Vec<_>>();
    suggestions.sort_by(|left, right| {
        left.0
            .cmp(&right.0)
            .then_with(|| left.1.name.cmp(&right.1.name))
    });
    suggestions
        .into_iter()
        .take(3)
        .map(|(_, profile)| profile)
        .collect()
}

fn levenshtein(left: &str, right: &str) -> usize {
    let right_chars = right.chars().collect::<Vec<_>>();
    let mut costs = (0..=right_chars.len()).collect::<Vec<_>>();

    for (left_index, left_char) in left.chars().enumerate() {
        let mut previous = costs[0];
        costs[0] = left_index + 1;
        for (right_index, right_char) in right_chars.iter().enumerate() {
            let old = costs[right_index + 1];
            let substitution = previous + usize::from(left_char != *right_char);
            let insertion = costs[right_index] + 1;
            let deletion = old + 1;
            costs[right_index + 1] = substitution.min(insertion).min(deletion);
            previous = old;
        }
    }

    *costs.last().unwrap_or(&0)
}

fn ensure_session_id() -> String {
    state::current_session_id().unwrap_or_else(state::new_session_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws_config::{RegionDisplay, SsoInventory};
    use std::path::PathBuf;

    fn inventory_with_profiles(profiles: Vec<SsoProfile>) -> SsoInventory {
        SsoInventory::from_parts_for_test(PathBuf::from("/tmp/config"), profiles)
    }

    fn profile(name: &str) -> SsoProfile {
        SsoProfile {
            name: name.to_string(),
            sso_session: Some("corp".to_string()),
            sso_start_url: "https://example.awsapps.com/start".to_string(),
            sso_region: "us-east-1".to_string(),
            account_id: "123456789012".to_string(),
            role_name: "Admin".to_string(),
            region: RegionDisplay::Unset,
        }
    }

    #[test]
    fn computes_statuses_for_every_inventory_profile() {
        let inventory = inventory_with_profiles(vec![profile("dev"), profile("prod")]);

        assert_eq!(statuses_for_profiles(&inventory).len(), 2);
    }

    #[test]
    fn resolves_exact_profile_before_fragment_matching() {
        let inventory = inventory_with_profiles(vec![profile("prod"), profile("prod-admin")]);

        match resolve_query(&inventory, "prod") {
            QueryResolution::One(profile) => assert_eq!(profile.name, "prod"),
            _ => panic!("expected exact match"),
        }
    }

    #[test]
    fn suggests_near_profile_names() {
        let inventory = inventory_with_profiles(vec![profile("acme-prod-readonly")]);

        let suggestions = suggestions(&inventory, "acme-prod-redonly");
        assert_eq!(suggestions[0].name, "acme-prod-readonly");
    }

    #[test]
    fn computes_levenshtein_distance() {
        assert_eq!(levenshtein("kitten", "sitting"), 3);
    }
}

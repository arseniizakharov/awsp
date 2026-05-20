use anyhow::{Context, Result};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct SsoInventory {
    path: PathBuf,
    profiles: Vec<SsoProfile>,
    sso_sessions: BTreeMap<String, SsoSession>,
    diagnostics: Vec<ConfigDiagnostic>,
}

#[derive(Debug, Clone)]
pub struct SsoProfile {
    pub name: String,
    pub sso_session: Option<String>,
    pub sso_start_url: String,
    pub sso_region: String,
    pub account_id: String,
    pub role_name: String,
    pub region: RegionDisplay,
}

#[derive(Debug, Clone)]
pub struct SsoSession {
    pub start_url: Option<String>,
    pub region: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ConfigDiagnostic {
    pub subject: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegionDisplay {
    Profile(String),
    Default(String),
    Unset,
}

impl RegionDisplay {
    pub fn label(&self) -> String {
        match self {
            Self::Profile(region) => region.clone(),
            Self::Default(region) => format!("{region}*"),
            Self::Unset => "unset".to_string(),
        }
    }

    pub fn export_value(&self) -> Option<&str> {
        match self {
            Self::Profile(region) | Self::Default(region) => Some(region.as_str()),
            Self::Unset => None,
        }
    }
}

type Section = BTreeMap<String, String>;

impl SsoInventory {
    pub fn load_from_env() -> Result<Self> {
        let path = config_path()?;
        Self::load(path)
    }

    pub fn load(path: PathBuf) -> Result<Self> {
        let content = fs::read_to_string(&path)
            .with_context(|| format!("failed to read AWS config at {}", path.display()))?;
        let sections = parse_ini(&content);
        Ok(Self::from_sections(path, sections))
    }

    pub fn find_profile(&self, name: &str) -> Option<&SsoProfile> {
        self.profiles.iter().find(|profile| profile.name == name)
    }

    pub fn require_profile(&self, name: &str) -> Result<&SsoProfile> {
        self.find_profile(name)
            .with_context(|| format!("no complete AWS SSO profile named {name}"))
    }

    pub fn require_session(&self, name: &str) -> Result<&SsoSession> {
        self.sso_sessions
            .get(name)
            .with_context(|| format!("no sso-session named {name}"))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn profiles(&self) -> &[SsoProfile] {
        &self.profiles
    }

    pub fn default_profile_name(&self) -> Option<&str> {
        self.find_profile("default")
            .map(|profile| profile.name.as_str())
    }

    pub fn diagnostics(&self) -> &[ConfigDiagnostic] {
        &self.diagnostics
    }

    pub fn sso_session_count(&self) -> usize {
        self.sso_sessions.len()
    }

    pub fn modern_profile_count(&self) -> usize {
        self.profiles
            .iter()
            .filter(|profile| profile.sso_session.is_some())
            .count()
    }

    pub fn account_count(&self) -> usize {
        self.profiles
            .iter()
            .map(|profile| profile.account_id.as_str())
            .collect::<BTreeSet<_>>()
            .len()
    }

    #[cfg(test)]
    pub fn from_parts_for_test(path: PathBuf, profiles: Vec<SsoProfile>) -> Self {
        Self {
            path,
            profiles,
            sso_sessions: BTreeMap::new(),
            diagnostics: Vec::new(),
        }
    }

    fn from_sections(path: PathBuf, sections: BTreeMap<String, Section>) -> Self {
        let default_region = sections
            .get("default")
            .and_then(|section| get_nonempty(section, "region").map(str::to_string));

        let mut diagnostics = Vec::new();
        let mut profile_names = BTreeSet::new();
        let mut sso_sessions = BTreeMap::new();

        for (section_name, section) in &sections {
            if let Some(session_name) = sso_session_name(section_name) {
                sso_sessions.insert(
                    session_name.to_string(),
                    SsoSession {
                        start_url: get_nonempty(section, "sso_start_url").map(str::to_string),
                        region: get_nonempty(section, "sso_region").map(str::to_string),
                    },
                );
            }
        }

        let mut profiles = Vec::new();

        for (section_name, section) in &sections {
            let Some(profile_name) = profile_name(section_name) else {
                continue;
            };

            if profile_name.is_empty() {
                diagnostics.push(ConfigDiagnostic {
                    subject: section_name.clone(),
                    message: "profile name is empty".to_string(),
                });
                continue;
            }

            if !profile_names.insert(profile_name.to_string()) {
                diagnostics.push(ConfigDiagnostic {
                    subject: profile_name.to_string(),
                    message: "duplicate profile name".to_string(),
                });
                continue;
            }

            if get_nonempty(section, "sso_session").is_some() {
                if let Some(profile) = build_modern_profile(
                    profile_name,
                    section,
                    &sso_sessions,
                    &default_region,
                    &mut diagnostics,
                ) {
                    profiles.push(profile);
                }
                continue;
            }

            if let Some(profile) =
                build_legacy_profile(profile_name, section, &default_region, &mut diagnostics)
            {
                profiles.push(profile);
            }
        }

        profiles.sort_by(|left, right| {
            left.sso_session
                .as_deref()
                .unwrap_or("")
                .cmp(right.sso_session.as_deref().unwrap_or(""))
                .then_with(|| left.name.cmp(&right.name))
        });

        Self {
            path,
            profiles,
            sso_sessions,
            diagnostics,
        }
    }
}

fn build_modern_profile(
    profile_name: &str,
    section: &Section,
    sso_sessions: &BTreeMap<String, SsoSession>,
    default_region: &Option<String>,
    diagnostics: &mut Vec<ConfigDiagnostic>,
) -> Option<SsoProfile> {
    let sso_session = get_nonempty(section, "sso_session")?;
    let mut missing = missing_keys(section, &["sso_account_id", "sso_role_name"]);

    let Some(session) = sso_sessions.get(sso_session) else {
        missing.push(format!("sso-session {sso_session}"));
        diagnostics.push(ConfigDiagnostic {
            subject: profile_name.to_string(),
            message: format!("incomplete SSO profile; missing {}", missing.join(", ")),
        });
        return None;
    };
    if session.start_url.is_none() {
        missing.push(format!("sso-session {sso_session}.sso_start_url"));
    }
    if session.region.is_none() {
        missing.push(format!("sso-session {sso_session}.sso_region"));
    }

    if !missing.is_empty() {
        diagnostics.push(ConfigDiagnostic {
            subject: profile_name.to_string(),
            message: format!("incomplete SSO profile; missing {}", missing.join(", ")),
        });
        return None;
    }

    Some(SsoProfile {
        name: profile_name.to_string(),
        sso_session: Some(sso_session.to_string()),
        sso_start_url: session.start_url.clone().unwrap_or_default(),
        sso_region: session.region.clone().unwrap_or_default(),
        account_id: get_nonempty(section, "sso_account_id")
            .unwrap_or_default()
            .to_string(),
        role_name: get_nonempty(section, "sso_role_name")
            .unwrap_or_default()
            .to_string(),
        region: region_display(section, default_region),
    })
}

fn build_legacy_profile(
    profile_name: &str,
    section: &Section,
    default_region: &Option<String>,
    diagnostics: &mut Vec<ConfigDiagnostic>,
) -> Option<SsoProfile> {
    let sso_keys = [
        "sso_start_url",
        "sso_region",
        "sso_account_id",
        "sso_role_name",
    ];

    if !sso_keys
        .iter()
        .any(|key| get_nonempty(section, key).is_some())
    {
        return None;
    }

    let missing = missing_keys(section, &sso_keys);
    if !missing.is_empty() {
        diagnostics.push(ConfigDiagnostic {
            subject: profile_name.to_string(),
            message: format!(
                "incomplete legacy SSO profile; missing {}",
                missing.join(", ")
            ),
        });
        return None;
    }

    Some(SsoProfile {
        name: profile_name.to_string(),
        sso_session: None,
        sso_start_url: get_nonempty(section, "sso_start_url")
            .unwrap_or_default()
            .to_string(),
        sso_region: get_nonempty(section, "sso_region")
            .unwrap_or_default()
            .to_string(),
        account_id: get_nonempty(section, "sso_account_id")
            .unwrap_or_default()
            .to_string(),
        role_name: get_nonempty(section, "sso_role_name")
            .unwrap_or_default()
            .to_string(),
        region: region_display(section, default_region),
    })
}

fn region_display(section: &Section, default_region: &Option<String>) -> RegionDisplay {
    if let Some(region) = get_nonempty(section, "region") {
        RegionDisplay::Profile(region.to_string())
    } else if let Some(region) = default_region {
        RegionDisplay::Default(region.clone())
    } else {
        RegionDisplay::Unset
    }
}

fn missing_keys(section: &Section, keys: &[&str]) -> Vec<String> {
    keys.iter()
        .filter(|key| get_nonempty(section, key).is_none())
        .map(|key| key.to_string())
        .collect()
}

fn get_nonempty<'a>(section: &'a Section, key: &str) -> Option<&'a str> {
    section
        .get(key)
        .map(String::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn profile_name(section_name: &str) -> Option<&str> {
    if section_name == "default" {
        return Some("default");
    }

    section_name
        .strip_prefix("profile ")
        .map(str::trim)
        .filter(|name| !name.is_empty())
}

fn sso_session_name(section_name: &str) -> Option<&str> {
    section_name
        .strip_prefix("sso-session ")
        .map(str::trim)
        .filter(|name| !name.is_empty())
}

fn config_path() -> Result<PathBuf> {
    if let Ok(path) = env::var("AWS_CONFIG_FILE") {
        if !path.trim().is_empty() {
            return Ok(expand_tilde(path));
        }
    }

    let home = env::var("HOME").context("HOME is not set and AWS_CONFIG_FILE was not provided")?;
    Ok(Path::new(&home).join(".aws").join("config"))
}

fn expand_tilde(path: String) -> PathBuf {
    if path == "~" {
        if let Ok(home) = env::var("HOME") {
            return PathBuf::from(home);
        }
    }

    if let Some(rest) = path.strip_prefix("~/") {
        if let Ok(home) = env::var("HOME") {
            return Path::new(&home).join(rest);
        }
    }

    PathBuf::from(path)
}

fn parse_ini(content: &str) -> BTreeMap<String, Section> {
    let mut sections = BTreeMap::new();
    let mut current: Option<String> = None;

    for raw_line in content.lines() {
        let line = raw_line.trim();

        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            let name = line[1..line.len() - 1].trim().to_string();
            sections.entry(name.clone()).or_insert_with(BTreeMap::new);
            current = Some(name);
            continue;
        }

        let Some(section_name) = current.as_ref() else {
            continue;
        };

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };

        sections
            .entry(section_name.clone())
            .or_insert_with(BTreeMap::new)
            .insert(
                key.trim().to_string(),
                strip_wrapping_quotes(value.trim()).to_string(),
            );
    }

    sections
}

fn strip_wrapping_quotes(value: &str) -> &str {
    if value.len() >= 2 {
        let bytes = value.as_bytes();
        if (bytes[0] == b'"' && bytes[value.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[value.len() - 1] == b'\'')
        {
            return &value[1..value.len() - 1];
        }
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_modern_and_legacy_profiles() {
        let sections = parse_ini(
            r#"
            [default]
            region = eu-west-1

            [sso-session corp]
            sso_start_url = https://example.awsapps.com/start
            sso_region = us-east-1

            [profile prod]
            sso_session = corp
            sso_account_id = 123456789012
            sso_role_name = AdministratorAccess
            region = eu-central-1

            [profile legacy]
            sso_start_url = https://legacy.awsapps.com/start
            sso_region = eu-west-2
            sso_account_id = 210987654321
            sso_role_name = ReadOnlyAccess
            "#,
        );

        let config = SsoInventory::from_sections(PathBuf::from("/tmp/config"), sections);
        assert_eq!(config.profiles.len(), 2);
        assert_eq!(config.profiles[0].name, "legacy");
        assert_eq!(
            config.profiles[0].region,
            RegionDisplay::Default("eu-west-1".to_string())
        );
        assert_eq!(config.profiles[1].name, "prod");
        assert_eq!(
            config.profiles[1].region,
            RegionDisplay::Profile("eu-central-1".to_string())
        );
    }

    #[test]
    fn reports_incomplete_profiles() {
        let sections = parse_ini(
            r#"
            [profile broken]
            sso_session = missing
            sso_account_id = 123456789012
            "#,
        );

        let config = SsoInventory::from_sections(PathBuf::from("/tmp/config"), sections);
        assert!(config.profiles.is_empty());
        assert_eq!(config.diagnostics.len(), 1);
    }
}

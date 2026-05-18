use crate::aws_config::SsoProfile;
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::env;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginStatus {
    Valid,
    Expired,
    Unknown,
}

impl fmt::Display for LoginStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Valid => write!(formatter, "valid"),
            Self::Expired => write!(formatter, "expired"),
            Self::Unknown => write!(formatter, "unknown"),
        }
    }
}

pub fn status_for_profile(profile: &SsoProfile) -> LoginStatus {
    status_for_profile_in_dir(profile, &cache_dir())
}

fn status_for_profile_in_dir(profile: &SsoProfile, dir: &Path) -> LoginStatus {
    let Ok(entries) = fs::read_dir(dir) else {
        return LoginStatus::Unknown;
    };

    let mut saw_expired = false;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }

        let Ok(content) = fs::read_to_string(&path) else {
            continue;
        };

        let Ok(value) = serde_json::from_str::<Value>(&content) else {
            continue;
        };

        if !matches_profile_cache(&value, profile) {
            continue;
        }

        let Some(expires_at) = value.get("expiresAt").and_then(Value::as_str) else {
            continue;
        };

        let Some(expiry) = parse_aws_expiry(expires_at) else {
            continue;
        };

        if expiry > Utc::now() {
            return LoginStatus::Valid;
        }

        saw_expired = true;
    }

    if saw_expired {
        LoginStatus::Expired
    } else {
        LoginStatus::Unknown
    }
}

fn matches_profile_cache(value: &Value, profile: &SsoProfile) -> bool {
    let start_url_matches = value
        .get("startUrl")
        .and_then(Value::as_str)
        .map(|start_url| start_url == profile.sso_start_url)
        .unwrap_or(false);

    if !start_url_matches {
        return false;
    }

    value
        .get("region")
        .and_then(Value::as_str)
        .map(|region| region == profile.sso_region)
        .unwrap_or(true)
}

fn parse_aws_expiry(value: &str) -> Option<DateTime<Utc>> {
    let normalized = if let Some(prefix) = value.strip_suffix("UTC") {
        format!("{prefix}Z")
    } else {
        value.to_string()
    };

    DateTime::parse_from_rfc3339(&normalized)
        .map(|datetime| datetime.with_timezone(&Utc))
        .ok()
}

fn cache_dir() -> PathBuf {
    if let Ok(path) = env::var("AWS_SSO_CACHE_DIR") {
        if !path.trim().is_empty() {
            return PathBuf::from(path);
        }
    }

    let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    Path::new(&home).join(".aws").join("sso").join("cache")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws_config::RegionDisplay;
    use std::fs;

    fn profile() -> SsoProfile {
        SsoProfile {
            name: "prod".to_string(),
            sso_session: Some("corp".to_string()),
            sso_start_url: "https://example.awsapps.com/start".to_string(),
            sso_region: "us-east-1".to_string(),
            account_id: "123456789012".to_string(),
            role_name: "AdministratorAccess".to_string(),
            region: RegionDisplay::Unset,
        }
    }

    #[test]
    fn recognizes_valid_cache() {
        let tempdir = tempfile::tempdir().unwrap();
        fs::write(
            tempdir.path().join("cache.json"),
            r#"{
                "startUrl": "https://example.awsapps.com/start",
                "region": "us-east-1",
                "expiresAt": "2999-01-01T00:00:00UTC"
            }"#,
        )
        .unwrap();

        assert_eq!(
            status_for_profile_in_dir(&profile(), tempdir.path()),
            LoginStatus::Valid
        );
    }

    #[test]
    fn recognizes_expired_cache() {
        let tempdir = tempfile::tempdir().unwrap();
        fs::write(
            tempdir.path().join("cache.json"),
            r#"{
                "startUrl": "https://example.awsapps.com/start",
                "region": "us-east-1",
                "expiresAt": "2000-01-01T00:00:00Z"
            }"#,
        )
        .unwrap();

        assert_eq!(
            status_for_profile_in_dir(&profile(), tempdir.path()),
            LoginStatus::Expired
        );
    }
}

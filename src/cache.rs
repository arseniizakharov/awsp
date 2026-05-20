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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheStatus {
    pub state: LoginStatus,
    pub expires_at: Option<DateTime<Utc>>,
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

pub fn cache_status_for_profile(profile: &SsoProfile) -> CacheStatus {
    cache_status_for_profile_in_dir(profile, &cache_dir())
}

fn cache_status_for_profile_in_dir(profile: &SsoProfile, dir: &Path) -> CacheStatus {
    let Ok(entries) = fs::read_dir(dir) else {
        return CacheStatus::unknown();
    };

    let mut latest_expired = None;

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
            return CacheStatus {
                state: LoginStatus::Valid,
                expires_at: Some(expiry),
            };
        }

        latest_expired = latest_expired.max(Some(expiry));
    }

    if let Some(expires_at) = latest_expired {
        CacheStatus {
            state: LoginStatus::Expired,
            expires_at: Some(expires_at),
        }
    } else {
        CacheStatus::unknown()
    }
}

impl CacheStatus {
    pub fn unknown() -> Self {
        Self {
            state: LoginStatus::Unknown,
            expires_at: None,
        }
    }

    pub fn label(&self) -> String {
        match (self.state, self.expires_at) {
            (LoginStatus::Valid, Some(expires_at)) => {
                format!("valid {}", duration_until(expires_at))
            }
            (LoginStatus::Valid, None) => "valid".to_string(),
            (LoginStatus::Expired, Some(expires_at)) => {
                format!("expired ({})", duration_since(expires_at))
            }
            (LoginStatus::Expired, None) => "expired".to_string(),
            (LoginStatus::Unknown, _) => "unknown".to_string(),
        }
    }

    pub fn expires_in_seconds(&self) -> Option<i64> {
        self.expires_at
            .map(|expires_at| (expires_at - Utc::now()).num_seconds())
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

fn duration_until(expires_at: DateTime<Utc>) -> String {
    let seconds = (expires_at - Utc::now()).num_seconds().max(0);
    compact_duration(seconds)
}

fn duration_since(expires_at: DateTime<Utc>) -> String {
    let seconds = (Utc::now() - expires_at).num_seconds().max(0);
    compact_duration(seconds)
}

fn compact_duration(seconds: i64) -> String {
    let minutes = (seconds / 60).max(0);
    let days = minutes / (60 * 24);
    if days > 0 {
        return format!("{days}d ago");
    }

    let hours = minutes / 60;
    let remaining_minutes = minutes % 60;
    if hours > 0 {
        return format!("{hours}h {remaining_minutes}m");
    }
    format!("{remaining_minutes}m")
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
            cache_status_for_profile_in_dir(&profile(), tempdir.path()).state,
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
            cache_status_for_profile_in_dir(&profile(), tempdir.path()).state,
            LoginStatus::Expired
        );
    }

    #[test]
    fn labels_unknown_cache_status() {
        assert_eq!(CacheStatus::unknown().label(), "unknown");
    }
}

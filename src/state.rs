use anyhow::{Context, Result};
use chrono::Utc;
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;
use uuid::Uuid;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct State {
    pub last_profile: Option<String>,
    #[serde(default)]
    pub sessions: BTreeMap<String, SessionState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    pub profile: Option<String>,
    pub updated_at: String,
}

pub fn new_session_id() -> String {
    Uuid::new_v4().to_string()
}

pub fn current_session_id() -> Option<String> {
    env::var("AWSP_SESSION_ID")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub fn state_path() -> Result<PathBuf> {
    if let Ok(path) = env::var("AWSP_STATE_FILE") {
        if !path.trim().is_empty() {
            return Ok(PathBuf::from(path));
        }
    }

    let home = env::var("HOME").context("HOME is not set")?;
    Ok(Path::new(&home)
        .join(".config")
        .join("awsp")
        .join("state.json"))
}

pub fn get_session_profile(session_id: &str) -> Result<Option<String>> {
    let state = read_state()?;
    Ok(state
        .sessions
        .get(session_id)
        .and_then(|session| session.profile.clone()))
}

pub fn set_session_profile(session_id: &str, profile: &str) -> Result<()> {
    with_locked_state(|state| {
        state.last_profile = Some(profile.to_string());
        state.sessions.insert(
            session_id.to_string(),
            SessionState {
                profile: Some(profile.to_string()),
                updated_at: Utc::now().to_rfc3339(),
            },
        );
        Ok(())
    })
}

pub fn clear_session_profile(session_id: &str) -> Result<()> {
    with_locked_state(|state| {
        state.sessions.insert(
            session_id.to_string(),
            SessionState {
                profile: None,
                updated_at: Utc::now().to_rfc3339(),
            },
        );
        Ok(())
    })
}

pub fn read_state() -> Result<State> {
    let path = state_path()?;
    read_state_at(&path)
}

fn with_locked_state<F>(mut update: F) -> Result<()>
where
    F: FnMut(&mut State) -> Result<()>,
{
    let path = state_path()?;
    let parent = path.parent().context("state path has no parent")?;
    fs::create_dir_all(parent).with_context(|| format!("failed to create {}", parent.display()))?;

    let lock_path = parent.join("state.lock");
    let lock_file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&lock_path)
        .with_context(|| format!("failed to open {}", lock_path.display()))?;

    lock_file
        .lock_exclusive()
        .with_context(|| format!("failed to lock {}", lock_path.display()))?;

    let result = (|| {
        let mut state = read_state_at(&path)?;
        update(&mut state)?;
        write_state_atomic(&path, &state)
    })();

    let unlock_result = lock_file.unlock();
    result?;
    unlock_result.with_context(|| format!("failed to unlock {}", lock_path.display()))?;
    Ok(())
}

fn read_state_at(path: &Path) -> Result<State> {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(State::default()),
        Err(error) => {
            return Err(error).with_context(|| format!("failed to open {}", path.display()))
        }
    };

    let mut content = String::new();
    file.read_to_string(&mut content)
        .with_context(|| format!("failed to read {}", path.display()))?;

    if content.trim().is_empty() {
        return Ok(State::default());
    }

    serde_json::from_str(&content).with_context(|| format!("failed to parse {}", path.display()))
}

fn write_state_atomic(path: &Path, state: &State) -> Result<()> {
    let parent = path.parent().context("state path has no parent")?;
    let mut temp = NamedTempFile::new_in(parent)
        .with_context(|| format!("failed to create temp file in {}", parent.display()))?;

    serde_json::to_writer_pretty(&mut temp, state).context("failed to serialize state")?;
    temp.write_all(b"\n")
        .context("failed to write state newline")?;
    temp.flush().context("failed to flush state")?;
    temp.as_file().sync_all().context("failed to sync state")?;
    temp.persist(path)
        .map(|_| ())
        .map_err(|error| error.error)
        .with_context(|| format!("failed to replace {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_session_ids() {
        let first = new_session_id();
        let second = new_session_id();
        assert_ne!(first, second);
        assert!(!first.is_empty());
    }
}

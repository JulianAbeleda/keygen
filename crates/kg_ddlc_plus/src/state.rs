//! Product persistence projection for the macOS compatibility target.
//!
//! This is deliberately a small, content-agnostic boundary: imported story
//! data supplies identifiers and text, while the player owns the durable state.
//! Bytes are written through `AtomicStore`, so the envelope checksum and
//! sandbox rules apply uniformly to preferences, slots, and progression.
use crate::{launcher::LauncherSnapshot, vn::ScreenText};
use keygen_player::storage::{AtomicStore, Preferences, StoreMetadata, Unlocks};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const STATE_SCHEMA: &str = "keygen.kg_ddlc_plus.state.v1";
pub const STATE_PATH: &str = "state/session.json";
pub const MAX_HISTORY: usize = 512;
pub const MAX_VARIABLES: usize = 256;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScreenshotState {
    pub width: u16,
    pub height: u16,
    pub scene_id: Option<String>,
}

impl Default for ScreenshotState {
    fn default() -> Self {
        Self {
            width: 384,
            height: 216,
            scene_id: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SessionState {
    pub revision: u64,
    pub launcher: Option<LauncherSnapshot>,
    pub preferences: Preferences,
    pub unlocks: Unlocks,
    pub history: Vec<ScreenText>,
    pub variables: BTreeMap<String, String>,
    pub screenshot: ScreenshotState,
    pub active_slot: Option<usize>,
}

impl Default for SessionState {
    fn default() -> Self {
        Self {
            revision: 1,
            launcher: None,
            preferences: Preferences::default(),
            unlocks: Unlocks::default(),
            history: Vec::new(),
            variables: BTreeMap::new(),
            screenshot: ScreenshotState::default(),
            active_slot: None,
        }
    }
}

impl SessionState {
    pub fn validate(&self) -> Result<(), String> {
        self.preferences.validate()?;
        if self.screenshot.width != 384 || self.screenshot.height != 216 {
            return Err("screenshot state must use the 384x216 logical viewport".into());
        }
        if self.history.len() > MAX_HISTORY {
            return Err("history exceeds bounded capacity".into());
        }
        if self.variables.len() > MAX_VARIABLES
            || self
                .variables
                .keys()
                .any(|key| key.is_empty() || key.contains('/'))
        {
            return Err("invalid or oversized story variable set".into());
        }
        Ok(())
    }

    pub fn record_line(&mut self, line: ScreenText) {
        self.history.push(line);
        if self.history.len() > MAX_HISTORY {
            let excess = self.history.len() - MAX_HISTORY;
            self.history.drain(..excess);
        }
    }

    pub fn set_variable(
        &mut self,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Result<(), String> {
        let key = key.into();
        if key.is_empty() || key.contains('/') {
            return Err("invalid story variable key".into());
        }
        if !self.variables.contains_key(&key) && self.variables.len() >= MAX_VARIABLES {
            return Err("story variable capacity reached".into());
        }
        self.variables.insert(key, value.into());
        Ok(())
    }

    pub fn reset_progression(&mut self) {
        self.unlocks.reset();
        self.history.clear();
        self.variables.clear();
        self.active_slot = None;
    }

    pub fn save(&self, store: &AtomicStore) -> Result<StoreMetadata, String> {
        self.validate()?;
        store.save(STATE_PATH, STATE_SCHEMA, self.revision, self)
    }

    pub fn load(store: &AtomicStore) -> Result<(StoreMetadata, Self), String> {
        let (metadata, state): (StoreMetadata, Self) = store.load(STATE_PATH, STATE_SCHEMA)?;
        state.validate()?;
        Ok((metadata, state))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, path::PathBuf};

    fn store() -> (AtomicStore, PathBuf) {
        let root = std::env::temp_dir().join(format!("keygen-ddlc-state-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        (AtomicStore::new(root.clone()), root)
    }

    #[test]
    fn round_trip_is_deterministic_and_checksummed() {
        let (store, root) = store();
        let mut state = SessionState::default();
        state.record_line(ScreenText {
            speaker: Some("test".into()),
            text: "hello".into(),
        });
        state.set_variable("route", "a").unwrap();
        state.unlocks.record("story.test", 12);
        let metadata = state.save(&store).unwrap();
        let (loaded_meta, loaded) = SessionState::load(&store).unwrap();
        assert_eq!(metadata, loaded_meta);
        assert_eq!(state, loaded);
        assert_eq!(loaded.screenshot, ScreenshotState::default());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn history_and_progression_are_bounded_and_resettable() {
        let mut state = SessionState::default();
        for n in 0..(MAX_HISTORY + 2) {
            state.record_line(ScreenText {
                speaker: None,
                text: n.to_string(),
            });
        }
        assert_eq!(state.history.len(), MAX_HISTORY);
        assert_eq!(state.history[0].text, "2");
        state.unlocks.record("unlock.test", 1);
        state.set_variable("x", "y").unwrap();
        state.reset_progression();
        assert!(
            state.history.is_empty()
                && state.variables.is_empty()
                && state.unlocks.events.is_empty()
        );
    }

    #[test]
    fn invalid_viewport_and_variable_keys_fail_validation() {
        let mut state = SessionState::default();
        state.screenshot.width = 800;
        assert!(state.validate().is_err());
        assert!(state.set_variable("bad/key", "x").is_err());
    }
}

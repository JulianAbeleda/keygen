//! Deterministic launcher state and routing for the macOS `kg_ddlc_plus` target.
//!
//! The launcher owns product navigation, lifecycle and resource handoff.  It
//! intentionally contains logical asset IDs only; the package compiler and
//! host resolve those IDs to player-owned bytes at the edge of the system.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub mod apps;

/// Stable logical IDs used by launcher screens.  These are not source paths.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct AssetId(String);

impl AssetId {
    pub fn new(value: impl Into<String>) -> Result<Self, String> {
        let value = value.into();
        if value.is_empty()
            || !value
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "-._".contains(c))
        {
            return Err(format!("invalid logical asset id: {value}"));
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LauncherApp {
    pub id: AppId,
    pub title: String,
    pub icon: AssetId,
    pub enabled: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum AppId {
    FileBrowser,
    FileViewer,
    Mail,
    SideStories,
    Gallery,
    Jukebox,
    Settings,
    Terminal,
    Story,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ScreenRoute {
    Bios,
    BootUp,
    Login,
    Desktop,
    App(AppId),
    Story,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Lifecycle {
    Created,
    Running,
    Suspended,
    Closing,
    Closed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum CloseRequest {
    Cancel,
    Confirm,
    SaveBusy,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum HandoffPhase {
    LauncherSuspended,
    StoryStarted,
    StoryReturned,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct StoryLaunch {
    pub entry_label: String,
    pub locale: String,
    pub parameters: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LauncherSnapshot {
    pub lifecycle: Lifecycle,
    pub route: ScreenRoute,
    pub first_run: bool,
    pub interaction_marker: bool,
    pub input_locked: bool,
    pub active_window: Option<AppId>,
    pub resource_group: String,
    pub audio_owner: String,
}

#[derive(Clone, Debug)]
pub struct LauncherState {
    pub lifecycle: Lifecycle,
    pub route: ScreenRoute,
    pub first_run: bool,
    pub interaction_marker: bool,
    pub input_locked: bool,
    pub active_window: Option<AppId>,
    pub resource_group: String,
    pub audio_owner: String,
    pub elapsed_seconds: f32,
    pub apps: BTreeMap<AppId, LauncherApp>,
}

impl LauncherState {
    pub fn new(first_run: bool) -> Self {
        let mut apps = BTreeMap::new();
        for (id, title, icon) in [
            (AppId::FileBrowser, "Files", "icon.files"),
            (AppId::Mail, "Mail", "icon.mail"),
            (AppId::SideStories, "Side Stories", "icon.stories"),
            (AppId::Gallery, "Gallery", "icon.gallery"),
            (AppId::Jukebox, "Jukebox", "icon.jukebox"),
            (AppId::Settings, "Settings", "icon.settings"),
            (AppId::Terminal, "Terminal", "icon.terminal"),
        ] {
            apps.insert(
                id,
                LauncherApp {
                    id,
                    title: title.into(),
                    icon: AssetId::new(icon).expect("built-in logical asset id"),
                    enabled: true,
                },
            );
        }
        Self {
            lifecycle: Lifecycle::Created,
            route: ScreenRoute::Bios,
            first_run,
            interaction_marker: false,
            input_locked: true,
            active_window: None,
            resource_group: "launcher".into(),
            audio_owner: "launcher".into(),
            elapsed_seconds: 0.0,
            apps,
        }
    }

    pub fn start(&mut self) {
        if self.lifecycle == Lifecycle::Created {
            self.lifecycle = Lifecycle::Running;
            self.route = ScreenRoute::Bios;
            self.elapsed_seconds = 0.0;
        }
    }

    /// Advances only the product timeline. Host frame clocks remain outside.
    pub fn update(&mut self, delta_seconds: f32, store_ready: bool) {
        if self.lifecycle != Lifecycle::Running || !delta_seconds.is_finite() || delta_seconds < 0.0
        {
            return;
        }
        self.elapsed_seconds += delta_seconds;
        match self.route {
            ScreenRoute::Bios if self.elapsed_seconds >= 3.0 => {
                self.transition(ScreenRoute::BootUp)
            }
            ScreenRoute::BootUp if self.elapsed_seconds >= 3.0 => {
                self.transition(ScreenRoute::Login)
            }
            ScreenRoute::Login if store_ready && self.elapsed_seconds >= 0.25 => {
                self.input_locked = false;
                self.transition(ScreenRoute::Desktop);
            }
            _ => {}
        }
    }

    pub fn select_app(&mut self, app: AppId) -> Result<(), String> {
        if self.lifecycle != Lifecycle::Running || self.input_locked {
            return Err("launcher input is locked".into());
        }
        let item = self
            .apps
            .get(&app)
            .ok_or_else(|| format!("unknown launcher app: {app:?}"))?;
        if !item.enabled {
            return Err(format!("launcher app is disabled: {app:?}"));
        }
        self.active_window = Some(app);
        self.route = ScreenRoute::App(app);
        self.elapsed_seconds = 0.0;
        Ok(())
    }

    pub fn request_close(&mut self, save_busy: bool) -> CloseRequest {
        if save_busy {
            CloseRequest::SaveBusy
        } else {
            CloseRequest::Confirm
        }
    }

    pub fn close(&mut self, confirmation: CloseRequest) -> Result<(), String> {
        match confirmation {
            CloseRequest::Cancel => Ok(()),
            CloseRequest::SaveBusy => Err("cannot close while save operation is busy".into()),
            CloseRequest::Confirm => {
                self.lifecycle = Lifecycle::Closing;
                self.input_locked = true;
                self.lifecycle = Lifecycle::Closed;
                Ok(())
            }
        }
    }

    pub fn begin_story(&mut self, launch: &StoryLaunch) -> Result<HandoffPhase, String> {
        if self.route != ScreenRoute::Desktop
            && !matches!(self.route, ScreenRoute::App(AppId::SideStories))
        {
            return Err("story handoff requested outside launcher desktop/app".into());
        }
        if launch.entry_label.is_empty() || launch.locale.is_empty() {
            return Err("story launch requires entry label and locale".into());
        }
        self.lifecycle = Lifecycle::Suspended;
        self.input_locked = true;
        self.route = ScreenRoute::Story;
        self.active_window = None;
        self.resource_group = "story".into();
        self.audio_owner = "story".into();
        Ok(HandoffPhase::StoryStarted)
    }

    pub fn return_from_story(&mut self) -> Result<HandoffPhase, String> {
        if self.route != ScreenRoute::Story {
            return Err("story return requested when story is not active".into());
        }
        self.lifecycle = Lifecycle::Running;
        self.route = ScreenRoute::Desktop;
        self.input_locked = false;
        self.resource_group = "launcher".into();
        self.audio_owner = "launcher".into();
        self.elapsed_seconds = 0.0;
        Ok(HandoffPhase::StoryReturned)
    }

    pub fn snapshot(&self) -> LauncherSnapshot {
        LauncherSnapshot {
            lifecycle: self.lifecycle,
            route: self.route,
            first_run: self.first_run,
            interaction_marker: self.interaction_marker,
            input_locked: self.input_locked,
            active_window: self.active_window,
            resource_group: self.resource_group.clone(),
            audio_owner: self.audio_owner.clone(),
        }
    }

    fn transition(&mut self, route: ScreenRoute) {
        self.route = route;
        self.elapsed_seconds = 0.0;
        self.input_locked = !matches!(route, ScreenRoute::Desktop | ScreenRoute::App(_));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_run_boots_to_desktop_only_after_store_ready() {
        let mut state = LauncherState::new(true);
        state.start();
        state.update(3.0, false);
        assert_eq!(state.route, ScreenRoute::BootUp);
        state.update(3.0, false);
        assert_eq!(state.route, ScreenRoute::Login);
        state.update(1.0, false);
        assert_eq!(state.route, ScreenRoute::Login);
        state.update(0.25, true);
        assert_eq!(state.route, ScreenRoute::Desktop);
    }

    #[test]
    fn app_window_and_story_handoff_restore_ownership() {
        let mut state = LauncherState::new(false);
        state.start();
        state.update(3.0, true);
        state.update(3.0, true);
        state.update(0.25, true);
        state.select_app(AppId::SideStories).unwrap();
        let launch = StoryLaunch {
            entry_label: "side_story_1".into(),
            locale: "en".into(),
            parameters: BTreeMap::new(),
        };
        assert_eq!(
            state.begin_story(&launch).unwrap(),
            HandoffPhase::StoryStarted
        );
        assert_eq!(state.snapshot().resource_group, "story");
        assert_eq!(
            state.return_from_story().unwrap(),
            HandoffPhase::StoryReturned
        );
        assert_eq!(state.snapshot().audio_owner, "launcher");
    }

    #[test]
    fn save_busy_close_is_blocked_and_cancel_is_safe() {
        let mut state = LauncherState::new(false);
        state.start();
        assert_eq!(state.request_close(true), CloseRequest::SaveBusy);
        assert!(state.close(CloseRequest::SaveBusy).is_err());
        assert!(state.close(CloseRequest::Cancel).is_ok());
        assert_eq!(state.lifecycle, Lifecycle::Running);
    }

    #[test]
    fn logical_asset_ids_reject_paths() {
        assert!(AssetId::new("sprite.monika.normal").is_ok());
        assert!(AssetId::new("../secret").is_err());
    }
}

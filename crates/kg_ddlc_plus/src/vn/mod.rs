//! Content-agnostic native visual-novel screen contracts (KGD-320..329).
//!
//! These reducers describe presentation state only.  Content importers supply strings,
//! sprite ids, and story effects; no DDLC assets or copyrighted script text is embedded.
use keygen_engine::{
    audio::{AudioChannel, AudioCommand},
    input::{Action, InputEvent, InputState},
    story::Effect,
};
use keygen_player::storage::{Preferences, Unlocks};
use serde::{Deserialize, Serialize};

pub mod choice;
pub mod dialogue;
pub mod history;
pub mod main_menu;
pub mod name_input;
pub mod poetry;
pub mod preferences;
pub mod save;
pub mod special;
pub mod typewriter;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScreenText {
    pub speaker: Option<String>,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScreenId {
    Dialogue,
    Choice,
    MainMenu,
    History,
    Save,
    Preferences,
    NameInput,
    Poetry,
    Special(String),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ScreenCommand {
    Open(ScreenId),
    Close,
    Advance,
    Select(usize),
    Back,
    SetName(String),
    SetPreference(Preferences),
    Save(usize),
    Load(usize),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ScreenStack {
    pub active: Option<ScreenId>,
    pub history: Vec<ScreenId>,
}
impl ScreenStack {
    pub fn open(&mut self, id: ScreenId) {
        if let Some(current) = self.active.replace(id) {
            self.history.push(current);
        }
    }
    pub fn close(&mut self) {
        self.active = self.history.pop();
    }
}

#[derive(Clone, Debug)]
pub struct RuntimeContext {
    pub input: InputState,
    pub preferences: Preferences,
    pub unlocks: Unlocks,
    pub audio: Vec<AudioCommand>,
    pub effects: Vec<Effect>,
}
impl Default for RuntimeContext {
    fn default() -> Self {
        Self {
            input: InputState {
                focused: true,
                ..Default::default()
            },
            preferences: Default::default(),
            unlocks: Default::default(),
            audio: Vec::new(),
            effects: Vec::new(),
        }
    }
}
impl RuntimeContext {
    pub fn submit(&mut self, event: InputEvent) -> Option<Action> {
        let action = self.input.action(&event).map(|a| a.action);
        self.input.apply(event);
        action
    }
    pub fn queue_audio(&mut self, channel: AudioChannel, clip: impl Into<String>) {
        self.audio.push(AudioCommand::Play {
            channel,
            clip: keygen_engine::audio::AudioClip {
                id: clip.into(),
                sample_rate: 48_000,
                channels: 2,
                frames: 1,
                loop_start: None,
                loop_end: None,
            },
            owner: keygen_engine::audio::AudioOwner::Story,
            looped: false,
        });
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScreenResult {
    pub command: Option<ScreenCommand>,
    pub consumed: bool,
}
impl ScreenResult {
    pub fn consumed(command: Option<ScreenCommand>) -> Self {
        Self {
            command,
            consumed: true,
        }
    }
    pub fn ignored() -> Self {
        Self {
            command: None,
            consumed: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stack_returns_to_previous_screen() {
        let mut s = ScreenStack::default();
        s.open(ScreenId::MainMenu);
        s.open(ScreenId::Preferences);
        s.close();
        assert_eq!(s.active, Some(ScreenId::MainMenu));
    }
    #[test]
    fn context_maps_confirm() {
        let mut c = RuntimeContext::default();
        let e = InputEvent {
            frame: 1,
            device: keygen_engine::input::Device::Keyboard,
            kind: keygen_engine::input::InputEventKind::Key {
                key: keygen_engine::input::Key::Enter,
                down: true,
                repeat: false,
            },
        };
        assert_eq!(c.submit(e), Some(Action::Confirm));
    }
}

//! Normalized, replayable input events. Platform adapters translate native
//! events into this model; engine reducers never inspect platform APIs.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub enum Key {
    Escape,
    Enter,
    Space,
    Up,
    Down,
    Left,
    Right,
    Char(char),
}
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub enum PointerButton {
    Primary,
    Secondary,
    Middle,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Device {
    Keyboard,
    Pointer,
    Controller,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum InputEventKind {
    Key {
        key: Key,
        down: bool,
        repeat: bool,
    },
    Pointer {
        x: f32,
        y: f32,
        button: Option<PointerButton>,
        down: bool,
    },
    Text(String),
    Focus {
        active: bool,
    },
    Device {
        device: Device,
        connected: bool,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InputEvent {
    pub frame: u64,
    pub device: Device,
    pub kind: InputEventKind,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    Confirm,
    Cancel,
    Up,
    Down,
    Left,
    Right,
    Menu,
    Custom(String),
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActionEvent {
    pub frame: u64,
    pub action: Action,
    pub pressed: bool,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct InputState {
    pub focused: bool,
    pub locked: bool,
    pub text: String,
    pub events: Vec<InputEvent>,
}
impl InputState {
    pub fn apply(&mut self, event: InputEvent) {
        match &event.kind {
            InputEventKind::Focus { active } => self.focused = *active,
            InputEventKind::Text(value) if self.focused && !self.locked => {
                self.text.push_str(value)
            }
            _ => {}
        }
        if self.events.len() >= 256 {
            self.events.remove(0);
        }
        self.events.push(event);
    }
    pub fn set_locked(&mut self, locked: bool) {
        self.locked = locked;
    }
    pub fn action(&self, event: &InputEvent) -> Option<ActionEvent> {
        if self.locked || !self.focused {
            return None;
        }
        let action = match event.kind {
            InputEventKind::Key {
                key: Key::Enter,
                down: true,
                ..
            } => Action::Confirm,
            InputEventKind::Key {
                key: Key::Escape,
                down: true,
                ..
            } => Action::Cancel,
            InputEventKind::Key {
                key: Key::Up,
                down: true,
                ..
            } => Action::Up,
            InputEventKind::Key {
                key: Key::Down,
                down: true,
                ..
            } => Action::Down,
            InputEventKind::Key {
                key: Key::Left,
                down: true,
                ..
            } => Action::Left,
            InputEventKind::Key {
                key: Key::Right,
                down: true,
                ..
            } => Action::Right,
            _ => return None,
        };
        Some(ActionEvent {
            frame: event.frame,
            action,
            pressed: true,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn key(frame: u64, key: Key) -> InputEvent {
        InputEvent {
            frame,
            device: Device::Keyboard,
            kind: InputEventKind::Key {
                key,
                down: true,
                repeat: false,
            },
        }
    }
    #[test]
    fn focus_and_lock_gate_actions() {
        let mut s = InputState::default();
        assert!(s.action(&key(1, Key::Enter)).is_none());
        s.apply(InputEvent {
            frame: 0,
            device: Device::Keyboard,
            kind: InputEventKind::Focus { active: true },
        });
        assert_eq!(
            s.action(&key(1, Key::Enter)).unwrap().action,
            Action::Confirm
        );
        s.set_locked(true);
        assert!(s.action(&key(2, Key::Enter)).is_none());
    }
    #[test]
    fn text_is_replayable_and_bounded() {
        let mut s = InputState {
            focused: true,
            ..InputState::default()
        };
        s.apply(InputEvent {
            frame: 1,
            device: Device::Keyboard,
            kind: InputEventKind::Text("abc".into()),
        });
        assert_eq!(s.text, "abc");
    }
}

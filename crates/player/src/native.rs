//! Platform-neutral contracts consumed by the native macOS host.
//!
//! The current presentation backend is minifb. This module deliberately keeps
//! native event translation and lifecycle policy independent of that backend,
//! so a future Metal/AppKit adapter can consume the same events without
//! changing the engine or story state.

use keygen_engine::input::{Device, InputEvent, InputEventKind, Key, PointerButton};

pub const SUPPORTED_OS: &str = "macOS";
pub const SUPPORTED_ARCH: &str = "arm64";

/// The host boundary is intentionally explicit: this product is qualified only
/// for Apple Silicon macOS.  Rendering and story code remain portable, but the
/// native window entrypoint must never silently run on an unqualified host.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HostQualification {
    pub os: &'static str,
    pub arch: &'static str,
    pub supported: bool,
}

pub const fn compile_target() -> HostQualification {
    HostQualification {
        os: if cfg!(target_os = "macos") {
            SUPPORTED_OS
        } else {
            "unsupported"
        },
        arch: if cfg!(target_arch = "aarch64") {
            SUPPORTED_ARCH
        } else {
            "unsupported"
        },
        supported: cfg!(all(target_os = "macos", target_arch = "aarch64")),
    }
}

pub fn runtime_target() -> HostQualification {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    HostQualification {
        os: if os == "macos" {
            SUPPORTED_OS
        } else {
            "unsupported"
        },
        arch: if arch == "aarch64" {
            SUPPORTED_ARCH
        } else {
            "unsupported"
        },
        supported: os == "macos" && arch == "aarch64",
    }
}

pub fn require_supported_host() -> Result<(), String> {
    let target = runtime_target();
    if target.supported {
        Ok(())
    } else {
        Err(format!(
            "kg_ddlc_plus native host requires macOS arm64 (detected {} {}); use --render or --validate for headless checks",
            target.os, target.arch
        ))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LifecycleEvent {
    Activated,
    Deactivated,
    QuitRequested,
    Closed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LifecycleState {
    Active,
    Inactive,
    Terminating,
    Closed,
}

impl Default for LifecycleState {
    fn default() -> Self {
        Self::Active
    }
}

impl LifecycleState {
    pub fn apply(&mut self, event: LifecycleEvent) -> bool {
        match event {
            LifecycleEvent::Activated if *self != Self::Terminating => {
                *self = Self::Active;
                true
            }
            LifecycleEvent::Deactivated if *self == Self::Active => {
                *self = Self::Inactive;
                true
            }
            LifecycleEvent::QuitRequested if *self != Self::Closed => {
                *self = Self::Terminating;
                true
            }
            LifecycleEvent::Closed => {
                *self = Self::Closed;
                true
            }
            _ => false,
        }
    }

    pub fn accepts_frame(self) -> bool {
        matches!(self, Self::Active | Self::Inactive)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NativePointer {
    pub x: f32,
    pub y: f32,
    pub button: Option<PointerButton>,
    pub down: bool,
}

/// Converts a window-space pointer into design-space coordinates while
/// preserving aspect-ratio letterboxing.
pub fn map_pointer(
    pointer: (f32, f32),
    window: (usize, usize),
    design: (usize, usize),
) -> (f32, f32) {
    let scale = (window.0 as f32 / design.0 as f32).min(window.1 as f32 / design.1 as f32);
    let offset_x = (window.0 as f32 - design.0 as f32 * scale) * 0.5;
    let offset_y = (window.1 as f32 - design.1 as f32 * scale) * 0.5;
    (
        (pointer.0 - offset_x) / scale,
        (pointer.1 - offset_y) / scale,
    )
}

pub fn key_event(frame: u64, key: Key, down: bool, repeat: bool) -> InputEvent {
    InputEvent {
        frame,
        device: Device::Keyboard,
        kind: InputEventKind::Key { key, down, repeat },
    }
}

pub fn pointer_event(frame: u64, pointer: NativePointer) -> InputEvent {
    InputEvent {
        frame,
        device: Device::Pointer,
        kind: InputEventKind::Pointer {
            x: pointer.x,
            y: pointer.y,
            button: pointer.button,
            down: pointer.down,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifecycle_is_ordered_and_idempotent() {
        let mut state = LifecycleState::default();
        assert!(state.apply(LifecycleEvent::Deactivated));
        assert!(!state.apply(LifecycleEvent::Deactivated));
        assert!(state.apply(LifecycleEvent::Activated));
        assert!(state.apply(LifecycleEvent::QuitRequested));
        assert!(!state.apply(LifecycleEvent::Activated));
        assert!(state.apply(LifecycleEvent::Closed));
    }

    #[test]
    fn pointer_mapping_round_trips_center_and_letterbox() {
        assert_eq!(
            map_pointer((640.0, 360.0), (1280, 720), (1280, 720)),
            (640.0, 360.0)
        );
        let (x, y) = map_pointer((800.0, 450.0), (1600, 900), (1280, 720));
        assert!((x - 640.0).abs() < 0.01);
        assert!((y - 360.0).abs() < 0.01);
    }

    #[test]
    fn native_events_are_replayable_engine_events() {
        let event = key_event(4, Key::Enter, true, false);
        assert_eq!(event.frame, 4);
        assert!(matches!(
            event.kind,
            InputEventKind::Key {
                key: Key::Enter,
                ..
            }
        ));
    }

    #[test]
    fn compile_target_reports_the_build_contract() {
        let target = compile_target();
        assert_eq!(
            target.supported,
            cfg!(all(target_os = "macos", target_arch = "aarch64"))
        );
        assert_eq!(
            target.os,
            if cfg!(target_os = "macos") {
                SUPPORTED_OS
            } else {
                "unsupported"
            }
        );
        assert_eq!(
            target.arch,
            if cfg!(target_arch = "aarch64") {
                SUPPORTED_ARCH
            } else {
                "unsupported"
            }
        );
    }

    #[test]
    fn runtime_target_is_deterministic_for_the_current_process() {
        let target = runtime_target();
        assert_eq!(
            target.supported,
            target.os == SUPPORTED_OS && target.arch == SUPPORTED_ARCH
        );
    }
}

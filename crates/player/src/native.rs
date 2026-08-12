//! Platform-neutral contracts consumed by the native macOS host.
//!
//! The current presentation backend is minifb. This module deliberately keeps
//! native event translation and lifecycle policy independent of that backend,
//! so a future Metal/AppKit adapter can consume the same events without
//! changing the engine or story state.

use keygen_engine::input::{Device, InputEvent, InputEventKind, Key, PointerButton};

pub const SUPPORTED_OS: &str = "macOS";
pub const SUPPORTED_ARCH: &str = "arm64";

/// Design-space geometry shared by headless captures and native windows.
/// Hosts must preserve this viewport's aspect ratio; the renderer owns the
/// conversion to drawable pixels and the input adapter uses [`map_pointer`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DesignViewport {
    pub width: usize,
    pub height: usize,
}

impl DesignViewport {
    pub const fn new(width: usize, height: usize) -> Self {
        Self { width, height }
    }

    pub const fn pixel_count(self) -> usize {
        self.width * self.height
    }
}

/// A fully-owned frame handed from the deterministic scene renderer to a
/// presentation backend. Pixels are packed RGBA8 in row-major order. Keeping
/// this contract independent of minifb makes an AppKit/Metal backend a
/// replaceable host adapter instead of a second renderer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HostFrame {
    pub viewport: DesignViewport,
    pub rgba8: Vec<u8>,
    pub frame: u64,
}

impl HostFrame {
    pub fn new(viewport: DesignViewport, frame: u64, rgba8: Vec<u8>) -> Result<Self, String> {
        let expected = viewport.pixel_count() * 4;
        if rgba8.len() != expected {
            return Err(format!(
                "RGBA frame has {} bytes; expected {expected}",
                rgba8.len()
            ));
        }
        Ok(Self {
            viewport,
            rgba8,
            frame,
        })
    }
}

/// Backend-neutral lifecycle and presentation surface. A native host may
/// implement this with AppKit/Metal; tests can implement it in memory.
pub trait PresentationBackend {
    fn present(&mut self, frame: HostFrame) -> Result<(), String>;
    fn lifecycle(&mut self, event: LifecycleEvent) -> Result<(), String>;
    fn lifecycle_state(&self) -> LifecycleState;
}

/// Audio is intentionally an effect sink, not a renderer concern. The story
/// VM emits logical clip/channel commands and a platform adapter decides how
/// to decode and schedule them (CoreAudio on macOS, a test sink in CI).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AudioCommand {
    Play {
        channel: String,
        clip: String,
        looped: bool,
    },
    Stop {
        channel: String,
    },
    SetVolume {
        channel: String,
        millibel: i16,
    },
}

pub trait AudioBackend {
    fn submit(&mut self, command: AudioCommand) -> Result<(), String>;
}

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
            "KeyGen native host requires macOS arm64 (detected {} {}); use --render or --validate for headless checks",
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

    #[test]
    fn host_frame_rejects_wrong_pixel_payload() {
        let viewport = DesignViewport::new(2, 3);
        assert!(HostFrame::new(viewport, 7, vec![0; 23]).is_err());
        let frame = HostFrame::new(viewport, 7, vec![0; 24]).expect("valid RGBA frame");
        assert_eq!(frame.frame, 7);
        assert_eq!(frame.viewport.pixel_count(), 6);
    }

    #[test]
    fn presentation_backend_can_be_tested_without_a_window() {
        struct MemoryBackend {
            state: LifecycleState,
            frames: Vec<u64>,
        }
        impl PresentationBackend for MemoryBackend {
            fn present(&mut self, frame: HostFrame) -> Result<(), String> {
                if !self.state.accepts_frame() {
                    return Err("backend is closed".into());
                }
                self.frames.push(frame.frame);
                Ok(())
            }
            fn lifecycle(&mut self, event: LifecycleEvent) -> Result<(), String> {
                self.state.apply(event);
                Ok(())
            }
            fn lifecycle_state(&self) -> LifecycleState {
                self.state
            }
        }

        let viewport = DesignViewport::new(1, 1);
        let mut backend = MemoryBackend {
            state: LifecycleState::default(),
            frames: Vec::new(),
        };
        backend
            .present(HostFrame::new(viewport, 3, vec![255; 4]).unwrap())
            .unwrap();
        backend.lifecycle(LifecycleEvent::QuitRequested).unwrap();
        backend.lifecycle(LifecycleEvent::Closed).unwrap();
        assert_eq!(backend.lifecycle_state(), LifecycleState::Closed);
        assert!(backend
            .present(HostFrame::new(viewport, 4, vec![0; 4]).unwrap())
            .is_err());
        assert_eq!(backend.frames, vec![3]);
    }
}

//! Platform-neutral contracts consumed by the native macOS host.
//!
//! The current presentation backend is minifb. This module deliberately keeps
//! native event translation and lifecycle policy independent of that backend,
//! so a future Metal/AppKit adapter can consume the same events without
//! changing the engine or story state.

use keygen_engine::input::{Device, InputEvent, InputEventKind, Key, PointerButton};
#[cfg(target_os = "macos")]
use std::process::Command;
use std::{
    collections::BTreeMap,
    collections::VecDeque,
    path::{Path, PathBuf},
    process::Child,
};

pub const SUPPORTED_OS: &str = "macOS";
pub const SUPPORTED_ARCH: &str = "arm64";

/// Apply geometry to the already-created native window. Keeping this adapter
/// separate from the deterministic renderer lets a bounds request preserve
/// the window's identity, input callback, focus, and application state.
pub fn set_window_bounds(
    title: &str,
    x: i32,
    y: i32,
    width: usize,
    height: usize,
) -> Result<(), String> {
    if title.is_empty() || width == 0 || height == 0 {
        return Err("window bounds require a title and positive dimensions".into());
    }
    #[cfg(target_os = "macos")]
    {
        let script = format!(
            "tell application \"System Events\" to tell (first process whose frontmost is true) to set position of first window whose name is {} to {{{}, {}}}; set size of first window whose name is {} to {{{}, {}}}",
            applescript_string(title),
            x,
            y,
            applescript_string(title),
            width,
            height
        );
        let result = Command::new("/usr/bin/osascript")
            .args(["-e", &script])
            .output()
            .map_err(|error| format!("cannot apply native window bounds: {error}"))?;
        if !result.status.success() {
            return Err("native window bounds request was refused".into());
        }
        return Ok(());
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (title, x, y, width, height);
        Err("in-place native window bounds are unsupported on this host".into())
    }
}

#[cfg(target_os = "macos")]
fn applescript_string(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

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

/// The platform capability selected by the host.  This is deliberately a
/// value, rather than a compile-time assumption, so the same runtime can be
/// qualified headlessly in CI and launched natively on Apple Silicon.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MacOsBackendKind {
    /// Existing software window backend. It is the portable fallback used by
    /// the current player executable.
    Minifb,
    /// Reserved for the AppKit/Metal adapter. Keeping this explicit prevents
    /// the fallback from being mistaken for a native renderer.
    AppKitMetal,
}

impl MacOsBackendKind {
    /// Select the backend currently compiled into the player.  The minifb
    /// backend is itself a native Cocoa window on macOS; keeping selection
    /// explicit prevents silently claiming a Metal renderer that is not
    /// linked into this binary yet.
    pub const fn compiled() -> Self {
        Self::Minifb
    }
}

/// Safe macOS launch adapter. It describes a bundle launch without invoking
/// shell commands or linking Cocoa. The eventual AppKit host can consume the
/// same argv/environment contract, while tests can validate it on any host.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MacOsLaunchAdapter {
    pub bundle: std::path::PathBuf,
    pub backend: MacOsBackendKind,
}

impl MacOsLaunchAdapter {
    pub fn new(bundle: impl Into<std::path::PathBuf>) -> Self {
        Self {
            bundle: bundle.into(),
            backend: MacOsBackendKind::Minifb,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        let name = self
            .bundle
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        if !name.ends_with(".app") {
            return Err(format!(
                "macOS launch target must be an .app bundle: {}",
                self.bundle.display()
            ));
        }
        let executable = self.bundle.join("Contents/MacOS");
        if !executable.is_dir() {
            return Err(format!(
                "macOS bundle is missing Contents/MacOS: {}",
                self.bundle.display()
            ));
        }
        Ok(())
    }

    /// Returns the executable path for an external launcher or test harness.
    /// No process is started here; ownership of process lifecycle stays with
    /// the application host.
    pub fn executable(&self) -> Result<std::path::PathBuf, String> {
        self.validate()?;
        let mut entries = std::fs::read_dir(self.bundle.join("Contents/MacOS"))
            .map_err(|e| format!("cannot inspect macOS executable directory: {e}"))?;
        entries
            .find_map(|entry| entry.ok().map(|e| e.path()))
            .ok_or_else(|| "macOS bundle has no executable in Contents/MacOS".into())
    }

    /// Launch the bundle through LaunchServices.  This is the real macOS
    /// process handoff used by external launchers; rendering remains owned by
    /// the executable inside the bundle.  It is intentionally unavailable on
    /// non-macOS hosts so CI cannot accidentally launch an arbitrary file.
    #[cfg(target_os = "macos")]
    pub fn launch(&self) -> Result<Child, String> {
        self.validate()?;
        Command::new("open")
            .args(["-n", "-W"])
            .arg(&self.bundle)
            .spawn()
            .map_err(|error| format!("cannot launch macOS app {}: {error}", self.bundle.display()))
    }

    /// Launching an app bundle is a macOS-only operation.
    #[cfg(not(target_os = "macos"))]
    pub fn launch(&self) -> Result<Child, String> {
        Err("macOS app launch requires a macOS host".into())
    }
}

/// A concrete, safe presentation backend used by native-host qualification.
/// It accepts frames and lifecycle events exactly as an AppKit/Metal backend
/// would, but retains the latest frame in memory. This makes host integration
/// testable without unsafe FFI while the actual renderer remains minifb.
#[derive(Clone, Debug, Default)]
pub struct MacOsQualificationBackend {
    state: LifecycleState,
    latest: Option<HostFrame>,
}

impl MacOsQualificationBackend {
    pub fn latest_frame(&self) -> Option<&HostFrame> {
        self.latest.as_ref()
    }
}

impl PresentationBackend for MacOsQualificationBackend {
    fn present(&mut self, frame: HostFrame) -> Result<(), String> {
        if !self.state.accepts_frame() {
            return Err("macOS backend is not presenting".into());
        }
        self.latest = Some(frame);
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

/// Generic launch identity passed from a bundle or an embedding host.  The
/// engine never infers a product name or a product-specific route from argv.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HostLaunchSpec {
    pub target: String,
    pub bundle_id: String,
    pub initial_route: String,
    pub restore_session: bool,
}

impl HostLaunchSpec {
    pub fn new(target: impl Into<String>, bundle_id: impl Into<String>) -> Self {
        Self {
            target: target.into(),
            bundle_id: bundle_id.into(),
            initial_route: "boot".into(),
            restore_session: true,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.target.is_empty() || self.bundle_id.is_empty() {
            return Err("host launch identity must not be empty".into());
        }
        if self.initial_route.is_empty() {
            return Err("host launch route must not be empty".into());
        }
        Ok(())
    }
}

/// Coordinates lifecycle events with persistence without coupling the player
/// to AppKit. A native host supplies the save callback and may then translate
/// the returned decision into its window/application API.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum QuitDecision {
    Continue,
    Quit,
}

pub trait SaveBarrier {
    fn flush(&mut self) -> Result<(), String>;
}

pub struct HostLifecycle<S> {
    state: LifecycleState,
    save: S,
}

impl<S: SaveBarrier> HostLifecycle<S> {
    pub fn new(save: S) -> Self {
        Self {
            state: LifecycleState::Active,
            save,
        }
    }

    pub fn state(&self) -> LifecycleState {
        self.state
    }

    pub fn event(&mut self, event: LifecycleEvent) -> Result<(), String> {
        if event == LifecycleEvent::QuitRequested {
            self.save.flush()?;
        }
        self.state.apply(event);
        Ok(())
    }

    pub fn request_quit(&mut self) -> Result<QuitDecision, String> {
        self.save.flush()?;
        self.state.apply(LifecycleEvent::QuitRequested);
        Ok(QuitDecision::Quit)
    }
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

/// Logical text-input events understood by the engine.  A Cocoa adapter can
/// translate `NSTextInputClient` callbacks into this contract without making
/// the story/runtime depend on AppKit.  Strings are owned and bounded by the
/// adapter before they cross the host boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TextInputEvent {
    CompositionStarted,
    CompositionUpdated(String),
    CompositionCommitted(String),
    CompositionCancelled,
}

/// Safe, platform-neutral composition buffer.  This is deliberately an
/// event adapter rather than a claim that the current minifb window accepts
/// IME input: `native_capabilities().text_input` remains false until a real
/// AppKit text-input client is connected to it.
#[derive(Clone, Debug)]
pub struct MacOsTextInputAdapter {
    max_bytes: usize,
    composition: String,
    events: VecDeque<TextInputEvent>,
}

impl MacOsTextInputAdapter {
    pub const DEFAULT_MAX_BYTES: usize = 4096;

    pub fn new(max_bytes: usize) -> Result<Self, String> {
        if max_bytes == 0 {
            return Err("text-input maximum must be greater than zero".into());
        }
        Ok(Self {
            max_bytes,
            composition: String::new(),
            events: VecDeque::new(),
        })
    }

    pub fn with_default_limit() -> Self {
        Self::new(Self::DEFAULT_MAX_BYTES).expect("default text-input limit is non-zero")
    }

    pub fn max_bytes(&self) -> usize {
        self.max_bytes
    }

    pub fn composition(&self) -> &str {
        &self.composition
    }

    pub fn begin_composition(&mut self) {
        self.composition.clear();
        self.events.push_back(TextInputEvent::CompositionStarted);
    }

    pub fn update_composition(&mut self, text: &str) {
        self.composition = bounded_text(text, self.max_bytes);
        self.events
            .push_back(TextInputEvent::CompositionUpdated(self.composition.clone()));
    }

    pub fn commit(&mut self, text: &str) {
        let committed = bounded_text(text, self.max_bytes);
        self.composition.clear();
        self.events
            .push_back(TextInputEvent::CompositionCommitted(committed));
    }

    pub fn cancel(&mut self) {
        self.composition.clear();
        self.events.push_back(TextInputEvent::CompositionCancelled);
    }

    pub fn drain_events(&mut self) -> impl Iterator<Item = TextInputEvent> + '_ {
        self.events.drain(..)
    }
}

fn bounded_text(text: &str, max_bytes: usize) -> String {
    if text.len() <= max_bytes {
        return text.to_owned();
    }
    let mut end = 0;
    for (index, character) in text.char_indices() {
        if index + character.len_utf8() > max_bytes {
            break;
        }
        end = index + character.len_utf8();
    }
    text[..end].to_owned()
}

/// System audio adapter for macOS. `afplay` is a shipped macOS client of the
/// CoreAudio stack, so this keeps the player free of a second decoder or an
/// unsafe FFI layer while still sending real audio to the system device.
/// Clips are resolved by the package boundary; the engine only deals in
/// logical clip IDs.
#[derive(Debug, Default)]
pub struct MacOsSystemAudioBackend {
    clips: BTreeMap<String, PathBuf>,
    players: BTreeMap<String, Child>,
}

impl MacOsSystemAudioBackend {
    pub fn new<I, K, P>(clips: I) -> Self
    where
        I: IntoIterator<Item = (K, P)>,
        K: Into<String>,
        P: Into<PathBuf>,
    {
        Self {
            clips: clips
                .into_iter()
                .map(|(k, p)| (k.into(), p.into()))
                .collect(),
            players: BTreeMap::new(),
        }
    }

    pub fn registered_clip(&self, id: &str) -> Option<&Path> {
        self.clips.get(id).map(PathBuf::as_path)
    }

    pub fn stop_all(&mut self) {
        for (_, mut player) in std::mem::take(&mut self.players) {
            let _ = player.kill();
        }
    }
}

impl Drop for MacOsSystemAudioBackend {
    fn drop(&mut self) {
        self.stop_all();
    }
}

impl AudioBackend for MacOsSystemAudioBackend {
    fn submit(&mut self, command: AudioCommand) -> Result<(), String> {
        match command {
            AudioCommand::Stop { channel } => {
                if let Some(mut player) = self.players.remove(&channel) {
                    let _ = player.kill();
                }
                Ok(())
            }
            AudioCommand::SetVolume { .. } => {
                Err("macOS afplay adapter does not support per-channel volume; use the engine mix before submission".into())
            }
            AudioCommand::Play { channel, clip, looped } => {
                let path = self
                    .clips
                    .get(&clip)
                    .ok_or_else(|| format!("audio clip is not registered: {clip}"))?;
                if !path.is_file() {
                    return Err(format!("audio clip does not exist: {}", path.display()));
                }
                if let Some(mut player) = self.players.remove(&channel) {
                    let _ = player.kill();
                }
                if !cfg!(target_os = "macos") {
                    return Err("CoreAudio playback requires macOS".into());
                }
                let mut command = std::process::Command::new("afplay");
                if looped {
                    command.arg("-R");
                }
                let player = command
                    .arg(path)
                    .spawn()
                    .map_err(|error| format!("cannot start CoreAudio afplay adapter: {error}"))?;
                self.players.insert(channel, player);
                Ok(())
            }
        }
    }
}

/// Capabilities are reported rather than inferred from the fallback window.
/// This prevents a software Cocoa window from being mistaken for a Metal
/// renderer by packaging and qualification tools.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeHostCapabilities {
    pub appkit_window: bool,
    pub metal_renderer: bool,
    pub core_audio: bool,
    pub text_input: bool,
}

pub const fn native_capabilities() -> NativeHostCapabilities {
    NativeHostCapabilities {
        appkit_window: cfg!(target_os = "macos"),
        metal_renderer: false,
        core_audio: cfg!(target_os = "macos"),
        text_input: false,
    }
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
    // Window backends can briefly report a zero-sized drawable while a resize
    // or display transition is in flight. Keep input finite and deterministic
    // during that interval instead of producing NaNs from a zero scale.
    if window.0 == 0 || window.1 == 0 || design.0 == 0 || design.1 == 0 {
        return (0.0, 0.0);
    }
    let scale = (window.0 as f32 / design.0 as f32).min(window.1 as f32 / design.1 as f32);
    let offset_x = (window.0 as f32 - design.0 as f32 * scale) * 0.5;
    let offset_y = (window.1 as f32 - design.1 as f32 * scale) * 0.5;
    (
        (pointer.0 - offset_x) / scale,
        (pointer.1 - offset_y) / scale,
    )
}

/// Native window defaults shared by all packaged scenes. The drawable keeps
/// the design viewport's aspect ratio while remaining freely resizable; the
/// minifb backend then letterboxes the unused axis. Keeping this policy in the
/// host adapter prevents individual scenes from accidentally choosing a
/// different stretch mode.
pub fn window_options(borderless: bool) -> minifb::WindowOptions {
    minifb::WindowOptions {
        borderless,
        title: !borderless,
        resize: !borderless,
        scale: minifb::Scale::X1,
        scale_mode: minifb::ScaleMode::AspectRatioStretch,
        ..minifb::WindowOptions::default()
    }
}

/// Ask AppKit for the active display's logical point size without linking the
/// deterministic engine to platform APIs. JXA is a system-provided, bounded
/// host adapter; it emits exactly `width,height` or fails closed.
#[cfg(target_os = "macos")]
pub fn active_display_size() -> Result<Option<(usize, usize)>, String> {
    let output = std::process::Command::new("/usr/bin/osascript")
        .args([
            "-l",
            "JavaScript",
            "-e",
            "ObjC.import('AppKit'); const f=$.NSScreen.mainScreen.frame; String(Number(f.size.width)+','+Number(f.size.height))",
        ])
        .output()
        .map_err(|error| format!("cannot query active macOS display: {error}"))?;
    if !output.status.success() {
        return Err("cannot query active macOS display".into());
    }
    let value = String::from_utf8(output.stdout)
        .map_err(|_| "active macOS display query returned invalid UTF-8")?;
    let (width, height) = value
        .trim()
        .split_once(',')
        .ok_or("active macOS display query returned invalid geometry")?;
    let size = (
        width
            .parse::<usize>()
            .map_err(|_| "active macOS display query returned invalid width")?,
        height
            .parse::<usize>()
            .map_err(|_| "active macOS display query returned invalid height")?,
    );
    if size.0 == 0 || size.1 == 0 {
        return Err("active macOS display query returned empty geometry".into());
    }
    Ok(Some(size))
}

#[cfg(not(target_os = "macos"))]
pub fn active_display_size() -> Result<Option<(usize, usize)>, String> {
    Ok(None)
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
    fn pointer_mapping_handles_zero_drawable_during_resize() {
        assert_eq!(map_pointer((12.0, 8.0), (0, 0), (1280, 720)), (0.0, 0.0));
        assert_eq!(map_pointer((12.0, 8.0), (1280, 720), (0, 0)), (0.0, 0.0));
    }

    #[test]
    fn window_policy_is_explicit_for_immersive_and_windowed_modes() {
        let immersive = window_options(true);
        assert!(immersive.borderless);
        assert!(!immersive.title);
        assert!(!immersive.resize);
        assert!(matches!(immersive.scale, minifb::Scale::X1));
        assert_eq!(immersive.scale_mode, minifb::ScaleMode::AspectRatioStretch);

        let windowed = window_options(false);
        assert!(!windowed.borderless);
        assert!(windowed.title);
        assert!(windowed.resize);
        assert!(matches!(windowed.scale, minifb::Scale::X1));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn active_display_query_returns_positive_logical_geometry() {
        let size = active_display_size().unwrap().unwrap();
        assert!(size.0 > 0 && size.1 > 0);
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
    fn text_input_preserves_composition_lifecycle_and_bounds_utf8() {
        let mut input = MacOsTextInputAdapter::new(4).unwrap();
        input.begin_composition();
        input.update_composition("ab😀");
        input.commit("日本語");
        assert_eq!(input.composition(), "");
        assert_eq!(
            input.drain_events().collect::<Vec<_>>(),
            vec![
                TextInputEvent::CompositionStarted,
                TextInputEvent::CompositionUpdated("ab".into()),
                TextInputEvent::CompositionCommitted("日".into()),
            ]
        );
    }

    #[test]
    fn text_input_cancel_discards_pending_composition() {
        let mut input = MacOsTextInputAdapter::with_default_limit();
        input.begin_composition();
        input.update_composition("候補");
        input.cancel();
        assert_eq!(input.composition(), "");
        assert!(matches!(
            input.drain_events().last(),
            Some(TextInputEvent::CompositionCancelled)
        ));
        assert!(MacOsTextInputAdapter::new(0).is_err());
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

    #[test]
    fn launch_spec_is_product_neutral_and_validated() {
        let spec = HostLaunchSpec::new("sample", "org.example.sample");
        assert_eq!(spec.initial_route, "boot");
        assert!(spec.validate().is_ok());
        assert!(HostLaunchSpec::new("", "org.example.sample")
            .validate()
            .is_err());
    }

    #[test]
    fn quit_flushes_before_transitioning() {
        struct Save {
            flushed: bool,
        }
        impl SaveBarrier for Save {
            fn flush(&mut self) -> Result<(), String> {
                self.flushed = true;
                Ok(())
            }
        }
        let mut host = HostLifecycle::new(Save { flushed: false });
        assert_eq!(host.request_quit(), Ok(QuitDecision::Quit));
        assert_eq!(host.state(), LifecycleState::Terminating);
        assert!(host.save.flushed);
    }

    #[test]
    fn qualification_backend_retains_latest_frame_and_closes() {
        let viewport = DesignViewport::new(1, 1);
        let mut backend = MacOsQualificationBackend::default();
        backend
            .present(HostFrame::new(viewport, 9, vec![1, 2, 3, 4]).unwrap())
            .unwrap();
        assert_eq!(backend.latest_frame().map(|frame| frame.frame), Some(9));
        backend.lifecycle(LifecycleEvent::Closed).unwrap();
        assert!(backend
            .present(HostFrame::new(viewport, 10, vec![0; 4]).unwrap())
            .is_err());
    }

    #[test]
    fn launch_adapter_rejects_non_app_bundle() {
        let adapter = MacOsLaunchAdapter::new("/tmp/keygen");
        assert!(adapter.validate().is_err());
    }

    #[test]
    fn compiled_backend_is_explicitly_the_native_window_fallback() {
        assert_eq!(MacOsBackendKind::compiled(), MacOsBackendKind::Minifb);
    }

    #[test]
    fn native_capabilities_do_not_claim_unimplemented_metal_or_text_input() {
        let capabilities = native_capabilities();
        assert!(!capabilities.metal_renderer);
        assert!(!capabilities.text_input);
    }

    #[test]
    fn system_audio_requires_registered_existing_clip() {
        let mut audio = MacOsSystemAudioBackend::new([("theme", "/definitely/missing/theme.wav")]);
        assert!(audio
            .submit(AudioCommand::Play {
                channel: "music".into(),
                clip: "unknown".into(),
                looped: false,
            })
            .is_err());
        assert!(audio
            .submit(AudioCommand::Play {
                channel: "music".into(),
                clip: "theme".into(),
                looped: false,
            })
            .is_err());
    }

    #[test]
    fn system_audio_stop_is_idempotent() {
        let mut audio = MacOsSystemAudioBackend::default();
        audio
            .submit(AudioCommand::Stop {
                channel: "music".into(),
            })
            .unwrap();
        audio.stop_all();
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn launch_adapter_does_not_launch_on_non_macos() {
        let adapter = MacOsLaunchAdapter::new("/tmp/keygen.app");
        assert!(adapter.launch().is_err());
    }
}

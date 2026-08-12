//! Typed launcher application contracts (KGD-230..237).
//!
//! These reducers deliberately know nothing about DDLC content or host I/O. A launcher
//! adapter supplies a virtual filesystem, unlock projection, and audio command sink.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum AppId {
    FileBrowser,
    FileViewer,
    Mail,
    SideStories,
    Gallery,
    Jukebox,
    Settings,
    Vm,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum VirtualEntryKind { File, Directory }

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct VirtualEntry {
    pub path: String,
    pub label: String,
    pub kind: VirtualEntryKind,
    pub size: u64,
    pub media: Option<MediaKind>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum MediaKind { Text, Sprite, Audio, Unknown }

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct VirtualFileSystem {
    pub entries: BTreeMap<String, VirtualEntry>,
    pub text: BTreeMap<String, String>,
}

impl VirtualFileSystem {
    pub fn children(&self, directory: &str) -> Vec<VirtualEntry> {
        let prefix = if directory == "/" { "/".to_owned() } else { format!("{}/", directory.trim_end_matches('/')) };
        self.entries.values().filter(|entry| entry.path.starts_with(&prefix) && !entry.path[prefix.len()..].contains('/')).cloned().collect()
    }
    pub fn entry(&self, path: &str) -> Option<&VirtualEntry> { self.entries.get(path) }
    pub fn read_text(&self, path: &str) -> Option<&str> { self.text.get(path).map(String::as_str) }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct UnlockState { pub unlocked: BTreeSet<String> }

impl UnlockState {
    pub fn allows(&self, key: &str) -> bool { self.unlocked.contains(key) }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AudioTrack { pub id: String, pub looped: bool }

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum AudioIntent { Play(AudioTrack), Pause, Stop, Queue(AudioTrack) }

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum AppEffect {
    Audio(AudioIntent),
    Open(AppId),
    Close,
    Notification(String),
    ResetVirtualMachine,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LaunchRequest { pub app: AppId, pub parameter: Option<String> }

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum AppAction {
    Select(usize),
    Open,
    Back,
    Delete,
    Reset,
    Search(String),
    SetPreference { key: String, value: PreferenceValue },
    Play,
    Pause,
    Queue(usize),
    Launch(LaunchRequest),
    Confirm,
    Cancel,
    Tick,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum PreferenceValue { Bool(bool), Number(i32), Choice(String) }

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct FileBrowserState { pub directory: String, pub rows: Vec<String>, pub selected: usize, pub pending_delete: Option<String> }

impl FileBrowserState {
    pub fn new(fs: &VirtualFileSystem) -> Self { let mut state = Self { directory: "/".into(), ..Self::default() }; state.refresh(fs); state }
    pub fn refresh(&mut self, fs: &VirtualFileSystem) { self.rows = fs.children(&self.directory).into_iter().map(|e| e.path).collect(); self.selected = self.selected.min(self.rows.len().saturating_sub(1)); }
    pub fn dispatch(&mut self, action: AppAction, fs: &VirtualFileSystem, unlocks: &UnlockState) -> Vec<AppEffect> {
        match action {
            AppAction::Select(index) if index < self.rows.len() => self.selected = index,
            AppAction::Open => if let Some(path) = self.rows.get(self.selected) { if let Some(entry) = fs.entry(path) { match entry.kind { VirtualEntryKind::Directory => { self.directory = path.clone(); self.refresh(fs); }, VirtualEntryKind::File => return vec![AppEffect::Open(AppId::FileViewer)], } } },
            AppAction::Delete => if let Some(path) = self.rows.get(self.selected) { if unlocks.allows("file-delete") { self.pending_delete = Some(path.clone()); } },
            AppAction::Confirm => { self.pending_delete = None; },
            AppAction::Cancel => self.pending_delete = None,
            AppAction::Back => if self.directory != "/" { self.directory = "/".into(); self.refresh(fs); },
            _ => {}
        }
        Vec::new()
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct FileViewerState { pub path: Option<String>, pub mode: Option<MediaKind>, pub content: Option<String> }

impl FileViewerState {
    pub fn open(&mut self, path: &str, fs: &VirtualFileSystem) -> bool { let Some(entry) = fs.entry(path) else { return false }; self.path = Some(path.into()); self.mode = entry.media; self.content = fs.read_text(path).map(str::to_owned); true }
    pub fn dispatch(&mut self, action: AppAction) -> Vec<AppEffect> { if matches!(action, AppAction::Back) { self.path = None; self.mode = None; self.content = None; } Vec::new() }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct MailState { pub messages: Vec<String>, pub selected: usize, pub last_viewed: Option<String>, pub notified: BTreeSet<String> }

impl MailState {
    pub fn refresh(&mut self, fs: &VirtualFileSystem, unlocks: &UnlockState) { self.messages = fs.entries.values().filter(|e| e.path.starts_with("/mail/") && e.kind == VirtualEntryKind::File && e.media == Some(MediaKind::Text) && unlocks.allows(e.path.as_str())).map(|e| e.path.clone()).collect(); self.selected = self.selected.min(self.messages.len().saturating_sub(1)); }
    pub fn dispatch(&mut self, action: AppAction) -> Vec<AppEffect> { match action { AppAction::Open => if let Some(path) = self.messages.get(self.selected) { self.last_viewed = Some(path.clone()); if self.notified.insert(path.clone()) { return vec![AppEffect::Notification("new-mail-read".into())]; } }, AppAction::Select(i) if i < self.messages.len() => self.selected = i, _ => {} } Vec::new() }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SideStoriesState { pub stories: Vec<String>, pub selected: usize, pub pending: Option<String> }
impl SideStoriesState {
    pub fn refresh(&mut self, unlocks: &UnlockState) { self.stories = unlocks.unlocked.iter().filter(|key| key.starts_with("story:")).cloned().collect(); self.selected = self.selected.min(self.stories.len().saturating_sub(1)); }
    pub fn dispatch(&mut self, action: AppAction) -> Vec<AppEffect> { match action { AppAction::Select(i) if i < self.stories.len() => self.selected = i, AppAction::Open => self.pending = self.stories.get(self.selected).cloned(), AppAction::Confirm => if let Some(id) = self.pending.take() { return vec![AppEffect::Open(AppId::SideStories), AppEffect::Notification(format!("launch:{id}"))]; }, AppAction::Cancel => self.pending = None, _ => {} } Vec::new() }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct GalleryState { pub categories: Vec<String>, pub selected: usize, pub selected_image: Option<String> }
impl GalleryState {
    pub fn refresh(&mut self, unlocks: &UnlockState) { self.categories = unlocks.unlocked.iter().filter(|key| key.starts_with("gallery:")).cloned().collect(); self.selected = self.selected.min(self.categories.len().saturating_sub(1)); }
    pub fn dispatch(&mut self, action: AppAction) { match action { AppAction::Select(i) if i < self.categories.len() => { self.selected = i; self.selected_image = self.categories.get(i).cloned(); }, AppAction::Back => self.selected_image = None, _ => {} } }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct JukeboxState { pub tracks: Vec<AudioTrack>, pub selected: usize, pub queued: Vec<usize>, pub playing: bool }
impl JukeboxState {
    pub fn dispatch(&mut self, action: AppAction) -> Vec<AppEffect> { match action { AppAction::Select(i) if i < self.tracks.len() => self.selected = i, AppAction::Play => if let Some(track) = self.tracks.get(self.selected).cloned() { self.playing = true; return vec![AppEffect::Audio(AudioIntent::Play(track))]; }, AppAction::Pause => { self.playing = false; return vec![AppEffect::Audio(AudioIntent::Pause)]; }, AppAction::Queue(i) if i < self.tracks.len() => { self.queued.push(i); return vec![AppEffect::Audio(AudioIntent::Queue(self.tracks[i].clone()))]; }, _ => {} } Vec::new() }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SettingsState { pub values: BTreeMap<String, PreferenceValue>, pub dirty: bool }
impl SettingsState {
    pub fn dispatch(&mut self, action: AppAction) -> Vec<AppEffect> { let closes = matches!(action, AppAction::Back | AppAction::Confirm); if let AppAction::SetPreference { key, value } = action { self.values.insert(key, value); self.dirty = true; } if closes { self.dirty = false; } Vec::new() }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct VmState { pub prompt: String, pub output: Vec<String>, pub reset_pending: bool, pub seed: u64 }
impl VmState {
    pub fn dispatch(&mut self, action: AppAction, unlocks: &mut UnlockState) -> Vec<AppEffect> { match action { AppAction::Search(command) => { self.output.push(format!("{}:{}", self.seed, command)); }, AppAction::Reset => if unlocks.allows("vm-reset") { self.reset_pending = true; }, AppAction::Confirm if self.reset_pending => { self.output.clear(); self.reset_pending = false; return vec![AppEffect::ResetVirtualMachine, AppEffect::Audio(AudioIntent::Stop)]; }, AppAction::Cancel => self.reset_pending = false, _ => {} } Vec::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fs() -> VirtualFileSystem { VirtualFileSystem { entries: [("/docs".into(), VirtualEntry { path: "/docs".into(), label: "Docs".into(), kind: VirtualEntryKind::Directory, size: 0, media: None }), ("/docs/readme".into(), VirtualEntry { path: "/docs/readme".into(), label: "Readme".into(), kind: VirtualEntryKind::File, size: 2, media: Some(MediaKind::Text) })].into_iter().collect(), text: [("/docs/readme".into(), "ok".into())].into_iter().collect() } }
    #[test] fn browser_opens_and_viewer_reads_virtual_file() { let fs = fs(); let mut browser = FileBrowserState::new(&fs); browser.dispatch(AppAction::Select(0), &fs, &UnlockState::default()); let effects = browser.dispatch(AppAction::Open, &fs, &UnlockState::default()); assert!(effects.is_empty()); browser.dispatch(AppAction::Open, &fs, &UnlockState::default()); let mut viewer = FileViewerState::default(); assert!(viewer.open("/docs/readme", &fs)); assert_eq!(viewer.content.as_deref(), Some("ok")); }
    #[test] fn unlock_projection_controls_apps() { let mut unlocks = UnlockState::default(); unlocks.unlocked.extend(["story:a".into(), "gallery:image".into()]); let mut stories = SideStoriesState::default(); stories.refresh(&unlocks); assert_eq!(stories.stories, vec!["story:a"]); let mut gallery = GalleryState::default(); gallery.refresh(&unlocks); assert_eq!(gallery.categories, vec!["gallery:image"]); }
    #[test] fn jukebox_emits_audio_intents() { let mut state = JukeboxState { tracks: vec![AudioTrack { id: "synthetic".into(), looped: true }], ..Default::default() }; assert_eq!(state.dispatch(AppAction::Play), vec![AppEffect::Audio(AudioIntent::Play(AudioTrack { id: "synthetic".into(), looped: true }))]); }
    #[test] fn vm_reset_is_gated_and_deterministic() { let mut vm = VmState { seed: 7, ..Default::default() }; let mut unlocks = UnlockState::default(); vm.dispatch(AppAction::Search("status".into()), &mut unlocks); assert!(vm.dispatch(AppAction::Reset, &mut unlocks).is_empty()); unlocks.unlocked.insert("vm-reset".into()); vm.dispatch(AppAction::Reset, &mut unlocks); assert_eq!(vm.dispatch(AppAction::Confirm, &mut unlocks), vec![AppEffect::ResetVirtualMachine, AppEffect::Audio(AudioIntent::Stop)]); }
}

//! Atomic, versioned player-side persistence. Logical paths are sandboxed
//! beneath the product data directory; callers never receive arbitrary paths.
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    path::{Component, Path, PathBuf},
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoreMetadata {
    pub schema: String,
    pub revision: u64,
    pub checksum: String,
    pub bytes: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Envelope {
    metadata: StoreMetadata,
    payload: Vec<u8>,
}
#[derive(Debug)]
pub struct AtomicStore {
    root: PathBuf,
}
impl AtomicStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn logical_path(&self, logical: &str) -> Result<PathBuf, String> {
        let path = Path::new(logical);
        if logical.is_empty()
            || path.is_absolute()
            || path
                .components()
                .any(|c| !matches!(c, Component::Normal(_)))
        {
            return Err(format!("invalid sandbox path: {logical}"));
        }
        Ok(self.root.join(path))
    }
    pub fn save<T: Serialize>(
        &self,
        logical: &str,
        schema: &str,
        revision: u64,
        value: &T,
    ) -> Result<StoreMetadata, String> {
        let payload = serde_json::to_vec(value).map_err(|e| format!("serialize store: {e}"))?;
        self.save_bytes(logical, schema, revision, &payload)
    }
    pub fn save_bytes(
        &self,
        logical: &str,
        schema: &str,
        revision: u64,
        payload: &[u8],
    ) -> Result<StoreMetadata, String> {
        let destination = self.logical_path(logical)?;
        let checksum = digest(payload);
        let metadata = StoreMetadata {
            schema: schema.into(),
            revision,
            checksum,
            bytes: payload.len() as u64,
        };
        let bytes = serde_json::to_vec(&Envelope {
            metadata: metadata.clone(),
            payload: payload.to_vec(),
        })
        .map_err(|e| format!("encode store: {e}"))?;
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("create store directory: {e}"))?;
        }
        let temp = destination.with_extension("tmp");
        let mut file = fs::File::create(&temp).map_err(|e| format!("create store temp: {e}"))?;
        file.write_all(&bytes)
            .map_err(|e| format!("write store temp: {e}"))?;
        file.sync_all()
            .map_err(|e| format!("sync store temp: {e}"))?;
        fs::rename(&temp, &destination).map_err(|e| format!("commit store: {e}"))?;
        Ok(metadata)
    }
    pub fn load<T: DeserializeOwned>(
        &self,
        logical: &str,
        schema: &str,
    ) -> Result<(StoreMetadata, T), String> {
        let (m, b) = self.load_bytes(logical, schema)?;
        let value = serde_json::from_slice(&b).map_err(|e| format!("decode store: {e}"))?;
        Ok((m, value))
    }
    pub fn load_bytes(
        &self,
        logical: &str,
        schema: &str,
    ) -> Result<(StoreMetadata, Vec<u8>), String> {
        let path = self.logical_path(logical)?;
        let bytes = fs::read(path).map_err(|e| format!("read store: {e}"))?;
        let envelope: Envelope =
            serde_json::from_slice(&bytes).map_err(|e| format!("decode envelope: {e}"))?;
        if envelope.metadata.schema != schema {
            return Err(format!(
                "store schema mismatch: expected {schema}, got {}",
                envelope.metadata.schema
            ));
        }
        if envelope.metadata.bytes != envelope.payload.len() as u64
            || envelope.metadata.checksum != digest(&envelope.payload)
        {
            return Err("store checksum mismatch".into());
        }
        Ok((envelope.metadata, envelope.payload))
    }
    pub fn remove(&self, logical: &str) -> Result<(), String> {
        let path = self.logical_path(logical)?;
        match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(format!("remove store: {e}")),
        }
    }
}
fn digest(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    format!("{:x}", h.finalize())
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Preferences {
    pub text_speed: f32,
    pub music_volume: f32,
    pub sound_volume: f32,
    pub fullscreen: bool,
    pub language: String,
    pub skip_unseen: bool,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            text_speed: 1.0,
            music_volume: 1.0,
            sound_volume: 1.0,
            fullscreen: false,
            language: "en".into(),
            skip_unseen: false,
        }
    }
}
impl Preferences {
    pub fn validate(&self) -> Result<(), String> {
        if !self.text_speed.is_finite() || self.text_speed <= 0.0 {
            return Err("text speed must be positive".into());
        }
        for (name, v) in [("music", self.music_volume), ("sound", self.sound_volume)] {
            if !v.is_finite() || !(0.0..=1.0).contains(&v) {
                return Err(format!("{name} volume out of range"));
            }
        }
        if self.language.is_empty() {
            return Err("language is empty".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnlockEvent {
    pub id: String,
    pub frame: u64,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Unlocks {
    pub events: Vec<UnlockEvent>,
}
impl Unlocks {
    pub fn record(&mut self, id: impl Into<String>, frame: u64) -> bool {
        let id = id.into();
        if self.events.iter().any(|e| e.id == id) {
            false
        } else {
            self.events.push(UnlockEvent { id, frame });
            self.events.sort_by(|a, b| a.id.cmp(&b.id));
            true
        }
    }
    pub fn contains(&self, id: &str) -> bool {
        self.events.iter().any(|e| e.id == id)
    }
    pub fn reset(&mut self) {
        self.events.clear()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn store() -> (AtomicStore, PathBuf) {
        let root = std::env::temp_dir().join(format!("keygen-store-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        (AtomicStore::new(root.clone()), root)
    }
    #[test]
    fn atomic_metadata_and_checksum() {
        let (s, r) = store();
        let m = s
            .save_bytes("save/slot1.json", "save.v1", 1, &[1, 2, 3])
            .unwrap();
        assert_eq!(m.bytes, 3);
        let (_, v) = s.load_bytes("save/slot1.json", "save.v1").unwrap();
        assert_eq!(v, vec![1, 2, 3]);
        let _ = fs::remove_dir_all(r);
    }
    #[test]
    fn traversal_and_absolute_paths_fail() {
        let (s, _) = store();
        assert!(s.logical_path("../escape").is_err());
        assert!(s.logical_path("/tmp/escape").is_err());
    }
    #[test]
    fn preferences_validate_and_unlocks_are_idempotent() {
        let mut p = Preferences::default();
        assert!(p.validate().is_ok());
        p.music_volume = 2.0;
        assert!(p.validate().is_err());
        let mut u = Unlocks::default();
        assert!(u.record("mail.1", 4));
        assert!(!u.record("mail.1", 9));
        assert!(u.contains("mail.1"));
        u.reset();
        assert!(!u.contains("mail.1"));
    }
}

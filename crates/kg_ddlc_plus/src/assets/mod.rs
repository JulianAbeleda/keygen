//! Content-addressed, provenance-first asset importing for the private target.
//!
//! This module deliberately stores no recovered content in the repository.  A
//! caller supplies a player-owned source file and chooses an output directory;
//! the catalog records how that private package was produced.

use png::{ColorType, Decoder};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Cursor,
    path::{Component, Path, PathBuf},
};

pub const CATALOG_SCHEMA: &str = "keygen.assets.v1";
pub const IMPORTER_VERSION: &str = "kg-ddlc-plus-assets-1";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportMode {
    Copy,
    Translate,
    Reimplement,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AssetRecord {
    pub logical_id: String,
    pub kind: String,
    pub source_sha256: String,
    pub output_sha256: String,
    pub import_mode: ImportMode,
    pub importer_version: String,
    pub blob: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<ImageInfo>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ImageInfo {
    pub width: u32,
    pub height: u32,
    pub color_type: String,
    pub pixel_sha256: String,
    pub alpha_bounds: Option<[u32; 4]>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AssetCatalog {
    pub schema: String,
    pub importer_version: String,
    pub blobs: Vec<String>,
    pub assets: Vec<AssetRecord>,
}

impl AssetCatalog {
    pub fn new() -> Self {
        Self {
            schema: CATALOG_SCHEMA.into(),
            importer_version: IMPORTER_VERSION.into(),
            blobs: Vec::new(),
            assets: Vec::new(),
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema != CATALOG_SCHEMA || self.importer_version.is_empty() {
            return Err("unsupported asset catalog schema or importer version".into());
        }
        let mut seen = std::collections::BTreeSet::new();
        for blob in &self.blobs {
            safe_relative(blob)?;
            if !seen.insert(blob) {
                return Err(format!("duplicate blob: {blob}"));
            }
        }
        let mut ids = std::collections::BTreeSet::new();
        for asset in &self.assets {
            if asset.logical_id.is_empty() || !ids.insert(&asset.logical_id) {
                return Err(format!(
                    "duplicate or empty logical asset id: {}",
                    asset.logical_id
                ));
            }
            for hash in [&asset.source_sha256, &asset.output_sha256] {
                if !is_sha256(hash) {
                    return Err(format!("invalid asset hash for {}", asset.logical_id));
                }
            }
            safe_relative(&asset.blob)?;
            if !self.blobs.iter().any(|blob| blob == &asset.blob) {
                return Err(format!("asset blob is undeclared: {}", asset.blob));
            }
            if asset.importer_version.is_empty() || asset.kind.is_empty() {
                return Err(format!("asset metadata incomplete: {}", asset.logical_id));
            }
            if let Some(image) = &asset.image {
                if image.width == 0 || image.height == 0 || !is_sha256(&image.pixel_sha256) {
                    return Err(format!("invalid image metadata: {}", asset.logical_id));
                }
            }
        }
        Ok(())
    }

    pub fn write(&self, path: &Path) -> Result<(), String> {
        self.validate()?;
        let bytes = serde_json::to_vec_pretty(self).map_err(|e| format!("encode catalog: {e}"))?;
        fs::write(path, bytes).map_err(|e| format!("write {}: {e}", path.display()))
    }
}

impl Default for AssetCatalog {
    fn default() -> Self {
        Self::new()
    }
}

pub struct CatalogWriter {
    root: PathBuf,
    pub catalog: AssetCatalog,
}

impl CatalogWriter {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            catalog: AssetCatalog::new(),
        }
    }

    /// Imports a PNG by preserving its original bytes and recording decoded
    /// RGBA dimensions, pixel hash, and alpha bounds.
    pub fn import_png(&mut self, logical_id: &str, source: &Path) -> Result<AssetRecord, String> {
        let bytes = fs::read(source).map_err(|e| format!("read {}: {e}", source.display()))?;
        let source_sha256 = sha256(&bytes);
        let info = inspect_png(&bytes)?;
        let output_sha256 = source_sha256.clone();
        let blob = format!("blobs/sha256/{output_sha256}.png");
        safe_relative(&blob)?;
        let destination = self.root.join(&blob);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("create {}: {e}", parent.display()))?;
        }
        if !destination.exists() {
            fs::write(&destination, &bytes)
                .map_err(|e| format!("write {}: {e}", destination.display()))?;
        }
        if !self.catalog.blobs.contains(&blob) {
            self.catalog.blobs.push(blob.clone());
        }
        let record = AssetRecord {
            logical_id: logical_id.into(),
            kind: "image".into(),
            source_sha256,
            output_sha256,
            import_mode: ImportMode::Copy,
            importer_version: IMPORTER_VERSION.into(),
            blob,
            image: Some(info),
        };
        self.catalog.assets.push(record.clone());
        Ok(record)
    }
}

fn inspect_png(bytes: &[u8]) -> Result<ImageInfo, String> {
    let decoder = Decoder::new(Cursor::new(bytes));
    let mut reader = decoder
        .read_info()
        .map_err(|e| format!("decode PNG: {e}"))?;
    let mut buffer = vec![0; reader.output_buffer_size()];
    let output = reader
        .next_frame(&mut buffer)
        .map_err(|e| format!("decode PNG frame: {e}"))?;
    let pixels = &buffer[..output.buffer_size()];
    let rgba = match output.color_type {
        ColorType::Rgba => pixels.to_vec(),
        ColorType::Rgb => pixels
            .chunks_exact(3)
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        ColorType::Grayscale => pixels.iter().flat_map(|p| [*p, *p, *p, 255]).collect(),
        ColorType::GrayscaleAlpha => pixels
            .chunks_exact(2)
            .flat_map(|p| [p[0], p[0], p[0], p[1]])
            .collect(),
        ColorType::Indexed => {
            return Err("indexed PNGs require palette translation before import".into())
        }
    };
    let mut bounds: Option<[u32; 4]> = None;
    for (index, alpha) in rgba.chunks_exact(4).map(|p| p[3]).enumerate() {
        if alpha != 0 {
            let x = (index as u32) % output.width;
            let y = (index as u32) / output.width;
            bounds = Some(match bounds {
                Some([l, t, r, b]) => [l.min(x), t.min(y), r.max(x), b.max(y)],
                None => [x, y, x, y],
            });
        }
    }
    Ok(ImageInfo {
        width: output.width,
        height: output.height,
        color_type: format!("{:?}", output.color_type),
        pixel_sha256: sha256(&rgba),
        alpha_bounds: bounds,
    })
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}
fn safe_relative(value: &str) -> Result<(), String> {
    let path = Path::new(value);
    if value.is_empty()
        || path.is_absolute()
        || path.components().any(|c| {
            matches!(
                c,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(format!("unsafe asset path: {value}"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn png() -> Vec<u8> {
        let mut bytes = Vec::new();
        let mut encoder = png::Encoder::new(&mut bytes, 1, 1);
        encoder.set_color(ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&[255, 0, 0, 255]).unwrap();
        drop(writer);
        bytes
    }
    #[test]
    fn png_import_records_pixels_and_deduplicates_blob() {
        let dir = std::env::temp_dir().join(format!("kg-assets-{}", std::process::id()));
        let source = dir.join("fixture.png");
        fs::create_dir_all(&dir).unwrap();
        fs::write(&source, png()).unwrap();
        let mut writer = CatalogWriter::new(&dir);
        let record = writer.import_png("fixture/red", &source).unwrap();
        assert_eq!(
            record.image.as_ref().unwrap().alpha_bounds,
            Some([0, 0, 0, 0])
        );
        assert_eq!(writer.catalog.blobs.len(), 1);
        writer.catalog.validate().unwrap();
        let _ = fs::remove_dir_all(dir);
    }
    #[test]
    fn catalog_rejects_undeclared_blob() {
        let mut catalog = AssetCatalog::new();
        catalog.assets.push(AssetRecord {
            logical_id: "x".into(),
            kind: "image".into(),
            source_sha256: "a".repeat(64),
            output_sha256: "b".repeat(64),
            import_mode: ImportMode::Copy,
            importer_version: IMPORTER_VERSION.into(),
            blob: "missing".into(),
            image: None,
        });
        assert!(catalog.validate().is_err());
    }
}

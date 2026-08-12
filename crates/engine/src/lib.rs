#![forbid(unsafe_code)]
//! Deterministic, platform-neutral 2D composition for KeyGen.
//!
//! This crate does not read files, open windows, access a network, or require
//! an editor. Hosts provide validated scene data and asset bytes explicitly.

mod compositor;
mod image;
pub mod model;
mod surface;

pub use compositor::{ease, entrance_settled, Scene, SceneAssets};
pub use surface::Surface;

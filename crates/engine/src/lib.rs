#![forbid(unsafe_code)]
//! Deterministic, platform-neutral 2D composition for KeyGen.
//!
//! This crate does not read files, open windows, access a network, or require
//! an editor. Hosts provide validated scene data and asset bytes explicitly.

pub mod audio;
mod compositor;
mod image;
pub mod input;
pub mod model;
pub mod story;
mod surface;

pub mod scene;

pub use compositor::{ease, entrance_settled, Scene, SceneAssets};
pub use surface::Surface;

//! ORBIS — Octane Render Bridge for Iray Scenes: DAZ Studio scenes → Octane X (`.orbx`).
//!
//! USD/ORBX writers, DAZ Iray → Octane materials, layered images and the
//! texture memory budget. The `orbis` binary is the command-line front end
//! the DAZ Studio script runs; other tools can use this crate directly.

pub mod daz;
pub mod daz_material;
pub mod dson;
pub mod layered_image;
pub mod material;
pub mod orbx;
pub mod texture_budget;
pub mod usd;

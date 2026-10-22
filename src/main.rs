//! `orbis` — export a saved DAZ Studio scene to Octane X.
//!
//! `orbis export <scene.duf> [-o out.orbx|out.usdz] [--preview] [--no-ground]
//! [--content-dir DIR]… [--open]`
//!
//! The DAZ Studio script runs this with the scene's path and the content
//! manager's directories, and shows its stdout. Progress and the report go to
//! stdout, logs to stderr.

use std::path::PathBuf;
use std::time::Instant;

use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use orbis::usd::{self, ExportOptions, OctaneMode};

#[derive(Parser)]
#[command(name = "orbis", version, about = "ORBIS — DAZ Studio scenes to Octane X")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Export a saved scene (`.duf`, with its `.dbz` beside it) to `.orbx` or `.usdz`.
    Export {
        /// The DAZ Studio scene.
        scene: PathBuf,
        /// Output file; the extension picks the format. Default:
        /// `~/Documents/ORBIS/<scene>.orbx`.
        #[arg(short, long)]
        out: Option<PathBuf>,
        /// Octane's fast direct-lighting kernel instead of the path tracer.
        #[arg(long)]
        preview: bool,
        /// Leave out the shadow-catching ground even when Iray draws one.
        #[arg(long)]
        no_ground: bool,
        /// DAZ content directory, in DAZ Studio's order (repeatable). Default:
        /// read from DAZ Studio's preferences.
        #[arg(long = "content-dir")]
        content_dirs: Vec<PathBuf>,
        /// Launch Octane X and reveal the file (Octane only opens files from
        /// its own Open panel; the path is put on the clipboard).
        #[arg(long)]
        open: bool,
    },
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "warn".into()))
        .init();
    match Cli::parse().command {
        Command::Export { scene, out, preview, no_ground, content_dirs, open } => {
            if !scene.is_file() {
                bail!("scene not found: {}", scene.display());
            }
            let out = match out {
                Some(o) => o,
                None => {
                    let dir = usd::default_output_dir();
                    std::fs::create_dir_all(&dir)?;
                    usd::orbx_output_path(&dir, &scene)
                }
            };
            let opts = ExportOptions {
                content_dirs: if content_dirs.is_empty() { orbis::daz::content_dirs() } else { content_dirs },
                octane_mode: if preview { OctaneMode::Preview } else { OctaneMode::Final },
                no_ground,
            };
            println!("exporting {} → {}", scene.display(), out.display());
            let t = Instant::now();
            let orbx = out.extension().is_some_and(|e| e.eq_ignore_ascii_case("orbx"));
            let r = if orbx {
                usd::export_scene_orbx(&scene, &out, &opts)?
            } else {
                usd::export_scene_usdz(&scene, &out, &opts)?
            };
            println!(
                "{} meshes, {} points, {} faces, {} textures ({} missing), {} render targets in {:.1?}",
                r.meshes,
                r.points,
                r.faces,
                r.textures,
                r.missing_textures.len(),
                r.render_targets,
                t.elapsed()
            );
            if let Some(limit) = r.texture_limit {
                println!("textures over the memory budget: {} scaled to {limit} px", r.textures_scaled);
            }
            for s in &r.skipped {
                println!("skipped: {s}");
            }
            for t in &r.missing_textures {
                println!("missing texture: {t}");
            }
            if open && orbx {
                usd::open_in_octane(&out)?;
            }
            println!("done: {}", out.display());
            Ok(())
        }
    }
}

//! `orbis` — export a saved DAZ Studio scene to Octane X.
//!
//! `orbis export <scene.duf> [--dump FILE] [-o out.orbx|out.usdz | --out-dir DIR] [--preview]
//! [--no-ground] [--content-dir DIR]…`
//! `orbis install [--library DIR]`
//!
//! The DAZ Studio script dumps the evaluated scene, runs `export` with the
//! scene's path and the dump, and shows its stdout. Progress and the report
//! go to stdout, logs to stderr.

use std::path::PathBuf;
use std::time::Instant;

use anyhow::{bail, Context, Result};
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
    /// Export a saved scene (`.duf` and its scene dump) to `.orbx` or `.usdz`.
    Export {
        /// The DAZ Studio scene.
        scene: PathBuf,
        /// A fresh scene dump from the DAZ Studio script; it is kept in the
        /// cache as the scene's dump. Default: the cached dump of the scene.
        #[arg(long)]
        dump: Option<PathBuf>,
        /// Output file; the extension picks the format. Default:
        /// `~/Documents/ORBIS/<scene>.orbx`.
        #[arg(short, long, conflicts_with = "out_dir")]
        out: Option<PathBuf>,
        /// Folder for `<scene>.orbx` (created if missing).
        #[arg(long)]
        out_dir: Option<PathBuf>,
        /// Octane's fast direct-lighting kernel instead of the path tracer.
        #[arg(long)]
        preview: bool,
        /// Leave out the shadow-catching ground even when Iray draws one.
        #[arg(long)]
        no_ground: bool,
        /// Skin as Octane's OpenPBR Surface (grittier) instead of Standard
        /// Surface (closest to Iray).
        #[arg(long)]
        skin_openpbr: bool,
        /// DAZ content directory, in DAZ Studio's order (repeatable). Default:
        /// the dump's, else read from DAZ Studio's preferences.
        #[arg(long = "content-dir")]
        content_dirs: Vec<PathBuf>,
        /// Report progress on stdout as `progress: <percent> <what>` lines
        /// (read by the DAZ Studio script).
        #[arg(long)]
        progress: bool,
    },
    /// Install the DAZ Studio plugin: this binary into Application Support,
    /// the script and its icons into a content library.
    Install {
        /// The library. Default: "My Library" from DAZ Studio's content directories.
        #[arg(long)]
        library: Option<PathBuf>,
    },
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "warn".into()))
        .init();
    match Cli::parse().command {
        Command::Install { library } => {
            let paths = orbis::install::InstallPaths::default_for(library)?;
            let plugin = orbis::install::install(&paths, &std::env::current_exe()?)?;
            println!("installed: {}", paths.binary.display());
            println!("installed: {}", plugin.display());
            println!(
                "in DAZ Studio: Content Library → {} → Scripts → ORBIS → Export to Octane",
                paths.library.file_name().map(|n| n.to_string_lossy()).unwrap_or_default()
            );
            Ok(())
        }
        Command::Export { scene, dump, out, out_dir, preview, no_ground, skin_openpbr, content_dirs, progress } => {
            if !scene.is_file() {
                bail!("scene not found: {}", scene.display());
            }
            let dump = match dump {
                Some(d) => usd::store_dump(&scene, &d)?,
                None => {
                    let d = usd::dump_cache_path(&scene);
                    if !d.is_file() {
                        bail!("no scene dump for {} — export it once from DAZ Studio (Scripts → ORBIS → Export to Octane)", scene.display());
                    }
                    d
                }
            };
            let out = match out {
                Some(o) => o,
                None => usd::orbx_output_path(&out_dir.unwrap_or_else(usd::default_output_dir), &scene),
            };
            if let Some(dir) = out.parent().filter(|d| !d.as_os_str().is_empty()) {
                std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
            }
            let opts = ExportOptions {
                content_dirs,
                octane_mode: if preview { OctaneMode::Preview } else { OctaneMode::Final },
                no_ground,
                skin_openpbr,
                progress: if progress {
                    usd::Progress::new(|percent, what| println!("progress: {percent} {what}"))
                } else {
                    usd::Progress::default()
                },
            };
            println!("exporting {} → {}", scene.display(), out.display());
            let t = Instant::now();
            let orbx = out.extension().is_some_and(|e| e.eq_ignore_ascii_case("orbx"));
            let r = if orbx {
                usd::export_scene_orbx(&scene, &dump, &out, &opts)?
            } else {
                usd::export_scene_usdz(&scene, &dump, &out, &opts)?
            };
            println!(
                "{} meshes, {} instances, {} points, {} faces, {} textures ({} missing), {} render targets in {:.1?}",
                r.meshes,
                r.instances,
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
            println!("done: {}", out.display());
            Ok(())
        }
    }
}

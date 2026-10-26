//! Read or replace the `project.ocs` of an `.orbx` (calibration variants,
//! scripts/dev/orbx-variants.py):
//!
//!   cargo run --release --example orbx_ocs -- get <in.orbx> <project.ocs>
//!   cargo run --release --example orbx_ocs -- set <in.orbx> <project.ocs> <out.orbx>

use anyhow::{bail, Result};

fn main() -> Result<()> {
    let a: Vec<String> = std::env::args().collect();
    match a.get(1).map(String::as_str) {
        Some("get") if a.len() == 4 => {
            let (_, files) = orbis::orbx::read_orbx(&std::fs::read(&a[2])?)?;
            let Some((_, ocs)) = files.iter().find(|(p, _)| p == "project.ocs") else { bail!("no project.ocs") };
            std::fs::write(&a[3], ocs)?;
        }
        Some("set") if a.len() == 5 => {
            let (version, mut files) = orbis::orbx::read_orbx(&std::fs::read(&a[2])?)?;
            let ocs = std::fs::read(&a[3])?;
            for (p, d) in &mut files {
                if p == "project.ocs" {
                    *d = ocs.clone();
                }
            }
            let refs: Vec<(&str, &[u8])> = files.iter().map(|(p, d)| (p.as_str(), d.as_slice())).collect();
            orbis::orbx::write_orbx(&mut std::io::BufWriter::new(std::fs::File::create(&a[4])?), &version, &refs)?;
        }
        _ => bail!("usage: orbx_ocs get <in.orbx> <project.ocs> | set <in.orbx> <project.ocs> <out.orbx>"),
    }
    Ok(())
}

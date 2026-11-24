//! `revi`: decompilation tools. `map` lists function extents from headers only,
//! `check` validates a decomp/ tree against that map.

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;

mod check;
mod claim;

#[derive(Parser)]
#[command(name = "revi", version, about = "GTA VI decompilation tools")]
enum Cli {
    /// List every function extent in an image.
    Map {
        file: PathBuf,
        /// Write the records here (CSV; JSONL with --jsonl).
        #[arg(long)]
        out: Option<PathBuf>,
        /// One JSON object per line instead of CSV.
        #[arg(long)]
        jsonl: bool,
    },
    /// Validate a decomp/ tree against its function map (the auto-merge gate).
    Check {
        /// The committed function map (CSV from `revi map`).
        map: PathBuf,
        /// The decomp/ tree.
        decomp: PathBuf,
    },
    /// Take an RVA. Refuses one that is claimed and not stale.
    Claim {
        /// The committed function map (CSV from `revi map`).
        map: PathBuf,
        /// The decomp/ tree.
        decomp: PathBuf,
        /// Function RVA, hex with `0x`.
        rva: String,
        /// Part of the function, in bytes (default: all of it).
        #[arg(long)]
        size: Option<u32>,
        /// Your handle, as it should appear in the claim.
        #[arg(long)]
        owner: String,
    },
    /// Regenerate decomp/STATUS.md.
    Status {
        map: PathBuf,
        decomp: PathBuf,
    },
}

fn main() -> Result<()> {
    match Cli::parse() {
        Cli::Map { file, out, jsonl } => victory_map::run(&file, out.as_deref(), jsonl),
        Cli::Check { map, decomp } => check::run(&map, &decomp),
        Cli::Claim {
            map,
            decomp,
            rva,
            size,
            owner,
        } => {
            let rva = u32::from_str_radix(rva.trim_start_matches("0x"), 16)
                .with_context(|| format!("bad rva {rva:?}"))?;
            claim::claim(&map, &decomp, rva, size, &owner)
        }
        Cli::Status { map, decomp } => claim::status(&map, &decomp),
    }
}

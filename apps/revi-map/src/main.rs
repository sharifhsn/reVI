//! `revi`: decompilation tools. `map` lists function extents from headers only,
//! `check` validates a decomp/ tree against that map.

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;

mod check;
mod claim;
mod diff;

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
        /// How many adjacent compiler-split chunks this body spans.
        #[arg(long, default_value_t = 1)]
        chunks: u32,
        /// Your handle, as it should appear in the claim.
        #[arg(long)]
        owner: String,
    },
    /// Classify every function: thunks, padding, duplicates, giants, the rest.
    Scan {
        file: PathBuf,
        /// Where the candidate CSV goes.
        #[arg(long)]
        out: PathBuf,
        /// Also write a ranked work queue (Markdown).
        #[arg(long)]
        queue: Option<PathBuf>,
        /// How many rows the queue keeps.
        #[arg(long, default_value_t = 200)]
        top: usize,
    },
    /// Compile one body and diff it against the original extent.
    Diff {
        /// The executable.
        exe: PathBuf,
        /// The committed map, to look the extent up.
        map: PathBuf,
        /// Function RVA in hex.
        rva: String,
        /// Source file to compile.
        source: PathBuf,
        /// Also write the report here.
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Regenerate decomp/STATUS.md.
    Status { map: PathBuf, decomp: PathBuf },
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
            chunks,
            owner,
        } => {
            let rva = u32::from_str_radix(rva.trim_start_matches("0x"), 16)
                .with_context(|| format!("bad rva {rva:?}"))?;
            claim::claim(&map, &decomp, rva, size, chunks, &owner)
        }
        Cli::Scan {
            file,
            out,
            queue,
            top,
        } => victory_map::scan::run(&file, &out, queue.as_deref(), top),
        Cli::Diff {
            exe,
            map,
            rva,
            source,
            out,
        } => {
            let rva = u32::from_str_radix(rva.trim_start_matches("0x"), 16)
                .with_context(|| format!("bad rva {rva:?}"))?;
            diff::run(&exe, &map, rva, &source, out.as_deref())
        }
        Cli::Status { map, decomp } => claim::status(&map, &decomp),
    }
}

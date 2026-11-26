//! `revi`: decompilation tools. `map` lists function extents from headers only,
//! `check` validates a decomp/ tree against that map.

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;

mod call;
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
        /// Directory with `nodes.csv`, to rank by in-degree.
        #[arg(long)]
        graph: Option<PathBuf>,
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
    /// Walk code that has no `.pdata` record and classify the gaps.
    Sweep {
        file: PathBuf,
        #[arg(long)]
        out: PathBuf,
        /// Ignore gaps smaller than this many bytes.
        #[arg(long, default_value_t = 16)]
        min: u32,
    },
    /// Build the direct call graph and a reachability set.
    Graph {
        file: PathBuf,
        #[arg(long)]
        out: PathBuf,
        /// Shortest pointer run still counted as a table.
        #[arg(long, default_value_t = 4)]
        min_table: usize,
    },
    /// Call one function out of the image (leaf bodies only).
    Call {
        exe: PathBuf,
        /// Function RVA in hex.
        rva: String,
        /// Integer argument, repeatable, up to four.
        #[arg(long = "arg", value_parser = parse_int)]
        args: Vec<u64>,
        /// Dump memory after the call, repeatable. `buf[:len]`, `rva:0x<off>:len`
        /// (image-relative) or `abs:0x<addr>:len` (host address).
        #[arg(long)]
        dump: Vec<String>,
        /// Preload spans from a file, one `span hexbytes` per line.
        #[arg(long)]
        mem: Option<PathBuf>,
        /// Patch this compiled object's `.text` over the extent before calling it.
        #[arg(long)]
        obj: Option<PathBuf>,
        /// Allocate a zeroed buffer of this many bytes and pass its address as
        /// argument 1.
        #[arg(long, value_parser = parse_int)]
        arg_buf: Option<u64>,
        /// Write a float64 into the argument buffer, `offset:value` (offset hex).
        #[arg(long = "f64")]
        floats: Vec<String>,
        /// Write a 4-byte word into the argument buffer, `offset:value` (both hex).
        #[arg(long = "u32")]
        words: Vec<String>,
    },
    /// Regenerate decomp/STATUS.md.
    Status { map: PathBuf, decomp: PathBuf },
}

fn parse_int(text: &str) -> Result<u64, std::num::ParseIntError> {
    match text.strip_prefix("0x") {
        Some(hex) => u64::from_str_radix(hex, 16),
        None => text.parse(),
    }
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
            graph,
        } => victory_map::scan::run(&file, &out, queue.as_deref(), top, graph.as_deref()),
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
        Cli::Graph { file, out, min_table } => victory_map::graph::run(&file, &out, min_table),
        Cli::Sweep { file, out, min } => victory_map::sweep::run(&file, &out, min),
        Cli::Call {
            exe,
            rva,
            args,
            dump,
            mem,
            obj,
            arg_buf,
            floats,
            words,
        } => {
            let rva = u32::from_str_radix(rva.trim_start_matches("0x"), 16)
                .with_context(|| format!("bad rva {rva:?}"))?;
            let buf = arg_buf.map(|len| {
                let len = len as usize;
                let mut writes: Vec<(usize, u64, u8)> = Vec::new();
                for f in &floats {
                    let (o, v) = f.split_once(':').context("--f64 wants offset:value")?;
                    let off = u32::from_str_radix(o.trim_start_matches("0x"), 16)? as usize;
                    writes.push((off, v.parse::<f64>()?.to_bits(), 8));
                }
                for w in &words {
                    let (o, v) = w.split_once(':').context("--u32 wants offset:value")?;
                    let off = u32::from_str_radix(o.trim_start_matches("0x"), 16)? as usize;
                    writes.push((off, u32::from_str_radix(v.trim_start_matches("0x"), 16)? as u64, 4));
                }
                Ok::<_, anyhow::Error>((len, writes))
            }).transpose()?;
            let buf_len = buf.as_ref().map_or(64, |b| b.0);
            let mut dumps: Vec<(call::Span, usize)> = Vec::new();
            for d in &dump {
                let (span, l) = match d.rsplit_once(':') {
                    Some((s, l)) if !l.is_empty() && !s.ends_with(':') => (s, Some(l)),
                    _ => (d.as_str(), None),
                };
                let len = match l {
                    Some(l) => parse_int(l)? as usize,
                    None => buf_len,
                };
                let sp = if span == "buf" {
                    call::Span::Buf
                } else if let Some(off) = span.strip_prefix("rva:") {
                    call::Span::Rva(u64::from_str_radix(off.trim_start_matches("0x"), 16)?)
                } else if let Some(addr) = span.strip_prefix("abs:") {
                    call::Span::Abs(u64::from_str_radix(addr.trim_start_matches("0x"), 16)?)
                } else {
                    call::Span::Abs(u64::from_str_radix(span.trim_start_matches("0x"), 16)?)
                };
                dumps.push((sp, len));
            }
            call::run(&exe, rva, args.clone(), dumps, buf, mem.as_deref(), obj.as_deref())
        }
        Cli::Status { map, decomp } => claim::status(&map, &decomp),
    }
}

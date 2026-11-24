//! Function extents for a matching decompilation.
//!
//! Reads the image's exception directory and prints (or writes) one record per
//! function. No code bytes are copied out; RVAs, sizes and section names only.

pub mod scan;

use std::io::{BufWriter, Write};
use std::path::Path;

use anyhow::{Context, Result};
use victory_pe::ImageInfo;
use victory_pe::functions::unwind_functions;

/// One record, ready to be written out.
struct Record {
    begin: u32,
    size: u32,
    section: String,
    executable: bool,
}

pub fn run(file: &Path, out: Option<&Path>, jsonl: bool) -> Result<()> {
    let data = std::fs::read(file).with_context(|| format!("reading {}", file.display()))?;
    let info = ImageInfo::parse(&data).with_context(|| format!("parsing {}", file.display()))?;
    let functions = unwind_functions(&data, &info)
        .with_context(|| format!("reading function extents from {}", file.display()))?;

    let section_of = |rva: u32| {
        info.sections
            .iter()
            .find(|s| rva >= s.rva && rva - s.rva < s.virtual_size)
    };
    let records: Vec<Record> = functions
        .iter()
        .map(|f| {
            let section = section_of(f.begin_rva);
            Record {
                begin: f.begin_rva,
                size: f.len(),
                section: section.map_or_else(|| "?".into(), |s| s.name.clone()),
                executable: section.is_some_and(|s| s.is_executable()),
            }
        })
        .collect();

    let mut by_section: std::collections::BTreeMap<&str, (usize, u64)> = Default::default();
    for r in &records {
        let entry = by_section.entry(&r.section).or_default();
        entry.0 += 1;
        entry.1 += u64::from(r.size);
    }

    println!("file           {}", file.display());
    println!("blake3         {}", blake3::hash(&data).to_hex());
    println!("functions      {}", records.len());
    for (section, (count, bytes)) in &by_section {
        println!("  {section:<12} {count:>7} functions  {bytes:>12} bytes");
    }
    let non_exec = records.iter().filter(|r| !r.executable).count();
    if non_exec > 0 {
        println!("outside code   {non_exec} functions (thunks, data-in-code, padding)");
    }

    if let Some(out) = out {
        write_records(out, &records, jsonl)?;
        println!("written        {}", out.display());
    }
    Ok(())
}

fn write_records(out: &Path, records: &[Record], jsonl: bool) -> Result<()> {
    if let Some(parent) = out.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let file = std::fs::File::create(out).with_context(|| format!("creating {}", out.display()))?;
    let mut writer = BufWriter::new(file);
    if jsonl {
        for r in records {
            writeln!(
                writer,
                "{{\"rva\":{},\"size\":{},\"section\":\"{}\"}}",
                r.begin, r.size, r.section
            )?;
        }
    } else {
        writeln!(writer, "rva,size,section")?;
        for r in records {
            writeln!(writer, "{},{},{}", r.begin, r.size, r.section)?;
        }
    }
    writer.flush()?;
    Ok(())
}

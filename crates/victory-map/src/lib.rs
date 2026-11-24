//! Function extents for a matching decompilation.
//!
//! Reads the image's exception directory and prints (or writes) one record per
//! function. No code bytes are copied out; RVAs, sizes and section names only.

pub mod graph;
pub mod scan;
pub mod sweep;

use std::collections::BTreeMap;
use std::io::{BufWriter, Write};
use std::path::Path;

use anyhow::{Context, Result, bail};
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

pub fn read_map(map: &Path) -> Result<BTreeMap<u32, u32>> {
    let text =
        std::fs::read_to_string(map).with_context(|| format!("reading {}", map.display()))?;
    let mut extents = BTreeMap::new();
    for (index, line) in text.lines().enumerate() {
        if index == 0 {
            continue;
        }
        let mut fields = line.split(',');
        let (Some(rva), Some(size)) = (fields.next(), fields.next()) else {
            bail!("{}:{}: expected rva,size", map.display(), index + 1);
        };
        let rva: u32 = rva.parse().context("rva")?;
        let size: u32 = size.parse().context("size")?;
        if extents.insert(rva, size).is_some() {
            bail!("{}:{}: duplicate rva {rva:#x}", map.display(), index + 1);
        }
    }
    Ok(extents)
}


/// Bytes of an extent, if they are backed by file data.
#[must_use]
pub fn extent<'a>(
    data: &'a [u8],
    info: &victory_pe::ImageInfo,
    rva: u32,
    len: u32,
) -> Option<&'a [u8]> {
    let offset = info.rva_to_file_offset(rva)? as usize;
    data.get(offset..offset + len as usize)
}

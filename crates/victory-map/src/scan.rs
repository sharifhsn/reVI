//! Classify mapped functions so the human queue is small.
//!
//! Nothing here is clever about semantics: it looks at bytes to find thunks,
//! padding, duplicates and generated giants, then scores what is left by how
//! cheap it is to decompile (small, no calls, few dependencies). The output is
//! a work list, not a verdict; anything can be reclassified by hand.

use std::collections::HashMap;
use std::io::{BufWriter, Write};
use std::path::Path;

use anyhow::{Context, Result};
use victory_pe::ImageInfo;
use victory_pe::functions::unwind_functions;

/// `IMAGE_SCN_MEM_EXECUTE`, re-stated because the scan is section-agnostic.
const MAX_GIANT: u32 = 16 * 1024;

/// What the scan decided about one extent.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Class {
    Pad,
    Thunk,
    Giant,
    Unique,
    Alias,
}

impl Class {
    fn name(self) -> &'static str {
        match self {
            Class::Pad => "pad",
            Class::Thunk => "thunk",
            Class::Giant => "giant",
            Class::Unique => "unique",
            Class::Alias => "alias",
        }
    }
}

pub struct Row {
    pub rva: u32,
    pub size: u32,
    pub class: Class,
    pub score: i32,
    pub signals: Vec<&'static str>,
    pub alias_of: Option<u32>,
}

pub fn run(file: &Path, out: &Path, queue: Option<&Path>, top: usize) -> Result<()> {
    let data = std::fs::read(file).with_context(|| format!("reading {}", file.display()))?;
    let info = ImageInfo::parse(&data).with_context(|| format!("parsing {}", file.display()))?;
    let functions = unwind_functions(&data, &info)
        .with_context(|| format!("reading extents from {}", file.display()))?;
    let iat: Vec<(u32, u32)> = info
        .imports
        .iter()
        .flat_map(|m| m.entries.iter())
        .map(|e| (e.iat_slot_rva, e.iat_slot_rva + 8))
        .collect();

    let mut seen: HashMap<u64, u32> = HashMap::new();
    let mut rows = Vec::with_capacity(functions.len());
    for f in &functions {
        let Some(bytes) = extent(&data, &info, f.begin_rva, f.len()) else {
            continue;
        };
        let (class, signals, score) = classify(bytes, f.begin_rva, &info, &iat);
        let mut alias_of = None;
        let class = if class == Class::Unique {
            let digest = digest(bytes);
            match seen.get(&digest) {
                Some(&first) => {
                    alias_of = Some(first);
                    Class::Alias
                }
                None => {
                    seen.insert(digest, f.begin_rva);
                    Class::Unique
                }
            }
        } else {
            class
        };
        rows.push(Row {
            rva: f.begin_rva,
            size: f.len(),
            class,
            score,
            signals,
            alias_of,
        });
    }

    write_csv(out, &rows)?;
    let counts = count(&rows);
    for (class, n) in &counts {
        println!("{class:<7} {n}");
    }
    if let Some(queue) = queue {
        write_queue(queue, &rows, top)?;
        println!("queue   {} (top {top})", queue.display());
    }
    Ok(())
}

fn classify(
    bytes: &[u8],
    base_rva: u32,
    info: &ImageInfo,
    iat: &[(u32, u32)],
) -> (Class, Vec<&'static str>, i32) {
    if bytes.len() as u32 > MAX_GIANT {
        return (Class::Giant, vec!["generated?"], -50);
    }
    if bytes.len() < 4 || bytes.iter().all(|b| matches!(b, 0xCC | 0x90 | 0x00)) {
        return (Class::Pad, vec!["fill"], 0);
    }
    if is_thunk(bytes) {
        return (Class::Thunk, vec!["single jump"], 0);
    }
    let mut signals = Vec::new();
    let mut score = 100i32;
    if !has_call(bytes) {
        signals.push("leaf");
        score += 40;
    }
    if bytes.len() <= 32 {
        signals.push("tiny");
        score += 20;
    }
    if has_string_ref(bytes, base_rva, info) {
        signals.push("strings");
        score += 15;
    }
    if has_import_call(bytes, base_rva, iat) {
        signals.push("import");
        score += 10;
    }
    if has_indirect_call(bytes) {
        signals.push("indirect");
        score -= 10;
    }
    (Class::Unique, signals, score)
}

/// `jmp [rip+x]` (FF 25) or `jmp rel32` (E9) as the whole body.
fn is_thunk(b: &[u8]) -> bool {
    match b {
        [0xFF, 0x25, _, _, _, _] | [0xE9, _, _, _, _] => true,
        [0xEB, _] => true,
        [0xE9 | 0xEB, ..] if b.len() <= 16 => true,
        _ => false,
    }
}

fn has_call(b: &[u8]) -> bool {
    b.contains(&0xE8) || b.windows(2).any(|w| w == [0xFF, 0x15] || w == [0xFF, 0x55])
}

fn has_indirect_call(b: &[u8]) -> bool {
    b.windows(2).any(|w| w[0] == 0xFF && w[1] & 0x38 == 0x10 && w[1] != 0x15)
}

/// A RIP-relative `lea` into a read-only section: almost always a literal.
fn has_string_ref(b: &[u8], base_rva: u32, info: &ImageInfo) -> bool {
    for (i, w) in b.windows(3).enumerate() {
        if w[0] == 0x48 && w[1] == 0x8D && w[2] & 0xC7 == 0x05 {
            let Some(disp) = b.get(i + 3..i + 7) else {
                continue;
            };
            let disp = i32::from_le_bytes(disp.try_into().expect("4 bytes"));
            let target = base_rva.wrapping_add((i as i64 + 7 + i64::from(disp)) as u32);
            if let Some(section) = info
                .sections
                .iter()
                .find(|s| target >= s.rva && target - s.rva < s.virtual_size)
                && matches!(section.name.as_str(), ".rdata" | ".rodata" | ".data")
            {
                return true;
            }
        }
    }
    false
}

fn has_import_call(b: &[u8], base_rva: u32, iat: &[(u32, u32)]) -> bool {
    for (i, w) in b.windows(2).enumerate() {
        if w != [0xFF, 0x15] {
            continue;
        }
        let Some(disp) = b.get(i + 2..i + 6) else {
            continue;
        };
        let disp = i32::from_le_bytes(disp.try_into().expect("4 bytes"));
        let target = base_rva.wrapping_add((i as i64 + 6 + i64::from(disp)) as u32);
        if iat.iter().any(|&(start, end)| target >= start && target < end) {
            return true;
        }
    }
    false
}

/// Instructions with the target bytes zeroed, so a clone relinked elsewhere
/// still hashes the same.
fn digest(b: &[u8]) -> u64 {
    let mut copy = b.to_vec();
    for i in 0..copy.len() {
        let is_rel32 = matches!(copy[i], 0xE8 | 0xE9)
            || (copy[i] == 0x0F && copy.get(i + 1).is_some_and(|b| (0x80..=0x8F).contains(b)));
        if is_rel32 {
            let at = if copy[i] == 0x0F { i + 2 } else { i + 1 };
            for slot in copy.iter_mut().skip(at).take(4) {
                *slot = 0;
            }
        }
    }
    // FNV-1a, then fold the length in.
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in copy {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    hash ^ (b.len() as u64)
}

fn extent<'a>(data: &'a [u8], info: &ImageInfo, rva: u32, len: u32) -> Option<&'a [u8]> {
    let offset = info.rva_to_file_offset(rva)? as usize;
    data.get(offset..offset + len as usize)
}

fn count(rows: &[Row]) -> Vec<(&'static str, usize)> {
    let mut counts: HashMap<&'static str, usize> = HashMap::new();
    for row in rows {
        *counts.entry(row.class.name()).or_default() += 1;
    }
    let mut out: Vec<_> = counts.into_iter().collect();
    out.sort();
    out
}

fn write_csv(out: &Path, rows: &[Row]) -> Result<()> {
    if let Some(parent) = out.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }
    let mut writer = BufWriter::new(std::fs::File::create(out)?);
    writeln!(writer, "rva,size,class,score,signals,alias_of")?;
    for r in rows {
        let alias = r.alias_of.map(|a| format!("{a:08x}")).unwrap_or_default();
        writeln!(
            writer,
            "{},{},{},{},{},{}",
            r.rva,
            r.size,
            r.class.name(),
            r.score,
            r.signals.join("+"),
            alias
        )?;
    }
    Ok(())
}

fn write_queue(path: &Path, rows: &[Row], top: usize) -> Result<()> {
    let mut queue: Vec<&Row> = rows
        .iter()
        .filter(|r| r.class == Class::Unique)
        .collect();
    queue.sort_by_key(|r| (-r.score, r.size));
    let mut text = String::from(
        "<!-- generated by `revi scan`; do not edit -->\n\
         # Queue\n\n\
         Cheapest first. Claim with `revi claim decomp/map/functions.csv decomp <rva>`.\n\
         Signals are hints, not requirements: `leaf` = no calls, `tiny` = <= 32 bytes,\n\
         `strings` = references a literal, `import` = wraps an import.\n\n",
    );
    for r in queue.iter().take(top) {
        text.push_str(&format!(
            "* `{:#010x}` {} B — {} ({})\n",
            r.rva,
            r.size,
            r.signals.join(", "),
            r.score
        ));
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, text).with_context(|| format!("writing {}", path.display()))
}

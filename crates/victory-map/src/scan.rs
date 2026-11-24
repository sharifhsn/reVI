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

use crate::extent;
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
    /// How many `.pdata` extents this body was assembled from (>1 = split).
    pub chunks: u32,
    pub class: Class,
    pub score: i32,
    pub signals: Vec<&'static str>,
    pub alias_of: Option<u32>,
}

pub fn run(
    file: &Path,
    out: &Path,
    queue: Option<&Path>,
    top: usize,
    graph: Option<&Path>,
) -> Result<()> {
    let hot = graph.map(read_indegree).transpose()?.unwrap_or_default();
    let data = std::fs::read(file).with_context(|| format!("reading {}", file.display()))?;
    let info = ImageInfo::parse(&data).with_context(|| format!("parsing {}", file.display()))?;
    let functions = unwind_functions(&data, &info)
        .with_context(|| format!("reading extents from {}", file.display()))?;
    // IAT slot -> symbol, for naming idioms by the import they call.
    let mut iat: std::collections::BTreeMap<u32, String> = Default::default();
    for module in &info.imports {
        for entry in &module.entries {
            iat.insert(entry.iat_slot_rva, entry.symbol.to_string());
        }
    }

    let merged = merge(&data, &info, &functions);
    let mut seen: HashMap<u64, u32> = HashMap::new();
    let mut rows = Vec::with_capacity(merged.len());
    for f in &merged {
        let Some(bytes) = extent(&data, &info, f.begin_rva, f.len()) else {
            continue;
        };
        let (class, mut signals, mut score) = classify(bytes, f.begin_rva, &info, &iat);
        if let Some(&n) = hot.get(&f.begin_rva) {
            if n >= 20 {
                signals.push("hot");
                score += 25;
            } else {
                signals.push("called");
                score += 10;
            }
        }
        if f.chunks > 1 {
            signals.push("split");
            score -= 5 * (f.chunks as i32 - 1).min(6);
        }
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
            chunks: f.chunks,
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
    iat: &std::collections::BTreeMap<u32, String>,
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
    if starts_like_tail(bytes) {
        signals.push("tail");
        score -= 30;
    }
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
    for (signal, delta) in idioms(bytes, base_rva, iat) {
        signals.push(signal);
        score += delta;
    }
    (Class::Unique, signals, score)
}

/// Starts by restoring a frame it did not build: an epilogue fragment after a
/// hot/cold split that still ends in `ret`.
fn starts_like_tail(b: &[u8]) -> bool {
    matches!(
        b.get(..5),
        Some([0x48, 0x8B, 0x5C | 0x6C | 0x74 | 0x7C, 0x24, _])
    ) || matches!(b.get(..4), Some([0x48, 0x83 | 0x81, 0xC4, _]))
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
/// Compiler and library idioms that say what a body is. Each is cheap to spot
/// and, unlike a score, survives into the queue as a label.
fn idioms(
    b: &[u8],
    base_rva: u32,
    iat: &std::collections::BTreeMap<u32, String>,
) -> Vec<(&'static str, i32)> {
    let mut found = Vec::new();
    if b.windows(9).any(|w| w == [0x65, 0x48, 0x8B, 0x04, 0x25, 0x58, 0x00, 0x00, 0x00]) {
        found.push(("tls", 10));
    }
    if b.contains(&0xCD) && b.windows(2).any(|w| w == [0xCD, 0x29]) {
        found.push(("fastfail", 10));
    }
    if b.windows(3).any(|w| w == [0xF0, 0x0F, 0xB1]) {
        found.push(("trylock", 5));
    }
    if b.windows(3).any(|w| w == [0xF0, 0x0F, 0xB0]) {
        found.push(("atomic", 5));
    }
    for (i, w) in b.windows(2).enumerate() {
        if w != [0xFF, 0x15] {
            continue;
        }
        let Some(disp) = b.get(i + 2..i + 6) else { continue };
        let disp = i32::from_le_bytes(disp.try_into().expect("4"));
        let slot = base_rva.wrapping_add((i as i64 + 6 + i64::from(disp)) as u32);
        let Some(name) = iat.get(&slot) else { continue };
        let signal = if name.contains("Semaphore") {
            Some(("semaphore", 10))
        } else if name.starts_with("Rtl") {
            Some(("unwind", 5))
        } else if name.starts_with("Wait") {
            Some(("wait", 10))
        } else if name.starts_with("CloseHandle") {
            Some(("close", 5))
        } else {
            None
        };
        if let Some(pair) = signal
            && !found.contains(&pair)
        {
            found.push(pair);
        }
    }
    found
}

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

fn has_import_call(b: &[u8], base_rva: u32, iat: &std::collections::BTreeMap<u32, String>) -> bool {
    for (i, w) in b.windows(2).enumerate() {
        if w != [0xFF, 0x15] {
            continue;
        }
        let Some(disp) = b.get(i + 2..i + 6) else {
            continue;
        };
        let disp = i32::from_le_bytes(disp.try_into().expect("4 bytes"));
        let target = base_rva.wrapping_add((i as i64 + 6 + i64::from(disp)) as u32);
        if iat.contains_key(&target) {
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

/// In-degree per function from a `nodes.csv`.
fn read_indegree(dir: &Path) -> Result<HashMap<u32, u32>> {
    let text = std::fs::read_to_string(dir.join("nodes.csv"))
        .with_context(|| format!("reading {}", dir.join("nodes.csv").display()))?;
    let mut map = HashMap::new();
    for line in text.lines().skip(1) {
        let mut fields = line.split(',');
        let (Some(rva), Some(degree)) = (fields.next(), fields.next()) else {
            continue;
        };
        if let (Ok(rva), Ok(degree)) = (rva.parse(), degree.parse::<u32>())
            && degree > 0
        {
            map.insert(rva, degree);
        }
    }
    Ok(map)
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

/// A body after merging compiler-split chunks.
struct Body {
    begin_rva: u32,
    end_rva: u32,
    chunks: u32,
}

impl Body {
    fn len(&self) -> u32 {
        self.end_rva - self.begin_rva
    }
}

/// MSVC splits functions around EH funclets and cold parts, and each piece
/// carries its own unwind record. Join a piece to the next while the first
/// does not end in a return or unconditional jump.
fn merge(
    data: &[u8],
    info: &ImageInfo,
    functions: &[victory_pe::functions::UnwindFunction],
) -> Vec<Body> {
    let mut bodies: Vec<Body> = Vec::with_capacity(functions.len());
    for f in functions {
        // A chunk whose own last jump points *behind* its start is a cold tail
        // of the body before it: attach backwards whatever the previous chunk
        // ended with.
        let loops_back = extent(data, info, f.begin_rva, f.len())
            .is_some_and(ends_with_back_jump);
        if loops_back
            && let Some(last) = bodies.last_mut()
            && last.end_rva == f.begin_rva
        {
            last.end_rva = f.end_rva;
            last.chunks += 1;
            continue;
        }
        match bodies.last_mut() {
            Some(last) if last.end_rva == f.begin_rva && !ends_of(last, data, info) => {
                last.end_rva = f.end_rva;
                last.chunks += 1;
            }
            _ => bodies.push(Body {
                begin_rva: f.begin_rva,
                end_rva: f.end_rva,
                chunks: 1,
            }),
        }
    }
    bodies
}

/// True when the bytes end with a short/near unconditional jump backwards.
fn ends_with_back_jump(bytes: &[u8]) -> bool {
    match bytes {
        [.., 0xEB, d] => i8::from_le_bytes([*d]) < 0,
        [.., 0xE9, a, b, c, d] => i32::from_le_bytes([*a, *b, *c, *d]) < 0,
        _ => false,
    }
}

/// Does the body currently end in a return or unconditional jump? Look at the
/// last few bytes, because a `jmp rel8` ends with its displacement.
fn ends_of(body: &Body, data: &[u8], info: &ImageInfo) -> bool {
    let start = body.end_rva.saturating_sub(8);
    let Some(window) = extent(data, info, start, body.end_rva - start) else {
        return true;
    };
    matches!(
        window,
        [.., 0xC3 | 0xC2] | [.., 0xE9 | 0xEB, _] | [.., 0xFF, 0x25, _, _, _, _]
    )
}

fn write_csv(out: &Path, rows: &[Row]) -> Result<()> {
    if let Some(parent) = out.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }
    let mut writer = BufWriter::new(std::fs::File::create(out)?);
    writeln!(writer, "rva,size,chunks,class,score,signals,alias_of")?;
    for r in rows {
        let alias = r.alias_of.map(|a| format!("{a:08x}")).unwrap_or_default();
        writeln!(
            writer,
            "{},{},{},{},{},{},{}",
            r.rva,
            r.size,
            r.chunks,
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
        let chunks = if r.chunks > 1 {
            format!(" in {} chunks", r.chunks)
        } else {
            String::new()
        };
        text.push_str(&format!(
            "* `{:#010x}` {} B{chunks} — {} ({})\n",
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

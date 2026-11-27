//! Direct call graph: who calls whom.
//!
//! Edges come from three sources: `call rel32`/`jmp rel32` to a mapped extent,
//! `call [rip+iat]` to an imported function, and function addresses stored in
//! pointer tables (vtable-shaped). Reachability is computed from the entry
//! point, the exports and every pointer-table slot, which is the closest thing
//! to a root set a stripped binary offers.
//!
//! Output is nodes and edges as CSV plus a summary JSON. No code bytes.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::io::{BufWriter, Write};
use std::path::Path;

use anyhow::{Context, Result};
use iced_x86::{Decoder, DecoderOptions, FlowControl, Instruction, OpKind};
use victory_pe::ImageInfo;
use victory_pe::functions::unwind_functions;

pub fn run(file: &Path, out_dir: &Path, min_table: usize) -> Result<()> {
    let data = std::fs::read(file).with_context(|| format!("reading {}", file.display()))?;
    let info = ImageInfo::parse(&data).with_context(|| format!("parsing {}", file.display()))?;
    let functions = unwind_functions(&data, &info)?;
    let starts: BTreeSet<u32> = functions.iter().map(|f| f.begin_rva).collect();

    let mut iat: BTreeMap<u32, String> = BTreeMap::new();
    for module in &info.imports {
        for entry in &module.entries {
            iat.insert(entry.iat_slot_rva, format!("{:?}", entry.symbol));
        }
    }

    // Pointer-table slots: runs of >= min_table text pointers in .rdata/.data.
    let mut table_refs: BTreeMap<u32, u32> = BTreeMap::new();
    let mut tables = 0usize;
    let mut roots: BTreeSet<u32> = BTreeSet::new();
    roots.insert(info.entry_rva);
    roots.extend(info.exports.iter().filter_map(|e| e.rva));

    for section in info.sections.iter().filter(|s| matches!(s.name.as_str(), ".rdata" | ".data")) {
        let Some(bytes) = crate::extent(&data, &info, section.rva, section.file_size) else {
            continue;
        };
        let mut run: Vec<u32> = Vec::new();
        for chunk in bytes.as_chunks::<8>().0 {
            let value = u64::from_le_bytes(*chunk);
            let rva = value.wrapping_sub(info.image_base);
            match u32::try_from(rva).ok().filter(|r| starts.contains(r)) {
                Some(r) => run.push(r),
                None => {
                    if run.len() >= min_table {
                        tables += 1;
                        for r in &run {
                            *table_refs.entry(*r).or_default() += 1;
                            roots.insert(*r);
                        }
                    }
                    run.clear();
                }
            }
        }
        if run.len() >= min_table {
            tables += 1;
            for r in &run {
                *table_refs.entry(*r).or_default() += 1;
                roots.insert(*r);
            }
        }
    }

    // Decode every extent; collect call and tail-jump edges.
    let mut out_degree: BTreeMap<u32, u32> = BTreeMap::new();
    let mut in_degree: BTreeMap<u32, u32> = BTreeMap::new();
    let mut edges: Vec<(u32, u32, &'static str)> = Vec::new();
    let mut import_edges: BTreeMap<u32, u32> = BTreeMap::new();
    // Which function reads which data slot; a table slot's reader is its owner.
    let mut slot_reader: BTreeMap<u32, u32> = BTreeMap::new();

    for f in &functions {
        let Some(code) = crate::extent(&data, &info, f.begin_rva, f.len()) else {
            continue;
        };
        let mut decoder = Decoder::with_ip(64, code, u64::from(f.begin_rva), DecoderOptions::NONE);
        let mut instruction = Instruction::default();
        while decoder.can_decode() {
            decoder.decode_out(&mut instruction);
            if instruction.is_invalid() {
                break;
            }
            match instruction.flow_control() {
                FlowControl::Call | FlowControl::IndirectCall | FlowControl::UnconditionalBranch
                | FlowControl::IndirectBranch
                    if instruction.op0_kind() == OpKind::NearBranch64 =>
                {
                    let target = instruction.near_branch64() as u32;
                    if starts.contains(&target) {
                        let kind = if matches!(
                            instruction.flow_control(),
                            FlowControl::Call | FlowControl::IndirectCall
                        ) {
                            "call"
                        } else {
                            "jump"
                        };
                        edges.push((f.begin_rva, target, kind));
                        *out_degree.entry(f.begin_rva).or_default() += 1;
                        *in_degree.entry(target).or_default() += 1;
                    }
                }
                FlowControl::Call | FlowControl::IndirectCall
                    if instruction.op0_kind() == OpKind::Memory
                        && instruction.is_ip_rel_memory_operand() =>
                {
                    let slot = instruction.ip_rel_memory_address() as u32;
                    if iat.contains_key(&slot) {
                        *import_edges.entry(f.begin_rva).or_default() += 1;
                    } else {
                        slot_reader.entry(slot).or_insert(f.begin_rva);
                    }
                }
                _ => {}
            }
        }
    }

    // Data-held code pointers: vtable slots and dispatch tables written by a
    // reloc. The edge points at the target, from whichever function owns the
    // slot's enclosed data, so a pointer-reached body stops reading callerless.
    let owner_of = |slot: u32| -> Option<u32> { slot_reader.get(&slot).copied() };
    if let Some(at) = victory_pe::layout::data_directory_offset(&data, victory_pe::layout::dir::BASERELOC) {
        let dir_rva = victory_pe::layout::read_u32(&data, at).unwrap_or(0);
        let dir_size = victory_pe::layout::read_u32(&data, at + 4).unwrap_or(0);
        if dir_rva != 0 && dir_size != 0 {
            if let Some(blob) = crate::extent(&data, &info, dir_rva, dir_size) {
                let mut cur = 0usize;
                while cur + 8 <= blob.len() {
                    let page = u32::from_le_bytes(blob[cur..cur + 4].try_into().expect("4"));
                    let block = u32::from_le_bytes(blob[cur + 4..cur + 8].try_into().expect("4")) as usize;
                    cur += 8;
                    let stop = cur + block.saturating_sub(8);
                    while cur + 2 <= stop && cur + 2 <= blob.len() {
                        let e = u16::from_le_bytes(blob[cur..cur + 2].try_into().expect("2"));
                        cur += 2;
                        let kind = e >> 12;
                        let off = (e & 0x0FFF) as u32;
                        if kind == 0 {
                            continue; // padding
                        }
                        let slot = page.wrapping_add(off);
                        let Some(f) = owner_of(slot) else { continue };
                        let v = match crate::extent(&data, &info, slot, 8) {
                            Some(b) => u64::from_le_bytes(b.try_into().expect("8"))
                                .wrapping_sub(info.image_base),
                            None => continue,
                        };
                        let Ok(v) = u32::try_from(v) else { continue };
                        if kind != 10 && kind != 3 {
                            continue;
                        }
                        if starts.contains(&v) {
                            edges.push((f, v, "ptr"));
                            *out_degree.entry(f).or_default() += 1;
                            *in_degree.entry(v).or_default() += 1;
                        }
                    }
                }
            }
        }
    }

    // Reachability from the roots.
    let mut reachable: BTreeSet<u32> = BTreeSet::new();
    let mut queue: VecDeque<u32> = roots.iter().copied().collect();
    while let Some(node) = queue.pop_front() {
        if !reachable.insert(node) {
            continue;
        }
        for (from, to, _) in edges.iter().filter(|(from, _, _)| *from == node) {
            let _ = from;
            if !reachable.contains(to) {
                queue.push_back(*to);
            }
        }
    }

    std::fs::create_dir_all(out_dir)?;
    let mut nodes = BufWriter::new(std::fs::File::create(out_dir.join("nodes.csv"))?);
    writeln!(nodes, "rva,in_degree,out_degree,table_refs,import_calls,reachable")?;
    for f in &functions {
        let rva = f.begin_rva;
        writeln!(
            nodes,
            "{rva},{},{},{},{},{}",
            in_degree.get(&rva).unwrap_or(&0),
            out_degree.get(&rva).unwrap_or(&0),
            table_refs.get(&rva).unwrap_or(&0),
            import_edges.get(&rva).unwrap_or(&0),
            u8::from(reachable.contains(&rva)),
        )?;
    }
    let mut file_edges = BufWriter::new(std::fs::File::create(out_dir.join("edges.csv"))?);
    writeln!(file_edges, "from,to,kind")?;
    for (from, to, kind) in &edges {
        writeln!(file_edges, "{from},{to},{kind}")?;
    }

    let summary = serde_json::json!({
        "functions": functions.len(),
        "edges": edges.len(),
        "pointer_tables": tables,
        "table_functions": table_refs.len(),
        "import_callers": import_edges.len(),
        "roots": roots.len(),
        "reachable": reachable.len(),
        "roots_breakdown": {
            "entry": 1,
            "exports": info.exports.len(),
            "pointer_tables": table_refs.len(),
        },
    });
    std::fs::write(
        out_dir.join("graph-summary.json"),
        serde_json::to_string_pretty(&summary)? + "\n",
    )?;
    println!("{summary:#}");
    Ok(())
}

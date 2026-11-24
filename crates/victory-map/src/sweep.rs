//! Find code that has no `.pdata` record.
//!
//! Records are a lower bound: leaf code reachable only through a jump table,
//! hand-written assembly and thunks can be absent, and the bytes between two
//! records are a mix of code, alignment and data-in-code. This walks every gap
//! in an executable section with a linear decoder and says what it looks like.
//!
//! It classifies, it does not decide. `code` gaps are candidates and the queue
//! still starts from verified ground.

use std::io::{BufWriter, Write};
use std::path::Path;

use anyhow::{Context, Result};
use iced_x86::{Decoder, DecoderOptions, FlowControl, Formatter, Instruction, IntelFormatter};
use victory_pe::ImageInfo;
use victory_pe::functions::unwind_functions;

/// Gap classes. `Code` needs a terminator and a prologue-shaped first
/// instruction; `Data` decodes badly or starts with something code cannot be.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GapClass {
    Code,
    Mixed,
    Data,
}

impl GapClass {
    fn name(self) -> &'static str {
        match self {
            GapClass::Code => "code",
            GapClass::Mixed => "mixed",
            GapClass::Data => "data",
        }
    }
}

pub fn run(file: &Path, out: &Path, min: u32) -> Result<()> {
    let data = std::fs::read(file).with_context(|| format!("reading {}", file.display()))?;
    let info = ImageInfo::parse(&data).with_context(|| format!("parsing {}", file.display()))?;
    let functions = unwind_functions(&data, &info)?;

    let mut gaps: Vec<(u32, u32)> = Vec::new();
    for section in info.sections.iter().filter(|s| s.is_executable()) {
        let mut cursor = section.rva;
        let end = section.rva.saturating_add(section.virtual_size);
        for f in functions
            .iter()
            .filter(|f| f.begin_rva >= section.rva && f.end_rva <= end)
        {
            if f.begin_rva > cursor {
                gaps.push((cursor, f.begin_rva - cursor));
            }
            cursor = f.end_rva.max(cursor);
        }
        if end > cursor {
            gaps.push((cursor, end - cursor));
        }
    }

    let mut writer = BufWriter::new(std::fs::File::create(out)?);
    writeln!(writer, "rva,size,class,first")?;
    let mut counts = [0usize; 3];
    let mut bytes = [0u64; 3];
    for (rva, size) in gaps {
        if size < min {
            continue;
        }
        let Some(code) = crate::extent(&data, &info, rva, size) else {
            continue;
        };
        let (class, first) = classify(code);
        counts[class as usize] += 1;
        bytes[class as usize] += u64::from(size);
        writeln!(writer, "{rva},{size},{},{}", class.name(), first)?;
    }
    writer.flush()?;
    for (i, label) in ["code", "mixed", "data"].iter().enumerate() {
        println!("{label:<6} {:>7} gaps  {:>10} bytes", counts[i], bytes[i]);
    }
    Ok(())
}

/// Decode the gap and decide what it looks like.
fn classify(code: &[u8]) -> (GapClass, String) {
    let mut decoder = Decoder::with_ip(64, code, 0, DecoderOptions::NONE);
    let mut formatter = IntelFormatter::new();
    let mut instruction = Instruction::default();
    formatter.options_mut().set_uppercase_hex(false);

    let mut text = String::new();
    let mut decoded = 0u32;
    let mut first = String::new();
    let mut terminated = false;
    while decoder.can_decode() {
        decoder.decode_out(&mut instruction);
        if instruction.is_invalid() {
            break;
        }
        if decoded == 0 {
            formatter.format(&instruction, &mut first);
        }
        decoded += instruction.len() as u32;
        if matches!(
            instruction.flow_control(),
            FlowControl::Return
                | FlowControl::UnconditionalBranch
                | FlowControl::IndirectBranch
                | FlowControl::Exception
        ) && decoded + 1 >= code.len() as u32
        {
            terminated = true;
        }
        if decoded > 4096 {
            break;
        }
    }
    text.push_str(&first);

    let coverage = f64::from(decoded) / code.len() as f64;
    let prologue = {
        let f = first.split_whitespace().next().unwrap_or("");
        matches!(
            f,
            "push" | "mov" | "sub" | "lea" | "xor" | "test" | "cmp" | "endbr64" | "nop" | "ret"
        )
    };
    let class = if coverage < 0.6 || !prologue {
        GapClass::Data
    } else if terminated && coverage > 0.95 {
        GapClass::Code
    } else {
        GapClass::Mixed
    };
    (class, text)
}

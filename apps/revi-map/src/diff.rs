//! `revi diff`: compile one body and compare it with the original.
//!
//! The compiler is whatever `$REVI_CC` points at (default `cl`); the flags are
//! fixed so two people comparing the same body compare the same thing. Both
//! sides are normalised: relocation displacements become `----`, because they
//! are link-time values in the original and zero in a fresh object.

use std::path::Path;
use std::fmt::Write as _;

use anyhow::{Context, Result, bail};
use victory_map::read_map;

// Project contract: C++20, RTTI off (the binary is stripped), exceptions on.
// /GS is left out by default because only some bodies carry a cookie.
const FLAGS: &[&str] = &["/nologo", "/O2", "/GS-", "/GR-", "/EHsc", "/std:c++20", "/c"];

/// Compare the object built from `source` with the extent at `rva`.
pub fn run(exe: &Path, map: &Path, rva: u32, source: &Path, out: Option<&Path>) -> Result<()> {
    let extents = read_map(map)?;
    let Some(&size) = extents.get(&rva) else {
        bail!("{rva:#08x} is not in the map");
    };
    let data = std::fs::read(exe).with_context(|| format!("reading {}", exe.display()))?;
    let info = victory_pe::ImageInfo::parse(&data)?;
    if victory_map::extent(&data, &info, rva, size).is_none() {
        bail!("{rva:#08x} is not backed by file data");
    }

    let dir = std::env::temp_dir().join(format!("revi-diff-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    let stem = source.file_stem().unwrap_or_default().to_string_lossy().into_owned();
    let copy = dir.join(format!("{stem}{}", source.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default()));
    std::fs::copy(source, &copy).with_context(|| format!("copying {}", source.display()))?;

    let cc = std::env::var("REVI_CC").unwrap_or_else(|_| "cl".into());
    let extra = std::env::var("REVI_CFLAGS").unwrap_or_default();
    let status = std::process::Command::new(&cc)
        .args(extra.split_whitespace())
        .args(FLAGS)
        .arg(&copy)
        .current_dir(&dir)
        .status()
        .with_context(|| format!("running {cc} (is REVI_CC set?)"))?;
    if !status.success() {
        bail!("{cc} failed");
    }
    let obj = dir.join(format!("{stem}.obj"));
    let dump = std::process::Command::new("objdump")
        .args(["-d", "--no-show-raw-insn", "-M", "intel"])
        .arg(&obj)
        .output()
        .context("running objdump")?;
    let ours = instructions(&String::from_utf8_lossy(&dump.stdout));
    let theirs = target_instructions(&data, &info, rva, size);

    let score = score(&ours, &theirs);
    let report = render(rva, size, &ours, &theirs, score);
    print!("{report}");
    if let Some(out) = out {
        std::fs::write(out, &report)?;
    }
    let _ = std::fs::remove_dir_all(&dir);
    if score < 100 {
        std::process::exit(1);
    }
    Ok(())
}

/// Instruction texts from `objdump` output, normalised.
fn instructions(dump: &str) -> Vec<String> {
    dump.lines()
        .filter_map(|line| line.split_once(":\t").map(|(_, rest)| rest.trim().to_owned()))
        .map(|text| normalise(&text))
        .filter(|text| !text.is_empty())
        .collect()
}

fn target_instructions(
    data: &[u8],
    info: &victory_pe::ImageInfo,
    rva: u32,
    size: u32,
) -> Vec<String> {
    let Some(bytes) = victory_map::extent(data, info, rva, size) else {
        return Vec::new();
    };
    let dir = std::env::temp_dir().join(format!("revi-target-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("target.bin");
    let _ = std::fs::write(&path, bytes);
    let dump = std::process::Command::new("objdump")
        .args(["-D", "--no-show-raw-insn", "-b", "binary", "-m", "i386:x86-64", "-M", "intel"])
        .arg(&path)
        .output();
    let _ = std::fs::remove_dir_all(&dir);
    let Ok(dump) = dump else { return Vec::new() };
    instructions(&String::from_utf8_lossy(&dump.stdout))
}

/// Blank relocation displacements and branch targets so objects compare.
fn normalise(text: &str) -> String {
    // Drop objdump's trailing comment, then blank every hex displacement: they
    // are relocations in the original and zero in a fresh object.
    let text = text.split("  #").next().unwrap_or(text);
    // Drop ` <symbol+offset>` and the bare target before it.
    let text = match text.find(" <") {
        Some(at) => text[..at].trim_end().trim_end_matches(|c: char| c.is_ascii_hexdigit()),
        None => text,
    };
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find("0x") {
        out.push_str(&rest[..at]);
        out.push_str("----");
        let tail = &rest[at..];
        let end = tail
            .find(|c: char| !c.is_ascii_hexdigit() && c != 'x')
            .unwrap_or(tail.len());
        rest = &tail[end..];
    }
    out.push_str(rest);
    out = out.replace("+----]", "]").replace("+----h]", "]");
    // A call to an import is `call [rip+x]` in the linked original and a plain
    // `call` here, because the object is not linked.
    out = out.replace("call QWORD PTR [rip]", "call");
    out = out.replace("call ----", "call");
    out.split_whitespace()
        .map(|w| w.trim_end_matches(','))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Longest common subsequence over instruction text: insertions and deletions
/// shift positions, so index-wise comparison under-reports badly.
fn score(ours: &[String], theirs: &[String]) -> u32 {
    let n = ours.len().max(theirs.len()).max(1);
    let mut prev = vec![0usize; ours.len() + 1];
    for t in theirs {
        let mut cur = vec![0usize; ours.len() + 1];
        for (j, o) in ours.iter().enumerate() {
            cur[j + 1] = if o == t {
                prev[j] + 1
            } else {
                cur[j].max(prev[j + 1])
            };
        }
        prev = cur;
    }
    (100 * prev[ours.len()] / n) as u32
}

fn render(rva: u32, size: u32, ours: &[String], theirs: &[String], score: u32) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "rva {rva:#010x}  size {size}  original {} instrs  ours {}", theirs.len(), ours.len());
    let _ = writeln!(out, "score {score}%");
    let width = theirs.iter().map(String::len).max().unwrap_or(0).min(46);
    for i in 0..ours.len().max(theirs.len()) {
        let a = theirs.get(i).map(String::as_str).unwrap_or("");
        let b = ours.get(i).map(String::as_str).unwrap_or("");
        let mark = if a == b { " " } else { "!" };
        let _ = writeln!(out, "{mark} {a:<width$} | {b}", width = width);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalise_blanks_displacements_and_comments() {
        let a = normalise("mov rax,QWORD PTR [rip+0x8ab12c5]  # 0x56540");
        let b = normalise("mov rax,QWORD PTR [rip+0x0]");
        assert_eq!(a, b);
        assert_eq!(normalise("add rsp,0x20"), "add rsp,----");
    }

    #[test]
    fn zero_offset_collapses() {
        assert_eq!(normalise("vmovss xmm0,[rsp+rax*4+0x0]"), "vmovss xmm0,[rsp+rax*4]");
    }

    #[test]
    fn score_counts_subsequence_not_positions() {
        let t = ["a", "b", "c", "d"].map(String::from);
        let ours = ["b", "c", "d"].map(String::from);
        assert_eq!(score(&ours, &t), 75);
        assert_eq!(score(&t, &t), 100);
    }
}

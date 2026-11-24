//! `revi check`: validate a `decomp/` tree against the function map.
//!
//! This is the gate the auto-merge bot runs. It never needs the game: the map
//! is committed, claims are files named after RVAs, and everything else must
//! hang off a claim.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Context, Result, bail};

/// What a claim file may say.
const STATUSES: [&str; 4] = ["claimed", "ported", "matching", "stale"];

#[derive(serde::Deserialize)]
struct Claim {
    rva: u32,
    size: u32,
    status: String,
    /// Who took it. Free text; the bot only checks it is there.
    owner: String,
}

pub fn run(map: &Path, decomp: &Path) -> Result<()> {
    let extents = read_map(map)?;
    let claims_dir = decomp.join("claims");
    let mut claims: BTreeMap<u32, Claim> = BTreeMap::new();
    let mut problems: Vec<String> = Vec::new();

    for entry in read_dir(&claims_dir)? {
        let Some(rva) = rva_from_file(&entry) else {
            problems.push(format!("claims/{}: name is not 0x<rva>.json", name(&entry)));
            continue;
        };
        let text = std::fs::read_to_string(&entry)
            .with_context(|| format!("reading {}", entry.display()))?;
        let claim: Claim =
            serde_json::from_str(&text).with_context(|| format!("parsing {}", entry.display()))?;
        if claim.rva != rva {
            problems.push(format!(
                "claims/{}: rva says {:#x}",
                name(&entry),
                claim.rva
            ));
        }
        let Some(&mapped) = extents.get(&rva) else {
            problems.push(format!("claims/{}: rva not in the map", name(&entry)));
            continue;
        };
        if claim.size == 0 || claim.size > mapped {
            problems.push(format!(
                "claims/{}: size {} outside the mapped {mapped}",
                name(&entry),
                claim.size
            ));
        }
        if !STATUSES.contains(&claim.status.as_str()) {
            problems.push(format!(
                "claims/{}: status {:?} is not one of {STATUSES:?}",
                name(&entry),
                claim.status
            ));
        }
        if claim.owner.trim().is_empty() {
            problems.push(format!("claims/{}: no owner", name(&entry)));
        }
        claims.insert(rva, claim);
    }

    for (dir, suffix) in [("src", "c"), ("notes", "md"), ("tests", "json")] {
        for entry in read_dir(&decomp.join(dir))? {
            let Some(rva) = rva_from_file(&entry) else {
                problems.push(format!(
                    "{dir}/{}: name is not 0x<rva>.{suffix}",
                    name(&entry)
                ));
                continue;
            };
            let expected = format!("0x{rva:08x}.{suffix}");
            if name(&entry) != expected {
                problems.push(format!("{dir}/{}: rename to {expected}", name(&entry)));
            }
            if !claims.contains_key(&rva) {
                problems.push(format!("{dir}/{}: no claim", name(&entry)));
            }
        }
    }

    let matching = claims.values().filter(|c| c.status == "matching").count();
    let ported = claims.values().filter(|c| c.status == "ported").count();
    println!("map extents    {}", extents.len());
    println!("claims         {}", claims.len());
    println!("  ported       {ported}");
    println!("  matching     {matching}");
    println!("problems       {}", problems.len());
    for problem in &problems {
        println!("  {problem}");
    }
    if !problems.is_empty() {
        bail!("{} problems", problems.len());
    }
    Ok(())
}

pub(crate) fn read_map(map: &Path) -> Result<BTreeMap<u32, u32>> {
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

fn read_dir(dir: &Path) -> Result<Vec<std::path::PathBuf>> {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .with_context(|| format!("reading {}", dir.display()))?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(|e| e.path())
        .filter(|p| p.is_file() && !name(p).starts_with('.'))
        .collect();
    entries.sort();
    Ok(entries)
}

fn name(path: &Path) -> String {
    path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned()
}

/// `0x056a1ec8.json` → `0x056a1ec8`.
fn rva_from_file(path: &Path) -> Option<u32> {
    let stem = path.file_stem()?.to_str()?;
    let hex = stem.strip_prefix("0x")?;
    if hex.is_empty() || hex.len() > 8 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    u32::from_str_radix(hex, 16).ok()
}

/// Every RVA present in any of the per-function directories.
#[must_use]
pub fn claimed_rvas(decomp: &Path) -> BTreeSet<u32> {
    let mut rvas = BTreeSet::new();
    if let Ok(entries) = read_dir(&decomp.join("claims")) {
        rvas.extend(entries.iter().filter_map(|p| rva_from_file(p)));
    }
    rvas
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_canonical_names() {
        assert_eq!(
            rva_from_file(Path::new("0x056a1ec8.json")),
            Some(0x056a_1ec8)
        );
        assert_eq!(rva_from_file(Path::new("0x10.c")), Some(0x10));
        assert_eq!(rva_from_file(Path::new("junk.json")), None);
        assert_eq!(rva_from_file(Path::new("0x.json")), None);
        assert_eq!(rva_from_file(Path::new("0x123456789.json")), None);
    }
}

//! `revi call`: run one function out of the image, with no game files.
//!
//! Maps the executable at its preferred base, applies base relocations, and
//! calls an extent with arguments the caller supplies. Enough for leaf bodies
//! that touch only their arguments: no imports are resolved and no init runs,
//! so a body that needs a global or an import will fault or read nonsense.
//!
//! That is the point — it turns "plausible reading" into a recorded return
//! value for the class of body where a vector is cheap, without the package.

use std::path::Path;
#[cfg(target_arch = "x86_64")]
use std::sync::Mutex;

use anyhow::{Result, bail};
#[cfg(target_arch = "x86_64")]
use {anyhow::Context, victory_pe::ImageInfo};

#[cfg(not(target_arch = "x86_64"))]
pub fn run(_exe: &Path, _rva: u32, _args: Vec<u64>, _dump: Option<(u64, usize)>, _buf: Option<(usize, Vec<(usize, f64)>)>) -> Result<()> {
    bail!("revi call needs an x86-64 host; this is {}", std::env::consts::ARCH)
}

/// Map the image and call `rva` with up to four integer arguments.
#[cfg(target_arch = "x86_64")]
pub fn run(
    exe: &Path,
    rva: u32,
    mut args: Vec<u64>,
    dump: Option<(u64, usize)>,
    buf: Option<(usize, Vec<(usize, f64)>)>,
) -> Result<()> {
    let data = std::fs::read(exe).with_context(|| format!("reading {}", exe.display()))?;
    let info = ImageInfo::parse(&data)?;
    if info.entry_rva == 0 {
        bail!("no entry point");
    }
    let size = info.size_of_image as usize;
    let base = info.image_base as usize;

    // Map at the preferred base so RIP-relative operands stay valid.
    let mapped = unsafe {
        libc_mmap(
            base as *mut u8,
            size,
            PROT_READ | PROT_WRITE,
            MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED_NOREPLACE,
            -1,
            0,
        )
    };
    if mapped as usize != base {
        bail!("could not map at {base:#x} (got {mapped:p}); close other mappings");
    }
    unsafe { std::ptr::write_bytes(mapped, 0, size) };

    // Sections, then headers (for the relocations we read from the file).
    for section in &info.sections {
        let src = data
            .get(section.file_offset as usize..)
            .with_context(|| format!("section {}", section.name))?;
        let len = (section.file_size as usize).min(section.virtual_size as usize);
        unsafe {
            std::ptr::copy_nonoverlapping(src.as_ptr(), mapped.add(section.rva as usize), len);
        }
    }
    apply_relocations(&data, &info, mapped)?;

    // Make code executable, keep data writable.
    for section in &info.sections {
        let prot = if section.is_executable() {
            PROT_READ | PROT_EXEC
        } else {
            PROT_READ | PROT_WRITE
        };
        unsafe {
            libc_mprotect(
                mapped.add(section.rva as usize),
                section.virtual_size as usize,
                prot,
            );
        }
    }

    // A stack that is not this thread's, so a deep callee cannot eat ours.
    let mut buf_ptr = 0u64;
    // Optional argument buffer, passed in the first argument register.
    if let Some((len, writes)) = &buf {
        let mem = unsafe { libc_mmap(std::ptr::null_mut(), *len, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0) };
        if mem as isize == -1 {
            bail!("buffer mmap failed");
        }
        unsafe { std::ptr::write_bytes(mem, 0, *len) };
        for (off, value) in writes {
            if off + 8 <= *len {
                unsafe { std::ptr::write_unaligned(mem.add(*off).cast::<f64>(), *value) };
            }
        }
        args.insert(0, mem as u64);
        buf_ptr = mem as u64;
    }

    // Optional fake TEB for bodies that read gs:0x58 (thread index etc.).
    if std::env::var("REVI_STUBS").is_ok() {
        IMAGE_BASE.store(info.image_base, std::sync::atomic::Ordering::Relaxed);
    }
    // Optional import stubs. Every slot gets a tiny trampoline that records
    // the call and returns 0 (or performs the one API whose effect matters).
    let mut symbols: Vec<(usize, String)> = Vec::new();
    if std::env::var("REVI_STUBS").is_ok() {
        let page = unsafe { libc_mmap(std::ptr::null_mut(), 1 << 16, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0) };
        if page as isize == -1 {
            bail!("stub mmap failed");
        }
        let mut cursor = page;
        let mut emit = |body: &[u8]| -> *mut u8 {
            let at = cursor;
            unsafe {
                std::ptr::copy_nonoverlapping(body.as_ptr(), cursor, body.len());
                cursor = cursor.add(body.len());
            }
            at
        };
        if let Ok(path) = std::env::var("REVI_MAP")
            && let Ok(map) = victory_map::read_map(Path::new(&path))
            && let Some(pdata) = info.sections.iter().find(|s| s.name == ".pdata")
        {
            let entry = unsafe {
                std::slice::from_raw_parts(
                    mapped.add(pdata.rva as usize),
                    pdata.file_size as usize,
                )
            };
            let mut table = Vec::new();
            for (i, chunk) in entry.as_chunks::<12>().0.iter().enumerate() {
                let begin = u32::from_le_bytes(chunk[0..4].try_into().expect("4"));
                let end = u32::from_le_bytes(chunk[4..8].try_into().expect("4"));
                if map.contains_key(&begin) {
                    table.push((
                        u64::from(begin),
                        u64::from(end),
                        mapped as u64 + pdata.rva as u64 + (i * 12) as u64,
                    ));
                }
            }
            if let Ok(mut unwind) = UNWIND.lock() {
                *unwind = table;
            }
        }

        let mut index = 0usize;
        for module in &info.imports {
            for entry in &module.entries {
                let name = entry.symbol.to_string();
                let capture = name == "RtlCaptureContext";
                let unresolved = name == "RtlLookupFunctionEntry";
                // sub rsp,0x28 ; rax = return address (caller)
                let mut body: Vec<u8> = vec![0x48, 0x83, 0xEC, 0x28, 0x48, 0x8B, 0x44, 0x24, 0x28];
                if capture {
                    body.extend([0x48, 0x89, 0x81, 0xF8, 0x00, 0x00, 0x00]);
                }
                if unresolved {
                    body.extend([0x49, 0x89, 0xCA, 0x49, 0x89, 0xD3]); // r10=rcx, r11=rdx
                }
                // r9 = from ; r8 = arg2 ; rdx = arg1 ; rcx = slot
                body.extend([0x49, 0x89, 0xC1, 0x49, 0x89, 0xD0, 0x48, 0x89, 0xCA, 0x48, 0xC7, 0xC1]);
                body.extend((index as u32).to_le_bytes());
                // mov rax, imm64 ; call rax ; [hook] ; epilogue. The stub page
                // is far from the binary, so no rel32 calls.
                let mut patches: Vec<(usize, u64)> = Vec::new();
                body.extend([0x48, 0xB8]);
                patches.push((body.len(), record as usize as u64));
                body.extend([0u8; 8]);
                body.extend([0xFF, 0xD0]);
                if unresolved {
                    body.extend([0x4C, 0x89, 0xD1, 0x4C, 0x89, 0xDA]);
                    body.extend([0x48, 0xB8]);
                    patches.push((body.len(), lookup as usize as u64));
                    body.extend([0u8; 8]);
                    body.extend([0xFF, 0xD0]);
                }
                if !unresolved {
                    match stub_return(&name) {
                        Some(value) => {
                            body.extend([0xB8]);
                            body.extend(value.to_le_bytes());
                        }
                        None => body.extend([0x31, 0xC0]),
                    }
                }
                body.extend([0x48, 0x83, 0xC4, 0x28, 0xC3]);
                let at = emit(&body);
                for (offset, target) in patches {
                    unsafe {
                        std::ptr::copy_nonoverlapping(
                            target.to_le_bytes().as_ptr(),
                            at.add(offset),
                            8,
                        );
                    }
                }
                let slot = unsafe { mapped.add(entry.iat_slot_rva as usize) };
                unsafe { slot.cast::<u64>().write_unaligned(at as u64) };
                symbols.push((index, entry.symbol.to_string()));
                index += 1;
            }
        }
        unsafe { libc_mprotect(page, 1 << 16, PROT_READ | PROT_EXEC) };
    }

    if std::env::var("REVI_GS").is_ok() {
        let teb = unsafe { libc_mmap(std::ptr::null_mut(), 0x2000, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0) };
        let block = unsafe { libc_mmap(std::ptr::null_mut(), 0x2000, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0) };
        let obj = unsafe { libc_mmap(std::ptr::null_mut(), 0x2000, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0) };
        if teb as isize == -1 || block as isize == -1 || obj as isize == -1 {
            bail!("teb mmap failed");
        }
        unsafe {
            *teb.add(0x58).cast::<*mut u8>() = block; // gs:0x58, the first TLS slot
            *block.cast::<*mut u8>() = obj; // slot -> object pointer
            *obj.add(0x1294) = 0;           // thread slot index
        }
        let rc = unsafe { libc_arch_prctl(0x1001, teb as u64) }; // ARCH_SET_GS

        if rc != 0 {
            bail!("arch_prctl(ARCH_SET_GS) failed");
        }
    }

    let stack_size = 1 << 20;
    let stack = unsafe {
        libc_mmap(
            std::ptr::null_mut(),
            stack_size,
            PROT_READ | PROT_WRITE,
            MAP_PRIVATE | MAP_ANONYMOUS,
            -1,
            0,
        )
    };
    if stack as isize == -1 {
        bail!("stack mmap failed");
    }
    let top = stack as usize + stack_size - 64;

    let entry = unsafe { mapped.add(rva as usize) };
    let (rax, rdx, rcx_out): (u64, u64, u64);
    static mut XMM: [u8; 48] = [0; 48];
    let scratch = &raw mut XMM;
    unsafe {
        std::arch::asm!(
            "mov r12, rsp",
            "mov rsp, {top}",
            "and rsp, -16",
            "sub rsp, 32",          // shadow space for the callee
            "mov [r15], rcx",
            "mov [r15+8], rdx",
            "call {fun}",
            "movdqu [r15+16], xmm0",
            "mov rsp, r12",
            inout("r15") scratch => _,
            top = in(reg) top,
            fun = in(reg) entry,
            inout("rcx") args.first().copied().unwrap_or(0) => rcx_out,
            inout("rdx") args.get(1).copied().unwrap_or(0) => rdx,
            inout("r8") args.get(2).copied().unwrap_or(0) => _,
            inout("r9") args.get(3).copied().unwrap_or(0) => _,
            lateout("rax") rax,

            out("r12") _,
            clobber_abi("win64"),
        );
    }

    let raw = unsafe { [XMM[16..32].to_vec(), XMM[32..48].to_vec()].concat() };
    let inrcx = u64::from_le_bytes(unsafe { XMM[..8].try_into().expect("8") });
    let inrdx = u64::from_le_bytes(unsafe { XMM[8..16].try_into().expect("8") });
    let low = u64::from_le_bytes(raw[..8].try_into().expect("8"));
    let f = f64::from_bits(low);
    println!(
        "rva {rva:#010x} args {args:x?} -> rax {rax:#x} rcx {rcx_out:#x} rdx {rdx:#x} xmm0 low {low:#018x} (as f64 {f}) xmm-raw {}",
        hex(&raw)
    );
    println!("in rcx {inrcx:#x} rdx {inrdx:#x}");
    if !symbols.is_empty() {
        let records = RECORDS.lock().map(|r| r.clone()).unwrap_or_default();
        for r in &records {
            let name = symbols
                .iter()
                .find(|(i, _)| *i as u64 == r.0)
                .map(|(_, n)| n.as_str())
                .unwrap_or("?");
            println!("stub {name} args {:x} {:x} {:x} {:x} from {:#x}", r.1, r.2, r.3, r.4, r.5);
        }
        if let Ok(mut r) = RECORDS.lock() {
            r.clear();
        }
    }
    if let Some((addr, len)) = dump {
        let at = if addr == 0 { buf_ptr } else { addr };
        if at != 0 {
            let slice = unsafe { std::slice::from_raw_parts(at as *const u8, len) };
            println!("{}", hex(slice));
        }
    }
    Ok(())
}

#[cfg(target_arch = "x86_64")]
fn apply_relocations(data: &[u8], info: &ImageInfo, mapped: *mut u8) -> Result<()> {
    use victory_pe::layout;
    let Some(at) = layout::data_directory_offset(data, layout::dir::BASERELOC) else {
        return Ok(());
    };
    let dir_rva = layout::read_u32(data, at).unwrap_or(0);
    let dir_size = layout::read_u32(data, at + 4).unwrap_or(0);
    if dir_rva == 0 || dir_size == 0 {
        return Ok(());
    }
    let blob = victory_map::extent(data, info, dir_rva, dir_size).context("reloc directory")?;
    let mut cursor = 0usize;
    let end = blob.len();
    while cursor + 8 <= end {
        let page = read_u32(blob, cursor)? as usize;
        let block = read_u32(blob, cursor + 4)? as usize;
        cursor += 8;
        let stop = cursor + block.saturating_sub(8);
        while cursor + 2 <= stop {
            let entry = read_u16(blob, cursor)?;
            cursor += 2;
            let kind = entry >> 12;
            let offset = (entry & 0xfff) as usize;
            let slot = page + offset;
            if slot + 8 > info.size_of_image as usize {
                continue;
            }
            match kind {
                10 => {
                    let value = unsafe { std::ptr::read_unaligned(mapped.add(slot).cast::<u64>()) };
                    let fixed = value.wrapping_add(info.image_base);
                    unsafe { std::ptr::write_unaligned(mapped.add(slot).cast::<u64>(), fixed) };
                }
                3 => {
                    let value = unsafe { std::ptr::read_unaligned(mapped.add(slot).cast::<u32>()) };
                    let fixed = value.wrapping_add(info.image_base as u32);
                    unsafe { std::ptr::write_unaligned(mapped.add(slot).cast::<u32>(), fixed) };
                }
                _ => {}
            }
        }
        cursor = stop;
    }
    Ok(())
}

#[cfg(target_arch = "x86_64")]
fn read_u32(data: &[u8], at: usize) -> Result<u32> {
    data.get(at..at + 4)
        .map(|b| u32::from_le_bytes(b.try_into().expect("4")))
        .context("truncated")
}

#[cfg(target_arch = "x86_64")]
fn read_u16(data: &[u8], at: usize) -> Result<u16> {
    data.get(at..at + 2)
        .map(|b| u16::from_le_bytes(b.try_into().expect("2")))
        .context("truncated")
}

#[cfg(target_arch = "x86_64")]
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(target_arch = "x86_64")]
#[cfg(target_arch = "x86_64")]
#[cfg(target_arch = "x86_64")]
static IMAGE_BASE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
#[cfg(target_arch = "x86_64")]
static UNWIND: Mutex<Vec<(u64, u64, u64)>> = Mutex::new(Vec::new());

#[cfg(target_arch = "x86_64")]
static RECORDS: Mutex<Vec<(u64, u64, u64, u64, u64, u64)>> = Mutex::new(Vec::new());

/// Called by every stub trampoline. Never panics: it runs on a foreign stack.
#[cfg(target_arch = "x86_64")]
extern "win64" fn record(slot: u64, a: u64, b: u64, from: u64) {
    if let Ok(mut records) = RECORDS.lock() {
        let base = IMAGE_BASE.load(std::sync::atomic::Ordering::Relaxed);
        records.push((slot, a, b, 0, 0, from.saturating_sub(base)));
    }
}

#[cfg(target_arch = "x86_64")]
/// Plausible `RtlLookupFunctionEntry`: answer from the map when one is loaded,
/// and always publish the image base the caller asked for.
#[cfg(target_arch = "x86_64")]
extern "win64" fn lookup(pc: u64, out_base: *mut u64) -> u64 {
    let base = IMAGE_BASE.load(std::sync::atomic::Ordering::Relaxed);
    if !out_base.is_null() {
        unsafe { out_base.write_unaligned(base) };
    }
    let rva = pc.wrapping_sub(base);
    let Ok(table) = UNWIND.lock() else { return 0 };
    table
        .iter()
        .find(|(start, end, _)| *start <= rva as u64 && (rva as u64) < *end)
        .map_or(0, |(_, _, record)| *record)
}

/// Known-good returns for stubs whose caller checks the result. Override with
/// `REVI_STUB_RET=Symbol=0x<hex>,...`.
#[cfg(target_arch = "x86_64")]
fn stub_return(symbol: &str) -> Option<u32> {
    if let Ok(spec) = std::env::var("REVI_STUB_RET") {
        for pair in spec.split(',') {
            if let Some((name, value)) = pair.split_once('=')
                && name.trim() == symbol
                && let Ok(v) = u32::from_str_radix(value.trim().trim_start_matches("0x"), 16)
            {
                return Some(v);
            }
        }
    }
    // Creating any handle must succeed; a plausible non-null value is enough.
    matches!(
        symbol,
        "CreateSemaphoreExW" | "CreateEventW" | "CreateEventA" | "CreateEventExW"
    )
    .then_some(0x100)
}

const PROT_READ: i32 = 1;
#[cfg(target_arch = "x86_64")]
const PROT_WRITE: i32 = 2;
#[cfg(target_arch = "x86_64")]
const PROT_EXEC: i32 = 4;
#[cfg(target_arch = "x86_64")]
const MAP_PRIVATE: i32 = 0x02;
#[cfg(target_arch = "x86_64")]
const MAP_ANONYMOUS: i32 = 0x20;
#[cfg(target_arch = "x86_64")]
const MAP_FIXED_NOREPLACE: i32 = 0x10_0000;

#[cfg(target_arch = "x86_64")]
unsafe extern "C" {
    #[link_name = "mmap"]
    fn libc_mmap(addr: *mut u8, len: usize, prot: i32, flags: i32, fd: i32, off: i64) -> *mut u8;
    #[link_name = "mprotect"]
    fn libc_mprotect(addr: *mut u8, len: usize, prot: i32) -> i32;
    #[link_name = "arch_prctl"]
    fn libc_arch_prctl(code: i32, addr: u64) -> i32;
}

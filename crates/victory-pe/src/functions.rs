//! Function boundaries from the x64 exception directory (`.pdata`).
//!
//! Every x86-64 image carries one `RUNTIME_FUNCTION` record per function that
//! can throw or needs unwinding, and that is most of the code: the compiler
//! emits them from the same source-level functions it compiles. Each record is
//! 12 bytes, `{ begin_rva, end_rva, unwind_info_rva }`, sorted by `begin_rva`.
//!
//! For a matching decompilation this is the ground truth for *what* to match:
//! the list of functions the compiler actually produced, with their extents,
//! read out of the headers alone. Nothing here executes or ships code bytes.
//!
//! Records are a lower bound, not a census of semantics: leaf functions with no
//! prologue, hand-written assembly and code reached only through jump tables
//! can be absent, and the region between two records can hold jump tables,
//! import thunks or alignment padding. Callers get the extents and decide.

use bytemuck::{Pod, Zeroable};

use crate::image::ImageInfo;
use crate::{PeError, layout};

/// Bytes per `RUNTIME_FUNCTION` record.
pub const RUNTIME_FUNCTION_SIZE: usize = size_of::<RuntimeFunction>();

/// Guard against a bogus directory size inflating the allocation.
pub const MAX_FUNCTIONS: usize = 4_000_000;

/// One `RUNTIME_FUNCTION` entry, as the header stores it.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct RuntimeFunction {
    pub begin_rva: u32,
    pub end_rva: u32,
    /// RVA of the unwind info (`UNWIND_INFO`), not needed yet.
    pub unwind_info_rva: u32,
}

/// One function extent from `.pdata`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnwindFunction {
    /// First byte of the function (RVA).
    pub begin_rva: u32,
    /// One past the last byte of the function (RVA).
    pub end_rva: u32,
}

impl UnwindFunction {
    /// Extent in bytes. Never zero: an empty extent is rejected on read.
    #[must_use]
    pub const fn len(self) -> u32 {
        self.end_rva.saturating_sub(self.begin_rva)
    }

    /// Extents are never empty, so this is only for `len_without_is_empty`.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.len() == 0
    }
}

/// Read every function extent from the exception directory.
///
/// The `data` buffer must be the same image [`ImageInfo`] was parsed from.
pub fn unwind_functions(data: &[u8], info: &ImageInfo) -> Result<Vec<UnwindFunction>, PeError> {
    let at = layout::data_directory_offset(data, layout::dir::EXCEPTION)
        .ok_or_else(|| PeError::Malformed("truncated data directories".into()))?;
    let directory_rva = layout::read_u32(data, at)
        .ok_or_else(|| PeError::Malformed("truncated exception directory".into()))?;
    let directory_size = layout::read_u32(data, at + 4)
        .ok_or_else(|| PeError::Malformed("truncated exception directory".into()))?;
    if directory_rva == 0 || directory_size == 0 {
        return Ok(Vec::new());
    }
    let directory_size = directory_size as usize;
    if !directory_size.is_multiple_of(RUNTIME_FUNCTION_SIZE) {
        return Err(PeError::Malformed(format!(
            "exception directory size {directory_size} is not a multiple of {RUNTIME_FUNCTION_SIZE}"
        )));
    }
    let count = directory_size / RUNTIME_FUNCTION_SIZE;
    if count > MAX_FUNCTIONS {
        return Err(PeError::Malformed(format!(
            "exception directory holds {count} records"
        )));
    }
    let offset = rva_to_offset(info, directory_rva, directory_size)
        .ok_or_else(|| PeError::Malformed("exception directory outside of file data".into()))?;
    let records = data
        .get(offset..offset + directory_size)
        .ok_or_else(|| PeError::Malformed("truncated exception directory".into()))?;

    let mut functions: Vec<UnwindFunction> = Vec::with_capacity(count);
    for record in bytemuck::cast_slice::<u8, RuntimeFunction>(records) {
        let begin_rva = record.begin_rva;
        let end_rva = record.end_rva;
        if begin_rva >= end_rva {
            return Err(PeError::Malformed(format!(
                "function {begin_rva:#x} ends at {end_rva:#x}"
            )));
        }
        if end_rva > info.size_of_image || rva_to_offset(info, begin_rva, 1).is_none() {
            return Err(PeError::Malformed(format!(
                "function {begin_rva:#x}..{end_rva:#x} outside the image"
            )));
        }
        if let Some(previous) = functions.last()
            && begin_rva < previous.end_rva
        {
            return Err(PeError::Malformed(format!(
                "functions overlapping at {begin_rva:#x}"
            )));
        }
        functions.push(UnwindFunction { begin_rva, end_rva });
    }
    Ok(functions)
}

/// Map an RVA range to a file offset, honouring the raw size of its section.
fn rva_to_offset(info: &ImageInfo, rva: u32, len: usize) -> Option<usize> {
    let len = u32::try_from(len).ok()?;
    for section in &info.sections {
        let offset = rva.checked_sub(section.rva)?;
        if offset.checked_add(len)? <= section.file_size {
            return usize::try_from(section.file_offset.checked_add(offset)?).ok();
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::SectionInfo;

    /// Build a minimal image whose exception directory sits at RVA 0x1000 in
    /// one `.pdata` section, with `records` as raw bytes.
    fn image_with_pdata(records: &[u8]) -> (Vec<u8>, ImageInfo) {
        const FILE_ALIGN: usize = 0x200;
        const PDATA_RVA: u32 = 0x1000;
        const PDATA_OFFSET: usize = 0x400;
        let mut data = vec![0u8; PDATA_OFFSET + FILE_ALIGN];
        // DOS header, then an optional header with 16 data directories.
        data[layout::DOS_E_LFANEW..layout::DOS_E_LFANEW + 4]
            .copy_from_slice(&0x80u32.to_le_bytes());
        let opt = layout::optional_header_offset(&data).unwrap();
        data[opt + layout::OPT_NUMBER_OF_RVA_AND_SIZES
            ..opt + layout::OPT_NUMBER_OF_RVA_AND_SIZES + 4]
            .copy_from_slice(&16u32.to_le_bytes());
        // Optional header data directory 3: exception table.
        let at = layout::data_directory_offset(&data, layout::dir::EXCEPTION).unwrap();
        data[at..at + 4].copy_from_slice(&PDATA_RVA.to_le_bytes());
        data[at + 4..at + 8].copy_from_slice(&u32::try_from(records.len()).unwrap().to_le_bytes());
        data[PDATA_OFFSET..PDATA_OFFSET + records.len()].copy_from_slice(records);
        let info = ImageInfo {
            size_of_image: 0x3000,
            sections: vec![SectionInfo {
                name: ".pdata".into(),
                rva: PDATA_RVA,
                virtual_size: 0x200,
                file_offset: u32::try_from(PDATA_OFFSET).unwrap(),
                file_size: 0x200,
                characteristics: 0x4000_0040,
            }],
            ..empty_image()
        };
        (data, info)
    }

    fn empty_image() -> ImageInfo {
        ImageInfo {
            machine: crate::MACHINE_AMD64,
            characteristics: 0,
            timestamp: 0,
            subsystem: 0,
            dll_characteristics: 0,
            image_base: 0x1_4000_0000,
            size_of_image: 0x3000,
            entry_rva: 0,
            stack_reserve: 0,
            stack_commit: 0,
            sections: Vec::new(),
            imports: Vec::new(),
            delay_imports: Vec::new(),
            exports: Vec::new(),
            tls: None,
            has_relocations: false,
            has_exception_table: true,
            has_load_config: false,
            codeview: None,
        }
    }

    fn record(begin: u32, end: u32) -> RuntimeFunction {
        RuntimeFunction {
            begin_rva: begin,
            end_rva: end,
            unwind_info_rva: 0,
        }
    }

    fn records_bytes(records: &[RuntimeFunction]) -> Vec<u8> {
        bytemuck::cast_slice(records).to_vec()
    }

    #[test]
    fn reads_extents_in_order() {
        let (data, info) = image_with_pdata(&records_bytes(&[
            record(0x1100, 0x1140),
            record(0x1140, 0x11c0),
        ]));
        let functions = unwind_functions(&data, &info).unwrap();
        assert_eq!(functions.len(), 2);
        assert_eq!(functions[1].len(), 0x80);
    }

    #[test]
    fn accepts_adjacent_and_rejects_overlap() {
        let (data, info) = image_with_pdata(&records_bytes(&[
            record(0x1100, 0x1140),
            record(0x1130, 0x1140),
        ]));
        let error = unwind_functions(&data, &info).unwrap_err().to_string();
        assert!(error.contains("overlapping"), "{error}");
    }

    #[test]
    fn rejects_reversed_extent() {
        let (data, info) = image_with_pdata(&records_bytes(&[record(0x1200, 0x1100)]));
        let error = unwind_functions(&data, &info).unwrap_err().to_string();
        assert!(error.contains("ends at"), "{error}");
    }

    #[test]
    fn empty_directory_is_no_functions() {
        let (mut data, info) = image_with_pdata(&[]);
        let at = layout::data_directory_offset(&data, layout::dir::EXCEPTION).unwrap();
        data[at..at + 8].fill(0);
        assert!(unwind_functions(&data, &info).unwrap().is_empty());
    }
}

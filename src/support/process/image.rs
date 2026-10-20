use std::fs::File;
use std::io::Read;
use std::path::Path;

use crate::support::paths::has_extension;

const HEADER_LIMIT: u64 = 4096;
const IMAGE_SUBSYSTEM_WINDOWS_CUI: u16 = 3;

const SUBSYSTEM_OFFSET: usize = 68;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProgramKind {
    Script,

    Console,

    Graphical,
}

impl ProgramKind {
    pub fn of(path: &Path) -> Self {
        if is_script(path) {
            return Self::Script;
        }
        match read_header(path).as_deref().and_then(subsystem) {
            Some(IMAGE_SUBSYSTEM_WINDOWS_CUI) => Self::Console,
            _ => Self::Graphical,
        }
    }
}

pub fn is_script(path: &Path) -> bool {
    has_extension(path, "bat") || has_extension(path, "cmd")
}

fn read_header(path: &Path) -> Option<Vec<u8>> {
    let mut header = Vec::new();
    File::open(path)
        .ok()?
        .take(HEADER_LIMIT)
        .read_to_end(&mut header)
        .ok()?;
    Some(header)
}

fn subsystem(header: &[u8]) -> Option<u16> {
    if header.get(0..2)? != b"MZ" {
        return None;
    }
    let pe = usize::try_from(read_u32(header, 0x3C)?).ok()?;
    if header.get(pe..pe.checked_add(4)?)? != b"PE\0\0" {
        return None;
    }
    let coff = pe + 4;
    let optional_size = usize::from(read_u16(header, coff.checked_add(16)?)?);
    let optional = coff.checked_add(20)?;
    if optional_size < SUBSYSTEM_OFFSET + 2 {
        return None;
    }
    match read_u16(header, optional)? {
        0x10B | 0x20B => read_u16(header, optional.checked_add(SUBSYSTEM_OFFSET)?),
        _ => None,
    }
}

fn read_u16(bytes: &[u8], offset: usize) -> Option<u16> {
    let field = bytes.get(offset..offset.checked_add(2)?)?;
    Some(u16::from_le_bytes([field[0], field[1]]))
}

fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    let field = bytes.get(offset..offset.checked_add(4)?)?;
    Some(u32::from_le_bytes([field[0], field[1], field[2], field[3]]))
}

#[cfg(test)]
#[path = "../../../tests/unit/support/process/image__tests.rs"]
mod tests;

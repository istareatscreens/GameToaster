use std::path::PathBuf;

use super::*;

const PE_OFFSET: usize = 0x80;
const OPTIONAL: usize = PE_OFFSET + 24;

fn image(magic: u16, subsystem: u16) -> Vec<u8> {
    let optional_size: u16 = if magic == 0x20B { 240 } else { 224 };
    let mut bytes = vec![0u8; OPTIONAL + usize::from(optional_size)];
    bytes[0..2].copy_from_slice(b"MZ");
    bytes[0x3C..0x40].copy_from_slice(&(PE_OFFSET as u32).to_le_bytes());
    bytes[PE_OFFSET..PE_OFFSET + 4].copy_from_slice(b"PE\0\0");
    bytes[PE_OFFSET + 20..PE_OFFSET + 22].copy_from_slice(&optional_size.to_le_bytes());
    bytes[OPTIONAL..OPTIONAL + 2].copy_from_slice(&magic.to_le_bytes());
    bytes[OPTIONAL + 68..OPTIONAL + 70].copy_from_slice(&subsystem.to_le_bytes());
    bytes
}

fn with(mut bytes: Vec<u8>, offset: usize, value: &[u8]) -> Vec<u8> {
    bytes[offset..offset + value.len()].copy_from_slice(value);
    bytes
}

#[test]
fn the_subsystem_is_read_from_pe32_and_pe32_plus_images() {
    assert_eq!(subsystem(&image(0x10B, 3)), Some(3));
    assert_eq!(subsystem(&image(0x10B, 2)), Some(2));
    assert_eq!(subsystem(&image(0x20B, 3)), Some(3));
    assert_eq!(subsystem(&image(0x20B, 2)), Some(2));
}

#[test]
fn bytes_that_are_not_complete_pe_headers_have_no_subsystem() {
    let valid = image(0x10B, 3);
    assert_eq!(subsystem(&[]), None);
    assert_eq!(subsystem(b"MZ"), None);
    assert_eq!(subsystem(&valid[..OPTIONAL + 69]), None, "truncated field");
    assert_eq!(subsystem(&with(valid.clone(), 0, b"ZM")), None, "DOS mark");
    assert_eq!(
        subsystem(&with(valid.clone(), PE_OFFSET, b"PE\0X")),
        None,
        "PE mark"
    );
    assert_eq!(
        subsystem(&with(valid.clone(), 0x3C, &u32::MAX.to_le_bytes())),
        None,
        "PE offset past any file"
    );
    assert_eq!(
        subsystem(&with(
            valid.clone(),
            0x3C,
            &(valid.len() as u32 - 2).to_le_bytes()
        )),
        None,
        "PE offset past the bytes read"
    );
    assert_eq!(
        subsystem(&with(valid.clone(), PE_OFFSET + 20, &69u16.to_le_bytes())),
        None,
        "optional header too small to hold the field"
    );
    assert_eq!(
        subsystem(&with(valid, OPTIONAL, &0x107u16.to_le_bytes())),
        None,
        "unknown optional header"
    );
}

#[test]
fn files_are_classified_by_extension_then_image() {
    let directory = tempfile::tempdir().unwrap();
    let write = |name: &str, bytes: &[u8]| {
        let path = directory.path().join(name);
        std::fs::write(&path, bytes).unwrap();
        path
    };
    assert_eq!(
        ProgramKind::of(&write("console.exe", &image(0x20B, 3))),
        ProgramKind::Console
    );
    assert_eq!(
        ProgramKind::of(&write("window.exe", &image(0x20B, 2))),
        ProgramKind::Graphical
    );
    assert_eq!(
        ProgramKind::of(&write("text.exe", b"not an image")),
        ProgramKind::Graphical
    );
    assert_eq!(
        ProgramKind::of(&write("run.BAT", &image(0x20B, 3))),
        ProgramKind::Script,
        "a script is a script by its name, whatever its bytes"
    );
    assert_eq!(
        ProgramKind::of(&directory.path().join("missing.exe")),
        ProgramKind::Graphical
    );
    assert_eq!(
        ProgramKind::of(&directory.path().join("missing.cmd")),
        ProgramKind::Script
    );
}

#[test]
fn windows_programs_are_classified_by_their_own_headers() {
    let system =
        PathBuf::from(std::env::var_os("SystemRoot").expect("SystemRoot")).join("System32");
    assert_eq!(
        ProgramKind::of(&system.join("cmd.exe")),
        ProgramKind::Console
    );
    assert_eq!(
        ProgramKind::of(&system.join("notepad.exe")),
        ProgramKind::Graphical
    );
}

#[test]
fn scripts_are_recognised_by_extension_in_any_case() {
    assert!(is_script(Path::new(r"C:\g\run.BAT")));
    assert!(is_script(Path::new(r"C:\g\run.cmd")));
    assert!(!is_script(Path::new(r"C:\g\game.exe")));
    assert!(!is_script(Path::new(r"C:\g\bat")));
}

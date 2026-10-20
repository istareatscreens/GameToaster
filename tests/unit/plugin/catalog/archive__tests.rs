use super::*;

fn zip_with(entries: &[(&str, &[u8])], output: &Path) {
    zip_with_method(entries, output, zip::CompressionMethod::Stored);
}

fn zip_with_method(entries: &[(&str, &[u8])], output: &Path, method: zip::CompressionMethod) {
    let file = fs::File::create(output).unwrap();
    let mut writer = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default().compression_method(method);
    for (name, bytes) in entries {
        writer.start_file(*name, options).unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap();
}

fn mark_entries_encrypted(output: &Path) {
    let mut bytes = fs::read(output).unwrap();
    for offset in 0..bytes.len().saturating_sub(10) {
        let flag_offset = if bytes[offset..].starts_with(b"PK\x03\x04") {
            Some(offset + 6)
        } else if bytes[offset..].starts_with(b"PK\x01\x02") {
            Some(offset + 8)
        } else {
            None
        };
        if let Some(flag_offset) = flag_offset {
            let flags = u16::from_le_bytes([bytes[flag_offset], bytes[flag_offset + 1]]) | 1;
            bytes[flag_offset..flag_offset + 2].copy_from_slice(&flags.to_le_bytes());
        }
    }
    fs::write(output, bytes).unwrap();
}

fn zip_with_symlink(output: &Path) {
    let file = fs::File::create(output).unwrap();
    let mut writer = zip::ZipWriter::new(file);
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    writer.start_file("manifest.toml", options).unwrap();
    writer.write_all(b"manifestVersion = 1").unwrap();
    writer
        .add_symlink("payload-link", "outside.bin", options)
        .unwrap();
    writer.finish().unwrap();
}

#[test]
fn bounded_extract_accepts_root_manifest_and_payload() {
    let root = tempfile::tempdir().unwrap();
    let archive = root.path().join("ok.zip");
    zip_with(
        &[
            ("manifest.toml", b"manifestVersion = 1"),
            ("payload/file.bin", b"abc"),
        ],
        &archive,
    );
    let staging = root.path().join("stage");
    extract_bounded(&archive, &staging, ArchiveLimits::default()).unwrap();
    assert_eq!(fs::read(staging.join("payload/file.bin")).unwrap(), b"abc");
}

#[test]
fn rejects_traversal_case_collision_missing_root_and_limits() {
    for (name, entries) in [
        (
            "traversal",
            vec![
                ("manifest.toml", b"x".as_slice()),
                ("../outside.bin", b"x".as_slice()),
            ],
        ),
        (
            "collision",
            vec![
                ("manifest.toml", b"x".as_slice()),
                ("A.bin", b"x".as_slice()),
                ("a.BIN", b"y".as_slice()),
            ],
        ),
        (
            "nested-manifest",
            vec![("folder/manifest.toml", b"x".as_slice())],
        ),
    ] {
        let root = tempfile::tempdir().unwrap();
        let archive = root.path().join(format!("{name}.zip"));
        zip_with(&entries, &archive);
        assert!(
            extract_bounded(
                &archive,
                &root.path().join("stage"),
                ArchiveLimits::default()
            )
            .is_err(),
            "{name}"
        );
        if name == "traversal" {
            assert!(!root.path().join("outside.bin").exists());
        }
    }

    let root = tempfile::tempdir().unwrap();
    let archive = root.path().join("large.zip");
    zip_with(
        &[("manifest.toml", b"x"), ("payload.bin", b"0123456789")],
        &archive,
    );
    let limits = ArchiveLimits {
        max_entry_size: 5,
        ..ArchiveLimits::default()
    };
    assert!(extract_bounded(&archive, &root.path().join("stage"), limits).is_err());
}

#[test]
fn rejects_encryption_links_overwrite_and_every_resource_limit() {
    let root = tempfile::tempdir().unwrap();

    let encrypted = root.path().join("encrypted.zip");
    zip_with(
        &[("manifest.toml", b"x"), ("payload.bin", b"secret")],
        &encrypted,
    );
    mark_entries_encrypted(&encrypted);
    let error = extract_bounded(
        &encrypted,
        &root.path().join("encrypted-stage"),
        ArchiveLimits::default(),
    )
    .unwrap_err();
    assert!(error.contains("encrypted ZIP entry"), "{error}");

    let symlink = root.path().join("symlink.zip");
    zip_with_symlink(&symlink);
    let error = extract_bounded(
        &symlink,
        &root.path().join("symlink-stage"),
        ArchiveLimits::default(),
    )
    .unwrap_err();
    assert!(error.contains("link or special ZIP entry"), "{error}");

    let ordinary = root.path().join("ordinary.zip");
    zip_with(
        &[("manifest.toml", b"x"), ("payload.bin", b"0123456789")],
        &ordinary,
    );
    for (label, limits, expected) in [
        (
            "entry-count",
            ArchiveLimits {
                max_entries: 1,
                ..ArchiveLimits::default()
            },
            "entries; limit",
        ),
        (
            "total-size",
            ArchiveLimits {
                max_entry_size: 100,
                max_total_size: 5,
                ..ArchiveLimits::default()
            },
            "expanded size exceeds",
        ),
    ] {
        let error = extract_bounded(
            &ordinary,
            &root.path().join(format!("{label}-stage")),
            limits,
        )
        .unwrap_err();
        assert!(error.contains(expected), "{label}: {error}");
    }

    let nested = root.path().join("nested.zip");
    zip_with(
        &[("manifest.toml", b"x"), ("one/two/payload.bin", b"x")],
        &nested,
    );
    let error = extract_bounded(
        &nested,
        &root.path().join("nested-stage"),
        ArchiveLimits {
            max_nesting: 2,
            ..ArchiveLimits::default()
        },
    )
    .unwrap_err();
    assert!(error.contains("nesting limit"), "{error}");

    let compressible = root.path().join("compressible.zip");
    let repeated = vec![0_u8; 8192];
    zip_with_method(
        &[("manifest.toml", b"x"), ("payload.bin", &repeated)],
        &compressible,
        zip::CompressionMethod::Deflated,
    );
    let error = extract_bounded(
        &compressible,
        &root.path().join("compression-stage"),
        ArchiveLimits {
            max_compression_ratio: 1,
            ..ArchiveLimits::default()
        },
    )
    .unwrap_err();
    assert!(error.contains("compression ratio"), "{error}");

    let occupied = root.path().join("occupied-stage");
    fs::create_dir_all(&occupied).unwrap();
    let sentinel = occupied.join("sentinel.txt");
    fs::write(&sentinel, b"keep").unwrap();
    let error = extract_bounded(&ordinary, &occupied, ArchiveLimits::default()).unwrap_err();
    assert!(error.contains("must be empty"), "{error}");
    assert_eq!(fs::read(sentinel).unwrap(), b"keep");
}

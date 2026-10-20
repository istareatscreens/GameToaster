pub(crate) fn canonical_tempdir() -> tempfile::TempDir {
    let temp = std::fs::canonicalize(std::env::temp_dir()).expect("the temporary folder exists");
    tempfile::tempdir_in(crate::support::paths::without_verbatim_prefix(&temp))
        .expect("a temporary folder can be created")
}

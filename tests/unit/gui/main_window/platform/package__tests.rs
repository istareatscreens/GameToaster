use super::*;
use windows::Win32::Foundation::{ERROR_ACCESS_DENIED, ERROR_SUCCESS};

#[test]
fn the_package_probe_result_decides_the_install_kind() {
    assert_eq!(
        classify(APPMODEL_ERROR_NO_PACKAGE),
        Ok(InstallKind::Portable)
    );
    assert_eq!(
        classify(ERROR_INSUFFICIENT_BUFFER),
        Ok(InstallKind::Packaged)
    );
    for unexpected in [ERROR_ACCESS_DENIED, ERROR_SUCCESS] {
        let error = classify(unexpected).unwrap_err();
        assert!(error.contains(&unexpected.0.to_string()), "{error}");
    }
}

#[test]
fn a_test_process_has_no_package_identity() {
    assert_eq!(InstallKind::current(), Ok(InstallKind::Portable));
}

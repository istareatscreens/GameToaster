use super::*;

#[test]
fn a_jvs_endpoint_is_the_mxjvs_driver_or_a_serial_port() {
    assert_eq!(normalize_endpoint("COM4").unwrap(), r"\\.\COM4");
    assert_eq!(normalize_endpoint(r"\\.\mxjvs").unwrap(), r"\\.\mxjvs");
    assert_eq!(normalize_endpoint(r" \\.\MXJVS ").unwrap(), r"\\.\mxjvs");
    assert!(normalize_endpoint("COM0").is_err());
    assert!(normalize_endpoint("COM4junk").is_err());
}

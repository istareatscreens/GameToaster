use super::*;

#[test]
fn device_path_to_instance_id() {
    assert_eq!(
        instance_id_from_device_path(
            r"\\?\DISPLAY#GSM0001#5&ffbf381&0&UID4352#{e6f07b5f-ee97-4a90-b076-33f57bf4eaa7}"
        )
        .as_deref(),
        Some(r"DISPLAY\GSM0001\5&ffbf381&0&UID4352")
    );
}

#[test]
fn parses_detailed_timing_image_size() {
    let mut edid = [0u8; 128];
    edid[0..8].copy_from_slice(&[0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00]);

    edid[66] = 0x84;
    edid[67] = 0x40;
    edid[68] = 0x36;
    assert_eq!(parse_edid_mm(&edid), Some((900, 1600)));
}

#[test]
fn falls_back_to_max_image_size_cm() {
    let mut edid = [0u8; 128];
    edid[0..8].copy_from_slice(&[0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00]);
    edid[21] = 53;
    edid[22] = 30;
    assert_eq!(parse_edid_mm(&edid), Some((530, 300)));
}

#[test]
fn rejects_non_edid_blob() {
    assert_eq!(parse_edid_mm(&[0u8; 128]), None);
    assert_eq!(parse_edid_mm(&[0u8; 10]), None);
}

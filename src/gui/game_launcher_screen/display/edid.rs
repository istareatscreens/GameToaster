use std::mem::size_of;

use windows::core::{w, PCWSTR};
use windows::Win32::Devices::DeviceAndDriverInstallation::{
    SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInfo, SetupDiGetClassDevsW,
    SetupDiGetDeviceInstanceIdW, SetupDiOpenDevRegKey, DICS_FLAG_GLOBAL, DIGCF_PRESENT, DIREG_DEV,
    GUID_DEVCLASS_MONITOR, SP_DEVINFO_DATA,
};
use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::System::Registry::{RegCloseKey, RegQueryValueExW, KEY_READ};

pub(in crate::gui::game_launcher_screen) fn physical_size_mm(
    device_path: &str,
) -> Option<(u32, u32)> {
    let wanted = instance_id_from_device_path(device_path)?;
    unsafe {
        let dev_info = SetupDiGetClassDevsW(
            Some(&GUID_DEVCLASS_MONITOR),
            PCWSTR::null(),
            None,
            DIGCF_PRESENT,
        )
        .ok()?;
        let mut result = None;
        let mut index = 0u32;
        loop {
            let mut data = SP_DEVINFO_DATA {
                cbSize: size_of::<SP_DEVINFO_DATA>() as u32,
                ..Default::default()
            };
            if SetupDiEnumDeviceInfo(dev_info, index, &mut data).is_err() {
                break;
            }
            index += 1;
            if let Some(instance) = device_instance_id(dev_info, &data) {
                if instance.eq_ignore_ascii_case(&wanted) {
                    result = read_edid_size(dev_info, &data);
                    break;
                }
            }
        }
        let _ = SetupDiDestroyDeviceInfoList(dev_info);
        result.filter(|(w, h)| plausible(*w) && plausible(*h))
    }
}

fn plausible(mm: u32) -> bool {
    (50..=5000).contains(&mm)
}

fn instance_id_from_device_path(device_path: &str) -> Option<String> {
    let trimmed = device_path.strip_prefix(r"\\?\").unwrap_or(device_path);

    let core = trimmed.split("#{").next().unwrap_or(trimmed);
    if core.is_empty() {
        return None;
    }
    Some(core.replace('#', "\\"))
}

unsafe fn device_instance_id(
    dev_info: windows::Win32::Devices::DeviceAndDriverInstallation::HDEVINFO,
    data: &SP_DEVINFO_DATA,
) -> Option<String> {
    let mut buffer = [0u16; 256];
    let mut required = 0u32;
    SetupDiGetDeviceInstanceIdW(dev_info, data, Some(&mut buffer), Some(&mut required)).ok()?;
    let end = buffer.iter().position(|c| *c == 0).unwrap_or(buffer.len());
    Some(String::from_utf16_lossy(&buffer[..end]))
}

unsafe fn read_edid_size(
    dev_info: windows::Win32::Devices::DeviceAndDriverInstallation::HDEVINFO,
    data: &SP_DEVINFO_DATA,
) -> Option<(u32, u32)> {
    let key =
        SetupDiOpenDevRegKey(dev_info, data, DICS_FLAG_GLOBAL.0, 0, DIREG_DEV, KEY_READ.0).ok()?;
    let mut size = 0u32;
    let first = RegQueryValueExW(key, w!("EDID"), None, None, None, Some(&mut size));
    let parsed = if first == ERROR_SUCCESS && (128..=512).contains(&size) {
        let mut blob = vec![0u8; size as usize];
        let read = RegQueryValueExW(
            key,
            w!("EDID"),
            None,
            None,
            Some(blob.as_mut_ptr()),
            Some(&mut size),
        );
        (read == ERROR_SUCCESS)
            .then(|| parse_edid_mm(&blob))
            .flatten()
    } else {
        None
    };
    let _ = RegCloseKey(key);
    parsed
}

fn parse_edid_mm(edid: &[u8]) -> Option<(u32, u32)> {
    if edid.len() < 128 || edid[0..8] != [0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00] {
        return None;
    }

    let horizontal = ((edid[68] as u32 & 0xF0) << 4) | edid[66] as u32;
    let vertical = ((edid[68] as u32 & 0x0F) << 8) | edid[67] as u32;
    if horizontal > 0 && vertical > 0 {
        return Some((horizontal, vertical));
    }

    let cm_h = edid[21] as u32;
    let cm_v = edid[22] as u32;
    if cm_h > 0 && cm_v > 0 {
        return Some((cm_h * 10, cm_v * 10));
    }
    None
}

#[cfg(test)]
#[path = "../../../../tests/unit/gui/game_launcher_screen/display/edid__tests.rs"]
mod tests;

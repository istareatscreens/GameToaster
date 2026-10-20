use std::mem::size_of;

use windows::core::PCWSTR;
use windows::Win32::Devices::Display::{
    DisplayConfigGetDeviceInfo, GetDisplayConfigBufferSizes, QueryDisplayConfig,
    DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME, DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
    DISPLAYCONFIG_DEVICE_INFO_HEADER, DISPLAYCONFIG_MODE_INFO, DISPLAYCONFIG_MODE_INFO_TYPE_SOURCE,
    DISPLAYCONFIG_PATH_INFO, DISPLAYCONFIG_ROTATION_ROTATE180, DISPLAYCONFIG_ROTATION_ROTATE270,
    DISPLAYCONFIG_ROTATION_ROTATE90, DISPLAYCONFIG_SOURCE_DEVICE_NAME,
    DISPLAYCONFIG_TARGET_DEVICE_NAME, QDC_ONLY_ACTIVE_PATHS,
};
use windows::Win32::Foundation::{BOOL, ERROR_SUCCESS, LPARAM, LUID, RECT, TRUE};
use windows::Win32::Graphics::Gdi::{
    CreateDCW, DeleteDC, EnumDisplayMonitors, GetDeviceCaps, GetMonitorInfoW, HDC, HMONITOR,
    HORZSIZE, MONITORINFO, MONITORINFOEXW, VERTSIZE,
};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};

use super::geometry::PixelRect;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(in crate::gui::game_launcher_screen) struct StableMonitorId {
    pub device_name: String,
    pub device_path: String,
    pub manufacturer: Option<String>,
    pub product_code: Option<u16>,
    pub serial: Option<u32>,
}

impl StableMonitorId {
    pub fn key(&self) -> String {
        format!(
            "{}|{}|{}|{}|{}",
            self.device_name,
            self.device_path,
            self.manufacturer.as_deref().unwrap_or("unknown"),
            self.product_code
                .map(|value| value.to_string())
                .unwrap_or_default(),
            self.serial
                .map(|value| value.to_string())
                .unwrap_or_default(),
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::gui::game_launcher_screen) struct MonitorDescriptor {
    pub index: u32,
    pub id: StableMonitorId,
    pub friendly_name: String,
    pub bounds: PixelRect,
    pub rotation_degrees: u16,
    pub primary: bool,
    pub dpi_x: u32,
    pub dpi_y: u32,
    pub physical_width_mm: Option<u32>,
    pub physical_height_mm: Option<u32>,

    pub native_pixels: bool,
}

#[derive(Clone, Debug)]
struct DisplayTarget {
    gdi_name: String,
    friendly_name: String,
    id: StableMonitorId,
    rotation_degrees: u16,

    native_bounds: Option<PixelRect>,
}

pub(in crate::gui::game_launcher_screen) fn enumerate() -> Result<Vec<MonitorDescriptor>, String> {
    let targets = display_targets()?;
    let mut context = EnumContext {
        targets: &targets,
        monitors: Vec::new(),
    };
    unsafe {
        EnumDisplayMonitors(
            None,
            None,
            Some(enum_monitor),
            LPARAM(&mut context as *mut EnumContext<'_> as isize),
        )
    }
    .ok()
    .map_err(|error| format!("failed to enumerate display monitors: {error}"))?;

    for (index, monitor) in context.monitors.iter_mut().enumerate() {
        monitor.index = index as u32;
    }
    Ok(context.monitors)
}

const SNAPSHOT_ATTEMPTS: usize = 4;

#[derive(Clone, Debug, Eq, PartialEq)]
struct SnapshotFailure {
    call: &'static str,
    code: u32,
}

impl std::fmt::Display for SnapshotFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} failed with {}", self.call, self.code)
    }
}

fn snapshot_with_retry<T>(
    mut attempt: impl FnMut() -> Result<T, SnapshotFailure>,
) -> Result<T, String> {
    let mut failures = 0;
    loop {
        match attempt() {
            Ok(value) => return Ok(value),
            Err(failure) => {
                failures += 1;
                if failures == SNAPSHOT_ATTEMPTS {
                    return Err(format!(
                        "display configuration snapshot failed after {failures} attempts: {failure}"
                    ));
                }
            }
        }
    }
}

fn display_targets() -> Result<Vec<DisplayTarget>, String> {
    snapshot_with_retry(snapshot_display_targets)
}

fn snapshot_display_targets() -> Result<Vec<DisplayTarget>, SnapshotFailure> {
    let mut path_count = 0;
    let mut mode_count = 0;
    let result = unsafe {
        GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut path_count, &mut mode_count)
    };
    if result != ERROR_SUCCESS {
        return Err(SnapshotFailure {
            call: "GetDisplayConfigBufferSizes",
            code: result.0,
        });
    }
    let mut paths = vec![DISPLAYCONFIG_PATH_INFO::default(); path_count as usize];
    let mut modes = vec![Default::default(); mode_count as usize];
    let result = unsafe {
        QueryDisplayConfig(
            QDC_ONLY_ACTIVE_PATHS,
            &mut path_count,
            paths.as_mut_ptr(),
            &mut mode_count,
            modes.as_mut_ptr(),
            None,
        )
    };
    if result != ERROR_SUCCESS {
        return Err(SnapshotFailure {
            call: "QueryDisplayConfig",
            code: result.0,
        });
    }
    paths.truncate(path_count as usize);
    modes.truncate(mode_count as usize);

    let mut targets = Vec::with_capacity(paths.len());
    for path in paths {
        let mut source = DISPLAYCONFIG_SOURCE_DEVICE_NAME {
            header: DISPLAYCONFIG_DEVICE_INFO_HEADER {
                r#type: DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME,
                size: size_of::<DISPLAYCONFIG_SOURCE_DEVICE_NAME>() as u32,
                adapterId: path.sourceInfo.adapterId,
                id: path.sourceInfo.id,
            },
            ..Default::default()
        };
        let source_result = unsafe { DisplayConfigGetDeviceInfo(&mut source.header) };
        if source_result != 0 {
            return Err(SnapshotFailure {
                call: "DisplayConfigGetDeviceInfo(source name)",
                code: source_result as u32,
            });
        }
        let mut target = DISPLAYCONFIG_TARGET_DEVICE_NAME {
            header: DISPLAYCONFIG_DEVICE_INFO_HEADER {
                r#type: DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
                size: size_of::<DISPLAYCONFIG_TARGET_DEVICE_NAME>() as u32,
                adapterId: path.targetInfo.adapterId,
                id: path.targetInfo.id,
            },
            ..Default::default()
        };
        let target_result = unsafe { DisplayConfigGetDeviceInfo(&mut target.header) };
        if target_result != 0 {
            return Err(SnapshotFailure {
                call: "DisplayConfigGetDeviceInfo(target name)",
                code: target_result as u32,
            });
        }
        let device_path = wide_string(&target.monitorDevicePath);
        let rotation_degrees = if path.targetInfo.rotation == DISPLAYCONFIG_ROTATION_ROTATE90 {
            90
        } else if path.targetInfo.rotation == DISPLAYCONFIG_ROTATION_ROTATE180 {
            180
        } else if path.targetInfo.rotation == DISPLAYCONFIG_ROTATION_ROTATE270 {
            270
        } else {
            0
        };
        targets.push(DisplayTarget {
            gdi_name: wide_string(&source.viewGdiDeviceName),
            friendly_name: wide_string(&target.monitorFriendlyDeviceName),
            native_bounds: source_bounds(&modes, &path, rotation_degrees),
            id: StableMonitorId {
                device_name: wide_string(&source.viewGdiDeviceName),
                device_path,
                manufacturer: decode_pnp_manufacturer(target.edidManufactureId),
                product_code: (target.edidProductCodeId != 0).then_some(target.edidProductCodeId),

                serial: None,
            },
            rotation_degrees,
        });
    }
    Ok(targets)
}

fn luid_eq(a: LUID, b: LUID) -> bool {
    a.LowPart == b.LowPart && a.HighPart == b.HighPart
}

fn source_bounds(
    modes: &[DISPLAYCONFIG_MODE_INFO],
    path: &DISPLAYCONFIG_PATH_INFO,
    rotation_degrees: u16,
) -> Option<PixelRect> {
    let adapter = path.sourceInfo.adapterId;
    let source_id = path.sourceInfo.id;
    modes.iter().find_map(|mode| {
        if mode.infoType != DISPLAYCONFIG_MODE_INFO_TYPE_SOURCE
            || !luid_eq(mode.adapterId, adapter)
            || mode.id != source_id
        {
            return None;
        }

        let source = unsafe { mode.Anonymous.sourceMode };
        if source.width == 0 || source.height == 0 {
            return None;
        }
        let (width, height) = desktop_dims(source.width, source.height, rotation_degrees);
        Some(PixelRect {
            x: source.position.x,
            y: source.position.y,
            width,
            height,
        })
    })
}

fn desktop_dims(width: u32, height: u32, rotation_degrees: u16) -> (u32, u32) {
    if rotation_degrees == 90 || rotation_degrees == 270 {
        (height, width)
    } else {
        (width, height)
    }
}

struct EnumContext<'a> {
    targets: &'a [DisplayTarget],
    monitors: Vec<MonitorDescriptor>,
}

unsafe extern "system" fn enum_monitor(
    monitor: HMONITOR,
    _hdc: HDC,
    _clip: *mut RECT,
    data: LPARAM,
) -> BOOL {
    let context = &mut *(data.0 as *mut EnumContext<'_>);
    let mut info = MONITORINFOEXW::default();
    info.monitorInfo.cbSize = size_of::<MONITORINFOEXW>() as u32;
    if !GetMonitorInfoW(monitor, &mut info.monitorInfo as *mut MONITORINFO).as_bool() {
        return TRUE;
    }
    let gdi_name = wide_string(&info.szDevice);
    let target = context
        .targets
        .iter()
        .find(|target| target.gdi_name.eq_ignore_ascii_case(&gdi_name));
    let id = target
        .map(|target| target.id.clone())
        .unwrap_or_else(|| StableMonitorId {
            device_name: gdi_name.clone(),
            device_path: gdi_name.clone(),
            manufacturer: None,
            product_code: None,
            serial: None,
        });
    let mut dpi_x = 96;
    let mut dpi_y = 96;
    let _ = GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y);

    let rotation = target.map(|target| target.rotation_degrees).unwrap_or(0);

    let edid_mm = target
        .and_then(|target| super::edid::physical_size_mm(&target.id.device_path))
        .map(|(w, h)| desktop_dims(w, h, rotation));
    let (physical_width_mm, physical_height_mm) = if let Some((w, h)) = edid_mm {
        (Some(w), Some(h))
    } else {
        let display_dc = CreateDCW(
            PCWSTR::null(),
            PCWSTR(info.szDevice.as_ptr()),
            PCWSTR::null(),
            None,
        );
        if display_dc.is_invalid() {
            (None, None)
        } else {
            let width = GetDeviceCaps(display_dc, HORZSIZE);
            let height = GetDeviceCaps(display_dc, VERTSIZE);
            let _ = DeleteDC(display_dc);
            (
                (width > 0).then_some(width as u32),
                (height > 0).then_some(height as u32),
            )
        }
    };
    let rect = info.monitorInfo.rcMonitor;
    let logical_bounds = PixelRect {
        x: rect.left,
        y: rect.top,
        width: (rect.right - rect.left).max(0) as u32,
        height: (rect.bottom - rect.top).max(0) as u32,
    };

    let (bounds, native_pixels) = match target.and_then(|target| target.native_bounds) {
        Some(native) => (native, true),
        None => (logical_bounds, false),
    };
    context.monitors.push(MonitorDescriptor {
        index: 0,
        id,
        friendly_name: target
            .map(|target| target.friendly_name.clone())
            .filter(|name| !name.is_empty())
            .unwrap_or(gdi_name),
        bounds,
        rotation_degrees: target.map(|target| target.rotation_degrees).unwrap_or(0),
        primary: info.monitorInfo.dwFlags & 1 != 0,
        dpi_x,
        dpi_y,
        physical_width_mm,
        physical_height_mm,
        native_pixels,
    });
    TRUE
}

fn wide_string(buffer: &[u16]) -> String {
    let end = buffer
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(buffer.len());
    String::from_utf16_lossy(&buffer[..end])
}

fn decode_pnp_manufacturer(raw: u16) -> Option<String> {
    if raw == 0 {
        return None;
    }
    let swapped = raw.swap_bytes();
    let values = [
        ((swapped >> 10) & 0x1F) as u8,
        ((swapped >> 5) & 0x1F) as u8,
        (swapped & 0x1F) as u8,
    ];
    values
        .iter()
        .all(|value| (1..=26).contains(value))
        .then(|| {
            values
                .iter()
                .map(|value| (b'A' + value - 1) as char)
                .collect()
        })
}

pub(in crate::gui::game_launcher_screen) fn same_physical_monitor(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    match (a.split('|').nth(1), b.split('|').nth(1)) {
        (Some(path_a), Some(path_b)) => !path_a.is_empty() && path_a == path_b,
        _ => false,
    }
}

pub(in crate::gui::game_launcher_screen) fn select<'a>(
    monitors: &'a [MonitorDescriptor],
    wanted_id: Option<&str>,
    wanted_adapter: Option<u32>,
) -> Option<&'a MonitorDescriptor> {
    if let Some(id) = wanted_id.filter(|id| !id.is_empty()) {
        if let Some(found) = monitors
            .iter()
            .find(|monitor| same_physical_monitor(&monitor.id.key(), id))
        {
            return Some(found);
        }
    }
    if let Some(index) = wanted_adapter {
        if let Some(found) = monitors.iter().find(|monitor| monitor.index == index) {
            return Some(found);
        }
    }
    monitors
        .iter()
        .find(|monitor| monitor.primary)
        .or_else(|| monitors.first())
}

#[cfg(test)]
#[path = "../../../../tests/unit/gui/game_launcher_screen/display/monitor__tests.rs"]
mod tests;

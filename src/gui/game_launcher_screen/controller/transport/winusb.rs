use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use windows::core::{GUID, HRESULT, PCWSTR};
use windows::Win32::Devices::DeviceAndDriverInstallation::{
    SetupDiEnumDeviceInfo, SetupDiEnumDeviceInterfaces, SetupDiGetClassDevsW,
    SetupDiGetDeviceInstanceIdW, SetupDiGetDeviceRegistryPropertyW, SetupDiOpenDevRegKey,
    DICS_FLAG_GLOBAL, DIGCF_ALLCLASSES, DIGCF_DEVICEINTERFACE, DIGCF_PRESENT, DIREG_DEV,
    SPDRP_HARDWAREID, SPDRP_SERVICE, SP_DEVICE_INTERFACE_DATA, SP_DEVINFO_DATA,
};
use windows::Win32::Devices::Usb::{
    UsbdPipeTypeBulk, WinUsb_AbortPipe, WinUsb_Free, WinUsb_GetOverlappedResult, WinUsb_Initialize,
    WinUsb_QueryInterfaceSettings, WinUsb_QueryPipe, WinUsb_ReadPipe, USB_INTERFACE_DESCRIPTOR,
    WINUSB_INTERFACE_HANDLE, WINUSB_PIPE_INFORMATION,
};
use windows::Win32::Foundation::{
    ERROR_IO_PENDING, ERROR_OPERATION_ABORTED, GENERIC_READ, GENERIC_WRITE, WAIT_OBJECT_0,
};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAG_OVERLAPPED, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows::Win32::System::Registry::{RegCloseKey, RegQueryValueExW, KEY_READ};
use windows::Win32::System::Threading::{
    CreateEventW, ResetEvent, SetEvent, WaitForMultipleObjects, INFINITE,
};
use windows::Win32::System::IO::OVERLAPPED;

use crate::game_launcher_screen::ControllerConfig;

use super::super::framework::adapter::{ButtonDecoder, ControllerAdapter, ControllerSession};
use super::super::framework::audit::ResourceAudit;
use super::super::framework::descriptor::{
    hex_u16_option, ControllerDriverDescriptor, DiscoveredController,
};
use super::super::framework::events::{EventSink, SessionEvent};
use super::handles::{ReaderGuard, TrackedHandle};

const CHUNK: usize = 64;

pub(in crate::gui::game_launcher_screen::controller) struct WinUsbAdapter {
    descriptor: &'static ControllerDriverDescriptor,
    identities: &'static [(u16, u16)],
    interface_number: u8,
    interface_class: Option<GUID>,
    report_len: usize,
    decoder: Arc<dyn ButtonDecoder>,
}

impl WinUsbAdapter {
    pub(in crate::gui::game_launcher_screen::controller) fn new(
        descriptor: &'static ControllerDriverDescriptor,
        identities: &'static [(u16, u16)],
        interface_number: u8,
        interface_class: Option<GUID>,
        report_len: usize,
        decoder: Arc<dyn ButtonDecoder>,
    ) -> Self {
        Self {
            descriptor,
            identities,
            interface_number,
            interface_class,
            report_len,
            decoder,
        }
    }
}

impl ControllerAdapter for WinUsbAdapter {
    fn descriptor(&self) -> &'static ControllerDriverDescriptor {
        self.descriptor
    }

    fn discover(&self) -> Result<Vec<DiscoveredController>, String> {
        let mut found = Vec::new();
        for &(vid, pid) in self.identities {
            if find_path(vid, pid, self.interface_number, self.interface_class).is_some() {
                let mut config = self.descriptor.default_config();
                config.options.insert("vid".into(), format!("{vid:04X}"));
                config.options.insert("pid".into(), format!("{pid:04X}"));
                found.push(DiscoveredController {
                    config,
                    label_key: self.descriptor.label_key,
                });
            }
        }
        Ok(found)
    }

    fn connect(
        &self,
        config: &ControllerConfig,
        events: EventSink,
        audit: Arc<ResourceAudit>,
    ) -> Result<Box<dyn ControllerSession>, String> {
        self.validate(config)?;
        let vid = hex_u16_option(config, "vid")?;
        let pid = hex_u16_option(config, "pid")?;
        let path = find_path(vid, pid, self.interface_number, self.interface_class)
            .ok_or_else(|| format!("no USB controller {vid:04X}:{pid:04X} is connected"))?;
        let device = Arc::new(open(&path, &audit)?);
        let cancellation = Arc::new(TrackedHandle::new(
            unsafe { CreateEventW(None, true, false, None) }
                .map_err(|error| format!("failed to create USB cancellation event: {error}"))?,
            &audit,
        )?);
        let read_event = TrackedHandle::new(
            unsafe { CreateEventW(None, true, false, None) }
                .map_err(|error| format!("failed to create USB read event: {error}"))?,
            &audit,
        )?;
        let (done_tx, done) = mpsc::sync_channel(1);
        let thread_cancel = Arc::clone(&cancellation);
        let thread_device = Arc::clone(&device);
        let decoder = Arc::clone(&self.decoder);
        let report_len = self.report_len;
        let reader = thread::Builder::new()
            .name("gametoaster-controller-winusb".into())
            .spawn(move || {
                let _guard = ReaderGuard::new(&audit);
                let _ = events.try_send(SessionEvent::Connected);
                read_loop(
                    &thread_device,
                    &thread_cancel,
                    &read_event,
                    report_len,
                    decoder.as_ref(),
                    &events,
                );
                let _ = events.try_send(SessionEvent::Disconnected);
                let _ = done_tx.try_send(());
            })
            .map_err(|error| format!("failed to start USB reader: {error}"))?;
        Ok(Box::new(WinUsbSession {
            device: Some(device),
            cancellation: Some(cancellation),
            reader: Some(reader),
            done,
        }))
    }
}

struct WinUsbDevice {
    interface: WINUSB_INTERFACE_HANDLE,
    in_pipe: u8,
    _file: TrackedHandle,
}

unsafe impl Send for WinUsbDevice {}
unsafe impl Sync for WinUsbDevice {}

impl Drop for WinUsbDevice {
    fn drop(&mut self) {
        unsafe {
            let _ = WinUsb_Free(self.interface);
        }
    }
}

struct WinUsbSession {
    device: Option<Arc<WinUsbDevice>>,
    cancellation: Option<Arc<TrackedHandle>>,
    reader: Option<thread::JoinHandle<()>>,
    done: mpsc::Receiver<()>,
}

impl WinUsbSession {
    fn signal_cancel(&self) {
        if let Some(cancellation) = &self.cancellation {
            unsafe {
                let _ = SetEvent(cancellation.raw());
            }
        }
        if let Some(device) = &self.device {
            unsafe {
                let _ = WinUsb_AbortPipe(device.interface, device.in_pipe);
            }
        }
    }
}

impl ControllerSession for WinUsbSession {
    fn disconnect(&mut self, timeout: Duration) -> Result<(), String> {
        self.signal_cancel();
        if self.reader.is_some() {
            self.done.recv_timeout(timeout).map_err(|_| {
                "USB reader did not acknowledge cancellation within the handoff timeout".to_string()
            })?;
        }
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
        self.cancellation = None;
        self.device = None;
        Ok(())
    }
}

impl Drop for WinUsbSession {
    fn drop(&mut self) {
        self.signal_cancel();
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
    }
}

fn read_loop(
    device: &WinUsbDevice,
    cancellation: &TrackedHandle,
    read_event: &TrackedHandle,
    report_len: usize,
    decoder: &dyn ButtonDecoder,
    events: &EventSink,
) {
    let mut chunk = [0u8; CHUNK];
    let mut framer = ReportFramer::new(report_len);
    loop {
        unsafe {
            let _ = ResetEvent(read_event.raw());
        }
        let overlapped = OVERLAPPED {
            hEvent: read_event.raw(),
            ..Default::default()
        };
        let started = unsafe {
            WinUsb_ReadPipe(
                device.interface,
                device.in_pipe,
                Some(&mut chunk),
                None,
                Some(&overlapped),
            )
        };
        if let Err(error) = started {
            if error.code() != HRESULT::from_win32(ERROR_IO_PENDING.0) {
                if error.code() != HRESULT::from_win32(ERROR_OPERATION_ABORTED.0) {
                    let _ =
                        events.try_send(SessionEvent::Error(format!("USB read failed: {error}")));
                }
                break;
            }
        }
        let wait = unsafe {
            WaitForMultipleObjects(&[cancellation.raw(), read_event.raw()], false, INFINITE)
        };
        if wait == WAIT_OBJECT_0 {
            unsafe {
                let _ = WinUsb_AbortPipe(device.interface, device.in_pipe);
                let mut ignored = 0;
                let _ =
                    WinUsb_GetOverlappedResult(device.interface, &overlapped, &mut ignored, true);
            }
            break;
        }
        let mut transferred = 0u32;
        if let Err(error) = unsafe {
            WinUsb_GetOverlappedResult(device.interface, &overlapped, &mut transferred, false)
        } {
            if error.code() != HRESULT::from_win32(ERROR_OPERATION_ABORTED.0) {
                let _ = events.try_send(SessionEvent::Error(format!(
                    "failed to complete USB read: {error}"
                )));
            }
            break;
        }
        let received = &chunk[..(transferred as usize).min(CHUNK)];
        framer.push(received, |report| match decoder.decode(report) {
            Ok(snapshot) => {
                let _ = events.try_send(SessionEvent::Snapshot(snapshot));
            }
            Err(error) => {
                let _ = events.try_send(SessionEvent::Error(error));
            }
        });
    }
}

pub(in crate::gui::game_launcher_screen::controller) struct ReportFramer {
    report: Vec<u8>,
    filled: usize,
}

impl ReportFramer {
    pub(in crate::gui::game_launcher_screen::controller) fn new(report_len: usize) -> Self {
        Self {
            report: vec![0; report_len.max(1)],
            filled: 0,
        }
    }

    pub(in crate::gui::game_launcher_screen::controller) fn push(
        &mut self,
        mut bytes: &[u8],
        mut on_report: impl FnMut(&[u8]),
    ) {
        while !bytes.is_empty() {
            let take = (self.report.len() - self.filled).min(bytes.len());
            self.report[self.filled..self.filled + take].copy_from_slice(&bytes[..take]);
            self.filled += take;
            bytes = &bytes[take..];
            if self.filled == self.report.len() {
                on_report(&self.report);
                self.filled = 0;
            }
        }
    }
}

pub(in crate::gui::game_launcher_screen::controller) fn node_interface(
    hardware_ids: &[String],
    vid: u16,
    pid: u16,
) -> Option<u8> {
    let wanted = format!("VID_{vid:04X}&PID_{pid:04X}");
    for id in hardware_ids {
        let upper = id.to_ascii_uppercase();
        if !upper.contains(&wanted) {
            continue;
        }
        return Some(match upper.find("&MI_") {
            Some(at) => upper
                .get(at + 4..at + 6)
                .and_then(|digits| u8::from_str_radix(digits, 16).ok())
                .unwrap_or(0),
            None => 0,
        });
    }
    None
}

pub(in crate::gui::game_launcher_screen::controller) fn parse_guids(text: &str) -> Vec<GUID> {
    text.split(['\0', ';', ','])
        .map(|part| part.trim().trim_start_matches('{').trim_end_matches('}'))
        .filter(|part| !part.is_empty())
        .filter_map(|part| GUID::try_from(part).ok())
        .collect()
}

fn utf16_strings(bytes: &[u8]) -> Vec<String> {
    let words: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect();
    words
        .split(|&word| word == 0)
        .filter(|part| !part.is_empty())
        .map(String::from_utf16_lossy)
        .collect()
}

fn node_property(
    set: windows::Win32::Devices::DeviceAndDriverInstallation::HDEVINFO,
    info: &SP_DEVINFO_DATA,
    property: windows::Win32::Devices::DeviceAndDriverInstallation::SETUP_DI_REGISTRY_PROPERTY,
) -> Vec<String> {
    let mut buffer = vec![0u8; 2048];
    let read = unsafe {
        SetupDiGetDeviceRegistryPropertyW(set, info, property, None, Some(&mut buffer), None)
    };
    if read.is_err() {
        return Vec::new();
    }
    utf16_strings(&buffer)
}

fn node_guids(
    set: windows::Win32::Devices::DeviceAndDriverInstallation::HDEVINFO,
    info: &SP_DEVINFO_DATA,
) -> Vec<GUID> {
    let Ok(key) =
        (unsafe { SetupDiOpenDevRegKey(set, info, DICS_FLAG_GLOBAL.0, 0, DIREG_DEV, KEY_READ.0) })
    else {
        return Vec::new();
    };
    let mut guids = Vec::new();
    for name in ["DeviceInterfaceGUIDs", "DeviceInterfaceGUID"] {
        let wide: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
        let mut buffer = vec![0u8; 1024];
        let mut size = buffer.len() as u32;
        let status = unsafe {
            RegQueryValueExW(
                key,
                PCWSTR(wide.as_ptr()),
                None,
                None,
                Some(buffer.as_mut_ptr()),
                Some(&mut size),
            )
        };
        if status.is_err() {
            continue;
        }
        buffer.truncate(size as usize);
        guids = parse_guids(&utf16_strings(&buffer).join("\0"));
        if !guids.is_empty() {
            break;
        }
    }
    unsafe {
        let _ = RegCloseKey(key);
    }
    guids
}

fn class_path(class: &GUID, instance: &[u16]) -> Option<Vec<u16>> {
    let set = unsafe {
        SetupDiGetClassDevsW(
            Some(class),
            PCWSTR(instance.as_ptr()),
            None,
            DIGCF_PRESENT | DIGCF_DEVICEINTERFACE,
        )
    }
    .ok()?;
    let _guard = super::hid::DeviceInfoGuard(set);
    let mut interface = SP_DEVICE_INTERFACE_DATA {
        cbSize: std::mem::size_of::<SP_DEVICE_INTERFACE_DATA>() as u32,
        ..Default::default()
    };
    unsafe { SetupDiEnumDeviceInterfaces(set, None, class, 0, &mut interface) }.ok()?;
    super::hid::interface_path(set, &interface).ok()
}

fn find_path(vid: u16, pid: u16, number: u8, own_class: Option<GUID>) -> Option<Vec<u16>> {
    let usb: Vec<u16> = "USB".encode_utf16().chain(Some(0)).collect();
    let set = unsafe {
        SetupDiGetClassDevsW(
            None,
            PCWSTR(usb.as_ptr()),
            None,
            DIGCF_PRESENT | DIGCF_ALLCLASSES,
        )
    }
    .ok()?;
    let _guard = super::hid::DeviceInfoGuard(set);
    let mut index = 0;
    loop {
        let mut info = SP_DEVINFO_DATA {
            cbSize: std::mem::size_of::<SP_DEVINFO_DATA>() as u32,
            ..Default::default()
        };
        if unsafe { SetupDiEnumDeviceInfo(set, index, &mut info) }.is_err() {
            return None;
        }
        index += 1;
        let ids = node_property(set, &info, SPDRP_HARDWAREID);
        if node_interface(&ids, vid, pid) != Some(number) {
            continue;
        }
        let service = node_property(set, &info, SPDRP_SERVICE);
        if !service
            .iter()
            .any(|name| name.eq_ignore_ascii_case("WINUSB"))
        {
            continue;
        }
        let mut instance = vec![0u16; 512];
        if unsafe { SetupDiGetDeviceInstanceIdW(set, &info, Some(&mut instance), None) }.is_err() {
            continue;
        }
        let mut classes = node_guids(set, &info);
        if let Some(class) = own_class {
            if !classes.contains(&class) {
                classes.push(class);
            }
        }
        if let Some(path) = classes
            .iter()
            .find_map(|class| class_path(class, &instance))
        {
            return Some(path);
        }
    }
}

fn open(path: &[u16], audit: &Arc<ResourceAudit>) -> Result<WinUsbDevice, String> {
    let handle = unsafe {
        CreateFileW(
            PCWSTR(path.as_ptr()),
            GENERIC_READ.0 | GENERIC_WRITE.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_FLAG_OVERLAPPED,
            None,
        )
    }
    .map_err(|error| format!("failed to open the USB controller: {error}"))?;
    let file = TrackedHandle::new(handle, audit)?;
    let mut interface = WINUSB_INTERFACE_HANDLE::default();
    unsafe { WinUsb_Initialize(file.raw(), &mut interface) }
        .map_err(|error| format!("failed to start WinUSB on the controller: {error}"))?;
    let mut device = WinUsbDevice {
        interface,
        in_pipe: 0,
        _file: file,
    };
    let mut settings = USB_INTERFACE_DESCRIPTOR::default();
    unsafe { WinUsb_QueryInterfaceSettings(device.interface, 0, &mut settings) }
        .map_err(|error| format!("failed to read the controller's USB interface: {error}"))?;
    for endpoint in 0..settings.bNumEndpoints {
        let mut pipe = WINUSB_PIPE_INFORMATION::default();
        if unsafe { WinUsb_QueryPipe(device.interface, 0, endpoint, &mut pipe) }.is_err() {
            continue;
        }
        if pipe.PipeType == UsbdPipeTypeBulk && pipe.PipeId & 0x80 != 0 {
            device.in_pipe = pipe.PipeId;
            break;
        }
    }
    if device.in_pipe == 0 {
        return Err("the USB controller has no input pipe".into());
    }
    Ok(device)
}

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/game_launcher_screen/controller/transport/winusb__tests.rs"]
mod tests;

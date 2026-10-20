use std::mem::size_of;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use windows::core::{HRESULT, PCWSTR};
use windows::Win32::Devices::DeviceAndDriverInstallation::{
    SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInterfaces, SetupDiGetClassDevsW,
    SetupDiGetDeviceInterfaceDetailW, DIGCF_DEVICEINTERFACE, DIGCF_PRESENT, HDEVINFO,
    SP_DEVICE_INTERFACE_DATA, SP_DEVICE_INTERFACE_DETAIL_DATA_W,
};
use windows::Win32::Devices::HumanInterfaceDevice::{
    HidD_FreePreparsedData, HidD_GetAttributes, HidD_GetHidGuid, HidD_GetPreparsedData,
    HidP_GetCaps, HIDD_ATTRIBUTES, HIDP_CAPS, PHIDP_PREPARSED_DATA,
};
use windows::Win32::Foundation::{
    ERROR_IO_PENDING, ERROR_NO_MORE_ITEMS, ERROR_OPERATION_ABORTED, GENERIC_READ, GENERIC_WRITE,
    WAIT_OBJECT_0,
};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, ReadFile, FILE_FLAG_OVERLAPPED, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows::Win32::System::Threading::{
    CreateEventW, ResetEvent, SetEvent, WaitForMultipleObjects, INFINITE,
};
use windows::Win32::System::IO::{CancelIoEx, GetOverlappedResult, OVERLAPPED};

use crate::game_launcher_screen::ControllerConfig;

use super::super::framework::adapter::{ButtonDecoder, ControllerAdapter, ControllerSession};
use super::super::framework::audit::ResourceAudit;
use super::super::framework::descriptor::{
    hex_u16_option, ControllerDriverDescriptor, DiscoveredController,
};
use super::super::framework::events::{EventSink, SessionEvent};
use super::handles::{ReaderGuard, TrackedHandle};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct HidIdentity {
    pub vid: u16,
    pub pid: u16,
    pub minimum_input_report_len: usize,
}

impl HidIdentity {
    pub fn validate_report_len(&self, actual: usize) -> Result<(), String> {
        if actual < self.minimum_input_report_len {
            Err(format!(
                "HID input report is {actual} bytes; {} are required",
                self.minimum_input_report_len
            ))
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct HidCandidate {
    path: Vec<u16>,
    vid: u16,
    pid: u16,
    input_report_len: usize,
}

pub(in crate::gui::game_launcher_screen::controller) struct HidAdapter {
    descriptor: &'static ControllerDriverDescriptor,
    identities: &'static [(u16, u16)],
    minimum_input_report_len: usize,
    decoder: Arc<dyn ButtonDecoder>,
}

impl HidAdapter {
    pub(in crate::gui::game_launcher_screen::controller) fn new(
        descriptor: &'static ControllerDriverDescriptor,
        identities: &'static [(u16, u16)],
        minimum_input_report_len: usize,
        decoder: Arc<dyn ButtonDecoder>,
    ) -> Self {
        Self {
            descriptor,
            identities,
            minimum_input_report_len,
            decoder,
        }
    }

    pub(in crate::gui::game_launcher_screen::controller) fn match_present(
        &self,
        present: &[(u16, u16, usize)],
    ) -> Vec<DiscoveredController> {
        let mut found = Vec::new();
        for &(vid, pid) in self.identities {
            if present
                .iter()
                .any(|&(present_vid, present_pid, _)| present_vid == vid && present_pid == pid)
            {
                let mut config = self.descriptor.default_config();
                config.options.insert("vid".into(), format!("{vid:04X}"));
                config.options.insert("pid".into(), format!("{pid:04X}"));
                found.push(DiscoveredController {
                    config,
                    label_key: self.descriptor.label_key,
                });
            }
        }
        found
    }
}

impl ControllerAdapter for HidAdapter {
    fn descriptor(&self) -> &'static ControllerDriverDescriptor {
        self.descriptor
    }

    fn discover(&self) -> Result<Vec<DiscoveredController>, String> {
        Ok(self.match_present(&discover_present()?))
    }

    fn connect(
        &self,
        config: &ControllerConfig,
        events: EventSink,
        audit: Arc<ResourceAudit>,
    ) -> Result<Box<dyn ControllerSession>, String> {
        self.validate(config)?;
        let wanted = HidIdentity {
            vid: hex_u16_option(config, "vid")?,
            pid: hex_u16_option(config, "pid")?,
            minimum_input_report_len: self.minimum_input_report_len,
        };
        let candidate = select_matching(enumerate_candidates()?, &wanted)?;
        let device = Arc::new(open_path(&candidate.path, &audit)?);
        let cancellation = Arc::new(TrackedHandle::new(
            unsafe { CreateEventW(None, true, false, None) }
                .map_err(|error| format!("failed to create HID cancellation event: {error}"))?,
            &audit,
        )?);
        let read_event = Arc::new(TrackedHandle::new(
            unsafe { CreateEventW(None, true, false, None) }
                .map_err(|error| format!("failed to create HID read event: {error}"))?,
            &audit,
        )?);
        let (done_tx, done_rx) = mpsc::sync_channel(1);
        let thread_device = device.clone();
        let thread_cancel = cancellation.clone();
        let thread_read_event = read_event.clone();
        let decoder = self.decoder.clone();
        let report_len = candidate.input_report_len;
        let reader = thread::Builder::new()
            .name("gametoaster-controller-hid".into())
            .spawn(move || {
                let _guard = ReaderGuard::new(&audit);
                let _ = events.try_send(SessionEvent::Connected);
                hid_read_loop(
                    &thread_device,
                    &thread_cancel,
                    &thread_read_event,
                    report_len,
                    decoder.as_ref(),
                    &events,
                );
                let _ = events.try_send(SessionEvent::Disconnected);
                let _ = done_tx.try_send(());
            })
            .map_err(|error| format!("failed to start HID reader: {error}"))?;
        Ok(Box::new(HidSession {
            device: Some(device),
            cancellation: Some(cancellation),
            read_event: Some(read_event),
            reader: Some(reader),
            done: done_rx,
        }))
    }
}

struct HidSession {
    device: Option<Arc<TrackedHandle>>,
    cancellation: Option<Arc<TrackedHandle>>,
    read_event: Option<Arc<TrackedHandle>>,
    reader: Option<JoinHandle<()>>,
    done: mpsc::Receiver<()>,
}

impl HidSession {
    fn signal_cancel(&self) {
        if let Some(cancellation) = &self.cancellation {
            unsafe {
                let _ = SetEvent(cancellation.raw());
            }
        }
        if let Some(device) = &self.device {
            unsafe {
                let _ = CancelIoEx(device.raw(), None);
            }
        }
    }

    fn release_after_join(&mut self) {
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
        self.read_event = None;
        self.cancellation = None;
        self.device = None;
    }
}

impl ControllerSession for HidSession {
    fn disconnect(&mut self, timeout: Duration) -> Result<(), String> {
        self.signal_cancel();
        if self.reader.is_some() {
            self.done.recv_timeout(timeout).map_err(|_| {
                "HID reader did not acknowledge cancellation within the handoff timeout".to_string()
            })?;
        }
        self.release_after_join();
        Ok(())
    }
}

impl Drop for HidSession {
    fn drop(&mut self) {
        self.signal_cancel();
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
    }
}

fn hid_read_loop(
    device: &TrackedHandle,
    cancellation: &TrackedHandle,
    read_event: &TrackedHandle,
    report_len: usize,
    decoder: &dyn ButtonDecoder,
    events: &EventSink,
) {
    let mut report = vec![0u8; report_len];
    loop {
        unsafe {
            let _ = ResetEvent(read_event.raw());
        }
        let mut overlapped = OVERLAPPED {
            hEvent: read_event.raw(),
            ..Default::default()
        };
        let pending =
            unsafe { ReadFile(device.raw(), Some(&mut report), None, Some(&mut overlapped)) };
        if let Err(error) = pending {
            if error.code() != HRESULT::from_win32(ERROR_IO_PENDING.0) {
                if error.code() != HRESULT::from_win32(ERROR_OPERATION_ABORTED.0) {
                    let _ =
                        events.try_send(SessionEvent::Error(format!("HID read failed: {error}")));
                }
                break;
            }
        }
        let wait = unsafe {
            WaitForMultipleObjects(&[cancellation.raw(), read_event.raw()], false, INFINITE)
        };
        if wait == WAIT_OBJECT_0 {
            unsafe {
                let _ = CancelIoEx(device.raw(), Some(&overlapped));
            }
            break;
        }
        if wait.0 != WAIT_OBJECT_0.0 + 1 {
            let _ = events.try_send(SessionEvent::Error(format!(
                "unexpected HID wait result 0x{:08X}",
                wait.0
            )));
            break;
        }
        let mut transferred = 0;
        if let Err(error) =
            unsafe { GetOverlappedResult(device.raw(), &overlapped, &mut transferred, false) }
        {
            if error.code() != HRESULT::from_win32(ERROR_OPERATION_ABORTED.0) {
                let _ = events.try_send(SessionEvent::Error(format!(
                    "failed to complete HID read: {error}"
                )));
            }
            break;
        }
        match decoder.decode(&report[..transferred as usize]) {
            Ok(snapshot) => {
                let _ = events.try_send(SessionEvent::Snapshot(snapshot));
            }
            Err(error) => {
                let _ = events.try_send(SessionEvent::Error(error));
            }
        }
    }
}

pub(in crate::gui::game_launcher_screen) fn discover_present(
) -> Result<Vec<(u16, u16, usize)>, String> {
    Ok(enumerate_candidates()?
        .into_iter()
        .map(|candidate| (candidate.vid, candidate.pid, candidate.input_report_len))
        .collect())
}

fn enumerate_candidates() -> Result<Vec<HidCandidate>, String> {
    let audit = ResourceAudit::new();
    let guid = unsafe { HidD_GetHidGuid() };
    let info = unsafe {
        SetupDiGetClassDevsW(
            Some(&guid),
            PCWSTR::null(),
            None,
            DIGCF_PRESENT | DIGCF_DEVICEINTERFACE,
        )
    }
    .map_err(|error| format!("failed to enumerate HID interfaces: {error}"))?;
    let _guard = DeviceInfoGuard(info);
    let mut result = Vec::new();
    let mut index = 0;
    loop {
        let mut interface = SP_DEVICE_INTERFACE_DATA {
            cbSize: size_of::<SP_DEVICE_INTERFACE_DATA>() as u32,
            ..Default::default()
        };
        match unsafe { SetupDiEnumDeviceInterfaces(info, None, &guid, index, &mut interface) } {
            Ok(()) => {}
            Err(error) if error.code() == HRESULT::from_win32(ERROR_NO_MORE_ITEMS.0) => break,
            Err(error) => {
                return Err(format!(
                    "failed to enumerate HID interface {index}: {error}"
                ))
            }
        }
        index += 1;
        let path = interface_path(info, &interface)?;
        let Ok(handle) = open_path(&path, &audit) else {
            continue;
        };
        let mut attributes = HIDD_ATTRIBUTES {
            Size: size_of::<HIDD_ATTRIBUTES>() as u32,
            ..Default::default()
        };
        if !unsafe { HidD_GetAttributes(handle.raw(), &mut attributes) }.as_bool() {
            continue;
        }
        let mut preparsed = PHIDP_PREPARSED_DATA::default();
        if !unsafe { HidD_GetPreparsedData(handle.raw(), &mut preparsed) }.as_bool() {
            continue;
        }
        let mut capabilities = HIDP_CAPS::default();
        let status = unsafe { HidP_GetCaps(preparsed, &mut capabilities) };
        unsafe {
            let _ = HidD_FreePreparsedData(preparsed);
        }
        if status.0 < 0 {
            continue;
        }
        result.push(HidCandidate {
            path,
            vid: attributes.VendorID,
            pid: attributes.ProductID,
            input_report_len: capabilities.InputReportByteLength as usize,
        });
    }
    Ok(result)
}

fn select_matching(
    candidates: Vec<HidCandidate>,
    wanted: &HidIdentity,
) -> Result<HidCandidate, String> {
    let matches: Vec<HidCandidate> = candidates
        .into_iter()
        .filter(|candidate| candidate.vid == wanted.vid && candidate.pid == wanted.pid)
        .collect();
    match matches.as_slice() {
        [] => Err(format!(
            "no HID controller matched VID 0x{:04X}, PID 0x{:04X}",
            wanted.vid, wanted.pid
        )),
        [candidate] => {
            wanted.validate_report_len(candidate.input_report_len)?;
            Ok(candidate.clone())
        }
        _ => Err(format!(
            "multiple HID controllers matched VID 0x{:04X}, PID 0x{:04X}; disconnect duplicates or configure a distinct identity",
            wanted.vid, wanted.pid
        )),
    }
}

fn interface_path(
    info: HDEVINFO,
    interface: &SP_DEVICE_INTERFACE_DATA,
) -> Result<Vec<u16>, String> {
    let mut required = 0;
    let _ = unsafe {
        SetupDiGetDeviceInterfaceDetailW(info, interface, None, 0, Some(&mut required), None)
    };
    if required < size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>() as u32 {
        return Err("SetupAPI returned an invalid HID path size".into());
    }
    let words = (required as usize).div_ceil(size_of::<usize>());
    let mut storage = vec![0usize; words];
    let detail = storage.as_mut_ptr() as *mut SP_DEVICE_INTERFACE_DETAIL_DATA_W;
    unsafe {
        (*detail).cbSize = size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>() as u32;
        SetupDiGetDeviceInterfaceDetailW(info, interface, Some(detail), required, None, None)
    }
    .map_err(|error| format!("failed to read HID interface path: {error}"))?;
    let path = unsafe { std::ptr::addr_of!((*detail).DevicePath) as *const u16 };
    let maximum = (required as usize - size_of::<u32>()) / size_of::<u16>();
    let length = (0..maximum)
        .find(|offset| unsafe { *path.add(*offset) == 0 })
        .ok_or_else(|| "HID interface path was not terminated".to_string())?;
    let mut value = unsafe { std::slice::from_raw_parts(path, length) }.to_vec();
    value.push(0);
    Ok(value)
}

fn open_path(path: &[u16], audit: &Arc<ResourceAudit>) -> Result<TrackedHandle, String> {
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
    .map_err(|error| format!("failed to open HID controller: {error}"))?;
    TrackedHandle::new(handle, audit)
}

struct DeviceInfoGuard(HDEVINFO);

impl Drop for DeviceInfoGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = SetupDiDestroyDeviceInfoList(self.0);
        }
    }
}

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/game_launcher_screen/controller/transport/hid__tests.rs"]
mod tests;

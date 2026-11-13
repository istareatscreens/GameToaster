use std::mem::size_of;
use std::sync::mpsc;
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use windows::core::{HRESULT, PCWSTR};
use windows::Win32::Devices::DeviceAndDriverInstallation::{
    SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInterfaces, SetupDiGetClassDevsW,
    SetupDiGetDeviceInterfaceDetailW, DIGCF_DEVICEINTERFACE, DIGCF_PRESENT, HDEVINFO,
    SP_DEVICE_INTERFACE_DATA, SP_DEVICE_INTERFACE_DETAIL_DATA_W,
};
use windows::Win32::Devices::HumanInterfaceDevice::{
    HidD_FreePreparsedData, HidD_GetAttributes, HidD_GetFeature, HidD_GetHidGuid,
    HidD_GetPreparsedData, HidP_Feature, HidP_GetCaps, HidP_GetValueCaps, HIDD_ATTRIBUTES,
    HIDP_CAPS, HIDP_STATUS_SUCCESS, HIDP_VALUE_CAPS, PHIDP_PREPARSED_DATA,
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
use super::super::framework::buttons::ButtonWord;
use super::super::framework::descriptor::{ControllerDriverDescriptor, DiscoveredController};
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::gui::game_launcher_screen::controller) struct HidUnits {
    pub feature_byte: usize,
    pub settings: &'static [u8],
}

pub(in crate::gui::game_launcher_screen::controller) struct HidAdapter {
    descriptor: &'static ControllerDriverDescriptor,
    identities: &'static [(u16, u16)],
    minimum_input_report_len: usize,
    decoder: Arc<dyn ButtonDecoder>,

    units: Option<HidUnits>,
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
            units: None,
        }
    }

    pub(in crate::gui::game_launcher_screen::controller) fn with_units(
        descriptor: &'static ControllerDriverDescriptor,
        identities: &'static [(u16, u16)],
        minimum_input_report_len: usize,
        decoder: Arc<dyn ButtonDecoder>,
        units: HidUnits,
    ) -> Self {
        Self {
            units: Some(units),
            ..Self::new(descriptor, identities, minimum_input_report_len, decoder)
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

    fn wanted_identity(&self, config: &ControllerConfig) -> Result<HidIdentity, String> {
        Ok(HidIdentity {
            vid: self.descriptor.hex_u16(config, "vid")?,
            pid: self.descriptor.hex_u16(config, "pid")?,
            minimum_input_report_len: self.minimum_input_report_len,
        })
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
        let wanted = self.wanted_identity(config)?;
        let candidates = enumerate_candidates()?;
        let Some(units) = self.units else {
            let candidate = select_matching(candidates, &wanted)?;
            let session = start_reader(&candidate, &audit, self.decoder.clone(), events)?;
            return Ok(Box::new(session));
        };

        let probe = ResourceAudit::new();
        let chosen = select_units(candidates, &wanted, units, |candidate| {
            feature_byte(&candidate.path, units.feature_byte, &probe)
        })?;
        let merge = Arc::new(Mutex::new(UnitMerge::new(events, chosen.len())));
        let mut sessions = Vec::with_capacity(chosen.len());
        for (unit, candidate) in chosen.iter().enumerate() {
            let events = UnitEvents {
                merge: merge.clone(),
                unit,
            };
            sessions.push(start_reader(
                candidate,
                &audit,
                self.decoder.clone(),
                events,
            )?);
        }
        Ok(Box::new(HidUnitsSession { units: sessions }))
    }
}

trait ReaderEvents: Send + 'static {
    fn started(&self);
    fn event(&self, event: SessionEvent);
    fn stopped(&self);
}

impl ReaderEvents for EventSink {
    fn started(&self) {
        let _ = self.try_send(SessionEvent::Connected);
    }

    fn event(&self, event: SessionEvent) {
        let _ = self.try_send(event);
    }

    fn stopped(&self) {
        let _ = self.try_send(SessionEvent::Disconnected);
    }
}

fn start_reader(
    candidate: &HidCandidate,
    audit: &Arc<ResourceAudit>,
    decoder: Arc<dyn ButtonDecoder>,
    events: impl ReaderEvents,
) -> Result<HidSession, String> {
    let device = Arc::new(open_path(&candidate.path, audit)?);
    let cancellation = Arc::new(TrackedHandle::new(
        unsafe { CreateEventW(None, true, false, None) }
            .map_err(|error| format!("failed to create HID cancellation event: {error}"))?,
        audit,
    )?);
    let read_event = Arc::new(TrackedHandle::new(
        unsafe { CreateEventW(None, true, false, None) }
            .map_err(|error| format!("failed to create HID read event: {error}"))?,
        audit,
    )?);
    let (done_tx, done_rx) = mpsc::sync_channel(1);
    let thread_device = device.clone();
    let thread_cancel = cancellation.clone();
    let thread_read_event = read_event.clone();
    let thread_audit = audit.clone();
    let report_len = candidate.input_report_len;
    let reader = thread::Builder::new()
        .name("gametoaster-controller-hid".into())
        .spawn(move || {
            let _guard = ReaderGuard::new(&thread_audit);
            events.started();
            hid_read_loop(
                &thread_device,
                &thread_cancel,
                &thread_read_event,
                report_len,
                decoder.as_ref(),
                &events,
            );
            events.stopped();
            let _ = done_tx.try_send(());
        })
        .map_err(|error| format!("failed to start HID reader: {error}"))?;
    Ok(HidSession {
        device: Some(device),
        cancellation: Some(cancellation),
        read_event: Some(read_event),
        reader: Some(reader),
        done: done_rx,
    })
}

struct UnitMerge {
    sink: EventSink,
    words: Vec<ButtonWord>,
    running: usize,
}

impl UnitMerge {
    fn new(sink: EventSink, units: usize) -> Self {
        Self {
            sink,
            words: vec![ButtonWord::EMPTY; units],
            running: 0,
        }
    }

    fn merged(&self) -> ButtonWord {
        ButtonWord::from_bits(self.words.iter().fold(0, |bits, word| bits | word.bits()))
    }

    fn started(&mut self) {
        self.running += 1;
        if self.running == 1 {
            let _ = self.sink.try_send(SessionEvent::Connected);
        }
    }

    fn event(&mut self, unit: usize, event: SessionEvent) {
        match event {
            SessionEvent::Snapshot(word) => {
                self.words[unit] = word;
                let _ = self.sink.try_send(SessionEvent::Snapshot(self.merged()));
            }
            other => {
                let _ = self.sink.try_send(other);
            }
        }
    }

    fn stopped(&mut self, unit: usize) {
        self.words[unit] = ButtonWord::EMPTY;
        self.running = self.running.saturating_sub(1);
        let _ = self.sink.try_send(if self.running == 0 {
            SessionEvent::Disconnected
        } else {
            SessionEvent::Snapshot(self.merged())
        });
    }
}

struct UnitEvents {
    merge: Arc<Mutex<UnitMerge>>,
    unit: usize,
}

impl UnitEvents {
    fn merge(&self) -> MutexGuard<'_, UnitMerge> {
        self.merge
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl ReaderEvents for UnitEvents {
    fn started(&self) {
        self.merge().started();
    }

    fn event(&self, event: SessionEvent) {
        self.merge().event(self.unit, event);
    }

    fn stopped(&self) {
        self.merge().stopped(self.unit);
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

struct HidUnitsSession {
    units: Vec<HidSession>,
}

impl ControllerSession for HidUnitsSession {
    fn disconnect(&mut self, timeout: Duration) -> Result<(), String> {
        for unit in &self.units {
            unit.signal_cancel();
        }
        let deadline = Instant::now() + timeout;
        for unit in &mut self.units {
            if unit.reader.is_some() {
                unit.done
                    .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                    .map_err(|_| {
                        "HID reader did not acknowledge cancellation within the handoff timeout"
                            .to_string()
                    })?;
            }
            unit.release_after_join();
        }
        Ok(())
    }
}

fn hid_read_loop(
    device: &TrackedHandle,
    cancellation: &TrackedHandle,
    read_event: &TrackedHandle,
    report_len: usize,
    decoder: &dyn ButtonDecoder,
    events: &dyn ReaderEvents,
) {
    let mut report = vec![0u8; report_len];

    let mut refused: Option<String> = None;
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
                    events.event(SessionEvent::Error(format!("HID read failed: {error}")));
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
            events.event(SessionEvent::Error(format!(
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
                events.event(SessionEvent::Error(format!(
                    "failed to complete HID read: {error}"
                )));
            }
            break;
        }
        match decoder.decode(&report[..transferred as usize]) {
            Ok(snapshot) => {
                refused = None;
                events.event(SessionEvent::Snapshot(snapshot));
            }
            Err(error) => {
                if refused.as_deref() != Some(error.as_str()) {
                    refused = Some(error.clone());
                    events.event(SessionEvent::Error(error));
                }
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

fn select_units(
    candidates: Vec<HidCandidate>,
    wanted: &HidIdentity,
    units: HidUnits,
    mut setting_of: impl FnMut(&HidCandidate) -> Option<u8>,
) -> Result<Vec<HidCandidate>, String> {
    let eligible: Vec<(HidCandidate, Option<u8>)> = candidates
        .into_iter()
        .filter(|candidate| {
            candidate.vid == wanted.vid
                && candidate.pid == wanted.pid
                && candidate.input_report_len >= wanted.minimum_input_report_len
        })
        .map(|candidate| {
            let setting = setting_of(&candidate);
            (candidate, setting)
        })
        .collect();
    let chosen: Vec<HidCandidate> = units
        .settings
        .iter()
        .filter_map(|&wanted_setting| {
            eligible
                .iter()
                .find(|(_, setting)| *setting == Some(wanted_setting))
                .map(|(candidate, _)| candidate.clone())
        })
        .collect();
    if chosen.is_empty() {
        let settings: Vec<String> = units.settings.iter().map(|s| format!("{s}P")).collect();
        return Err(format!(
            "no unit set to {} matched VID 0x{:04X}, PID 0x{:04X}",
            settings.join(" or "),
            wanted.vid,
            wanted.pid
        ));
    }
    Ok(chosen)
}

const FEATURE_REPORT_MAX: usize = 256;

fn feature_byte(path: &[u16], at: usize, audit: &Arc<ResourceAudit>) -> Option<u8> {
    let handle = open_path(path, audit).ok()?;
    let mut preparsed = PHIDP_PREPARSED_DATA::default();
    if !unsafe { HidD_GetPreparsedData(handle.raw(), &mut preparsed) }.as_bool() {
        return None;
    }
    let mut capabilities = HIDP_CAPS::default();
    let mut length = 0usize;
    let mut report_id = 0u8;
    if unsafe { HidP_GetCaps(preparsed, &mut capabilities) } == HIDP_STATUS_SUCCESS
        && capabilities.FeatureReportByteLength != 0
    {
        length = capabilities.FeatureReportByteLength as usize;
        let mut count = capabilities.NumberFeatureValueCaps;
        if count != 0 {
            let mut values = vec![HIDP_VALUE_CAPS::default(); count as usize];
            let status = unsafe {
                HidP_GetValueCaps(HidP_Feature, values.as_mut_ptr(), &mut count, preparsed)
            };
            if status == HIDP_STATUS_SUCCESS && count != 0 {
                report_id = values[0].ReportID;
            }
        }
    }
    unsafe {
        let _ = HidD_FreePreparsedData(preparsed);
    }
    if length == 0 || length > FEATURE_REPORT_MAX || length <= at {
        return None;
    }
    let mut report = vec![0u8; length];
    report[0] = report_id;
    if !unsafe { HidD_GetFeature(handle.raw(), report.as_mut_ptr().cast(), length as u32) }
        .as_bool()
    {
        return None;
    }
    Some(report[at])
}

pub(super) fn interface_path(
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

pub(super) struct DeviceInfoGuard(pub(super) HDEVINFO);

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

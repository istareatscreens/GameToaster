use std::sync::Arc;
use std::time::Duration;

use rust_i18n::t;

use super::super::super::framework::audit::ResourceAudit;
use super::super::super::transport::serial::{com_port_path, SerialTransport};
use super::mxjvs::MxjvsTransport;
use super::protocol::{decode, Frame};

const TRANSACTION_TIMEOUT: Duration = Duration::from_millis(250);

const MXJVS: &str = r"\\.\mxjvs";

pub(super) fn normalize_endpoint(endpoint: &str) -> Result<String, String> {
    let endpoint = endpoint.trim();
    if endpoint.eq_ignore_ascii_case(MXJVS) {
        return Ok(MXJVS.into());
    }
    com_port_path(endpoint).ok_or_else(|| t!("controller.validation.jvs_endpoint").to_string())
}

pub(super) trait JvsLink {
    fn send_without_reply(&mut self, request: &[u8]) -> Result<(), String>;
    fn transact(&mut self, request: &[u8]) -> Result<Frame, String>;
    fn sense(&self) -> Result<Option<bool>, String>;

    fn cancelled(&self) -> bool;
}

struct SerialJvsLink {
    transport: SerialTransport,
}

impl JvsLink for SerialJvsLink {
    fn send_without_reply(&mut self, request: &[u8]) -> Result<(), String> {
        self.transport.write_all(request, TRANSACTION_TIMEOUT)
    }

    fn transact(&mut self, request: &[u8]) -> Result<Frame, String> {
        self.transport.write_all(request, TRANSACTION_TIMEOUT)?;
        let mut wire = Vec::with_capacity(128);
        let mut chunk = [0u8; 128];
        loop {
            let count = self.transport.read_some(&mut chunk, TRANSACTION_TIMEOUT)?;
            if count == 0 {
                continue;
            }
            wire.extend_from_slice(&chunk[..count]);
            if wire.len() > 1024 {
                return Err("JVS response exceeded the bounded 1024-byte frame size".into());
            }
            if let Ok(frame) = decode(&wire) {
                return Ok(frame);
            }
        }
    }

    fn sense(&self) -> Result<Option<bool>, String> {
        self.transport.sense_asserted()
    }

    fn cancelled(&self) -> bool {
        self.transport.cancelled()
    }
}

struct MxjvsJvsLink {
    transport: MxjvsTransport,
}

impl JvsLink for MxjvsJvsLink {
    fn send_without_reply(&mut self, request: &[u8]) -> Result<(), String> {
        self.transport
            .exchange(request, TRANSACTION_TIMEOUT)
            .map(|_| ())
    }

    fn transact(&mut self, request: &[u8]) -> Result<Frame, String> {
        let reply = self.transport.exchange(request, TRANSACTION_TIMEOUT)?;
        if reply.is_empty() {
            return Err("mxjvs exchange returned an empty reply".into());
        }
        decode(&reply)
    }

    fn sense(&self) -> Result<Option<bool>, String> {
        Ok(None)
    }

    fn cancelled(&self) -> bool {
        self.transport.cancelled()
    }
}

#[derive(Clone)]
pub(super) enum JvsTransport {
    Serial(SerialTransport),
    Mxjvs(MxjvsTransport),
}

impl JvsTransport {
    pub(super) fn open(endpoint: &str, audit: &Arc<ResourceAudit>) -> Result<Self, String> {
        let path = normalize_endpoint(endpoint)?;
        if path == MXJVS {
            Ok(Self::Mxjvs(MxjvsTransport::open(MXJVS, audit)?))
        } else {
            let transport = SerialTransport::open(&path, audit)?;
            transport.purge_input()?;
            Ok(Self::Serial(transport))
        }
    }

    pub(super) fn cancel(&self) {
        match self {
            Self::Serial(transport) => transport.cancel(),
            Self::Mxjvs(transport) => transport.cancel(),
        }
    }

    pub(super) fn link(&self) -> Box<dyn JvsLink + Send> {
        match self.clone() {
            Self::Serial(transport) => Box::new(SerialJvsLink { transport }),
            Self::Mxjvs(transport) => Box::new(MxjvsJvsLink { transport }),
        }
    }
}

#[cfg(test)]
#[path = "../../../../../../tests/unit/gui/game_launcher_screen/controller/devices/jvs/link__tests.rs"]
mod tests;

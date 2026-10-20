#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Frame {
    Line(Vec<u8>),
    Overlong,
}

pub(crate) struct LineSplitter {
    buffer: Vec<u8>,
    discarding: bool,
    limit: usize,
}

impl LineSplitter {
    pub(crate) fn new(limit: usize) -> Self {
        Self {
            buffer: Vec::new(),
            discarding: false,
            limit,
        }
    }

    pub(crate) fn push(&mut self, bytes: &[u8]) -> Vec<Frame> {
        let mut frames = Vec::new();
        for &byte in bytes {
            if byte == b'\n' {
                if self.discarding {
                    frames.push(Frame::Overlong);
                    self.discarding = false;
                } else {
                    let mut line = std::mem::take(&mut self.buffer);
                    if line.last() == Some(&b'\r') {
                        line.pop();
                    }
                    frames.push(Frame::Line(line));
                }
            } else if self.discarding {
                continue;
            } else if self.buffer.len() == self.limit {
                self.buffer.clear();
                self.discarding = true;
            } else {
                self.buffer.push(byte);
            }
        }
        frames
    }
}

#[cfg(test)]
#[path = "../../../../tests/unit/plugin/runtime/channel/framing__tests.rs"]
mod tests;

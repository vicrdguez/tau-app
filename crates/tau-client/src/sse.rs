//! Server-sent events to frames: `data: <json>` and a blank line is a frame, a line `:` is a heartbeat.

use serde_json::Value;

/// What one stream delivers.
#[derive(Clone, Debug, PartialEq)]
pub enum Frame {
    /// `{"ops": [...]}`. The first frame of a stream is `[["r", value]]`.
    Ops(Vec<Value>),
    /// `{"end": {"reason": ...}}`, the last frame of a clean end.
    End(EndReason),
    /// `:`. Proves the stream alive; changes nothing.
    Heartbeat,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum EndReason {
    Stopping,
    Revoked,
    Failed,
    Other(String),
}

impl EndReason {
    pub fn parse(reason: &str) -> EndReason {
        match reason {
            "stopping" => EndReason::Stopping,
            "revoked" => EndReason::Revoked,
            "failed" => EndReason::Failed,
            other => EndReason::Other(other.to_owned()),
        }
    }
}

/// A frame that is not one of the two shapes.
#[derive(Clone, Debug, PartialEq)]
pub struct BadFrame(pub String);

/// Turns bytes, in pieces of any size, into frames.
#[derive(Default)]
pub struct SseParser {
    line: Vec<u8>,
    data: Vec<u8>,
    has_data: bool,
    last_was_cr: bool,
}

impl SseParser {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn feed(&mut self, bytes: &[u8]) -> Vec<Result<Frame, BadFrame>> {
        let mut out = Vec::new();
        for &b in bytes {
            let crlf = self.last_was_cr && b == b'\n';
            self.last_was_cr = b == b'\r';
            match b {
                _ if crlf => {}
                b'\r' | b'\n' => {
                    let line = std::mem::take(&mut self.line);
                    self.line_done(&line, &mut out);
                }
                _ => self.line.push(b),
            }
        }
        out
    }

    fn line_done(&mut self, line: &[u8], out: &mut Vec<Result<Frame, BadFrame>>) {
        if line.is_empty() {
            if std::mem::take(&mut self.has_data) {
                out.push(frame(&std::mem::take(&mut self.data)));
            }
            return;
        }
        if line[0] == b':' {
            out.push(Ok(Frame::Heartbeat));
            return;
        }
        let (field, value) = match line.iter().position(|&b| b == b':') {
            Some(at) => (
                &line[..at],
                line[at + 1..].strip_prefix(b" ").unwrap_or(&line[at + 1..]),
            ),
            None => (line, &[][..]),
        };
        if field == b"data" {
            if self.has_data {
                self.data.push(b'\n');
            }
            self.data.extend_from_slice(value);
            self.has_data = true;
        }
    }
}

fn frame(data: &[u8]) -> Result<Frame, BadFrame> {
    let bad = || BadFrame(String::from_utf8_lossy(data).into_owned());
    let value: Value = serde_json::from_slice(data).map_err(|_| bad())?;
    if let Some(ops) = value.get("ops").and_then(Value::as_array) {
        return Ok(Frame::Ops(ops.clone()));
    }
    let reason = value
        .get("end")
        .and_then(|end| end.get("reason"))
        .and_then(Value::as_str)
        .ok_or_else(bad)?;
    Ok(Frame::End(EndReason::parse(reason)))
}

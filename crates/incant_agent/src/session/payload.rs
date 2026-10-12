use super::{Failure, MAX_APPROVAL_BYTES};
use serde_json::Value;
use std::io::{self, Write};

struct Bounded(Vec<u8>);
impl Write for Bounded {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > MAX_APPROVAL_BYTES - self.0.len() {
            return Err(io::Error::other("approval payload limit"));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
/// Check depth/node count before serialization. No unbounded clone/to_string of
/// a provider-supplied value is used to construct an approval snapshot.
pub(super) fn prepare(value: &Value) -> Result<(Value, usize), Failure> {
    let mut pending = vec![(value, 0)];
    let mut visited = 0;
    let mut text_bytes = 0;
    while let Some((value, depth)) = pending.pop() {
        visited += 1;
        if depth > 32 || visited > 4096 {
            return Err(Failure::ApprovalInvalid);
        }
        let mut push = |value| {
            if pending.len() + visited >= 4096 {
                return Err(Failure::ApprovalInvalid);
            }
            pending.push((value, depth + 1));
            Ok(())
        };
        let mut text = |bytes| {
            if bytes > MAX_APPROVAL_BYTES - text_bytes {
                return Err(Failure::ApprovalPayloadLimit);
            }
            text_bytes += bytes;
            Ok(())
        };
        match value {
            Value::Array(a) => {
                for value in a {
                    push(value)?;
                }
            }
            Value::Object(o) => {
                for (key, value) in o {
                    text(key.len())?;
                    push(value)?;
                }
            }
            Value::String(s) => text(s.len())?,
            _ => {}
        }
    }
    let mut bytes = Bounded(Vec::new());
    serde_json::to_writer(&mut bytes, value).map_err(|_| Failure::ApprovalPayloadLimit)?;
    let value = serde_json::from_slice(&bytes.0).map_err(|_| Failure::ApprovalInvalid)?;
    Ok((value, bytes.0.len()))
}

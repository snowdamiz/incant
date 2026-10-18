//! Selected-test-only Metal compiler-input capture. Never substitutes generated MSL.
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fmt::Write, sync::Mutex, thread::ThreadId};

const LIMIT: usize = 1024 * 1024;
const TARGET: &str = "wgpu_hal::metal::device";
static LOGGER: Logger = Logger;
static STATE: Mutex<Option<State>> = Mutex::new(None);
struct Logger;
struct State {
    owner: ThreadId,
    active: bool,
    error: Option<&'static str>,
    logs: Vec<String>,
    bytes: usize,
}
struct Limited(String);
impl Write for Limited {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        if self.0.len().saturating_add(s.len()) > LIMIT {
            return Err(std::fmt::Error);
        }
        self.0.push_str(s);
        Ok(())
    }
}
impl log::Log for Logger {
    fn enabled(&self, m: &log::Metadata<'_>) -> bool {
        m.target() == TARGET && m.level() <= log::Level::Debug
    }
    fn log(&self, r: &log::Record<'_>) {
        if !self.enabled(r.metadata()) {
            return;
        }
        let mut guard = STATE.lock().expect("MSL logger poisoned");
        let Some(s) = guard.as_mut() else { return };
        if !s.active {
            return;
        }
        if s.owner != std::thread::current().id() {
            s.error = Some("concurrent Metal compilation during scoped capture");
            return;
        }
        let mut text = Limited(String::new());
        if write!(&mut text, "{}", r.args()).is_err()
            || s.bytes.saturating_add(text.0.len()) > LIMIT
            || s.logs.len() >= 2
        {
            s.error = Some("bounded MSL capture exceeded");
            return;
        }
        s.bytes += text.0.len();
        s.logs.push(text.0);
    }
    fn flush(&self) {}
}
pub(crate) struct Session(std::marker::PhantomData<std::rc::Rc<()>>);
impl Session {
    pub fn start() -> Self {
        assert!(log::STATIC_MAX_LEVEL >= log::LevelFilter::Debug);
        log::set_logger(&LOGGER).expect("visibility requires exclusive logger installation");
        log::set_max_level(log::LevelFilter::Debug);
        let mut state = STATE.lock().unwrap();
        assert!(state.is_none());
        *state = Some(State {
            owner: std::thread::current().id(),
            active: false,
            error: None,
            logs: Vec::new(),
            bytes: 0,
        });
        Self(std::marker::PhantomData)
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        let state = STATE.lock().unwrap().take().unwrap();
        // A failed capture already preserves its receipt before asserting. Do
        // not double-panic while unwinding a native API/capture refusal.
        if !std::thread::panicking() {
            assert!(!state.active);
            assert!(state.error.is_none(), "{:?}", state.error);
        }
    }
}
pub(super) fn capture<T>(
    enabled: bool,
    entries: &[(&str, &str)],
    create: impl FnOnce() -> T,
) -> (T, Value) {
    if !enabled {
        return (create(), Value::Null);
    }
    {
        let mut state = STATE.lock().unwrap();
        let s = state.as_mut().expect("MSL session required");
        assert_eq!(s.owner, std::thread::current().id());
        assert!(!s.active && s.error.is_none());
        s.logs.clear();
        s.active = true;
    }
    let result = create();
    let mut state = STATE.lock().unwrap();
    let s = state.as_mut().unwrap();
    s.active = false;
    let valid = s.error.is_none() && valid_logs(&s.logs, entries);
    // Return a failed receipt too; the caller writes it before refusing drawing.
    let receipt = json!({"complete":valid,"error":s.error,"actual_HAL_MSL":s.logs.iter().map(|text|
        json!({"text":text,"bytes":text.len(),"sha256":format!("{:x}",Sha256::digest(text.as_bytes()))})).collect::<Vec<_>>(),
        "cumulative_MSL_bytes":s.bytes,"limit_bytes":LIMIT,"native_machine_code_observed":false});
    (result, receipt)
}
fn valid_logs(logs: &[String], entries: &[(&str, &str)]) -> bool {
    logs.len() == entries.len()
        && logs.iter().zip(entries).all(|(text, (entry, stage))| {
            let prefix =
                format!("Naga generated shader for entry point '{entry}' and stage {stage}\n");
            text.starts_with(&prefix) && text.len() > prefix.len()
        })
}
#[test]
fn visibility_msl_requires_actual_complete_stage_records() {
    let entries = [("vertex", "Vertex"), ("fragment", "Fragment")];
    let mut logs: Vec<_> = entries
        .iter()
        .map(|(e, s)| {
            format!("Naga generated shader for entry point '{e}' and stage {s}\nmetal source")
        })
        .collect();
    assert!(valid_logs(&logs, &entries));
    assert!(!valid_logs(&logs[..1], &entries));
    logs.reverse();
    assert!(!valid_logs(&logs, &entries));
    assert!(!valid_logs(&[], &entries));
}

//! Data-only assertions over completed game ticks. No script or host IO capability.
use incant_script::PlaySession;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    fs::File,
    io::{Read, Write},
    path::Path,
};
use thiserror::Error;

const MAX_BYTES: usize = 256 * 1024;
const MAX_CHECKS: usize = 256;
const MAX_PREVIEW: usize = 1024;

#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PlayAssertions {
    format: String,
    version: u32,
    #[schemars(length(min = 1, max = 256))]
    checks: Vec<Check>,
}
#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Check {
    /// Unique descriptive name; at most 120 UTF-8 bytes.
    name: String,
    /// Absolute game tick, including an initial or restored checkpoint.
    tick: u64,
    /// JSON Pointer below /state, /script_state or /input.
    path: String,
    expect: Expect,
}
#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum Expect {
    Equals { value: Value },
    Approx { value: f64, tolerance: f64 },
    Range { min: f64, max: f64 },
    Exists { exists: bool },
}
#[derive(Debug, Error)]
pub enum AssertionError {
    #[error("assertion file exceeds 256 KiB or 256 checks")]
    Limit,
    #[error("unsupported assertion format or version")]
    Version,
    #[error("invalid assertion: {0}")]
    Invalid(&'static str),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
#[derive(Serialize)]
pub struct Outcome {
    #[serde(flatten)]
    check: Check,
    passed: bool,
    /// A bounded JSON preview; null means the path did not exist.
    actual: Option<Preview>,
}
#[derive(Serialize)]
struct Preview {
    json: String,
    truncated: bool,
}

#[derive(Default)]
pub struct Assertions {
    checks: std::collections::VecDeque<Check>,
    results: Vec<Outcome>,
}
impl Assertions {
    pub fn load(path: &Path, start: u64, end: u64) -> Result<Self, AssertionError> {
        let mut text = String::new();
        File::open(path)?
            .take(MAX_BYTES as u64 + 1)
            .read_to_string(&mut text)?;
        Self::from_text(&text, start, end)
    }
    fn from_text(text: &str, start: u64, end: u64) -> Result<Self, AssertionError> {
        if text.len() > MAX_BYTES {
            return Err(AssertionError::Limit);
        }
        let mut plan: PlayAssertions = serde_json::from_str(text)?;
        if plan.format != "incant-play-assertions" || plan.version != 1 {
            return Err(AssertionError::Version);
        }
        if plan.checks.len() > MAX_CHECKS {
            return Err(AssertionError::Limit);
        }
        if plan.checks.is_empty() {
            return Err(AssertionError::Invalid("at least one check is required"));
        }
        let mut names = BTreeSet::new();
        for check in &plan.checks {
            if check.name.trim().is_empty() || check.name.len() > 120 || !names.insert(&check.name)
            {
                return Err(AssertionError::Invalid(
                    "names must be nonempty, unique and at most 120 bytes",
                ));
            }
            if check.tick < start || check.tick > end {
                return Err(AssertionError::Invalid(
                    "tick is outside the requested playback interval",
                ));
            }
            let root = check.path.split('/').nth(1);
            if !check.path.starts_with('/')
                || !matches!(root, Some("state" | "script_state" | "input"))
                || check.path.len() > 1024
            {
                return Err(AssertionError::Invalid(
                    "path must be a bounded game-state JSON Pointer",
                ));
            }
            let mut chars = check.path.chars();
            while let Some(c) = chars.next() {
                if c == '~' && !matches!(chars.next(), Some('0' | '1')) {
                    return Err(AssertionError::Invalid("invalid JSON Pointer escape"));
                }
            }
            match &check.expect {
                Expect::Approx { value, tolerance }
                    if !value.is_finite() || !tolerance.is_finite() || *tolerance < 0. =>
                {
                    return Err(AssertionError::Invalid(
                        "approx requires a finite value and nonnegative finite tolerance",
                    ));
                }
                Expect::Range { min, max } if !min.is_finite() || !max.is_finite() || min > max => {
                    return Err(AssertionError::Invalid(
                        "range requires finite ordered bounds",
                    ));
                }
                _ => {}
            }
        }
        plan.checks.sort_by_key(|check| check.tick);
        Ok(Self {
            checks: plan.checks.into(),
            results: Vec::new(),
        })
    }
    pub fn evaluate(&mut self, tick: u64, play: &mut PlaySession) {
        if self.checks.front().is_none_or(|check| check.tick != tick) {
            return;
        }
        let snapshot = json!({"state": play.snapshot(), "script_state": play.host.state(), "input": play.input()});
        while self.checks.front().is_some_and(|check| check.tick == tick) {
            let check = self.checks.pop_front().expect("scheduled check");
            let actual = snapshot.pointer(&check.path);
            let passed = match &check.expect {
                Expect::Equals { value } => actual.is_some_and(|actual| json_equal(actual, value)),
                Expect::Approx { value, tolerance } => actual
                    .and_then(Value::as_f64)
                    .is_some_and(|v| (v - value).abs() <= *tolerance),
                Expect::Range { min, max } => actual
                    .and_then(Value::as_f64)
                    .is_some_and(|v| (*min..=*max).contains(&v)),
                Expect::Exists { exists } => actual.is_some() == *exists,
            };
            self.results.push(Outcome {
                check,
                passed,
                actual: actual.map(preview),
            });
        }
    }
    pub fn passed(&self) -> bool {
        self.checks.is_empty() && self.results.iter().all(|result| result.passed)
    }
    pub fn results(self) -> Vec<Outcome> {
        self.results
    }
}
// JSON has one number type. Compare 2 and 2.0 equally without rounding a large
// integer through f64 (for example 2^53+1 must not equal the float 2^53).
fn json_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(a, b)| json_equal(a, b))
        }
        (Value::Object(a), Value::Object(b)) => {
            a.len() == b.len()
                && a.iter()
                    .all(|(key, a)| b.get(key).is_some_and(|b| json_equal(a, b)))
        }
        (Value::Number(a), Value::Number(b)) if a.is_f64() != b.is_f64() => {
            let (float, integer) = if a.is_f64() { (a, b) } else { (b, a) };
            let f = float.as_f64().expect("float number");
            if f.fract() != 0. {
                return false;
            }
            if let Some(i) = integer.as_i64() {
                f >= i64::MIN as f64 && f < -(i64::MIN as f64) && f as i64 == i
            } else if let Some(i) = integer.as_u64() {
                f >= 0. && f < u64::MAX as f64 && f as u64 == i
            } else {
                false
            }
        }
        _ => a == b,
    }
}

fn preview(value: &Value) -> Preview {
    struct Capped(Vec<u8>);
    impl Write for Capped {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            let remaining = MAX_PREVIEW - self.0.len();
            self.0
                .extend_from_slice(&bytes[..bytes.len().min(remaining)]);
            if bytes.len() > remaining {
                return Err(std::io::Error::other("preview limit"));
            }
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut output = Capped(Vec::new());
    let truncated = serde_json::to_writer(&mut output, value).is_err();
    Preview {
        json: String::from_utf8_lossy(&output.0).into_owned(),
        truncated,
    }
}

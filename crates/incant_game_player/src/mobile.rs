//! Bounded platform entry for the ordinary cooked player. The platform owns the
//! request, paths and calling thread; no authoring or executable loading is added.
use serde::{Deserialize, Serialize};
use std::{fs, io::Write, path::Path};

pub const MAX_REQUEST_BYTES: usize = 64 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum MobileError {
    #[error("invalid mobile runtime request: {0}")]
    Invalid(&'static str),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    version: u32,
    package: String,
    output_directory: String,
    ticks: u64,
    input_recording: Option<String>,
    observations: Option<String>,
    ui_viewport: Option<[u32; 2]>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Report {
    pub format: String,
    pub version: u32,
    pub passed: bool,
    pub os: String,
    pub architecture: String,
    pub panic: String,
    pub debug_assertions: bool,
    pub execution: String,
    pub game: Option<serde_json::Value>,
    pub error: Option<String>,
}

fn valid_path(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 4096
        && Path::new(value).is_absolute()
        && !value.chars().any(char::is_control)
}

/// Run the build-linked program against a cooked package using the same parser,
/// simulation, input replay, assertions and PCM output as the desktop CLI.
///
/// Requires a new output directory. Completed/failed player runs write result.json
/// and return a report, including player I/O failure; request validation and
/// directory/result publication errors return an error. The
/// package is read-only. A result is CPU evidence, not device frame-time evidence.
pub fn execute(bytes: &[u8]) -> Result<Report, MobileError> {
    if bytes.is_empty() || bytes.len() > MAX_REQUEST_BYTES {
        return Err(MobileError::Invalid("request byte limit"));
    }
    let request: Request = serde_json::from_slice(bytes)?;
    if request.version != 1 || !(1..=3600).contains(&request.ticks) {
        return Err(MobileError::Invalid("version or tick bounds"));
    }
    for path in [
        Some(request.package.as_str()),
        Some(request.output_directory.as_str()),
        request.input_recording.as_deref(),
        request.observations.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
        if !valid_path(path) {
            return Err(MobileError::Invalid(
                "expected bounded absolute UTF-8 paths",
            ));
        }
    }
    if request
        .ui_viewport
        .is_some_and(|size| size.iter().any(|n| !(1..=65536).contains(n)))
    {
        return Err(MobileError::Invalid("viewport bounds"));
    }
    let output = Path::new(&request.output_directory);
    // Reserve this run before gameplay; never reuse another run's output.
    fs::create_dir(output)?;
    let mut args = vec![
        "--package".into(),
        request.package,
        "--ticks".into(),
        request.ticks.to_string(),
        "--output".into(),
        output.join("audio.pcm").to_string_lossy().into_owned(),
    ];
    if let Some(path) = request.input_recording {
        args.extend(["--input-recording".into(), path]);
    }
    if let Some(path) = request.observations {
        args.extend(["--observe".into(), path]);
    }
    if let Some([width, height]) = request.ui_viewport {
        args.extend(["--ui-viewport".into(), format!("{width}x{height}")]);
    }
    let mut message = None;
    let result = crate::run(args, |text| message = Some(text.to_owned()));
    let report = Report {
        format: "incant-mobile-runtime".into(),
        version: 1,
        passed: result.is_ok(),
        os: std::env::consts::OS.into(),
        architecture: std::env::consts::ARCH.into(),
        panic: if cfg!(panic = "abort") {
            "abort"
        } else {
            "unwind"
        }
        .into(),
        debug_assertions: cfg!(debug_assertions),
        execution: "offline_cpu".into(),
        game: message
            .map(|text| serde_json::from_str(&text))
            .transpose()?,
        error: result.err().map(|error| error.to_string()),
    };
    // Same-directory, create-only result publication. PCM and JSON remain
    // separate outputs; an I/O failure is not a multi-file rollback promise.
    let mut staged = tempfile::Builder::new()
        .prefix(".incant-result-")
        .tempfile_in(output)?;
    staged.write_all(&serde_json::to_vec(&report)?)?;
    staged.as_file().sync_all()?;
    staged
        .persist_noclobber(output.join("result.json"))
        .map_err(|error| error.error)?;
    Ok(report)
}

/// C entry for Swift/NDK hosts. Returns 0 for a passed run, 1 for a player error
/// (including player I/O; result.json explains it), 2 for an invalid request, or 3
/// for directory/result publication I/O failure.
/// The call is synchronous and opens no window or audio device.
///
/// # Safety
/// For nonzero length within MAX_REQUEST_BYTES, `request` must point to that many
/// initialized readable bytes, unmodified until return. It need not be terminated.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn incant_game_run_v1(request: *const u8, length: usize) -> i32 {
    if request.is_null() || length == 0 || length > MAX_REQUEST_BYTES {
        return 2;
    }
    // SAFETY: the bounded readable allocation is the platform caller's contract.
    let bytes = unsafe { std::slice::from_raw_parts(request, length) };
    match execute(bytes) {
        Ok(report) => i32::from(!report.passed),
        Err(MobileError::Io(_)) => 3,
        Err(MobileError::Invalid(_) | MobileError::Json(_)) => 2,
    }
}

#[cfg(test)]
mod tests;

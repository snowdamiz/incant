//! A retained native CPU child. Authoring is loaded once, before the ready
//! handshake; the command bus is then dropped. The pipe exposes no project
//! mutation, shell, supplied executable, source or bytecode-loading operation.
use incant_agent_native::Controller;
use incant_runtime::PHYSICS_VARIANT;
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{self, BufRead, Read, Write},
    path::PathBuf,
};

const VERSION: u32 = 1;
const MAX_REQUEST: usize = 256 * 1024;
const MAX_RESPONSE: usize = 8 * 1024 * 1024;

#[derive(clap::Args)]
pub struct Options {
    pub project: PathBuf,
    /// Exact project bytes selected by the trusted parent before it built us.
    #[arg(long)]
    pub project_sha256: String,
    #[arg(long)]
    pub scene: Option<String>,
    #[arg(long)]
    pub camera: Option<String>,
    /// Logical viewport, using the same startup contract as native-play.
    #[arg(long, value_parser = super::native_play::viewport)]
    pub ui_viewport: Option<[f32; 2]>,
    #[arg(long, default_value_t = 48000, value_parser = clap::value_parser!(u32).range(8000..=192000))]
    pub sample_rate: u32,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("native worker: {0}")]
    Invalid(&'static str),
    #[error("native worker IO: {0}")]
    Io(#[from] io::Error),
    #[error("native worker encoding: {0}")]
    Encode(#[from] serde_json::Error),
    #[error("native worker startup: {0}")]
    Startup(#[source] Box<dyn std::error::Error>),
}
type Result<T> = std::result::Result<T, Error>;
fn startup(error: impl std::error::Error + 'static) -> Error {
    Error::Startup(Box::new(error))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    version: u32,
    id: u64,
    request: Operation,
}
#[derive(Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
enum Operation {
    Step { params: Value },
    Observe { params: Value },
    Stop {},
}

fn frame(input: &mut impl BufRead) -> Result<Option<Vec<u8>>> {
    let mut bytes = Vec::new();
    input
        .take(MAX_REQUEST as u64 + 1)
        .read_until(b'\n', &mut bytes)?;
    if bytes.is_empty() {
        return Ok(None);
    }
    if bytes.len() > MAX_REQUEST {
        return Err(Error::Invalid("request exceeds 256 KiB; session closed"));
    }
    if bytes.pop() != Some(b'\n') {
        return Err(Error::Invalid("truncated request; session closed"));
    }
    Ok(Some(bytes))
}

fn send(output: &mut impl Write, response: Value) -> Result<()> {
    let bytes = serde_json::to_vec(&response)?;
    if bytes.len() > MAX_RESPONSE {
        return Err(Error::Invalid("response exceeds 8 MiB; session closed"));
    }
    output.write_all(&bytes)?;
    output.write_all(b"\n")?;
    output.flush()?;
    Ok(())
}
fn refusal(id: Option<u64>, code: &'static str, message: impl std::fmt::Display) -> Value {
    json!({"version":VERSION,"id":id,"error":{"code":code,"message":message.to_string()}})
}

/// No wall-clock simulation occurs while waiting for the parent. Each accepted
/// step advances exactly one native tick. Observation is explicitly selected;
/// it never serializes the authoring document or enumerates the whole world.
fn serve(
    controller: &mut Controller,
    input: &mut impl BufRead,
    output: &mut impl Write,
) -> Result<()> {
    let mut next = 1_u64;
    while let Some(bytes) = frame(input)? {
        let request: Request = match serde_json::from_slice(&bytes) {
            Ok(value) => value,
            Err(error) => {
                send(output, refusal(None, "invalid_request", error))?;
                continue;
            }
        };
        if request.version != VERSION {
            send(
                output,
                refusal(
                    Some(request.id),
                    "version",
                    "unsupported worker protocol version",
                ),
            )?;
            continue;
        }
        if request.id != next {
            send(
                output,
                refusal(
                    Some(request.id),
                    "sequence",
                    format!("expected request {next}"),
                ),
            )?;
            continue;
        }
        // A well-formed request consumes its ID even when native admission
        // refuses it. Retrying an accepted tick can never apply it twice.
        next = next
            .checked_add(1)
            .ok_or(Error::Invalid("request counter exhausted"))?;
        let stop = matches!(request.request, Operation::Stop {});
        let result = match request.request {
            Operation::Step { params } => controller.step(params),
            Operation::Observe { params } => controller.state(params),
            Operation::Stop {} => controller
                .stop(json!({}))
                .map(|()| json!({"playing":false})),
        };
        let response = match result {
            Ok(value) => json!({"version":VERSION,"id":request.id,"result":value}),
            Err(error) => refusal(Some(request.id), "native", error),
        };
        send(output, response)?;
        if stop {
            break;
        }
    }
    Ok(())
}

pub fn run(options: Options) -> Result<()> {
    if options.project_sha256.len() != 64
        || !options
            .project_sha256
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(Error::Invalid(
            "expected a lowercase SHA-256 project identity",
        ));
    }
    let project = options.project.canonicalize()?;
    let mut bytes = Vec::new();
    File::open(&project)?
        .take(64 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 64 * 1024 * 1024 {
        return Err(Error::Invalid("project exceeds 64 MiB"));
    }
    if format!("{:x}", Sha256::digest(&bytes)) != options.project_sha256 {
        return Err(Error::Invalid("project changed before worker startup"));
    }
    let project_text = std::str::from_utf8(&bytes).map_err(startup)?;
    let document = incant_doc::Project::from_text(project_text).map_err(startup)?;
    let bus = incant_cmd::CommandBus::new(document).map_err(startup)?;
    let mut controller = Controller::new(super::native_play::embedded::load);
    let state = controller
        .start(
            &bus,
            project
                .parent()
                .ok_or(Error::Invalid("missing project root"))?,
            json!({
                "scene":options.scene,"camera":options.camera,"ui_viewport":options.ui_viewport,
                "sample_rate":options.sample_rate,
            }),
        )
        .map_err(startup)?;
    drop(bus);
    drop(bytes);
    let mut output = io::stdout().lock();
    send(
        &mut output,
        json!({
            "event":"ready","version":VERSION,"physics_mode":PHYSICS_VARIANT.mode(),
            "physics_variant":PHYSICS_VARIANT,"project_sha256":options.project_sha256,
            "execution":"native_cpu","state":state,
        }),
    )?;
    // EOF, fatal framing errors and explicit Stop all drop the entire native
    // composition when this function returns. No background simulation thread.
    serve(&mut controller, &mut io::stdin().lock(), &mut output)
}

#[cfg(test)]
mod tests;

//! Standalone preview stays read-only; shared application file schemas carry no path.
use crate::Result;
use incant_cmd::{CommandBus, MAX_ENGINE_REVIEW_BYTES};
use incant_doc::Project;
use serde::Deserialize;
use serde_json::Value;
use std::{fs::File, io::Read, path::Path};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReviewRequest {
    expected_revision: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SaveRequest {
    #[serde(default, deserialize_with = "supplied_revision")]
    pub expected_revision: Option<u64>,
}

// Missing means the legacy host-local save request; a supplied null or non-u64 is invalid.
fn supplied_revision<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Option<u64>, D::Error> {
    u64::deserialize(deserializer).map(Some)
}

fn source(path: &Path) -> Result<String> {
    let mut source = String::new();
    File::open(path)?
        .take(MAX_ENGINE_REVIEW_BYTES as u64 + 1)
        .read_to_string(&mut source)?;
    if source.len() > MAX_ENGINE_REVIEW_BYTES {
        return Err(incant_cmd::EngineReviewError::Limit.into());
    }
    Ok(source)
}

pub(super) fn inspect(path: &Path, review: bool) -> Result<Value> {
    let source = source(path)?;
    let project = Project::from_text(&source)?;
    if review {
        let bus = CommandBus::new(project)?;
        Ok(serde_json::to_value(bus.review_engine_pin(&source, 0)?)?)
    } else {
        Ok(serde_json::to_value(project.engine_compatibility()?)?)
    }
}

pub(super) fn review(bus: &CommandBus, path: &Path, params: Value) -> Result<Value> {
    let request: ReviewRequest = serde_json::from_value(params)?;
    Ok(serde_json::to_value(bus.review_engine_pin(
        &source(path)?,
        request.expected_revision,
    )?)?)
}

pub(super) fn schemas() -> std::collections::BTreeMap<String, Value> {
    let mut schemas = incant_doc::schema_registry();
    schemas.insert(
        "Command".into(),
        serde_json::json!(schemars::schema_for!(incant_cmd::Command)),
    );
    schemas.insert(
        "EnginePinReview".into(),
        serde_json::json!(schemars::schema_for!(incant_cmd::EnginePinReview)),
    );
    schemas.extend(incant_cmd::project_file_schemas());
    schemas
}

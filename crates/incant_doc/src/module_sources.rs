//! Authoring metadata only. A source identity never selects a runtime module.
use crate::{Id, Project, safe_relative_path};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RustModuleSource {
    pub id: Id,
    #[schemars(length(min = 1, max = 256))]
    pub name: String,
    #[schemars(length(min = 1, max = 1024))]
    pub path: String,
}

pub(super) fn validate(project: &Project, issue: &mut impl FnMut(String, &str)) {
    let mut paths = BTreeSet::new();
    for (id, source) in &project.module_sources {
        let at = format!("/module_sources/{id}");
        if source.id != *id {
            issue(format!("{at}/id"), "map key and ID differ");
        }
        if source.name.trim().is_empty() || source.name.len() > 256 {
            issue(
                format!("{at}/name"),
                "module source name must contain 1..256 bytes",
            );
        }
        // Mirror the worker's source-only grammar; no Cargo/build-script inputs.
        if !safe_relative_path(&source.path)
            || source.path.len() > 1024
            || !source.path.ends_with(".rs")
            || !source.path.contains('/')
            || source.path.split('/').count() > 8
            || source.path.split('/').any(|part| {
                let stem = part
                    .split('.')
                    .next()
                    .unwrap_or_default()
                    .to_ascii_uppercase();
                !part
                    .as_bytes()
                    .first()
                    .is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'_')
                    || part.ends_with('.')
                    || part.len() > 128
                    || matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
                    || (stem.len() == 4
                        && (stem.starts_with("COM") || stem.starts_with("LPT"))
                        && stem.as_bytes()[3].is_ascii_digit())
                    || part.eq_ignore_ascii_case("build.rs")
                    || !part
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
            })
        {
            issue(
                format!("{at}/path"),
                "expected a bounded portable Rust source path, without build.rs",
            );
        }
        if !paths.insert(source.path.to_ascii_lowercase())
            || project
                .scripts
                .values()
                .any(|script| script.path.eq_ignore_ascii_case(&source.path))
            || project
                .assets
                .values()
                .any(|asset| asset.path.eq_ignore_ascii_case(&source.path))
        {
            issue(
                format!("{at}/path"),
                "registered source path aliases another source or asset",
            );
        }
    }
}

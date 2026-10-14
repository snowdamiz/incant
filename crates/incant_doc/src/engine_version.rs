//! Bounded authoring compatibility. Format conversion is deliberately separate.
use schemars::JsonSchema;
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const MAX_ENGINE_REQUIREMENT_BYTES: usize = 256;
pub const MAX_ENGINE_COMPARATORS: usize = 16;

/// SemVer requirement text, preserved exactly rather than silently normalized.
/// Build metadata is not an engine identity: SemVer matching ignores it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(try_from = "String", into = "String")]
#[schemars(with = "String", extend("minLength" = 1, "maxLength" = 256))]
pub struct EngineRequirement(String);

#[derive(Debug, Error)]
pub enum EngineVersionError {
    #[error("engine requirement must contain 1..=256 UTF-8 bytes and at most 16 comparators")]
    Limit,
    #[error("invalid engine requirement: {0}")]
    Syntax(semver::Error),
    #[error("engine {current} does not satisfy project requirement {required}")]
    Unsupported { current: String, required: String },
}

impl EngineRequirement {
    pub fn parse(text: impl Into<String>) -> Result<Self, EngineVersionError> {
        let text = text.into();
        Self::parsed(&text)?;
        Ok(Self(text))
    }

    fn parsed(text: &str) -> Result<VersionReq, EngineVersionError> {
        // Bound input and comparator construction before entering the parser.
        if text.trim().is_empty()
            || text.len() > MAX_ENGINE_REQUIREMENT_BYTES
            || text.bytes().filter(|&b| b == b',').count() >= MAX_ENGINE_COMPARATORS
        {
            return Err(EngineVersionError::Limit);
        }
        let parsed = VersionReq::parse(text).map_err(EngineVersionError::Syntax)?;
        if parsed.comparators.len() > MAX_ENGINE_COMPARATORS {
            return Err(EngineVersionError::Limit);
        }
        Ok(parsed)
    }

    pub fn exact_current() -> Self {
        Self(format!("={ENGINE_VERSION}"))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Standard SemVer prerelease opt-in applies. A stable range or `*` alone
    /// never admits a prerelease; an explicit matching prerelease comparator can.
    pub fn require_current(&self) -> Result<(), EngineVersionError> {
        let current = Version::parse(ENGINE_VERSION).map_err(EngineVersionError::Syntax)?;
        if Self::parsed(&self.0)?.matches(&current) {
            Ok(())
        } else {
            Err(EngineVersionError::Unsupported {
                current: ENGINE_VERSION.into(),
                required: self.0.clone(),
            })
        }
    }
}

impl TryFrom<String> for EngineRequirement {
    type Error = EngineVersionError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}
impl From<EngineRequirement> for String {
    fn from(value: EngineRequirement) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EngineDeclaration {
    LegacyUnpinned,
    Compatible,
}

/// A report for an already validated project, not permission to migrate a file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EngineCompatibility {
    pub current_engine: String,
    pub schema_version: u32,
    pub required_engine: Option<EngineRequirement>,
    pub declaration: EngineDeclaration,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn prereleases_need_explicit_same_release_comparator_and_build_is_not_identity() {
        let pre = Version::parse("1.2.3-beta.2").unwrap();
        for text in ["*", ">=1.0.0, <2.0.0", "=1.2.3", ">=1.1.0-beta.1"] {
            assert!(!EngineRequirement::parsed(text).unwrap().matches(&pre));
        }
        assert!(
            EngineRequirement::parsed(">=1.2.3-beta.1, <2.0.0")
                .unwrap()
                .matches(&pre)
        );
        assert!(
            !EngineRequirement::parsed("=1.2.3-beta.1")
                .unwrap()
                .matches(&pre)
        );
        assert!(
            EngineRequirement::parsed("=1.2.3+build-a")
                .unwrap()
                .matches(&Version::parse("1.2.3+build-b").unwrap())
        );
        EngineRequirement::exact_current()
            .require_current()
            .unwrap();
    }
}

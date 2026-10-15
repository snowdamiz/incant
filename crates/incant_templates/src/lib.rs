//! Authoring-only starter projects. Runtime crates never depend on this module.
//! Recipes are ordinary commands; the trusted host owns initial file creation.
mod create;
mod recipe;
pub use create::{CreatedProject, create};
pub use recipe::{TemplatePlan, plan};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, str::FromStr};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Template {
    Empty,
    ThirdPerson,
    Platformer2d,
    TopDown,
    FirstPerson,
}
pub const ALL: [Template; 5] = [
    Template::Empty,
    Template::ThirdPerson,
    Template::Platformer2d,
    Template::TopDown,
    Template::FirstPerson,
];
impl Template {
    pub fn name(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::ThirdPerson => "third-person",
            Self::Platformer2d => "platformer2d",
            Self::TopDown => "top-down",
            Self::FirstPerson => "first-person",
        }
    }
    pub fn description(self) -> &'static str {
        match self {
            Self::Empty => "Empty scene with an authored perspective camera",
            Self::ThirdPerson => "3D collision, jump, orbit controls and following camera",
            Self::Platformer2d => {
                "2D gravity, jump, solid tilemap, animated sprite and following camera"
            }
            Self::TopDown => {
                "Top-down movement, solid tilemap, animated sprite and following camera"
            }
            Self::FirstPerson => "3D collision, jump, look controls and eye-level camera",
        }
    }
    pub(crate) fn planar(self) -> bool {
        matches!(self, Self::Platformer2d | Self::TopDown)
    }
}
impl FromStr for Template {
    type Err = TemplateError;
    fn from_str(value: &str) -> Result<Self> {
        ALL.into_iter()
            .find(|kind| kind.name() == value)
            .ok_or(TemplateError::Unknown)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TemplateError {
    #[error(
        "unknown template; expected empty, third-person, platformer2d, top-down or first-person"
    )]
    Unknown,
    #[error(
        "template creation requires a new directory under an existing real directory, without symlink components"
    )]
    Destination,
    #[error(
        "template name must be nonblank, at most 256 UTF-8 bytes and contain no control characters"
    )]
    Name,
    #[error("template requires the matching imported bundled asset")]
    Asset,
    #[error("template verification failed: {0}")]
    Verification(&'static str),
    #[error(
        "template creation incomplete at {path}; no completed creation receipt was issued: {source}"
    )]
    Incomplete {
        path: PathBuf,
        source: Box<TemplateError>,
    },
    #[error(transparent)]
    Command(#[from] incant_cmd::CommandError),
    #[error(transparent)]
    Document(#[from] incant_doc::DocumentError),
    #[error(transparent)]
    Import(#[from] incant_import::ImportError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
pub type Result<T> = std::result::Result<T, TemplateError>;

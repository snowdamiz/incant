//! Authoring-side cooking. Native players depend on this only at build time.
//! Script-only builds omit the document/scene authoring dependency graph.
#[cfg(feature = "scene")]
mod scene;
#[cfg(feature = "scene")]
mod spatial;
#[cfg(feature = "scene")]
pub use scene::{CookError, cook_scene, stable_id};
#[cfg(feature = "script")]
mod script;
#[cfg(feature = "script")]
pub use script::{CookedScriptArtifact, ScriptCookError, cook_script};

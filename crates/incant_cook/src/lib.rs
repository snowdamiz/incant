//! Authoring-side cooking. Native players depend on this only at build time.
//! Script-only builds omit the document/scene authoring dependency graph.
#[cfg(feature = "scene")]
mod navigation;
#[cfg(feature = "scene")]
mod scene;
#[cfg(feature = "scene")]
mod spatial;
#[cfg(feature = "scene")]
pub use scene::{CookError, cook_scene, cook_scene_with_navigation, stable_id};
#[cfg(feature = "script")]
mod script;
#[cfg(feature = "script")]
pub use script::{CookedScriptArtifact, ScriptCookError, cook_script};

#[cfg(all(feature = "scene", feature = "script"))]
mod project_script;
#[cfg(all(feature = "scene", feature = "script"))]
pub use project_script::{
    CookedProjectProgram, PreparedScriptScene, ProjectScriptError, prepare_script_scene,
    prepare_script_scene_with_navigation,
};

#[cfg(feature = "script")]
mod build_script;
#[cfg(feature = "script")]
pub use build_script::embed_script;

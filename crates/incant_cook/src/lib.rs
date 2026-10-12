//! Authoring-side cooking. Native players depend on this only at build time.
//! Script-only builds omit the document/scene authoring dependency graph.
#[cfg(feature = "scene")]
mod animation;
#[cfg(feature = "scene")]
mod animation_graph;
#[cfg(feature = "scene")]
pub use animation::{AnimationResources, load_project_animation_resources};
#[cfg(feature = "scene")]
mod navigation;
#[cfg(feature = "scene")]
mod scene;
#[cfg(feature = "scene")]
mod spatial;
#[cfg(feature = "scene")]
mod text;
#[cfg(feature = "scene")]
mod ui;
#[cfg(feature = "scene")]
pub use scene::{
    CookError, cook_scene, cook_scene_with_navigation, cook_scene_with_resources, stable_id,
};
#[cfg(feature = "script")]
mod script;
#[cfg(feature = "script")]
pub use script::{CookedScriptArtifact, ScriptCookError, cook_script};

#[cfg(all(feature = "scene", feature = "script"))]
mod project_script;
#[cfg(all(feature = "scene", feature = "script"))]
pub use project_script::{
    CookedProjectProgram, PreparedScriptScene, ProjectScriptError, prepare_script_scene,
    prepare_script_scene_with_navigation, prepare_script_scene_with_resources,
};

#[cfg(feature = "script")]
mod build_script;
#[cfg(feature = "script")]
pub use build_script::embed_script;

#[cfg(feature = "typescript")]
mod typescript;
#[cfg(feature = "typescript")]
pub use typescript::{ProjectCompilation, TypeScriptCookError, compile_project_sources};

#[cfg(feature = "typescript")]
mod project_build;
#[cfg(feature = "typescript")]
pub use project_build::{PROJECT_ENV, SCENE_ENV, embed_project};

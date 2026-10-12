//! Authoring-only startup shared by native editor and headless Play.
//! Cook once, then transfer owned binary/runtime inputs to the simulation host.
//! No document, source text, command bus or project reference survives in PreparedPlay.
use incant_audio_runtime::{ClipBinding, MAX_SCENE_CLIPS};
use incant_input::InputActions;
use incant_render_game::NativeRenderSettings;
use incant_runtime::{CookedScene, StableId};
use incant_script::{CompiledScript, ScriptError};
use serde::Deserialize;
use std::{collections::BTreeSet, path::Path};
mod control;
pub use control::{PlayError, PlayMode, PlaySession};

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct StartOptions {
    pub scene: Option<String>,
    pub script_sha256: Option<String>,
    pub camera: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum StartupError {
    #[error("native play: {0}")]
    Invalid(&'static str),
    #[error("native play startup failure: {0}")]
    Native(#[source] Box<dyn std::error::Error>),
}
fn native<E: std::error::Error + 'static>(error: E) -> StartupError {
    StartupError::Native(Box::new(error))
}
fn invalid(message: &'static str) -> StartupError {
    StartupError::Invalid(message)
}
fn id(text: &str) -> Result<StableId, StartupError> {
    let value = text
        .parse::<ulid::Ulid>()
        .map_err(|_| invalid("expected canonical uppercase ULID"))?;
    if value.to_string() != text {
        return Err(invalid("expected canonical uppercase ULID"));
    }
    Ok(StableId(value.to_bytes()))
}

/// A one-time transfer. All path fields identify validated cooked asset caches;
/// loading or rendering them remains the consuming native host's responsibility.
/// This is an authoring boundary, never a normal shipped-player dependency.
pub struct PreparedPlay {
    pub scene: String,
    pub script_source_sha256: String,
    pub authored_scripts: bool,
    pub cooked: CookedScene,
    pub code: CompiledScript,
    pub actions: InputActions,
    pub clips: Vec<ClipBinding>,
    pub sample_rate: u32,
    pub render: Option<NativeRenderSettings>,
}
impl PreparedPlay {
    /// The host supplies an engine-owned fallback for a scene without scripts.
    /// Authored attachments always go through the fixed shared TypeScript cook.
    pub fn load(
        document: &incant_doc::Project,
        root: &Path,
        options: &StartOptions,
        sample_rate: u32,
        fallback: impl FnOnce() -> Result<CompiledScript, ScriptError>,
    ) -> Result<Self, StartupError> {
        if !(8000..=192000).contains(&sample_rate) {
            return Err(invalid("invalid sample rate"));
        }
        let scene = match &options.scene {
            Some(value) => value.clone(),
            None if document.scenes.len() == 1 => {
                document.scenes.keys().next().expect("one scene").clone()
            }
            None => {
                return Err(invalid(
                    "choose --scene when the project does not have exactly one scene",
                ));
            }
        };
        id(&scene)?;
        let camera = options.camera.as_deref().map(id).transpose()?;
        let authored_scripts = document.scenes.get(&scene).is_some_and(|scene| {
            scene
                .entities
                .values()
                .any(|entity| entity.components.contains_key("Script"))
        });
        let animations = incant_cook::load_project_animation_resources(document, &scene, root)
            .map_err(native)?;
        let (cooked, code) = if authored_scripts {
            let prepared = incant_cook::prepare_script_scene_with_resources(
                document,
                &scene,
                &Default::default(),
                &animations,
            )
            .map_err(native)?;
            incant_cook::compile_project_sources(prepared, root)
                .map_err(native)?
                .program
                .into_parts()
        } else {
            let bytes = incant_cook::cook_scene_with_resources(
                document,
                &scene,
                &Default::default(),
                &animations,
            )
            .map_err(native)?;
            (
                CookedScene::from_bytes(&bytes).map_err(native)?,
                fallback().map_err(native)?,
            )
        };
        if camera.is_none() && (!cooked.spatial().is_empty() || !cooked.visual_assets().is_empty())
        {
            return Err(invalid(
                "spatial content requires an explicit --camera and native render host",
            ));
        }
        let clip_ids: BTreeSet<_> = cooked
            .audio()
            .iter()
            .filter_map(|n| n.source.as_ref().map(|s| s.clip))
            .collect();
        if clip_ids.len() > MAX_SCENE_CLIPS {
            return Err(invalid("too many scene clips"));
        }
        let settings = camera
            .map(|camera| {
                let path = |kind, asset_kind, suffix| {
                    if cooked.visual_assets().iter().any(|a| a.kind == asset_kind) {
                        incant_assets::project_cache_directory(root, kind).map_err(native)
                    } else {
                        Ok(root.join(suffix))
                    }
                };
                Ok::<_, StartupError>(NativeRenderSettings {
                    model_cache: path(
                        incant_assets::CacheKind::Models,
                        incant_runtime::VisualAssetKind::Model,
                        ".incant/cache/models",
                    )?,
                    texture_cache: path(
                        incant_assets::CacheKind::Textures,
                        incant_runtime::VisualAssetKind::Texture,
                        ".incant/cache/textures",
                    )?,
                    camera,
                    consumer: (),
                })
            })
            .transpose()?;
        let audio_cache = if clip_ids.is_empty() {
            None
        } else {
            Some(
                incant_assets::project_cache_directory(root, incant_assets::CacheKind::Audio)
                    .map_err(native)?,
            )
        };
        let mut clips = Vec::with_capacity(clip_ids.len());
        for clip in clip_ids {
            let key = ulid::Ulid::from_bytes(clip.0).to_string();
            let asset = document
                .assets
                .get(&key)
                .ok_or_else(|| invalid("missing audio asset"))?;
            if asset.kind != "audio" {
                return Err(invalid("clip reference is not an audio asset"));
            }
            clips.push(
                ClipBinding::load(
                    clip,
                    audio_cache.as_deref().expect("referenced clips"),
                    &asset.sha256,
                )
                .map_err(native)?,
            );
        }
        let script_source_sha256 = code
            .metadata()
            .source_sha256
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        if options
            .script_sha256
            .as_ref()
            .is_some_and(|expected| expected != &script_source_sha256)
        {
            return Err(invalid(
                "selected script does not match requested source hash",
            ));
        }
        let actions = document.settings.input_actions.clone();
        Ok(Self {
            scene,
            script_source_sha256,
            authored_scripts,
            cooked,
            code,
            actions,
            clips,
            sample_rate,
            render: settings,
        })
    }
}

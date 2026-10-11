//! One startup boundary shared by the native CLI and retained RPC play.
//! The session contains no authoring document, command bus or source text.
use super::{PlayError, Result, embedded, id, invalid, native};
use incant_audio_runtime::{ClipBinding, MAX_SCENE_CLIPS};
use incant_game_runtime::{HostedOfflineGameSession, OfflineGameSession};
use incant_input::{InputActions, InputEvent, InputFrame};
use incant_render_game::{NativeRenderHost, NativeRenderSettings};
use incant_runtime::{CookedScene, NativeWorld};
use incant_script::{CompiledScript, NativeLogs};
use serde::Deserialize;
use std::{collections::BTreeSet, path::Path};

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct StartOptions {
    pub scene: Option<String>,
    pub script_sha256: Option<String>,
    pub camera: Option<String>,
}

pub(crate) struct Session {
    pub scene: String,
    pub script_source_sha256: String,
    pub authored_scripts: bool,
    game: Game,
}
impl Session {
    pub fn new(
        document: &incant_doc::Project,
        root: &Path,
        options: &StartOptions,
        sample_rate: u32,
    ) -> Result<Self> {
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
        let (cooked, code) = if authored_scripts {
            let prepared = incant_cook::prepare_script_scene(document, &scene).map_err(native)?;
            incant_cook::compile_project_sources(prepared, root)
                .map_err(native)?
                .program
                .into_parts()
        } else {
            let bytes = incant_cook::cook_scene(document, &scene).map_err(native)?;
            (
                CookedScene::from_bytes(&bytes).map_err(native)?,
                embedded::load().map_err(native)?,
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
                Ok::<_, PlayError>(NativeRenderSettings {
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
        let game = Game::new(cooked, code, actions, clips, sample_rate, settings)?;
        Ok(Self {
            scene,
            script_source_sha256,
            authored_scripts,
            game,
        })
    }
    pub fn world(&self) -> Result<&NativeWorld> {
        self.game.world()
    }
    pub fn input(&self) -> Result<&InputFrame> {
        self.game.input()
    }
    pub fn logs(&self) -> Result<&NativeLogs> {
        self.game.logs()
    }
    pub fn next_frames(&self) -> Result<usize> {
        self.game.next_frames()
    }
    pub fn frames(&self) -> Result<u128> {
        self.game.frames()
    }
    pub fn tick(&mut self, input: &[InputEvent], output: &mut [f32]) -> Result<()> {
        self.game.tick(input, output)
    }
    pub fn is_spatial(&self) -> bool {
        matches!(self.game, Game::Spatial(_))
    }
}

enum Game {
    Plain(Box<OfflineGameSession>),
    Spatial(Box<HostedOfflineGameSession<NativeRenderHost>>),
}
impl Game {
    fn new(
        scene: CookedScene,
        code: CompiledScript,
        actions: InputActions,
        clips: Vec<ClipBinding>,
        rate: u32,
        settings: Option<NativeRenderSettings>,
    ) -> Result<Self> {
        if let Some(settings) = settings {
            Ok(Self::Spatial(Box::new(
                HostedOfflineGameSession::new(scene, code, actions, clips, rate, settings)
                    .map_err(native)?,
            )))
        } else {
            Ok(Self::Plain(Box::new(
                OfflineGameSession::new(scene, code, actions, clips, rate).map_err(native)?,
            )))
        }
    }
    fn world(&self) -> Result<&NativeWorld> {
        match self {
            Self::Plain(g) => g.world().map_err(native),
            Self::Spatial(g) => g.world().map_err(native),
        }
    }
    fn input(&self) -> Result<&InputFrame> {
        match self {
            Self::Plain(g) => g.input().map_err(native),
            Self::Spatial(g) => g.input().map_err(native),
        }
    }
    fn logs(&self) -> Result<&NativeLogs> {
        match self {
            Self::Plain(g) => g.logs().map_err(native),
            Self::Spatial(g) => g.logs().map_err(native),
        }
    }
    fn next_frames(&self) -> Result<usize> {
        match self {
            Self::Plain(g) => g.next_output_frames().map_err(native),
            Self::Spatial(g) => g.next_output_frames().map_err(native),
        }
    }
    fn frames(&self) -> Result<u128> {
        match self {
            Self::Plain(g) => g.rendered_frames().map_err(native),
            Self::Spatial(g) => g.rendered_frames().map_err(native),
        }
    }
    fn tick(&mut self, input: &[InputEvent], output: &mut [f32]) -> Result<()> {
        match self {
            Self::Plain(g) => g.tick(input, output).map_err(native),
            Self::Spatial(g) => g.tick(input, output).map_err(native),
        }
    }
}

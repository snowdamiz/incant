//! Typed renderer input shared by authoring and native extraction. No project,
//! command bus, importer, GPU or agent dependency. Construct/replace at sync points.
use glam::DMat4;
use incant_assets_runtime::RuntimeAsset;
use incant_types::{Camera, DirectionalLight, PointLight, SpotLight};
use std::sync::Arc;
use thiserror::Error;
pub const MAX_INSTANCES: usize = 100_000;
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
#[derive(Debug, Error)]
pub enum SceneError {
    #[error("invalid render input: {0}")]
    Input(&'static str),
    #[error("model asset {0} is not loaded at the document's current fingerprint")]
    MissingModel(String),
    #[error("environment texture {0} is not loaded at the document's current fingerprint")]
    MissingEnvironment(String),
    #[error("sprite texture {0} is not loaded at the document's current fingerprint")]
    MissingSprite(String),
    #[error("sprite atlas frame {frame} is unavailable in texture {texture}")]
    MissingSpriteFrame { texture: String, frame: String },
    #[error(transparent)]
    Sprite(#[from] incant_2d::SpriteError),
    #[error("authored material asset overrides are not supported by the imported preview")]
    MaterialOverrides,
    #[error("model {0} has no scene to instantiate")]
    NoScene(String),
    #[error("model {0} contains repeated roots or nodes in its selected scene")]
    RepeatedNode(String),
    #[error("world transform exceeds renderer numeric range")]
    TransformRange,
    #[error("scene exceeds {MAX_INSTANCES} primitive instances")]
    InstanceLimit,
    #[error("world light transform exceeds renderer numeric range")]
    LightTransform,
    #[error("spot inner/outer angles cannot be distinguished at GPU precision")]
    LightCone,
    #[error("camera entity {0} was not found or has no Camera component")]
    MissingCamera(String),
    #[error("camera world transform cannot form a finite orthonormal view")]
    CameraTransform,
    #[error("camera projection exceeds GPU numeric precision")]
    CameraProjection,
    #[error("viewport is smaller than the pixel-perfect camera's {0}x{1} logical resolution")]
    PixelPerfectViewport(u32, u32),
    #[error("scene exceeds four enabled directional shadow lights")]
    ShadowLightLimit,
    #[error("scene exceeds 16 directional or 4096 local lights")]
    LightLimit,
}

#[derive(Debug, Default, Clone, serde::Serialize)]
pub struct SceneStats {
    pub diagnostic_entities: usize,
    pub model_entities: usize,
    pub primitive_instances: usize,
    pub model_triangles: u64,
    pub model_draw_calls: usize,
    pub sprite_entities: usize,
}

#[derive(Default)]
pub struct SceneInput {
    pub nodes: Vec<SceneNode>,
    pub environment: Option<EnvironmentInput>,
}
pub struct SceneNode {
    pub id: String,
    pub world: DMat4,
    pub camera: Option<Camera>,
    pub light: Option<SceneLight>,
    pub mesh: Option<MeshInput>,
    pub sprite: Option<SpriteInput>,
    /// Authoring-only diagnostic cubes. Native content opts in explicitly.
    pub diagnostic: bool,
}
#[derive(Clone, PartialEq)]
pub enum SceneLight {
    Directional(DirectionalLight),
    Point(PointLight),
    Spot(SpotLight),
}
pub struct MeshInput {
    pub source: Arc<RuntimeAsset>,
    pub cast_shadows: bool,
}
pub struct SpriteInput {
    pub source: Arc<RuntimeAsset>,
    pub sprite: incant_2d::SpriteRenderer,
}
pub struct EnvironmentInput {
    pub source: Arc<RuntimeAsset>,
    pub intensity: f64,
    pub rotation_degrees: f64,
}
/// A GPU-independent preparation interface lets authoring adapters remain outside
/// the renderer's normal dependency graph, including in renderer unit tests.
pub trait ScenePreparer {
    type Prepared;
    fn prepare_input(&self, input: &SceneInput) -> Result<Self::Prepared>;
    fn capture_prepared(&self, scene: &Self::Prepared, width: u32, height: u32) -> Result<Vec<u8>>;
}

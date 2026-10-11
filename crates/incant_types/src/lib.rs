//! Shared component values for authoring and the native runtime. No project,
//! command, CRDT, filesystem or agent dependency. Authoring validation remains
//! in incant_doc until the checked cook/runtime boundaries migrate separately.
mod audio;
mod camera;
mod collider_shapes;
mod lights;
mod physics;
mod physics_2d;
pub mod physics_validation;
mod render;
mod transform;
mod visual_asset;

/// Stable project/runtime identifier; allocation and reference validation belong
/// to the boundary that constructs or loads the data.
pub type Id = String;

/// Canonical 128 bits of an authored ULID. Runtime storage never reparses text.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
pub struct StableId(pub [u8; 16]);

pub use audio::{AudioBus, AudioListener, AudioSource, AudioSpatial};
pub use camera::{Camera, CameraProjection};
pub use collider_shapes::{ColliderPart, ColliderShape, PrimitiveColliderShape};
pub use lights::{DirectionalLight, DirectionalShadows, PointLight, SpotLight};
pub use physics::{AngularVelocity, BodyMotion, Collider, RigidBody};
pub use physics_2d::{AngularVelocity2D, Collider2D, ColliderShape2D};
pub use render::{EnvironmentLight, MeshRenderer, TextureUsage};
pub use transform::{Transform, Velocity};
pub use visual_asset::{VisualAssetKind, VisualAssetRef};

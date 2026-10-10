//! Shared component values for authoring and the native runtime. No project,
//! command, CRDT, filesystem or agent dependency. Authoring validation remains
//! in incant_doc until the checked cook/runtime boundaries migrate separately.
mod audio;
mod camera;
mod collider_shapes;
mod lights;
mod physics;
mod render;
mod transform;

/// Stable project/runtime identifier; allocation and reference validation belong
/// to the boundary that constructs or loads the data.
pub type Id = String;

pub use audio::{AudioBus, AudioListener, AudioSource, AudioSpatial};
pub use camera::{Camera, CameraProjection};
pub use collider_shapes::{ColliderPart, ColliderShape, PrimitiveColliderShape};
pub use lights::{DirectionalLight, DirectionalShadows, PointLight, SpotLight};
pub use physics::{AngularVelocity, BodyMotion, Collider, RigidBody};
pub use render::{EnvironmentLight, MeshRenderer, TextureUsage};
pub use transform::{Transform, Velocity};

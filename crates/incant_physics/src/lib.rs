//! Legacy authoring adapter. The solver lives in the document-free native
//! physics crate; this facade preserves project preparation and textual IDs.
mod navigation;
pub use incant_physics_runtime::{BodyState, CharacterOptions, CharacterStep, PhysicsError};
pub use navigation::navigation_geometry;
pub type RayQuery = incant_physics_runtime::RayQuery<String>;
pub type RayHit = incant_physics_runtime::RayHit<String>;
pub type CharacterQuery = incant_physics_runtime::CharacterQuery<String>;
pub type CharacterMovement = incant_physics_runtime::CharacterMovement<String>;
pub type TriggerEvent = incant_physics_runtime::TriggerEvent<String>;
use incant_doc::{
    AngularVelocity, Collider as DocCollider, DocumentError, Project, Transform, Velocity,
};
use std::collections::BTreeMap;
type Spec = incant_physics_runtime::PhysicsSpec;

/// Successfully decoded input; constructing this validates the entire project
/// before a live world can be changed.
pub struct PreparedPhysics(incant_physics_runtime::PreparedPhysics<String>);
impl PreparedPhysics {
    pub fn new(project: &Project) -> Result<Self, DocumentError> {
        Self::from_validated(project.validated()?)
    }
    pub fn from_validated(
        validated: incant_doc::ValidatedProject<'_>,
    ) -> Result<Self, DocumentError> {
        let project = validated.project();
        let mut scenes = BTreeMap::new();
        for scene in project.scenes.values() {
            let mut entities = BTreeMap::new();
            for entity in scene.entities.values() {
                let Some(collider) = entity.components.get("Collider") else {
                    continue;
                };
                let decode = |key: &str| entity.components.get(key).cloned();
                let transform: Transform =
                    serde_json::from_value(entity.components["Transform"].clone())?;
                let velocity: Option<Velocity> =
                    decode("Velocity").map(serde_json::from_value).transpose()?;
                let angular: Option<AngularVelocity> = decode("AngularVelocity")
                    .map(serde_json::from_value)
                    .transpose()?;
                let mut collider: DocCollider = serde_json::from_value(collider.clone())?;
                if let incant_doc::ColliderShape::Compound { parts } = &mut collider.shape {
                    // Reordering the presentation must not rebuild a live body
                    // and discard its sleep/contact state.
                    parts.sort_by(|a, b| a.id.cmp(&b.id));
                }
                entities.insert(
                    entity.id.clone(),
                    Spec {
                        body: decode("RigidBody")
                            .map(serde_json::from_value)
                            .transpose()?,
                        collider,
                        state: BodyState {
                            translation: transform.translation,
                            rotation: transform.rotation,
                            velocity: velocity.map_or([0.; 3], |v| v.linear),
                            angular_velocity: angular.map_or([0.; 3], |v| v.angular),
                            sleeping: false,
                        },
                    },
                );
            }
            scenes.insert(scene.id.clone(), entities);
        }
        Ok(Self(
            incant_physics_runtime::PreparedPhysics::new(scenes).map_err(|error| {
                DocumentError::Validation(vec![incant_doc::Diagnostic {
                    path: "/scenes".into(),
                    message: error.to_string(),
                }])
            })?,
        ))
    }
}

#[derive(Default)]
pub struct PhysicsRuntime(incant_physics_runtime::PhysicsRuntime<String>);
impl PhysicsRuntime {
    pub fn sync(&mut self, prepared: PreparedPhysics) -> Result<(), PhysicsError> {
        self.0.sync(prepared.0)
    }
    pub fn step(&mut self, dt: f64) -> Result<(), PhysicsError> {
        self.0.step(dt)
    }
    pub fn state_iter(&self) -> impl Iterator<Item = (&str, &BodyState)> {
        self.0.state_iter().map(|(id, state)| (id.as_str(), state))
    }
    pub fn states(&self) -> BTreeMap<String, BodyState> {
        self.0.states()
    }
    pub fn events(&self) -> &[TriggerEvent] {
        self.0.events()
    }
    pub fn raycast(&self, query: &RayQuery) -> Result<Option<RayHit>, PhysicsError> {
        self.0.raycast(query)
    }
    pub fn compute_character_motion(
        &self,
        query: &CharacterQuery,
        dt: f64,
    ) -> Result<CharacterMovement, PhysicsError> {
        self.0.compute_character_motion(query, dt)
    }
}

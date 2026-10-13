//! Fixed regular rigid-body state. No live construction or complete owner proof.
mod codec;
mod validate;
use crate::{Handle, Identity, LocalPose, PendingJournals, ValidatedAttachments};
pub use codec::{decode_body_state, encode_body_state};
pub use validate::validate_body_state;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum BodyKind {
    Dynamic = 0,
    Fixed = 1,
    KinematicPosition = 2,
    KinematicVelocity = 3,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BodyVelocity {
    pub linear: [u32; 3],
    /// Planar angular scalar occupies lane zero; other lanes are positive zero.
    pub angular: [u32; 3],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BodyForces {
    pub force: [u32; 3],
    pub torque: [u32; 3],
    pub gravity_scale: u32,
    pub user_force: [u32; 3],
    pub user_torque: [u32; 3],
    /// Absent from planar native bodies; must be false there.
    pub gyroscopic: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LocalMass {
    pub center: [u32; 3],
    /// Zero inverse mass/inertia means infinite mass/inertia, not an error.
    pub inverse_mass: u32,
    pub inverse_principal_inertia: [u32; 3],
    /// Spatial xyzw; absent (all positive zero) in planar records.
    pub principal_frame: [u32; 4],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdditionalMass {
    Mass(u32),
    Properties(LocalMass),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BodyMass {
    pub world_center: [u32; 3],
    pub effective_inverse_mass: [u32; 3],
    /// Spatial symmetric entries m11,m12,m13,m22,m23,m33. Planar scalar
    /// occupies lane zero. No tensor recomputation or sign folding occurs.
    pub effective_inverse_inertia: [u32; 6],
    pub local: LocalMass,
    pub locked_axes: u32,
    pub additional: Option<AdditionalMass>,
    pub max_extent: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BodyActivation {
    /// Negative thresholds are native never-sleep sentinels and are preserved.
    pub linear_threshold: u32,
    pub angular_threshold: u32,
    pub time_until_sleep: u32,
    pub time_since_can_sleep: u32,
    pub sleeping: bool,
    pub previous_pose: LocalPose,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BodyCcd {
    /// Native collider-less sentinel is f32::MAX (finite), not infinity.
    pub thickness: u32,
    pub active: bool,
    pub enabled: bool,
    pub soft_prediction: u32,
    pub allow_fast_rotation: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RigidBodyState {
    pub position: LocalPose,
    pub next_position: LocalPose,
    pub velocity: BodyVelocity,
    pub ccd_velocity: BodyVelocity,
    pub forces: BodyForces,
    /// Linear then angular coefficients.
    pub damping: [u32; 2],
    pub mass: BodyMass,
    pub activation: BodyActivation,
    pub ccd: BodyCcd,
    /// Active island, active set, persistent island, persistent island index.
    /// This section checks scalar domains; the joined island/owner witnesses
    /// check reciprocal membership.
    pub ids: [u32; 4],
    pub changes: u32,
    pub kind: BodyKind,
    pub dominance: i32,
    pub enabled: bool,
    /// Exact native usize values represented as u64; not permission to run
    /// unbounded solver work. A future live owner must admit its work budget.
    pub additional_iterations: [u64; 2],
    pub soft_body: Handle,
    pub soft_cluster: u32,
    pub soft_motion_margin: u32,
    pub user_data: [u32; 4],
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BodyStateTable {
    pub identity: Identity,
    /// Exact matched body arena payload-row order, including disabled bodies.
    pub bodies: Vec<RigidBodyState>,
}
#[derive(Clone, Copy)]
pub struct BodyContext<'a> {
    pub attachments: &'a ValidatedAttachments<'a>,
    /// Must eventually derive from checked owner journals; not authentication.
    pub journals: PendingJournals,
}
pub struct ValidatedBodyState<'a> {
    record: BodyStateTable,
    context: BodyContext<'a>,
}
impl<'a> ValidatedBodyState<'a> {
    pub(crate) fn attachments(&self) -> &'a ValidatedAttachments<'a> {
        self.context.attachments
    }
    pub fn record(&self) -> &BodyStateTable {
        &self.record
    }
    pub fn body(&self, handle: Handle) -> Option<&RigidBodyState> {
        let row = self.context.attachments.body_row(handle)?;
        self.record.bodies.get(row as usize)
    }
}

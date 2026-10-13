//! Fixed rigid collider properties joined to checked body, attachment and shape
//! witnesses. This inert boundary does not construct a collider or a live world.
//! Floating values are exact IEEE-754 f32 bits, not numeric u32 conversions.
//! Pose, mass and coefficients are validated without normalization or clamping.
mod codec;
mod validate;
use crate::{
    Handle, Identity, LocalMass, LocalPose, PendingJournals, ValidatedBodyState, ValidatedGeometry,
};
pub use codec::{decode_collider_state, encode_collider_state};
pub use validate::validate_collider_state;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum ColliderKind {
    Solid = 0,
    Sensor = 1,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
/// Exact native tri-state, including historical DisabledByParent after detach.
/// It cannot be reconstructed from the parent's current enabled boolean.
pub enum ColliderEnabled {
    Enabled = 0,
    DisabledByParent = 1,
    Disabled = 2,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum CombineRule {
    Average = 0,
    Min = 1,
    Multiply = 2,
    Max = 3,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum GroupTest {
    And = 0,
    Or = 1,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColliderGroups {
    pub memberships: u32,
    pub filter: u32,
    pub test: GroupTest,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColliderMass {
    Density(u32),
    Mass(u32),
    Properties(LocalMass),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColliderMaterial {
    pub friction: u32,
    pub restitution: u32,
    pub friction_rule: CombineRule,
    pub restitution_rule: CombineRule,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColliderState {
    pub kind: ColliderKind,
    pub position: LocalPose,
    pub mass: ColliderMass,
    pub changes: u32,
    pub material: ColliderMaterial,
    pub active_collision_types: u32,
    pub collision_groups: ColliderGroups,
    pub solver_groups: ColliderGroups,
    /// Known callback flags are retained as data, not permission to call hooks.
    pub active_hooks: u32,
    pub active_events: u32,
    pub enabled: ColliderEnabled,
    pub contact_skin: u32,
    pub force_event_threshold: u32,
    /// Any soft/deformable association is outside this rigid-only format.
    pub deformable: bool,
    pub user_data: [u32; 4],
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColliderStateTable {
    pub identity: Identity,
    /// Exact collider arena payload-row order, including standalone/disabled.
    pub colliders: Vec<ColliderState>,
}
#[derive(Clone, Copy)]
pub struct ColliderContext<'a> {
    pub bodies: &'a ValidatedBodyState<'a>,
    /// Exact collider-row ordered checked shape witnesses, not replacement shapes.
    pub geometries: &'a [ValidatedGeometry],
    /// Future whole-owner admission must derive this from actual checked journals.
    pub journals: PendingJournals,
}
pub struct ValidatedColliderState<'a> {
    record: ColliderStateTable,
    context: ColliderContext<'a>,
}
impl<'a> ValidatedColliderState<'a> {
    pub fn record(&self) -> &ColliderStateTable {
        &self.record
    }
    pub fn context(&self) -> ColliderContext<'a> {
        self.context
    }
    pub fn collider(&self, handle: Handle) -> Option<&ColliderState> {
        let row = self.context.bodies.attachments().collider_row(handle)?;
        self.record.colliders.get(row as usize)
    }
    /// Pinned Rapier uses MAX (not infinity) when force events are disabled.
    pub fn effective_force_threshold(&self, handle: Handle) -> Option<u32> {
        self.collider(handle).map(|c| {
            if c.active_events & 2 != 0 {
                c.force_event_threshold
            } else {
                f32::MAX.to_bits()
            }
        })
    }
}

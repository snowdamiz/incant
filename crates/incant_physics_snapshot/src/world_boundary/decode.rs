use super::{WorldBoundary, codec, join_world_boundary, metadata, shapes};
use crate::*;

/// Fully decodes and joins the canonical inert sections before invoking `f`.
/// The callback cannot return borrowed decoded storage. A single aggregate
/// budget covers every section and its validation scratch; no native code runs.
/// Declared phase/domain fields are consistency checks, not owner authentication.
pub fn with_decoded_world_boundary<T>(
    bytes: &[u8],
    expected: Identity,
    budget: &mut Budget,
    f: impl for<'a> FnOnce(WorldBoundary<'a>) -> Result<T>,
) -> Result<T> {
    let s = codec::sections(bytes, expected)?;
    let meta = metadata::decode(s[0])?;
    let ba = decode_arena(s[1], expected, budget)?;
    let ca = decode_arena(s[2], expected, budget)?;
    let attachments = decode_attachments(s[3], &ba, &ca, budget)?;
    let shapes = shapes::decode(s[4], expected, budget)?;
    let bodies = decode_body_state(
        s[5],
        BodyContext {
            attachments: &attachments,
            journals: meta.journals,
        },
        budget,
    )?;
    let colliders = decode_collider_state(
        s[6],
        ColliderContext {
            bodies: &bodies,
            geometries: &shapes,
            journals: meta.journals,
        },
        budget,
    )?;
    let graph = decode_graph(s[7], expected, budget)?;
    let manifolds = decode_manifolds(
        s[8],
        ManifoldContext {
            graph: &graph,
            attachments: &attachments,
            geometries: &shapes,
            journals: meta.journals,
        },
        budget,
    )?;
    let pairs = decode_pair_state(s[9], &manifolds, budget)?;
    let islands = decode_islands(
        s[10],
        IslandContext {
            bodies: &bodies,
            manifolds: &manifolds,
            domain: meta.domain,
        },
        budget,
    )?;
    let rigid = decode_rigid_owner(
        s[11],
        RigidOwnerContext {
            islands: &islands,
            pairs: &pairs,
            colliders: &colliders,
            multibody_epoch: meta.multibody_epoch,
        },
        budget,
    )?;
    let indices = decode_coarena(s[12], &ca, budget)?;
    let intersections = decode_graph(s[13], expected, budget)?;
    let narrow = decode_narrow_owner(
        s[14],
        NarrowOwnerContext {
            rigid: &rigid,
            indices: &indices,
            intersections: &intersections,
        },
        budget,
    )?;
    // The separate exact pending queue breaks only a Rust borrow cycle. The
    // broad section must repeat it byte-for-byte; it is never independently used.
    let pending = metadata::decode_pending(s[15], budget)?;
    let tree = decode_pending_bvh(
        s[16],
        PendingBvhContext {
            colliders: &colliders,
            pending_aabbs: &pending,
        },
        budget,
    )?;
    let adjacency = decode_coarena(s[17], &ca, budget)?;
    let broad = decode_broad_owner(
        s[18],
        BroadOwnerContext {
            tree: &tree,
            narrow: &narrow,
            adjacency: &adjacency,
        },
        budget,
    )?;
    let integration = decode_integration_config(s[19], expected)?;
    let ccd = decode_ccd_state(
        s[20],
        CcdContext {
            colliders: &colliders,
            integration: &integration,
        },
        budget,
    )?;
    let pipeline = decode_pipeline_state(
        s[21],
        PipelineContext {
            integration: &integration,
            ccd: &ccd,
        },
    )?;
    f(join_world_boundary(&broad, &pipeline)?)
}

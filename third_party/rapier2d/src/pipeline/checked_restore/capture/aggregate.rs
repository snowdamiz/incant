//! All observations and joins share one budget. Called only while the exclusive
//! completed-step capability is borrowed; these inert validators authenticate no phase.
use super::super::{
    WorkBudget, admit_numeric_frame,
    convert::{bits, pose_bits},
};
use super::{CaptureError, config, fields, util};
use crate::pipeline::PhysicsWorld;
use alloc::vec::Vec;
use incant_physics_snapshot::*;
pub(super) fn encode(
    w: &mut PhysicsWorld,
    id: Identity,
    budget: &mut Budget,
    work: &mut WorkBudget,
) -> core::result::Result<Vec<u8>, CaptureError> {
    let ba = validate_arena(w.bodies.bodies.checkpoint_capture(id, budget)?, budget)?;
    let ca = validate_arena(
        w.colliders.colliders.checkpoint_capture(id, budget)?,
        budget,
    )?;
    let journals = PendingJournals {
        modified_bodies: util::count(w.bodies.modified_bodies.len())?,
        modified_colliders: util::count(w.colliders.modified_colliders.len())?,
        removed_colliders: util::count(w.colliders.removed_colliders.len())?,
    };
    let n = util::total(w.bodies.iter().map(|(_, b)| b.colliders().len()))?;
    let mut a = AttachmentRecord {
        identity: id,
        bodies: budget.vector(w.bodies.len())?,
        colliders: budget.vector(w.colliders.len())?,
        attached: budget.vector(n)?,
    };
    let mut body_rows = budget.vector(w.bodies.len())?;
    let mut collider_rows = budget.vector(w.colliders.len())?;
    let mut shapes = budget.vector(w.colliders.len())?;
    for (_, b) in w.bodies.iter() {
        a.bodies.push(AttachmentSpan {
            start: a.attached.len() as u32,
            count: b.colliders().len() as u32,
        });
        a.attached.extend(
            b.colliders()
                .iter()
                .map(|h| util::handle(h.into_raw_parts())),
        );
        body_rows.push(fields::body(b)?);
    }
    for (_, c) in w.colliders.iter() {
        a.colliders.push(
            c.parent()
                .map(|h| {
                    Ok::<_, Error>(ParentAttachment {
                        body: util::handle(h.into_raw_parts()),
                        local_pose: pose_bits(*c.position_wrt_parent().ok_or(Error::Ownership)?),
                    })
                })
                .transpose()?,
        );
        collider_rows.push(fields::collider(c)?);
        shapes.push(fields::geometry(c, id, budget)?);
    }
    let a = validate_attachments(a, &ba, &ca, budget)?;
    let bodies = validate_body_state(
        BodyStateTable {
            identity: id,
            bodies: body_rows,
        },
        BodyContext {
            attachments: &a,
            journals,
        },
    )?;
    let colliders = validate_collider_state(
        ColliderStateTable {
            identity: id,
            colliders: collider_rows,
        },
        ColliderContext {
            bodies: &bodies,
            geometries: &shapes,
            journals,
        },
    )?;
    let (islands, scan, touched) = w.islands.checkpoint_capture(id, budget)?;
    let n =
        w.narrow_phase
            .checkpoint_capture(id, budget, scan, islands.sleep_scan_stamp, touched)?;
    let graph = validate_graph(n.graph, budget)?;
    let manifolds = validate_manifolds(
        n.manifolds,
        ManifoldContext {
            graph: &graph,
            attachments: &a,
            geometries: &shapes,
            journals,
        },
        budget,
    )?;
    let pairs = validate_pair_state(n.pairs, &manifolds, budget)?;
    let islands = validate_islands(
        islands,
        IslandContext {
            bodies: &bodies,
            manifolds: &manifolds,
            domain: IslandDomain::default(),
        },
        budget,
    )?;
    let rigid = validate_rigid_owner(
        n.rigid,
        RigidOwnerContext {
            islands: &islands,
            pairs: &pairs,
            colliders: &colliders,
            multibody_epoch: w.multibody_joints.topology_epoch,
        },
        budget,
    )?;
    let indices = validate_coarena(n.indices, &ca, budget)?;
    let intersections = validate_graph(n.intersections, budget)?;
    let narrow = validate_narrow_owner(
        n.narrow,
        NarrowOwnerContext {
            rigid: &rigid,
            indices: &indices,
            intersections: &intersections,
        },
        budget,
    )?;
    let (tree, adj, broad) = w.broad_phase.checkpoint_capture(id, budget)?;
    // The pending-tree witness borrows an independent charged exact list; the
    // broad record subsequently moves into its immutable owner.
    let pending = util::copy(&broad.pending_aabbs, budget)?;
    let tree = validate_pending_bvh(
        tree,
        PendingBvhContext {
            colliders: &colliders,
            pending_aabbs: &pending,
        },
        budget,
    )?;
    let adj = validate_coarena(adj, &ca, budget)?;
    let broad = validate_broad_owner(
        broad,
        BroadOwnerContext {
            tree: &tree,
            narrow: &narrow,
            adjacency: &adj,
        },
        budget,
    )?;
    let integration = validate_integration_config(config::config(&w.integration_parameters, id)?)?;
    let ccd = validate_ccd_state(
        config::ccd(&w.ccd_solver, id, budget)?,
        CcdContext {
            colliders: &colliders,
            integration: &integration,
        },
    )?;
    let o = w
        .physics_pipeline
        .checkpoint_observation()
        .map_err(|_| CaptureError::DeferredPoison)?;
    let pipeline = validate_pipeline_state(
        PipelineState {
            identity: id,
            gravity: bits(w.gravity),
            post_solve_ccd_active: o.post_solve_ccd_active,
            phase: PipelinePhase::CompletedPositive,
            deferred_bvh_receiver: o.deferred_bvh_receiver,
            deferred_bvh_inline: o.deferred_bvh_inline,
            quarantine: [
                o.quarantine_bodies as u64,
                o.quarantine_colliders as u64,
                o.quarantine_soft_bodies as u64,
                o.quarantine_body_workspace as u64,
                o.quarantine_collider_workspace as u64,
            ],
        },
        PipelineContext {
            integration: &integration,
            ccd: &ccd,
        },
    )?;
    let frame = join_world_boundary(&broad, &pipeline)?;
    let _ = admit_numeric_frame(frame, id, work)?;
    Ok(encode_world_boundary(frame, budget)?)
}

//! Closed private numeric staging; no native owner installation.
use super::*;
use crate::data::graph::{Graph, NodeIndex};
use crate::geometry::{ContactManifold, PairContacts, PairEventStatus, RigidPairContacts};
use crate::pipeline::checked_restore::{NumericFrame, allocation::*};
use incant_physics_snapshot::WorldBoundary;
mod manifold;
mod pair;
fn handle(h: incant_physics_snapshot::Handle) -> ColliderHandle {
    ColliderHandle::from_raw_parts(h.slot, h.generation)
}
fn dispatcher_plan() -> ConstructionResult<AllocationPlan> {
    // DefaultQueryDispatcher is a ZST, but Arc retains two atomic counters.
    let (l, _) = core::alloc::Layout::array::<usize>(2)
        .map_err(|_| ConstructionError::Limit)?
        .extend(core::alloc::Layout::new::<DefaultQueryDispatcher>())
        .map_err(|_| ConstructionError::Limit)?;
    AllocationPlan::array::<u8>(l.pad_to_align().size())
}
fn check<'a>(n: &NumericFrame<'a>) -> ConstructionResult<WorldBoundary<'a>> {
    let f = n.boundary();
    if n.identity().dimension as usize != crate::math::DIM {
        return Err(ConstructionError::Payload);
    }
    // Pair construction retains every admitted workspace through its own
    // checked adapter; unsupported dispatcher caches still fail explicitly.
    Ok(f)
}
impl NarrowPhase {
    pub(crate) fn checkpoint_dispatcher(
        &self,
    ) -> Arc<dyn parry::query::PersistentQueryDispatcher<ContactManifoldData, ContactData>> {
        self.query_dispatcher.clone()
    }
    pub(crate) fn checkpoint_plan(n: &NumericFrame<'_>) -> ConstructionResult<AllocationPlan> {
        let f = check(n)?;
        let w = f.broad().context().narrow;
        let c = w.context();
        let r = c.rigid.record();
        let mut p = Graph::<ColliderHandle, ContactPair>::checkpoint_plan(f.contact_graph())?;
        p.append(pair::plan(f)?)?;
        p.append(Graph::<ColliderHandle, IntersectionPair>::checkpoint_plan(
            c.intersections,
        )?)?;
        p.append(Coarena::<ColliderGraphIndices>::checkpoint_plan(c.indices)?)?;
        p.append(SolverContactGraph::checkpoint_plan(c.rigid)?)?;
        p.append(AllocationPlan::array::<u128>(r.body_color_masks.len())?)?;
        p.append(AllocationPlan::array::<u16>(r.pair_hints.len())?)?;
        for len in [
            r.force_pairs.len(),
            r.force_positions.len(),
            r.force_flagged.len(),
        ] {
            p.append(AllocationPlan::array::<u32>(len)?)?;
        }
        p.append(dispatcher_plan()?)?;
        Ok(p)
    }
    pub(crate) fn from_checkpoint(
        n: &NumericFrame<'_>,
        p: &mut AllocationPermit,
    ) -> ConstructionResult<Self> {
        let f = check(n)?;
        let w = f.broad().context().narrow;
        let c = w.context();
        let r = c.rigid.record();
        let contact_graph = InteractionGraph {
            graph: Graph::from_checkpoint(
                f.contact_graph(),
                p,
                |row, _| Ok(handle(f.manifolds().record().nodes[row as usize])),
                |row, p| pair::construct(f, row as usize, p),
            )?,
        };
        let intersection_graph = InteractionGraph {
            graph: Graph::from_checkpoint(
                c.intersections,
                p,
                |row, _| Ok(handle(w.record().nodes[row as usize])),
                |row, _| {
                    let s = w.record().intersections[row as usize];
                    Ok(IntersectionPair {
                        intersecting: s.intersecting,
                        start_event_emitted: s.start_event_emitted,
                    })
                },
            )?,
        };
        let graph_indices = Coarena::from_checkpoint(c.indices, p, |row, _| {
            let s = w.record().indices[row as usize];
            Ok(ColliderGraphIndices {
                contact_graph_index: NodeIndex::new(s.contact),
                intersection_graph_index: NodeIndex::new(s.intersection),
            })
        })?;
        let solver_contact_graph = SolverContactGraph::from_checkpoint(c.rigid, p)?;
        let mut body_solver_color_masks = p.vector(r.body_color_masks.len())?;
        body_solver_color_masks.extend_from_slice(&r.body_color_masks);
        let mut pair_solver_hints = p.vector(r.pair_hints.len())?;
        pair_solver_hints.extend(r.pair_hints.iter().map(|v| *v as u16));
        let mut copy = |v: &[u32]| -> ConstructionResult<Vec<u32>> {
            let mut out = p.vector(v.len())?;
            out.extend_from_slice(v);
            Ok(out)
        };
        let force_event_pairs = copy(&r.force_pairs)?;
        let force_event_pos = copy(&r.force_positions)?;
        let force_event_flagged = copy(&r.force_flagged)?;
        p.charge(dispatcher_plan()?)?;
        Ok(Self {
            query_dispatcher: Arc::new(DefaultQueryDispatcher),
            contact_graph,
            intersection_graph,
            graph_indices,
            body_solver_color_masks,
            pair_solver_hints,
            solver_contact_graph,
            solver_graph_valid: r.graph_valid,
            solver_graph_epoch: r.graph_epoch,
            solver_graph_mb_epoch: r.graph_multibody_epoch,
            force_event_pairs,
            force_event_pos,
            force_event_flagged,
            force_list_valid: r.force_valid,
            // Every omitted scratch field has a cited native overwrite/reset path.
            update_candidates: Vec::new(),
            body_qualify_info: Vec::new(),
            awake_body_mask: Vec::new(),
            solver_graph_dirty: Vec::new(),
            solver_color_todo: Vec::new(),
            soft_self: Default::default(),
            retired_pairs: Vec::new(),
        })
    }
}
#[cfg(all(test, feature = "serde-serialize", feature = "incant-checkpoint"))]
mod tests;

mod capture;

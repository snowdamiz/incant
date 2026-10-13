//! Empty-owner refusal is field-based, including removed/recycled native histories.
use super::*;
use crate::data::graph::{Edge, Graph, Node};
use crate::pipeline::checked_restore::{allocation::*, world::OmittedOwnerError};
impl ImpulseJointSet {
    // Only the private test capture issuer calls this until runtime ownership is integrated.
    #[cfg_attr(not(test), expect(dead_code))]
    pub(crate) fn checkpoint_empty_gate(&self) -> Result<(), OmittedOwnerError> {
        if !self.joint_ids.checkpoint_pristine_empty() {
            return Err(OmittedOwnerError::ImpulseArena);
        }
        if !self.rb_graph_ids.checkpoint_no_rows()
            || !self.joint_graph.graph.raw_nodes().is_empty()
            || !self.joint_graph.graph.raw_edges().is_empty()
        {
            return Err(OmittedOwnerError::ImpulseGraph);
        }
        if !self.to_wake_up.is_empty()
            || !self.to_join.is_empty()
            || !self.island_events.is_empty()
            || !self.modified_joints.is_empty()
            || self.all_joints_modified
        {
            return Err(OmittedOwnerError::ImpulseJournal);
        }
        // selection_epochs is invalidated by fresh pipeline before selecting; no live joint exists.
        Ok(())
    }
    pub(crate) fn checkpoint_empty_plan() -> ConstructionResult<AllocationPlan> {
        let mut plan = Arena::<crate::data::graph::EdgeIndex>::checkpoint_empty_plan()?;
        // InteractionGraph::new reserves ten nodes and edges even while empty.
        plan.append(AllocationPlan::array::<Node<RigidBodyHandle>>(10)?)?;
        plan.append(AllocationPlan::array::<Edge<ImpulseJoint>>(10)?)?;
        Ok(plan)
    }
    pub(crate) fn checkpoint_empty(
        epoch: u32,
        p: &mut AllocationPermit,
    ) -> ConstructionResult<Self> {
        Ok(Self {
            joint_ids: Arena::checkpoint_empty(p)?,
            rb_graph_ids: Coarena::new(),
            joint_graph: InteractionGraph {
                graph: Graph {
                    nodes: p.vector(10)?,
                    edges: p.vector(10)?,
                },
            },
            to_wake_up: HashSet::default(),
            to_join: HashSet::default(),
            island_events: Vec::new(),
            modified_joints: Vec::new(),
            all_joints_modified: false,
            assembly_epoch: epoch,
            selection_epochs: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn checked_world_fresh_pipeline_must_invalidate_joint_selection_memo() {
        let mut joints = ImpulseJointSet::new();
        let islands = crate::dynamics::IslandManager::new();
        let bodies = crate::dynamics::RigidBodySet::new();
        joints.selection_epochs = Some((islands.active_set_epoch, joints.assembly_epoch));
        let mut out = alloc::vec![usize::MAX];
        joints.select_active_interactions(&islands, &bodies, &mut out);
        assert_eq!(
            out,
            [usize::MAX],
            "negative control: cached selection does not initialize fresh output"
        );
        joints.invalidate_selection_memo();
        joints.select_active_interactions(&islands, &bodies, &mut out);
        assert!(out.is_empty());
    }
}

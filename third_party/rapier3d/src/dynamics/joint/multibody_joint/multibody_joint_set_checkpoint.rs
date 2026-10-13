//! Empty-owner refusal is field-based, including removed/recycled native histories.
use super::*;
use crate::data::graph::{Edge, Graph, Node};
use crate::pipeline::checked_restore::{allocation::*, world::OmittedOwnerError};
impl MultibodyJointSet {
    // Only the private test capture issuer calls this until runtime ownership is integrated.
    pub(crate) fn checkpoint_empty_gate(&self) -> Result<(), OmittedOwnerError> {
        if !self.multibodies.checkpoint_pristine_empty() {
            return Err(OmittedOwnerError::MultibodyArena);
        }
        if !self.rb2mb.checkpoint_no_rows()
            || !self.connectivity_graph.graph.raw_nodes().is_empty()
            || !self.connectivity_graph.graph.raw_edges().is_empty()
        {
            return Err(OmittedOwnerError::MultibodyGraph);
        }
        if !self.to_wake_up.is_empty()
            || !self.to_join.is_empty()
            || !self.island_chain_events.is_empty()
        {
            return Err(OmittedOwnerError::MultibodyJournal);
        }
        if self.topology_epoch != 0 {
            return Err(OmittedOwnerError::MultibodyEpoch);
        }
        Ok(())
    }
    pub(crate) fn checkpoint_empty_plan() -> ConstructionResult<AllocationPlan> {
        let mut plan = Arena::<Multibody>::checkpoint_empty_plan()?;
        // InteractionGraph::new reserves ten nodes and edges even while empty.
        plan.append(AllocationPlan::array::<Node<RigidBodyHandle>>(10)?)?;
        plan.append(AllocationPlan::array::<Edge<()>>(10)?)?;
        Ok(plan)
    }
    pub(crate) fn checkpoint_empty(p: &mut AllocationPermit) -> ConstructionResult<Self> {
        Ok(Self {
            multibodies: Arena::checkpoint_empty(p)?,
            rb2mb: Coarena::new(),
            connectivity_graph: InteractionGraph {
                graph: Graph {
                    nodes: p.vector(10)?,
                    edges: p.vector(10)?,
                },
            },
            to_wake_up: HashSet::default(),
            to_join: HashSet::default(),
            island_chain_events: Vec::new(),
            topology_epoch: 0,
        })
    }
}

//! Read-only solver access while foreign code holds exclusive numeric ECS views.
use crate::{NativeWorld, NumericColumns, RayHit, RayQuery, RuntimeError, StableId};
use incant_physics_runtime::{PhysicsError, PhysicsRuntime};
use std::{cell::RefCell, rc::Rc};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum QueryError {
    #[error("physics query capability has expired")]
    Expired,
    #[error("physics query belongs to a different scene")]
    Scene,
    #[error(transparent)]
    Physics(#[from] PhysicsError),
}

/// A read-only capability for one numeric loan. Retention cannot retain the
/// solver after the loan ends; later calls fail, including during a new loan.
/// Queries see the last committed solver poses, not pending numeric writes or
/// queued structural commands. No ECS pointer or live solver handle escapes.
#[derive(Clone)]
pub struct NativeQueries {
    solver: Rc<RefCell<Option<PhysicsRuntime<StableId>>>>,
    scene: StableId,
    used: bool,
}
impl NativeQueries {
    pub fn scene_id(&self) -> StableId {
        self.scene
    }
    pub fn raycast(
        &self,
        query: &RayQuery<StableId>,
    ) -> Result<Option<RayHit<StableId>>, QueryError> {
        let solver = self.solver.borrow();
        let solver = solver.as_ref().ok_or(QueryError::Expired)?;
        if query.scene_id != self.scene {
            return Err(QueryError::Scene);
        }
        if self.used {
            Ok(solver.raycast(query)?)
        } else {
            query.validate()?;
            Ok(None)
        }
    }
}

struct Restore<'a> {
    world: &'a mut NativeWorld,
    queries: NativeQueries,
}
impl Drop for Restore<'_> {
    fn drop(&mut self) {
        // Only owned values leave raycast; no caller can retain a RefCell loan.
        // This empties every retained capability before restoring solver ownership.
        if let Some(solver) = self.queries.solver.borrow_mut().take() {
            self.world.physics = solver;
        }
    }
}

impl NativeWorld {
    /// Lend numeric ECS columns and read-only committed physics synchronously.
    /// Moving ownership costs one bounded allocation, not a world/solver clone.
    /// The private guard exposes no world methods during the temporary move.
    /// NumericColumns validation and cleanup use only ECS data/hierarchy; they
    /// must not access the temporarily detached solver. Solver ownership is
    /// restored on success, error and unwinding, after numeric cleanup finishes.
    pub fn with_numeric_physics<R>(
        &mut self,
        operation: impl FnOnce(&mut NumericColumns<'_>, NativeQueries) -> R,
    ) -> Result<R, RuntimeError> {
        self.ensure_usable()?;
        let queries = NativeQueries {
            solver: Rc::new(RefCell::new(None)),
            scene: self.scene_id(),
            used: self.physics_used,
        };
        let guard = Restore {
            world: self,
            queries,
        };
        *guard.queries.solver.borrow_mut() = Some(std::mem::take(&mut guard.world.physics));
        let queries = guard.queries.clone();
        Ok(guard
            .world
            .with_numeric_columns(|columns| operation(columns, queries)))
    }
}

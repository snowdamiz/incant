//! Read-only solver access while foreign code holds exclusive numeric ECS views.
use crate::{
    CharacterMovement, CharacterQuery, NativeWorld, NumericColumns, RayHit, RayHit2D, RayQuery,
    RayQuery2D, RuntimeError, StableId, TriggerEvent, TriggerEvent2D,
};
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
    #[error(transparent)]
    Physics2D(#[from] incant_physics_2d_runtime::PhysicsError),
}

/// A read-only capability for one numeric loan. Retention cannot retain the
/// solver after the loan ends; later calls fail, including during a new loan.
/// Queries see the last committed solver poses, not pending numeric writes or
/// queued structural commands. No ECS pointer or live solver handle escapes.
#[derive(Clone)]
pub struct NativeQueries {
    solver: Rc<RefCell<Option<PhysicsRuntime<StableId>>>>,
    planar: Rc<RefCell<Option<incant_physics_2d_runtime::PhysicsWorld2D>>>,
    scene: StableId,
    used: bool,
}
impl NativeQueries {
    /// Raycast the committed planar solver; pending numeric/structural writes
    /// are intentionally invisible until the next synchronization point.
    pub fn raycast_2d(&self, query: &RayQuery2D) -> Result<Option<RayHit2D>, QueryError> {
        let solver = self.planar.borrow();
        Ok(solver.as_ref().ok_or(QueryError::Expired)?.raycast(query)?)
    }

    /// Owned, sorted sensor events from the last committed planar step.
    pub fn trigger_events_2d(&self) -> Result<Vec<TriggerEvent2D>, QueryError> {
        let solver = self.planar.borrow();
        Ok(solver
            .as_ref()
            .ok_or(QueryError::Expired)?
            .events()
            .to_vec())
    }

    /// Computes a displacement without moving the body. The caller applies it
    /// through the same numeric columns/structural synchronization as other edits.
    pub fn compute_character_motion(
        &self,
        query: &CharacterQuery<StableId>,
        dt: f64,
    ) -> Result<CharacterMovement<StableId>, QueryError> {
        let solver = self.solver.borrow();
        let solver = solver.as_ref().ok_or(QueryError::Expired)?;
        if query.scene_id != self.scene {
            return Err(QueryError::Scene);
        }
        Ok(solver.compute_character_motion(query, dt)?)
    }

    /// Owned events from the last committed solver step. Reading does not drain
    /// them; retaining or mutating the copy cannot affect this or later steps.
    pub fn trigger_events(&self) -> Result<Vec<TriggerEvent<StableId>>, QueryError> {
        let solver = self.solver.borrow();
        let solver = solver.as_ref().ok_or(QueryError::Expired)?;
        Ok(solver.events().to_vec())
    }

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
        if let Some(solver) = self.queries.planar.borrow_mut().take() {
            self.world.physics_2d = solver;
        }
        if let Some(solver) = self.queries.solver.borrow_mut().take() {
            self.world.physics = solver;
        }
    }
}

impl NativeWorld {
    /// Lend numeric ECS columns and read-only committed physics synchronously.
    /// Moving ownership costs two bounded allocations, not a world/solver clone.
    /// The private guard exposes no world methods during the temporary move.
    /// NumericColumns validation and cleanup use only ECS data/hierarchy; they
    /// must not access either temporarily detached solver. Solver ownership is
    /// restored on success, error and unwinding, after numeric cleanup finishes.
    pub fn with_numeric_physics<R>(
        &mut self,
        operation: impl FnOnce(&mut NumericColumns<'_>, NativeQueries) -> R,
    ) -> Result<R, RuntimeError> {
        self.ensure_usable()?;
        let queries = NativeQueries {
            solver: Rc::new(RefCell::new(None)),
            planar: Rc::new(RefCell::new(None)),
            scene: self.scene_id(),
            used: self.physics_used,
        };
        let guard = Restore {
            world: self,
            queries,
        };
        *guard.queries.solver.borrow_mut() = Some(std::mem::take(&mut guard.world.physics));
        *guard.queries.planar.borrow_mut() = Some(std::mem::take(&mut guard.world.physics_2d));
        let queries = guard.queries.clone();
        Ok(guard
            .world
            .with_numeric_columns(|columns| operation(columns, queries)))
    }
}

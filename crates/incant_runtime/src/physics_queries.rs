//! Read-only solver and navigation access while foreign code holds exclusive numeric ECS views.
use crate::{
    CharacterMovement, CharacterQuery, NativeWorld, NumericColumns, RayHit, RayHit2D, RayQuery,
    RayQuery2D, RuntimeError, StableId, TriggerEvent, TriggerEvent2D,
};
use incant_physics_runtime::{PhysicsError, PhysicsRuntime};
use std::{cell::RefCell, rc::Rc};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum QueryError {
    #[error("native query capability has expired")]
    Expired,
    #[error("physics query belongs to a different scene")]
    Scene,
    #[error(transparent)]
    Navigation(#[from] crate::SceneError),
    #[error(transparent)]
    Avoidance(#[from] incant_nav::NavigationError),
    #[error(transparent)]
    Physics(#[from] PhysicsError),
    #[error(transparent)]
    Physics2D(#[from] incant_physics_2d_runtime::PhysicsError),
}

/// A read-only capability for one numeric loan. Retention cannot retain the
/// solvers or navigation cache after the loan ends; later calls fail, including during a new loan.
/// Queries see the last committed solver poses/cache, not pending numeric writes or
/// queued structural commands. No ECS pointer or live solver handle escapes.
#[derive(Clone)]
pub struct NativeQueries {
    localization: Rc<RefCell<Option<crate::NativeLocalization>>>,
    solver: Rc<RefCell<Option<PhysicsRuntime<StableId>>>>,
    planar: Rc<RefCell<Option<incant_physics_2d_runtime::PhysicsWorld2D>>>,
    navigation: Rc<RefCell<Option<crate::navigation_runtime::NavigationRuntime>>>,
    scene: StableId,
    used: bool,
}
impl NativeQueries {
    pub fn localize(
        &self,
        request: &crate::LocalizeRequest,
    ) -> Result<crate::LocalizedText, QueryError> {
        Ok(self
            .localization
            .borrow()
            .as_ref()
            .ok_or(QueryError::Expired)?
            .localize(request)?)
    }
    pub fn locale(&self) -> Result<crate::LocaleSettings, QueryError> {
        Ok(self
            .localization
            .borrow()
            .as_ref()
            .ok_or(QueryError::Expired)?
            .settings()
            .clone())
    }
    pub fn localization_report(
        &self,
        request: crate::LocalizationReportRequest,
    ) -> Result<crate::LocalizationReport, QueryError> {
        Ok(self
            .localization
            .borrow()
            .as_ref()
            .ok_or(QueryError::Expired)?
            .report(request)?)
    }
    pub fn format_number(&self, value: f64) -> Result<String, QueryError> {
        let data = self.localization.borrow();
        let locale = &data.as_ref().ok_or(QueryError::Expired)?.settings().locale;
        Ok(
            incant_localization::format_number(locale, value)
                .map_err(crate::localization::error)?,
        )
    }
    pub fn format_date(
        &self,
        value: &crate::CalendarDate,
        length: crate::DateLength,
    ) -> Result<String, QueryError> {
        let data = self.localization.borrow();
        let locale = &data.as_ref().ok_or(QueryError::Expired)?.settings().locale;
        Ok(incant_localization::format_date(locale, value, length)
            .map_err(crate::localization::error)?)
    }

    /// Owned world-space path from the committed mesh, including explicit
    /// off-mesh traversal metadata. No movement or rebuild is performed.
    pub fn navigation_path(
        &self,
        entity: StableId,
        request: &crate::PathRequest,
    ) -> Result<Option<crate::NavigationPath>, QueryError> {
        let cache = self.navigation.borrow();
        Ok(cache
            .as_ref()
            .ok_or(QueryError::Expired)?
            .path(entity, request)?)
    }
    /// Owned adjacent grid cells from the committed grid. Grid coordinates
    /// remain indices; mapping them to world positions belongs to gameplay.
    pub fn grid_path(
        &self,
        entity: StableId,
        request: &crate::GridPathRequest,
    ) -> Result<Option<crate::GridPath>, QueryError> {
        let cache = self.navigation.borrow();
        Ok(cache
            .as_ref()
            .ok_or(QueryError::Expired)?
            .grid_path(entity, request)?)
    }

    /// Stateless world-XZ avoidance proposals for a caller-supplied bulk sample.
    /// IDs correlate input/output; they need not name entities in this world.
    /// Geometry, movement and physical collision remain the caller's concern.
    /// The capability still expires with its numeric loan, even for empty input.
    pub fn steer_agents(
        &self,
        request: &crate::SteeringQuery,
        dt: f64,
    ) -> Result<Vec<crate::SteeringVelocity>, QueryError> {
        if self.navigation.borrow().is_none() {
            return Err(QueryError::Expired);
        }
        if !dt.is_finite() || !(1. / 240. ..=1.).contains(&dt) {
            return Err(incant_nav::NavigationError::Invalid("steering time step".into()).into());
        }
        Ok(incant_nav::steer(request, dt as f32)?)
    }

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
        self.queries.localization.borrow_mut().take();
        // Only owned values leave queries; no caller can retain a RefCell loan.
        // Empty each retained capability before restoring its owned native state.
        if let Some(cache) = self.queries.navigation.borrow_mut().take() {
            self.world.navigation = cache;
        }
        if let Some(solver) = self.queries.planar.borrow_mut().take() {
            self.world.physics_2d = solver;
        }
        if let Some(solver) = self.queries.solver.borrow_mut().take() {
            self.world.physics = solver;
        }
    }
}

impl NativeWorld {
    /// Lend numeric ECS columns and read-only committed physics/navigation synchronously.
    /// Moving ownership costs four bounded allocations, not a world/solver/cache clone.
    /// The private guard exposes no world methods during the temporary move.
    /// NumericColumns validation and cleanup use only ECS data/hierarchy; they
    /// must not access the temporarily detached solvers/cache. Ownership is
    /// restored on success, error and unwinding, after numeric cleanup finishes.
    pub fn with_numeric_physics<R>(
        &mut self,
        operation: impl FnOnce(&mut NumericColumns<'_>, NativeQueries) -> R,
    ) -> Result<R, RuntimeError> {
        self.ensure_navigation_current()?;
        let queries = NativeQueries {
            localization: Rc::new(RefCell::new(Some(self.localization.clone()))),
            solver: Rc::new(RefCell::new(None)),
            planar: Rc::new(RefCell::new(None)),
            navigation: Rc::new(RefCell::new(None)),
            scene: self.scene_id(),
            used: self.physics_used,
        };
        let guard = Restore {
            world: self,
            queries,
        };
        *guard.queries.solver.borrow_mut() = Some(std::mem::take(&mut guard.world.physics));
        *guard.queries.planar.borrow_mut() = Some(std::mem::take(&mut guard.world.physics_2d));
        *guard.queries.navigation.borrow_mut() = Some(std::mem::take(&mut guard.world.navigation));
        let queries = guard.queries.clone();
        Ok(guard
            .world
            .with_numeric_columns(|columns| operation(columns, queries)))
    }
}

//! Synchronous, bounded read-only queries. Guest getters run without Rust
//! borrows; every invocation consumes its budget before arguments are inspected.
#[cfg(all(test, feature = "compiler"))]
pub(super) use crate::query_values::{FAIL_HIT_ALLOCATION, FAIL_RESULT_ALLOCATION};
use crate::{
    ScriptError, budget, diagnostics,
    query_values::{self, Kind, Request, ResultData},
};
#[cfg(feature = "diagnostic-script-queries")]
use incant_profile::diagnostic::queries::{Call, Capability, Stage};
use incant_runtime::NativeQueries;
use rquickjs::{Ctx, Exception, Function, Object, Value, function::Rest};
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, Mutex},
};

pub(crate) const MAX_PHYSICS_QUERIES: usize = 1024;
const MAX_CHARACTER_QUERIES: usize = 128;
const MAX_NAVIGATION_QUERIES: usize = 32;
const MAX_NAVIGATION_EXPANSIONS: u32 = 65_536;
#[derive(Default)]
struct State {
    active: bool,
    calls: usize,
    characters: usize,
    navigation: usize,
    expansions: u32,
    steering_agents: usize,
    steering_edges: usize,
    localization: usize,
    argument_bytes: usize,
    output_bytes: usize,
    failure: Option<ScriptError>,
}
pub(crate) struct TickQueries {
    queries: NativeQueries,
    deadline: Arc<Mutex<budget::ExecutionBudget>>,
    state: Rc<RefCell<State>>,
    dt: f64,
}
impl TickQueries {
    pub fn healthy(&self) -> bool {
        self.state.borrow().failure.is_none()
    }
    pub fn new(
        queries: NativeQueries,
        deadline: Arc<Mutex<budget::ExecutionBudget>>,
        dt: f64,
    ) -> Self {
        Self {
            queries,
            deadline,
            dt,
            state: Rc::default(),
        }
    }
    pub fn activate(&self) {
        self.state.borrow_mut().active = true;
    }
    pub fn revoke(&self) {
        self.state.borrow_mut().active = false;
    }
    pub fn check(&self) -> Result<(), ScriptError> {
        self.revoke();
        self.state.borrow_mut().failure.take().map_or(Ok(()), Err)
    }
    pub fn expose<'js>(&self, ctx: &Ctx<'js>) -> rquickjs::Result<Object<'js>> {
        self.expose_methods(
            ctx,
            &[
                ("raycast", Kind::Ray),
                ("computeCharacterMotion", Kind::Character),
                ("triggerEvents", Kind::Triggers),
                ("raycast2D", Kind::Ray2D),
                ("triggerEvents2D", Kind::Triggers2D),
            ],
        )
    }
    pub fn expose_navigation<'js>(&self, ctx: &Ctx<'js>) -> rquickjs::Result<Object<'js>> {
        self.expose_methods(
            ctx,
            &[
                ("findPath", Kind::Navigation),
                ("findGridPath", Kind::Grid),
                ("steerAgents", Kind::Steering),
            ],
        )
    }
    pub fn expose_localization<'js>(&self, ctx: &Ctx<'js>) -> rquickjs::Result<Object<'js>> {
        self.expose_methods(
            ctx,
            &[
                ("localize", Kind::Localize),
                ("locale", Kind::Locale),
                ("formatNumber", Kind::Number),
                ("formatDate", Kind::Date),
                ("report", Kind::Report),
            ],
        )
    }
    fn expose_methods<'js>(
        &self,
        ctx: &Ctx<'js>,
        methods: &[(&str, Kind)],
    ) -> rquickjs::Result<Object<'js>> {
        let object = Object::new_proto(ctx.clone(), None)?;
        for &(name, kind) in methods {
            let dt = self.dt;
            let state = Rc::clone(&self.state);
            let queries = self.queries.clone();
            let deadline = Arc::clone(&self.deadline);
            let function =
                Function::new(ctx.clone(), move |ctx: Ctx<'js>, args: Rest<Value<'js>>| {
                    #[cfg(feature = "diagnostic-script-queries")]
                    let observation = Call::start(match kind {
                        Kind::Character => Capability::Character,
                        Kind::Triggers => Capability::Triggers,
                        Kind::Navigation => Capability::Navigation,
                        Kind::Locale => Capability::Locale,
                        Kind::Localize => Capability::Localize,
                        Kind::Ray
                        | Kind::Ray2D
                        | Kind::Triggers2D
                        | Kind::Grid
                        | Kind::Steering
                        | Kind::Number
                        | Kind::Date
                        | Kind::Report => Capability::Other,
                    });
                    let localized = matches!(
                        kind,
                        Kind::Localize | Kind::Locale | Kind::Number | Kind::Date | Kind::Report
                    );
                    let cost = if localized { 4 } else { 1 };
                    let rejection = {
                        let mut state = state.borrow_mut();
                        if !state.active {
                            Some("native queries belong to their active tick only")
                        } else if state.failure.is_some() {
                            Some("native query capability already failed")
                        } else if state.calls > MAX_PHYSICS_QUERIES - cost
                            || (localized && state.localization == 64)
                            || (matches!(kind, Kind::Character)
                                && state.characters == MAX_CHARACTER_QUERIES)
                            || (matches!(kind, Kind::Navigation | Kind::Grid | Kind::Steering)
                                && state.navigation == MAX_NAVIGATION_QUERIES)
                        {
                            state.failure = Some(ScriptError::InputLimit);
                            Some("native query limit exceeded")
                        } else {
                            state.calls += cost;
                            if localized {
                                state.localization += 1;
                            }
                            if matches!(kind, Kind::Navigation | Kind::Grid | Kind::Steering) {
                                state.navigation += 1;
                            }
                            if matches!(kind, Kind::Character) {
                                state.characters += 1;
                            }
                            None
                        }
                    };
                    if let Some(message) = rejection {
                        return Err(Exception::throw_type(&ctx, message));
                    }
                    #[cfg(feature = "diagnostic-script-queries")]
                    let parse = observation.stage(Stage::Parse);
                    let request = query_values::parse(kind, &ctx, &args.0, queries.scene_id(), dt);
                    #[cfg(feature = "diagnostic-script-queries")]
                    parse.finish(request.is_ok());
                    let request = match request {
                        Ok(request) => request,
                        Err(error) => {
                            state
                                .borrow_mut()
                                .failure
                                .get_or_insert(ScriptError::QueryInput);
                            return Err(error);
                        }
                    };
                    // A recursive getter may have exhausted/failed this tick while parsing.
                    if state.borrow().failure.is_some() {
                        return Err(Exception::throw_type(
                            &ctx,
                            "native query capability failed",
                        ));
                    }
                    let input_bytes = crate::localization_values::input_bytes(&request);
                    {
                        let mut state = state.borrow_mut();
                        if input_bytes > 65_536 - state.argument_bytes {
                            state.failure = Some(ScriptError::InputLimit);
                            return Err(Exception::throw_type(
                                &ctx,
                                "localization argument budget exceeded",
                            ));
                        }
                        state.argument_bytes += input_bytes;
                    }
                    // Charge requested maximum work, including misses, before entering
                    // native search. Nested getters have already consumed their share.
                    let expansions = match &request {
                        Request::Navigation(_, q) => q.max_visited,
                        Request::Grid(_, q) => q.max_expansions,
                        _ => 0,
                    };
                    {
                        let mut state = state.borrow_mut();
                        if expansions > MAX_NAVIGATION_EXPANSIONS - state.expansions {
                            state.failure = Some(ScriptError::InputLimit);
                            return Err(Exception::throw_type(
                                &ctx,
                                "navigation expansion limit exceeded",
                            ));
                        }
                        state.expansions += expansions;
                        if let Request::Steering(query) = &request {
                            let agents = query.agents.len();
                            let edges: usize = query
                                .obstacles
                                .iter()
                                .map(|o| if o.closed { o.vertices.len() } else { 1 })
                                .sum();
                            if agents > 128 - state.steering_agents
                                || edges > 128 - state.steering_edges
                            {
                                state.failure = Some(ScriptError::InputLimit);
                                return Err(Exception::throw_type(
                                    &ctx,
                                    "steering work limit exceeded",
                                ));
                            }
                            state.steering_agents += agents;
                            state.steering_edges += edges;
                        }
                    }
                    if let Err(error) = diagnostics::check_deadline(&deadline) {
                        state.borrow_mut().failure.get_or_insert(error);
                        return Err(Exception::throw_type(
                            &ctx,
                            "native query deadline exceeded",
                        ));
                    }
                    // No JS runs while a solver/cache is borrowed. Results are owned.
                    #[cfg(feature = "diagnostic-script-queries")]
                    let execute = observation.stage(Stage::Execute);
                    let result = match request {
                        Request::Localize(request) => {
                            queries.localize(&request).map(ResultData::Localized)
                        }
                        Request::Locale => queries.locale().map(ResultData::Locale),
                        Request::Number(value) => {
                            queries.format_number(value).map(ResultData::Text)
                        }
                        Request::Date(value, length) => {
                            queries.format_date(&value, length).map(ResultData::Text)
                        }
                        Request::Report(request) => {
                            queries.localization_report(request).map(ResultData::Report)
                        }
                        Request::Navigation(entity, request) => queries
                            .navigation_path(entity, &request)
                            .map(ResultData::Navigation),
                        Request::Grid(entity, request) => {
                            queries.grid_path(entity, &request).map(ResultData::Grid)
                        }
                        Request::Steering(request) => {
                            queries.steer_agents(&request, dt).map(ResultData::Steering)
                        }
                        Request::Ray2D(request) => {
                            queries.raycast_2d(&request).map(ResultData::Ray2D)
                        }
                        Request::Triggers2D => {
                            queries.trigger_events_2d().map(ResultData::Triggers2D)
                        }
                        Request::Ray(request) => queries.raycast(&request).map(ResultData::Ray),
                        Request::Character(request) => queries
                            .compute_character_motion(&request, dt)
                            .map(ResultData::Character),
                        Request::Triggers => queries.trigger_events().map(ResultData::Triggers),
                    };
                    #[cfg(feature = "diagnostic-script-queries")]
                    execute.finish(result.is_ok());
                    let result = match result {
                        Ok(result) => result,
                        Err(error) => {
                            state
                                .borrow_mut()
                                .failure
                                .get_or_insert(ScriptError::PhysicsQuery(error));
                            return Err(Exception::throw_type(&ctx, "invalid native query"));
                        }
                    };
                    // QuickJS interrupts cannot preempt native search; account for the native
                    // call immediately, before any more guest code or result allocation.
                    if let Err(error) = diagnostics::check_deadline(&deadline) {
                        state.borrow_mut().failure.get_or_insert(error);
                        return Err(Exception::throw_type(
                            &ctx,
                            "native query deadline exceeded",
                        ));
                    }
                    {
                        let bytes = crate::localization_values::output_bytes(&result);
                        let mut state = state.borrow_mut();
                        if bytes > 262_144 - state.output_bytes {
                            state.failure = Some(ScriptError::InputLimit);
                            return Err(Exception::throw_type(
                                &ctx,
                                "localization output budget exceeded",
                            ));
                        }
                        state.output_bytes += bytes;
                    }
                    #[cfg(feature = "diagnostic-script-queries")]
                    let materialize = observation.stage(Stage::Materialize);
                    let result = query_values::materialize(&ctx, result);
                    #[cfg(feature = "diagnostic-script-queries")]
                    materialize.finish(result.is_ok());
                    match result {
                        Ok(result) => {
                            #[cfg(feature = "diagnostic-script-queries")]
                            observation.finish(true);
                            Ok(result)
                        }
                        Err(error) => {
                            let failure = if matches!(error, rquickjs::Error::Allocation) {
                                ScriptError::MemoryAllocation
                            } else {
                                ScriptError::Execution
                            };
                            state.borrow_mut().failure.get_or_insert(failure);
                            #[cfg(feature = "diagnostic-script-queries")]
                            observation.finish(false);
                            Err(error)
                        }
                    }
                })?;
            object.set(name, function)?;
        }
        Ok(object)
    }
}
impl Drop for TickQueries {
    fn drop(&mut self) {
        self.revoke();
    }
}

//! Synchronous, bounded read-only queries. Guest getters run without Rust
//! borrows; every invocation consumes its budget before arguments are inspected.
use crate::{ScriptError, budget, commands, diagnostics};
use incant_runtime::{NativeQueries, RayQuery, StableId};
use rquickjs::{Array, Ctx, Exception, Function, Object, Value, function::Rest, object::Property};
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, Mutex},
};

pub(crate) const MAX_RAYCASTS: usize = 1024;
#[derive(Default)]
struct State {
    active: bool,
    calls: usize,
    failure: Option<ScriptError>,
}
pub(crate) struct TickQueries {
    queries: NativeQueries,
    deadline: Arc<Mutex<budget::ExecutionBudget>>,
    state: Rc<RefCell<State>>,
}
impl TickQueries {
    pub fn new(queries: NativeQueries, deadline: Arc<Mutex<budget::ExecutionBudget>>) -> Self {
        Self {
            queries,
            deadline,
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
        let object = Object::new_proto(ctx.clone(), None)?;
        let state = Rc::clone(&self.state);
        let queries = self.queries.clone();
        let deadline = Arc::clone(&self.deadline);
        let function = Function::new(ctx.clone(), move |ctx: Ctx<'js>, args: Rest<Value<'js>>| {
            let rejection = {
                let mut state = state.borrow_mut();
                if !state.active {
                    Some("physics queries belong to their active tick only")
                } else if state.failure.is_some() {
                    Some("physics query capability already failed")
                } else if state.calls == MAX_RAYCASTS {
                    state.failure = Some(ScriptError::InputLimit);
                    Some("physics query limit exceeded")
                } else {
                    state.calls += 1;
                    None
                }
            };
            if let Some(message) = rejection {
                return Err(Exception::throw_type(&ctx, message));
            }
            let request = match parse(&ctx, &args.0, queries.scene_id()) {
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
                    "physics query capability failed",
                ));
            }
            if let Err(error) = diagnostics::check_deadline(&deadline) {
                state.borrow_mut().failure.get_or_insert(error);
                return Err(Exception::throw_type(
                    &ctx,
                    "physics query deadline exceeded",
                ));
            }
            // No JS runs while the solver is borrowed. Hits are owned scalars.
            let hit = match queries.raycast(&request) {
                Ok(hit) => hit,
                Err(error) => {
                    state
                        .borrow_mut()
                        .failure
                        .get_or_insert(ScriptError::PhysicsQuery(error));
                    return Err(Exception::throw_type(&ctx, "invalid native physics query"));
                }
            };
            // QuickJS interrupts cannot preempt Rapier; account for the native
            // call immediately, before any more guest code or result allocation.
            if let Err(error) = diagnostics::check_deadline(&deadline) {
                state.borrow_mut().failure.get_or_insert(error);
                return Err(Exception::throw_type(
                    &ctx,
                    "physics query deadline exceeded",
                ));
            }
            let Some(hit) = hit else {
                return Ok(Value::new_null(ctx));
            };
            match materialize(&ctx, hit) {
                Ok(result) => Ok(result),
                Err(error) => {
                    let failure = if matches!(error, rquickjs::Error::Allocation) {
                        ScriptError::MemoryAllocation
                    } else {
                        ScriptError::Execution
                    };
                    state.borrow_mut().failure.get_or_insert(failure);
                    Err(error)
                }
            }
        })?;
        object.set("raycast", function)?;
        Ok(object)
    }
}
impl Drop for TickQueries {
    fn drop(&mut self) {
        self.revoke();
    }
}

fn parse<'js>(
    ctx: &Ctx<'js>,
    args: &[Value<'js>],
    scene_id: StableId,
) -> rquickjs::Result<RayQuery<StableId>> {
    let [value] = args else {
        return Err(Exception::throw_type(ctx, "raycast expects one query"));
    };
    let object = value
        .as_object()
        .ok_or_else(|| Exception::throw_type(ctx, "expected a ray query object"))?;
    if !object.get::<_, Value>("scene_id")?.is_undefined() {
        return Err(Exception::throw_type(
            ctx,
            "native raycasts use the active scene only",
        ));
    }
    let vector = |name| commands::numbers(ctx, &object.get::<_, Value>(name)?);
    let number = |name| {
        object
            .get::<_, Value>(name)?
            .as_number()
            .ok_or_else(|| Exception::throw_type(ctx, "expected numeric ray field"))
    };
    let mask = |name| {
        let value = number(name)?;
        if value.is_finite() && (0. ..=u32::MAX as f64).contains(&value) && value.fract() == 0. {
            Ok(value as u32)
        } else {
            Err(Exception::throw_type(
                ctx,
                "expected unsigned 32-bit collision mask",
            ))
        }
    };
    let exclude = object.get::<_, Value>("exclude_entity")?;
    Ok(RayQuery {
        scene_id,
        origin: vector("origin")?,
        direction: vector("direction")?,
        max_distance: number("max_distance")?,
        include_sensors: object
            .get::<_, Value>("include_sensors")?
            .as_bool()
            .ok_or_else(|| Exception::throw_type(ctx, "expected include_sensors boolean"))?,
        exclude_entity: if exclude.is_undefined() {
            None
        } else {
            commands::optional_id(ctx, &exclude)?
        },
        memberships: mask("memberships")?,
        filter: mask("filter")?,
    })
}

// Mutable VM-owned results still define own properties, bypassing guest setters.
fn owned_vector<'js>(ctx: &Ctx<'js>, values: [f64; 3]) -> rquickjs::Result<Array<'js>> {
    let result = Array::new(ctx.clone())?;
    for (index, value) in values.into_iter().enumerate() {
        result.as_object().prop(
            index as u32,
            Property::from(value).enumerable().writable().configurable(),
        )?;
    }
    Ok(result)
}

#[cfg(test)]
thread_local! {
    // Deterministic bridge-allocation fault, absent from production builds.
    pub(super) static FAIL_HIT_ALLOCATION: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}
fn materialize<'js>(
    ctx: &Ctx<'js>,
    hit: incant_runtime::RayHit<StableId>,
) -> rquickjs::Result<Value<'js>> {
    #[cfg(test)]
    if FAIL_HIT_ALLOCATION.with(|flag| flag.replace(false)) {
        return Err(rquickjs::Error::Allocation);
    }
    let result = Object::new_proto(ctx.clone(), None)?;
    result.set(
        "entity_id",
        ulid::Ulid::from_bytes(hit.entity_id.0).to_string(),
    )?;
    result.set("distance", hit.distance)?;
    result.set("point", owned_vector(ctx, hit.point)?)?;
    result.set("normal", owned_vector(ctx, hit.normal)?)?;
    Ok(result.into_value())
}

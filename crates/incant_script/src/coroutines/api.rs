use super::*;
use crate::{bindings::expose_value, persistence::value};
use rquickjs::{Ctx, Exception, Function, Object, Value, function::Rest, object::Property};
use std::{cell::RefCell, rc::Rc};

struct State {
    store: Store,
    active: Option<Scope>,
    calls: usize,
    resumes: usize,
    failure: Option<ScriptError>,
    transfer: Cost,
    tick: u64,
}
pub(crate) struct Tick(Rc<RefCell<State>>);
impl Tick {
    pub fn new(store: Store, tick: u64) -> Self {
        Self(Rc::new(RefCell::new(State {
            store,
            active: None,
            calls: 0,
            resumes: 0,
            failure: None,
            transfer: Cost::default(),
            tick,
        })))
    }
    pub fn activate(&self, scope: Scope) {
        self.0.borrow_mut().active = Some(scope);
    }
    pub fn revoke(&self) {
        self.0.borrow_mut().active = None;
    }
    pub fn check(&self) -> Result<(), ScriptError> {
        if self.0.borrow().failure.is_some() {
            Err(ScriptError::CoroutineInput)
        } else {
            Ok(())
        }
    }
    pub fn take(&self) -> Result<Store, ScriptError> {
        self.revoke();
        self.check()?;
        Ok(std::mem::take(&mut self.0.borrow_mut().store))
    }
    pub fn due(&self, scope: Scope) -> Vec<(Key, Arc<Task>)> {
        let state = self.0.borrow();
        state
            .store
            .entries
            .iter()
            .filter(|(key, task)| key.scope == scope && task.due == state.tick)
            .map(|(key, task)| (key.clone(), Arc::clone(task)))
            .collect()
    }
    pub fn current(&self, key: &Key, task: &Arc<Task>) -> bool {
        self.0
            .borrow()
            .store
            .entries
            .get(key)
            .is_some_and(|v| Arc::ptr_eq(v, task))
    }
    pub fn task<'js>(
        &self,
        ctx: &Ctx<'js>,
        freeze: &Function<'js>,
        key: &Key,
        task: &Task,
    ) -> rquickjs::Result<Object<'js>> {
        {
            let mut state = self.0.borrow_mut();
            if state.resumes == MAX_TASKS {
                return Err(invalid(ctx));
            }
            state.resumes += 1;
            state
                .transfer
                .add(task.cost.bytes, task.cost.nodes)
                .map_err(|_| invalid(ctx))?;
        }
        let object = Object::new_proto(ctx.clone(), None)?;
        object.prop("id", Property::from(key.id.clone()).enumerable())?;
        object.prop(
            "state",
            Property::from(expose_value(ctx, freeze, &task.value)?).enumerable(),
        )?;
        if let Some(id) = task.entity {
            object.prop(
                "entity",
                Property::from(ulid::Ulid::from_bytes(id.0).to_string()).enumerable(),
            )?;
        }
        freeze.call::<_, ()>((object.clone(),))?;
        Ok(object)
    }
    pub fn finish<'js>(
        &self,
        ctx: &Ctx<'js>,
        inspect: &Function<'js>,
        key: &Key,
        old: &Arc<Task>,
        result: Value<'js>,
    ) -> rquickjs::Result<()> {
        // The fixed inspector can run proxy traps. No Rust RefCell borrow
        // spans inspection; native capabilities are still inside the loan.
        if !result.is_object() || result.is_array() || result.is_function() {
            return Err(invalid(ctx));
        }
        let rows: rquickjs::Array = inspect.call((result, 3u32))?;
        let mut fields = BTreeMap::new();
        for row in rows.iter::<rquickjs::Array>() {
            let row = row?;
            let name = value::text(ctx, &row.get(0)?, 10)?;
            if fields.insert(name, row.get::<Value>(1)?).is_some() {
                return Err(invalid(ctx));
            }
        }
        let kind = value::text(ctx, &fields.remove("kind").ok_or_else(|| invalid(ctx))?, 4)?;
        let mut cost = Cost::default();
        let next = match kind.as_str() {
            "done" if fields.is_empty() => None,
            "wait" if fields.len() == 2 => {
                let delay = fields
                    .remove("delayTicks")
                    .and_then(|v| v.as_number())
                    .and_then(ticks)
                    .ok_or_else(|| invalid(ctx))?;
                let raw = fields.remove("state").ok_or_else(|| invalid(ctx))?;
                let data = value::parse_limited(
                    ctx,
                    inspect,
                    raw,
                    &mut cost,
                    0,
                    FRAME_BYTES,
                    FRAME_NODES,
                )?;
                Some((delay, data, cost))
            }
            _ => return Err(invalid(ctx)),
        };
        let mut state = self.0.borrow_mut();
        state
            .transfer
            .add(cost.bytes, cost.nodes)
            .map_err(|_| invalid(ctx))?;
        if state.failure.is_some() {
            return Err(invalid(ctx));
        }
        // A cancel or cancel/start during the callback supersedes its result.
        if !state
            .store
            .entries
            .get(key)
            .is_some_and(|v| Arc::ptr_eq(v, old))
        {
            return Ok(());
        }
        if let Some((delay, value, cost)) = next {
            let task = Task {
                due: due(state.tick, delay).map_err(|_| invalid(ctx))?,
                value,
                cost,
                routine: old.routine.clone(),
                entity: old.entity,
            };
            state
                .store
                .insert(key.clone(), Arc::new(task))
                .map_err(|_| invalid(ctx))?;
        } else {
            state.store.remove(key);
        }
        Ok(())
    }
    pub fn expose<'js>(
        &self,
        ctx: &Ctx<'js>,
        scope: Scope,
        names: Arc<[String]>,
        allowed: Arc<[StableId]>,
        wrap: &Function<'js>,
    ) -> rquickjs::Result<Object<'js>> {
        let object = Object::new_proto(ctx.clone(), None)?;
        for name in ["start", "cancel"] {
            let shared = Rc::clone(&self.0);
            let names = Arc::clone(&names);
            let allowed = Arc::clone(&allowed);
            let native = Function::new(
                ctx.clone(),
                move |ctx: Ctx<'js>,
                      inspect: Function<'js>,
                      _freeze: Function<'js>,
                      args: Rest<Value<'js>>| {
                    {
                        let mut s = shared.borrow_mut();
                        if s.active != Some(scope) {
                            return Err(Exception::throw_type(
                                &ctx,
                                "coroutine capability expired or belongs to another source",
                            ));
                        }
                        if s.failure.is_some() {
                            return Err(invalid(&ctx));
                        }
                        if s.calls == MAX_CALLS {
                            s.failure = Some(ScriptError::InputLimit);
                            return Err(invalid(&ctx));
                        }
                        s.calls += 1;
                    }
                    let result = (|| {
                        let (id, routine, raw, owner) = match (name, args.0.as_slice()) {
                            ("cancel", [id]) => (id, None, None, None),
                            ("start", [id, routine, value]) => {
                                (id, Some(routine), Some(value), None)
                            }
                            ("start", [id, routine, value, owner]) => {
                                (id, Some(routine), Some(value), Some(owner))
                            }
                            _ => return Err(invalid(&ctx)),
                        };
                        let key = Key {
                            scope,
                            id: read_id(&ctx, id)?,
                        };
                        let task = if let (Some(routine), Some(raw)) = (routine, raw) {
                            let routine = read_id(&ctx, routine)?;
                            if !names.contains(&routine) {
                                return Err(invalid(&ctx));
                            }
                            let entity = match owner {
                                Some(owner) if !owner.is_undefined() => {
                                    let id = crate::commands::id(&ctx, owner)?;
                                    if allowed.binary_search(&id).is_err() {
                                        return Err(invalid(&ctx));
                                    }
                                    Some(id)
                                }
                                _ => None,
                            };
                            let mut cost = Cost::default();
                            let value = value::parse_limited(
                                &ctx,
                                &inspect,
                                raw.clone(),
                                &mut cost,
                                0,
                                FRAME_BYTES,
                                FRAME_NODES,
                            )?;
                            Some((routine, entity, value, cost))
                        } else {
                            None
                        };
                        let mut s = shared.borrow_mut();
                        if s.failure.is_some() || s.active != Some(scope) {
                            return Err(invalid(&ctx));
                        }
                        if let Some((routine, entity, value, cost)) = task {
                            s.transfer
                                .add(cost.bytes, cost.nodes)
                                .map_err(|_| invalid(&ctx))?;
                            if s.store.entries.contains_key(&key) {
                                return Err(invalid(&ctx));
                            }
                            let due = due(s.tick, 1).map_err(|_| invalid(&ctx))?;
                            s.store
                                .insert(
                                    key,
                                    Arc::new(Task {
                                        routine,
                                        entity,
                                        value,
                                        cost,
                                        due,
                                    }),
                                )
                                .map_err(|_| invalid(&ctx))?;
                        } else {
                            s.store.remove(&key);
                        }
                        Ok(())
                    })();
                    if result.is_err() {
                        shared
                            .borrow_mut()
                            .failure
                            .get_or_insert(ScriptError::CoroutineInput);
                    }
                    result
                },
            )?;
            let function: Function = wrap.call((native,))?;
            object.prop(name, Property::from(function).enumerable())?;
        }
        Ok(object)
    }
}
impl Drop for Tick {
    fn drop(&mut self) {
        let mut state = self.0.borrow_mut();
        state.active = None;
        state.store = Store::default();
    }
}
fn invalid(ctx: &Ctx<'_>) -> rquickjs::Error {
    Exception::throw_type(ctx, "invalid or excessive native coroutine operation")
}
fn read_id(ctx: &Ctx<'_>, raw: &Value<'_>) -> rquickjs::Result<String> {
    let text = value::text(ctx, raw, 64)?;
    if valid_id(&text) {
        Ok(text)
    } else {
        Err(invalid(ctx))
    }
}
fn ticks(n: f64) -> Option<u32> {
    (n.is_finite() && n >= 1. && n <= u32::MAX as f64 && n.fract() == 0.).then_some(n as u32)
}

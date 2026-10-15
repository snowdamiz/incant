//! Explicit native persistent data. Only accepted script transactions publish
//! changed keys. This is not a JavaScript heap/closure snapshot.
pub(crate) mod value;
use crate::{ScriptError, ScriptValue, schedule::Scope};
use incant_runtime::StableId;
use rquickjs::{Ctx, Exception, Function, Object, Value, function::Rest};
use std::{cell::RefCell, collections::BTreeMap, rc::Rc, sync::Arc};
use value::Cost;

const MAX_CALLS: usize = 256;
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Key {
    pub scope: Scope,
    pub entity: Option<StableId>,
}
#[derive(Clone)]
pub(crate) struct Entry {
    pub value: ScriptValue,
    pub cost: Cost,
}
#[derive(Default)]
pub(crate) struct Store {
    pub entries: BTreeMap<Key, Arc<Entry>>,
    pub cost: Cost,
}
pub(crate) struct Changes {
    retired: Vec<Key>,
    entries: BTreeMap<Key, Arc<Entry>>,
    cost: Cost,
}
impl Store {
    pub fn apply(&mut self, changes: Changes) {
        for key in changes.retired {
            self.entries.remove(&key);
        }
        self.entries.extend(changes.entries);
        self.cost = changes.cost;
    }
}
struct State {
    store: Option<Rc<RefCell<Store>>>,
    active: Option<Scope>,
    failure: Option<ScriptError>,
    calls: usize,
    transfer: Cost,
    changes: Changes,
}
pub(crate) struct TickState(Rc<RefCell<State>>);
impl TickState {
    pub fn healthy(&self) -> bool {
        self.0.borrow().failure.is_none()
    }

    pub fn new(store: Rc<RefCell<Store>>, retired: Vec<Key>) -> Self {
        let mut cost = store.borrow().cost;
        for key in &retired {
            if let Some(entry) = store.borrow().entries.get(key) {
                cost.bytes -= entry.cost.bytes;
                cost.nodes -= entry.cost.nodes;
            }
        }
        Self(Rc::new(RefCell::new(State {
            store: Some(store),
            active: None,
            failure: None,
            calls: 0,
            transfer: Cost::default(),
            changes: Changes {
                retired,
                entries: BTreeMap::new(),
                cost,
            },
        })))
    }
    pub fn activate(&self, scope: Scope) {
        self.0.borrow_mut().active = Some(scope);
    }
    pub fn revoke(&self) {
        self.0.borrow_mut().active = None;
    }
    pub fn take(&self) -> Result<Changes, ScriptError> {
        self.revoke();
        let mut state = self.0.borrow_mut();
        if let Some(error) = state.failure.take() {
            return Err(error);
        }
        let cost = state.changes.cost;
        Ok(Changes {
            retired: std::mem::take(&mut state.changes.retired),
            entries: std::mem::take(&mut state.changes.entries),
            cost,
        })
    }
    pub fn expose<'js>(
        &self,
        ctx: &Ctx<'js>,
        scope: Scope,
        allowed: Arc<[StableId]>,
        opted: bool,
        wrap: Function<'js>,
    ) -> rquickjs::Result<Object<'js>> {
        let object = Object::new_proto(ctx.clone(), None)?;
        for name in ["get", "set"] {
            let state = Rc::clone(&self.0);
            let allowed = Arc::clone(&allowed);
            let function = Function::new(
                ctx.clone(),
                move |ctx: Ctx<'js>,
                      inspect: Function<'js>,
                      freeze: Function<'js>,
                      args: Rest<Value<'js>>| {
                    let rejection = {
                        let mut state = state.borrow_mut();
                        if state.active != Some(scope) || state.store.is_none() {
                            Some("persistent state belongs to its active source update only")
                        } else if state.failure.is_some() {
                            Some("persistent state transaction failed")
                        } else if !opted || state.calls == MAX_CALLS {
                            state.failure = Some(ScriptError::PersistenceInput);
                            Some("persistent state requires opt-in and bounded calls")
                        } else {
                            state.calls += 1;
                            None
                        }
                    };
                    if let Some(message) = rejection {
                        return Err(Exception::throw_type(&ctx, message));
                    }
                    let result = (|| {
                        let (raw, target) = match (name, args.0.as_slice()) {
                            ("get", []) => (None, None),
                            ("get", [entity]) => (None, Some(entity)),
                            ("set", [value]) => (Some(value), None),
                            ("set", [value, entity]) => (Some(value), Some(entity)),
                            _ => {
                                return Err(Exception::throw_type(
                                    &ctx,
                                    "state.get(entity?) / state.set(value, entity?) expected",
                                ));
                            }
                        };
                        let entity = match target {
                            Some(value) if !value.is_undefined() => {
                                let id = crate::commands::id(&ctx, value)?;
                                if allowed.binary_search(&id).is_err() {
                                    return Err(Exception::throw_type(
                                        &ctx,
                                        "persistent state requires this source's live instance",
                                    ));
                                }
                                Some(id)
                            }
                            _ => None,
                        };
                        let key = Key { scope, entity };
                        if let Some(raw) = raw {
                            // No Rust borrow spans guest traps or typed conversion.
                            let mut cost = Cost::default();
                            let value = value::parse(&ctx, &inspect, raw.clone(), &mut cost, 0)?;
                            cost.add(34, 0).map_err(|_| {
                                Exception::throw_type(&ctx, "persistent state size limit")
                            })?;
                            let entry = Arc::new(Entry { value, cost });
                            let mut state = state.borrow_mut();
                            if state.failure.is_some() || state.active != Some(scope) {
                                return Err(Exception::throw_type(
                                    &ctx,
                                    "persistent state transaction failed",
                                ));
                            }
                            state.transfer.add(cost.bytes, cost.nodes).map_err(|_| {
                                Exception::throw_type(&ctx, "persistent state transfer limit")
                            })?;
                            let old = state.changes.entries.get(&key).cloned().or_else(|| {
                                state
                                    .store
                                    .as_ref()
                                    .unwrap()
                                    .borrow()
                                    .entries
                                    .get(&key)
                                    .cloned()
                            });
                            let mut total = state.changes.cost;
                            if let Some(old) = old {
                                total.bytes -= old.cost.bytes;
                                total.nodes -= old.cost.nodes;
                            }
                            total.add(cost.bytes, cost.nodes).map_err(|_| {
                                Exception::throw_type(&ctx, "persistent state capacity limit")
                            })?;
                            state.changes.entries.insert(key, entry);
                            state.changes.cost = total;
                            Ok(Value::new_undefined(ctx.clone()))
                        } else {
                            let entry = {
                                let mut state = state.borrow_mut();
                                let entry =
                                    state.changes.entries.get(&key).cloned().or_else(|| {
                                        state
                                            .store
                                            .as_ref()
                                            .unwrap()
                                            .borrow()
                                            .entries
                                            .get(&key)
                                            .cloned()
                                    });
                                let cost = entry
                                    .as_ref()
                                    .map_or(Cost { bytes: 1, nodes: 1 }, |e| e.cost);
                                state.transfer.add(cost.bytes, cost.nodes).map_err(|_| {
                                    Exception::throw_type(&ctx, "persistent state transfer limit")
                                })?;
                                entry
                            };
                            match entry {
                                Some(entry) => {
                                    crate::bindings::expose_value(&ctx, &freeze, &entry.value)
                                }
                                None => Ok(Value::new_null(ctx.clone())),
                            }
                        }
                    })();
                    if result.is_err() {
                        state
                            .borrow_mut()
                            .failure
                            .get_or_insert(ScriptError::PersistenceInput);
                    }
                    result
                },
            )?;
            // Keep JS references in traced JS closures, never inside the Rust
            // callback environment. Otherwise a retained guest API forms an
            // invisible context/root cycle and prevents QuickJS destruction.
            let function: Function = wrap.call((function,))?;
            object.set(name, function)?;
        }
        Ok(object)
    }
}
impl Drop for TickState {
    fn drop(&mut self) {
        let mut state = self.0.borrow_mut();
        state.active = None;
        state.store = None;
        state.changes.entries.clear();
    }
}

#[cfg(all(test, feature = "compiler"))]
mod tests;

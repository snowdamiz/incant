//! Revocable, data-only timer commands. No callbacks, coercion, payload JSON or
//! host clock access. Retained functions cannot affect a later update.
use crate::{
    ScriptError,
    schedule::{Action, Event, MAX_TIMER_ACTIONS, Schedule, Scope, ScopedAction},
};
use rquickjs::{Array, Ctx, Exception, Function, Object, Value, function::Rest, object::Property};
use std::{cell::RefCell, rc::Rc};

#[derive(Default)]
struct State {
    active: Option<Scope>,
    calls: usize,
    failure: Option<ScriptError>,
    actions: Vec<ScopedAction>,
}
#[derive(Default)]
pub(crate) struct TickTimers(Rc<RefCell<State>>);
impl TickTimers {
    pub fn activate(&self, scope: Scope) {
        self.0.borrow_mut().active = Some(scope);
    }
    pub fn revoke(&self) {
        self.0.borrow_mut().active = None;
    }
    pub fn take(&self) -> Result<Vec<ScopedAction>, ScriptError> {
        self.revoke();
        let mut state = self.0.borrow_mut();
        if let Some(error) = state.failure.take() {
            return Err(error);
        }
        Ok(std::mem::take(&mut state.actions))
    }
    pub fn expose<'js>(
        &self,
        ctx: &Ctx<'js>,
        scope: Scope,
        events: &[Event],
    ) -> rquickjs::Result<Object<'js>> {
        let object = Object::new_proto(ctx.clone(), None)?;
        let array = Array::new(ctx.clone())?;
        for (index, event) in events
            .iter()
            .filter(|event| event.scope == scope)
            .enumerate()
        {
            let value = Object::new_proto(ctx.clone(), None)?;
            value.prop("id", Property::from(event.id.clone()).enumerable())?;
            value.prop(
                "scheduled_tick",
                Property::from(event.scheduled_tick as f64).enumerable(),
            )?;
            array
                .as_object()
                .prop(index as u32, Property::from(value).enumerable())?;
        }
        object.set("events", array)?;
        for name in ["set", "cancel"] {
            let state = Rc::clone(&self.0);
            let function =
                Function::new(ctx.clone(), move |ctx: Ctx<'js>, args: Rest<Value<'js>>| {
                    let rejected = {
                        let mut state = state.borrow_mut();
                        if state.active != Some(scope) {
                            Some("timer functions belong to their active update only")
                        } else if state.failure.is_some() {
                            Some("timer capability already failed")
                        } else if state.calls == MAX_TIMER_ACTIONS {
                            state.failure = Some(ScriptError::InputLimit);
                            Some("timer command limit exceeded")
                        } else {
                            state.calls += 1;
                            None
                        }
                    };
                    if let Some(message) = rejected {
                        return Err(Exception::throw_type(&ctx, message));
                    }
                    // Only strict primitives are accepted. No valueOf/toString/getter runs.
                    let action = match parse(&ctx, name, &args.0) {
                        Ok(action) => action,
                        Err(error) => {
                            state
                                .borrow_mut()
                                .failure
                                .get_or_insert(ScriptError::TimerInput);
                            return Err(error);
                        }
                    };
                    state
                        .borrow_mut()
                        .actions
                        .push(ScopedAction { scope, action });
                    Ok(())
                })?;
            object.set(name, function)?;
        }
        Ok(object)
    }
}
impl Drop for TickTimers {
    fn drop(&mut self) {
        let mut state = self.0.borrow_mut();
        state.active = None;
        state.actions = Vec::new();
    }
}
pub(crate) fn clock<'js>(ctx: &Ctx<'js>, schedule: &Schedule) -> rquickjs::Result<Object<'js>> {
    let clock = Object::new_proto(ctx.clone(), None)?;
    clock.prop("tick", Property::from(schedule.tick as f64).enumerable())?;
    clock.prop(
        "elapsed_seconds",
        Property::from(schedule.elapsed_seconds).enumerable(),
    )?;
    Ok(clock)
}
fn parse<'js>(ctx: &Ctx<'js>, name: &str, args: &[Value<'js>]) -> rquickjs::Result<Action> {
    let invalid = || Exception::throw_type(ctx, "expected timer ID and positive u32 tick delays");
    let id = |value: &Value<'js>| -> rquickjs::Result<String> {
        // UTF-8 conversion stays in the VM's bounded allocation until the byte
        // cap is checked. Do not copy a large guest string into Rust first.
        let string = value.as_string().ok_or_else(invalid)?;
        let bytes = string.clone().to_cstring()?;
        if bytes.is_empty() || bytes.len() > 64 {
            return Err(invalid());
        }
        // QuickJS permits lone UTF-16 surrogates. CString::as_str assumes UTF-8;
        // use String::to_string's checked conversion only after the byte cap.
        drop(bytes);
        let value = string.to_string().map_err(|_| invalid())?;
        if !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_.:-".contains(&b))
        {
            return Err(invalid());
        }
        Ok(value)
    };
    let ticks = |value: &Value<'js>| -> rquickjs::Result<u32> {
        let value = value.as_number().ok_or_else(invalid)?;
        if !value.is_finite() || value < 1. || value > u32::MAX as f64 || value.fract() != 0. {
            return Err(invalid());
        }
        Ok(value as u32)
    };
    match (name, args) {
        ("cancel", [value]) => Ok(Action::Cancel(id(value)?)),
        ("set", [value, delay]) => Ok(Action::Set {
            id: id(value)?,
            delay: ticks(delay)?,
            interval: None,
        }),
        ("set", [value, delay, interval]) => Ok(Action::Set {
            id: id(value)?,
            delay: ticks(delay)?,
            interval: if interval.is_undefined() {
                None
            } else {
                Some(ticks(interval)?)
            },
        }),
        _ => Err(invalid()),
    }
}

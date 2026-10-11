//! Tick-local capabilities. Functions retain only bounded queue state, never
//! World/column pointers, and an old tick's state is never activated again.
use crate::ScriptError;
use incant_runtime::{
    CookedEntity, FrameCommands, MAX_FRAME_COMMANDS, StableId, StructuralCommand,
};
use incant_types::{Transform, Velocity};
use rquickjs::{Ctx, Exception, Function, Object, TypedArray, Value, function::Rest};
use std::{cell::RefCell, rc::Rc};

#[derive(Default)]
struct State {
    active: bool,
    failure: Option<ScriptError>,
    queue: FrameCommands,
}

#[derive(Default)]
pub(crate) struct TickCommands(Rc<RefCell<State>>);

impl TickCommands {
    pub fn expose<'js>(&self, ctx: &Ctx<'js>) -> rquickjs::Result<Object<'js>> {
        let object = Object::new_proto(ctx.clone(), None)?;
        for name in ["spawn", "despawn", "setParent", "setVelocity"] {
            let state = Rc::clone(&self.0);
            let function =
                Function::new(ctx.clone(), move |ctx: Ctx<'js>, args: Rest<Value<'js>>| {
                    // Drop all Rust borrows before parsing: array getters can
                    // call these functions recursively or throw/allocate.
                    let rejection = {
                        let mut state = state.borrow_mut();
                        if !state.active {
                            Some("structural commands belong to their active tick only")
                        } else if state.failure.is_some() {
                            Some("structural command buffer already failed")
                        } else if state.queue.len() == MAX_FRAME_COMMANDS {
                            state.failure = Some(ScriptError::InputLimit);
                            Some("structural command limit exceeded")
                        } else {
                            None
                        }
                    };
                    if let Some(message) = rejection {
                        return Err(Exception::throw_type(&ctx, message));
                    }
                    let command = match parse(&ctx, name, &args.0) {
                        Ok(command) => command,
                        Err(error) => {
                            state
                                .borrow_mut()
                                .failure
                                .get_or_insert(ScriptError::CommandInput);
                            return Err(error);
                        }
                    };
                    let rejected = {
                        let mut state = state.borrow_mut();
                        if !state.active || state.failure.is_some() {
                            true
                        } else if let Err(error) = state.queue.push(command) {
                            state.failure = Some(ScriptError::Structural(error));
                            true
                        } else {
                            false
                        }
                    };
                    if rejected {
                        return Err(Exception::throw_type(
                            &ctx,
                            "structural command buffer failed",
                        ));
                    }
                    Ok(())
                })?;
            // Own properties on a null prototype cannot invoke guest setters.
            object.set(name, function)?;
        }
        Ok(object)
    }

    pub fn activate(&self) {
        self.0.borrow_mut().active = true;
    }
    pub fn revoke(&self) {
        self.0.borrow_mut().active = false;
    }
    pub fn take(&self) -> Result<FrameCommands, ScriptError> {
        self.revoke();
        let mut state = self.0.borrow_mut();
        if let Some(error) = state.failure.take() {
            return Err(error);
        }
        Ok(std::mem::take(&mut state.queue))
    }
}

impl Drop for TickCommands {
    fn drop(&mut self) {
        let mut state = self.0.borrow_mut();
        state.active = false;
        // Retained JS functions cannot retain the queue's native allocation.
        state.queue = FrameCommands::default();
    }
}

fn parse<'js>(
    ctx: &Ctx<'js>,
    name: &str,
    args: &[Value<'js>],
) -> rquickjs::Result<StructuralCommand> {
    Ok(match (name, args) {
        ("despawn", [entity]) => StructuralCommand::Despawn(id(ctx, entity)?),
        ("setParent", [entity, parent]) => StructuralCommand::SetParent {
            entity: id(ctx, entity)?,
            parent: optional_id(ctx, parent)?,
        },
        ("setVelocity", [entity, velocity]) => StructuralCommand::SetVelocity {
            entity: id(ctx, entity)?,
            velocity: motion(ctx, velocity)?,
        },
        ("spawn", [entity, transform, parent, velocity]) => {
            let id = id(ctx, entity)?;
            let t = numbers::<10>(ctx, transform)?;
            StructuralCommand::Spawn(CookedEntity {
                id,
                parent: optional_id(ctx, parent)?,
                transform: Transform {
                    translation: [t[0], t[1], t[2]],
                    rotation: [t[3], t[4], t[5], t[6]],
                    scale: [t[7], t[8], t[9]],
                },
                velocity: motion(ctx, velocity)?,
            })
        }
        _ => {
            return Err(Exception::throw_type(
                ctx,
                "unsupported structural command arguments",
            ));
        }
    })
}

pub(crate) fn id(ctx: &Ctx<'_>, value: &Value<'_>) -> rquickjs::Result<StableId> {
    let invalid = || Exception::throw_type(ctx, "expected a canonical uppercase ULID string");
    let text = value.as_string().ok_or_else(invalid)?.to_string()?;
    if text.len() != 26 {
        return Err(invalid());
    }
    let id = ulid::Ulid::from_string(&text).map_err(|_| invalid())?;
    if id.to_string() != text {
        return Err(invalid());
    }
    Ok(StableId(id.to_bytes()))
}

pub(crate) fn optional_id(ctx: &Ctx<'_>, value: &Value<'_>) -> rquickjs::Result<Option<StableId>> {
    if value.is_null() {
        Ok(None)
    } else {
        id(ctx, value).map(Some)
    }
}

fn motion<'js>(ctx: &Ctx<'js>, value: &Value<'js>) -> rquickjs::Result<Option<Velocity>> {
    if value.is_null() {
        Ok(None)
    } else {
        Ok(Some(Velocity {
            linear: numbers(ctx, value)?,
        }))
    }
}

pub(crate) fn numbers<'js, const N: usize>(
    ctx: &Ctx<'js>,
    value: &Value<'js>,
) -> rquickjs::Result<[f64; N]> {
    if !value.is_array() && TypedArray::<f64>::from_value(value.clone()).is_err() {
        return Err(Exception::throw_type(
            ctx,
            "expected an Array or Float64Array",
        ));
    }
    let object = value.as_object().expect("array object");
    if object.get::<_, Value>("length")?.as_number() != Some(N as f64) {
        return Err(Exception::throw_type(
            ctx,
            "wrong structural numeric field count",
        ));
    }
    let mut result = [0.; N];
    for (index, number) in result.iter_mut().enumerate() {
        // Read each scalar through QuickJS. Never form a Rust slice into a JS
        // array (which may itself be an active ECS view), or borrow queue state
        // across a guest getter. Structural value validation happens at apply.
        *number = object
            .get::<_, Value>(index as u32)?
            .as_number()
            .ok_or_else(|| Exception::throw_type(ctx, "expected numeric structural fields"))?;
    }
    Ok(result)
}

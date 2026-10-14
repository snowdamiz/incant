//! Tick-local haptic bindings. Parse outside RefCell borrows because property
//! access may execute guest code. No backend handles or actuation occur here.
use super::{ScriptError, State};
use incant_input::haptics::{HapticEffect, HapticRequest, HapticTarget};
use rquickjs::{Array, Ctx, Exception, Function, Object, Value, function::Rest};
use std::{cell::RefCell, rc::Rc};

pub(super) fn expose<'js>(
    ctx: &Ctx<'js>,
    object: &Object<'js>,
    state: &Rc<RefCell<State>>,
) -> rquickjs::Result<()> {
    for name in ["hapticCapabilities", "playHaptic", "stopHaptic"] {
        let state = Rc::clone(state);
        object.set(
            name,
            Function::new(ctx.clone(), move |ctx: Ctx<'js>, args: Rest<Value<'js>>| {
                {
                    let state = state.borrow();
                    if !state.active || state.failure.is_some() {
                        return Err(Exception::throw_type(
                            &ctx,
                            "haptics belong to their active healthy tick only",
                        ));
                    }
                }
                let result = if name == "hapticCapabilities" {
                    if !args.0.is_empty() {
                        Err(Exception::throw_type(
                            &ctx,
                            "hapticCapabilities takes no arguments",
                        ))
                    } else {
                        let current = state.borrow().haptic_capabilities;
                        capabilities(&ctx, current)
                    }
                } else {
                    parse(&ctx, name, &args.0).and_then(|request| {
                        let mut state = state.borrow_mut();
                        if !state.active || state.failure.is_some() {
                            return Err(Exception::throw_type(
                                &ctx,
                                "haptic command buffer failed",
                            ));
                        }
                        if state.queue.len() + state.restarts + state.haptics.len()
                            >= incant_runtime::MAX_FRAME_COMMANDS
                        {
                            state.failure = Some(ScriptError::InputLimit);
                            return Err(Exception::throw_type(
                                &ctx,
                                "shared command limit exceeded",
                            ));
                        }
                        let capabilities = state.haptic_capabilities;
                        if let Err(error) = state.haptics.push(request, &capabilities) {
                            state.failure = Some(ScriptError::Haptics(error));
                            return Err(Exception::throw_type(
                                &ctx,
                                "invalid, unavailable or excessive haptic request",
                            ));
                        }
                        Ok(Value::new_undefined(ctx.clone()))
                    })
                };
                if result.is_err() {
                    state
                        .borrow_mut()
                        .failure
                        .get_or_insert(ScriptError::CommandInput);
                }
                result
            })?,
        )?;
    }
    Ok(())
}
fn capabilities<'js>(
    ctx: &Ctx<'js>,
    capabilities: incant_input::haptics::HapticCapabilities,
) -> rquickjs::Result<Value<'js>> {
    let array = Array::new(ctx.clone())?;
    for (index, target) in capabilities.targets().enumerate() {
        let object = Object::new_proto(ctx.clone(), None)?;
        match target {
            HapticTarget::Gamepad { id, generation } => {
                object.set("kind", "gamepad")?;
                object.set("id", id)?;
                object.set("generation", generation)?;
            }
            HapticTarget::Device { generation } => {
                object.set("kind", "device")?;
                object.set("generation", generation)?;
            }
        }
        // Own data properties avoid inherited setters.
        array
            .as_object()
            .prop(index as u32, rquickjs::object::Property::from(object))?;
    }
    Ok(array.into_value())
}
fn number<'js>(ctx: &Ctx<'js>, value: Value<'js>) -> rquickjs::Result<f64> {
    value
        .as_number()
        .filter(|n| n.is_finite())
        .ok_or_else(|| Exception::throw_type(ctx, "expected finite number"))
}
fn integer<'js>(ctx: &Ctx<'js>, value: Value<'js>) -> rquickjs::Result<u32> {
    let n = number(ctx, value)?;
    if n.fract() != 0. || !(0. ..=f64::from(u32::MAX)).contains(&n) {
        return Err(Exception::throw_type(ctx, "expected u32 integer"));
    }
    Ok(n as u32)
}
fn amplitude<'js>(ctx: &Ctx<'js>, value: Value<'js>) -> rquickjs::Result<f32> {
    let n = number(ctx, value)?;
    if !(0. ..=1.).contains(&n) {
        return Err(Exception::throw_type(ctx, "expected normalized amplitude"));
    }
    Ok(n as f32)
}
fn parse<'js>(ctx: &Ctx<'js>, name: &str, args: &[Value<'js>]) -> rquickjs::Result<HapticRequest> {
    if args.len() != if name == "stopHaptic" { 1 } else { 2 } {
        return Err(Exception::throw_type(ctx, "invalid haptic arguments"));
    }
    let target = args[0]
        .as_object()
        .ok_or_else(|| Exception::throw_type(ctx, "expected haptic target"))?;
    let kind: String = target.get("kind")?;
    let generation = integer(ctx, target.get("generation")?)?;
    let target = match kind.as_str() {
        "gamepad" => HapticTarget::Gamepad {
            id: integer(ctx, target.get("id")?)?,
            generation,
        },
        "device" => HapticTarget::Device { generation },
        _ => return Err(Exception::throw_type(ctx, "unknown haptic target")),
    };
    let effect = if name == "stopHaptic" {
        HapticEffect::Stop
    } else {
        let effect = args[1]
            .as_object()
            .ok_or_else(|| Exception::throw_type(ctx, "expected haptic effect"))?;
        let kind: String = effect.get("kind")?;
        match kind.as_str() {
            "rumble" => HapticEffect::Rumble {
                strong: amplitude(ctx, effect.get("strong")?)?,
                weak: amplitude(ctx, effect.get("weak")?)?,
                duration_ms: integer(ctx, effect.get("durationMs")?)?,
            },
            "impact" => HapticEffect::Impact {
                intensity: amplitude(ctx, effect.get("intensity")?)?,
            },
            _ => return Err(Exception::throw_type(ctx, "unknown haptic effect")),
        }
    };
    Ok(HapticRequest { target, effect })
}

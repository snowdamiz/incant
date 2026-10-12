//! Strict primitive controls share the tick-local structural queue and refusal.
use incant_runtime::{AnimationControl, AnimationSample, StructuralCommand};
use incant_types::{MAX_ANIMATION_CLIPS, MAX_ANIMATION_SPEED};
use rquickjs::{Ctx, Exception, Value};

pub(crate) fn parse<'js>(
    ctx: &Ctx<'js>,
    args: &[Value<'js>],
) -> rquickjs::Result<StructuralCommand> {
    let invalid = || Exception::throw_type(ctx, "unsupported animation control arguments");
    let [entity, action, rest @ ..] = args else {
        return Err(invalid());
    };
    let action = action.as_string().ok_or_else(invalid)?;
    // QuickJS owns this conversion under the VM memory quota. Check its length
    // before copying into a Rust string; no arbitrary native text allocation.
    let bytes = action.clone().to_cstring()?;
    if bytes.is_empty() || bytes.len() > 7 {
        return Err(invalid());
    }
    drop(bytes);
    let action = action.to_string().map_err(|_| invalid())?;
    let seconds = |value: &Value<'js>| {
        value
            .as_number()
            .filter(|n| n.is_finite() && *n >= 0.)
            .ok_or_else(invalid)
    };
    let entity_text = entity.as_string().ok_or_else(invalid)?;
    let entity_bytes = entity_text.clone().to_cstring()?;
    if entity_bytes.len() != 26 {
        return Err(invalid());
    }
    drop(entity_bytes);
    let control = match (action.as_str(), rest) {
        ("select", [clip, time]) => {
            let clip = if clip.is_null() {
                None
            } else {
                Some(
                    clip.as_number()
                        .filter(|n| {
                            n.is_finite()
                                && n.fract() == 0.
                                && *n >= 0.
                                && *n < MAX_ANIMATION_CLIPS as f64
                        })
                        .ok_or_else(invalid)? as u16,
                )
            };
            AnimationControl::Select(AnimationSample {
                clip,
                seconds: seconds(time)?,
            })
        }
        ("seek", [time]) => AnimationControl::Seek(seconds(time)?),
        ("playing", [playing]) => {
            AnimationControl::SetPlaying(playing.as_bool().ok_or_else(invalid)?)
        }
        ("looping", [looping]) => {
            AnimationControl::SetLooping(looping.as_bool().ok_or_else(invalid)?)
        }
        ("speed", [speed]) => AnimationControl::SetSpeed(
            speed
                .as_number()
                .filter(|n| n.is_finite() && n.abs() <= MAX_ANIMATION_SPEED)
                .ok_or_else(invalid)?,
        ),
        _ => return Err(invalid()),
    };
    Ok(StructuralCommand::ControlAnimation {
        entity: crate::commands::id(ctx, entity)?,
        control,
    })
}

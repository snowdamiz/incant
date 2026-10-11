//! Owned planar query conversion; guest getters run before borrowing the solver.
use crate::{commands, query_values::owned_vector};
use incant_runtime::{RayHit2D, RayQuery2D, TriggerEvent2D};
use rquickjs::{Array, Ctx, Exception, Object, Value, object::Property};

pub(super) fn ray<'js>(ctx: &Ctx<'js>, args: &[Value<'js>]) -> rquickjs::Result<RayQuery2D> {
    let [value] = args else {
        return Err(Exception::throw_type(ctx, "raycast2D expects one query"));
    };
    let object = value
        .as_object()
        .ok_or_else(|| Exception::throw_type(ctx, "expected a planar ray query object"))?;
    if !object.get::<_, Value>("scene_id")?.is_undefined() {
        return Err(Exception::throw_type(
            ctx,
            "native raycasts use the active scene only",
        ));
    }
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
    Ok(RayQuery2D {
        origin: commands::numbers(ctx, &object.get::<_, Value>("origin")?)?,
        direction: commands::numbers(ctx, &object.get::<_, Value>("direction")?)?,
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

pub(super) fn hit<'js>(ctx: &Ctx<'js>, hit: Option<RayHit2D>) -> rquickjs::Result<Value<'js>> {
    let Some(hit) = hit else {
        return Ok(Value::new_null(ctx.clone()));
    };
    #[cfg(test)]
    if super::query_values::FAIL_HIT_ALLOCATION.with(|flag| flag.replace(false)) {
        return Err(rquickjs::Error::Allocation);
    }
    let result = Object::new_proto(ctx.clone(), None)?;
    result.set(
        "entity_id",
        ulid::Ulid::from_bytes(hit.entity.0).to_string(),
    )?;
    result.set("distance", hit.distance)?;
    result.set("point", owned_vector(ctx, hit.point)?)?;
    result.set("normal", owned_vector(ctx, hit.normal)?)?;
    Ok(result.into_value())
}

pub(super) fn events<'js>(
    ctx: &Ctx<'js>,
    events: Vec<TriggerEvent2D>,
) -> rquickjs::Result<Value<'js>> {
    let result = Array::new(ctx.clone())?;
    for (i, event) in events.into_iter().enumerate() {
        let item = Object::new_proto(ctx.clone(), None)?;
        item.set("first", ulid::Ulid::from_bytes(event.first.0).to_string())?;
        item.set("second", ulid::Ulid::from_bytes(event.second.0).to_string())?;
        item.set("entered", event.entered)?;
        result.as_object().prop(
            i as u32,
            Property::from(item).enumerable().writable().configurable(),
        )?;
    }
    Ok(result.into_value())
}

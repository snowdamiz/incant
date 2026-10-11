//! Native physics argument/result conversion. No Rust loan spans guest getters.
use crate::commands;
use incant_runtime::{
    CharacterMovement, CharacterOptions, CharacterQuery, CharacterStep, RayHit, RayQuery, StableId,
    TriggerEvent,
};
use rquickjs::{Array, Ctx, Exception, Object, Value, object::Property};

#[derive(Clone, Copy)]
pub(super) enum Kind {
    Ray,
    Character,
    Triggers,
}
pub(super) enum Request {
    Ray(RayQuery<StableId>),
    Character(CharacterQuery<StableId>),
    Triggers,
}
pub(super) enum ResultData {
    Ray(Option<RayHit<StableId>>),
    Character(CharacterMovement<StableId>),
    Triggers(Vec<TriggerEvent<StableId>>),
}
pub(super) fn parse<'js>(
    kind: Kind,
    ctx: &Ctx<'js>,
    args: &[Value<'js>],
    scene: StableId,
) -> rquickjs::Result<Request> {
    match kind {
        Kind::Ray => ray(ctx, args, scene).map(Request::Ray),
        Kind::Character => character(ctx, args, scene).map(Request::Character),
        Kind::Triggers if args.is_empty() => Ok(Request::Triggers),
        Kind::Triggers => Err(Exception::throw_type(
            ctx,
            "triggerEvents expects no arguments",
        )),
    }
}
fn number(ctx: &Ctx<'_>, object: &Object<'_>, name: &str) -> rquickjs::Result<f64> {
    object
        .get::<_, Value>(name)?
        .as_number()
        .ok_or_else(|| Exception::throw_type(ctx, "expected numeric character field"))
}
fn boolean(ctx: &Ctx<'_>, object: &Object<'_>, name: &str) -> rquickjs::Result<bool> {
    object
        .get::<_, Value>(name)?
        .as_bool()
        .ok_or_else(|| Exception::throw_type(ctx, "expected boolean character field"))
}
fn character<'js>(
    ctx: &Ctx<'js>,
    args: &[Value<'js>],
    scene: StableId,
) -> rquickjs::Result<CharacterQuery<StableId>> {
    let [value] = args else {
        return Err(Exception::throw_type(
            ctx,
            "computeCharacterMotion expects one query",
        ));
    };
    let object = value
        .as_object()
        .ok_or_else(|| Exception::throw_type(ctx, "expected a character query object"))?;
    if !object.get::<_, Value>("scene_id")?.is_undefined() {
        return Err(Exception::throw_type(
            ctx,
            "native character queries use the active scene only",
        ));
    }
    let entity_id = commands::id(ctx, &object.get::<_, Value>("entity_id")?)?;
    let translation = commands::numbers(ctx, &object.get::<_, Value>("translation")?)?;
    let value = object.get::<_, Value>("options")?;
    let options = if value.is_undefined() {
        CharacterOptions::default()
    } else {
        let object = value
            .as_object()
            .ok_or_else(|| Exception::throw_type(ctx, "expected character options"))?;
        let snap = object.get::<_, Value>("snap_to_ground")?;
        let snap_to_ground = if snap.is_null() || snap.is_undefined() {
            None
        } else {
            Some(snap.as_number().ok_or_else(|| {
                Exception::throw_type(ctx, "expected numeric ground snap or null")
            })?)
        };
        let step = object.get::<_, Value>("autostep")?;
        let autostep = if step.is_null() || step.is_undefined() {
            None
        } else {
            let step = step
                .as_object()
                .ok_or_else(|| Exception::throw_type(ctx, "expected character autostep or null"))?;
            Some(CharacterStep {
                max_height: number(ctx, step, "max_height")?,
                min_width: number(ctx, step, "min_width")?,
                include_dynamic_bodies: boolean(ctx, step, "include_dynamic_bodies")?,
            })
        };
        CharacterOptions {
            offset: number(ctx, object, "offset")?,
            slide: boolean(ctx, object, "slide")?,
            max_slope_climb_angle: number(ctx, object, "max_slope_climb_angle")?,
            min_slope_slide_angle: number(ctx, object, "min_slope_slide_angle")?,
            snap_to_ground,
            autostep,
        }
    };
    Ok(CharacterQuery {
        scene_id: scene,
        entity_id,
        translation,
        options,
    })
}
pub(super) fn materialize<'js>(ctx: &Ctx<'js>, data: ResultData) -> rquickjs::Result<Value<'js>> {
    #[cfg(test)]
    if FAIL_RESULT_ALLOCATION.with(|flag| flag.replace(false)) {
        return Err(rquickjs::Error::Allocation);
    }
    match data {
        ResultData::Ray(Some(hit)) => ray_value(ctx, hit),
        ResultData::Ray(None) => Ok(Value::new_null(ctx.clone())),
        ResultData::Character(movement) => {
            let result = Object::new_proto(ctx.clone(), None)?;
            result.set("translation", owned_vector(ctx, movement.translation)?)?;
            result.set("grounded", movement.grounded)?;
            result.set("sliding_down_slope", movement.sliding_down_slope)?;
            let collisions = Array::new(ctx.clone())?;
            for (i, id) in movement.collisions.into_iter().enumerate() {
                collisions.as_object().prop(
                    i as u32,
                    Property::from(ulid::Ulid::from_bytes(id.0).to_string())
                        .enumerable()
                        .writable()
                        .configurable(),
                )?;
            }
            result.set("collisions", collisions)?;
            Ok(result.into_value())
        }
        ResultData::Triggers(events) => {
            let result = Array::new(ctx.clone())?;
            for (i, event) in events.into_iter().enumerate() {
                let item = Object::new_proto(ctx.clone(), None)?;
                for (name, id) in [
                    ("scene_id", event.scene_id),
                    ("first", event.first),
                    ("second", event.second),
                ] {
                    item.set(name, ulid::Ulid::from_bytes(id.0).to_string())?;
                }
                item.set("entered", event.entered)?;
                result.as_object().prop(
                    i as u32,
                    Property::from(item).enumerable().writable().configurable(),
                )?;
            }
            Ok(result.into_value())
        }
    }
}
#[cfg(test)]
thread_local! {
    pub(super) static FAIL_RESULT_ALLOCATION: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

fn ray<'js>(
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
fn ray_value<'js>(
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

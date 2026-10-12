//! Bounded fixed-field avoidance conversion. No JSON or native loan spans guest getters.
use crate::{
    commands,
    query_values::{Request, owned_vector},
};
use incant_runtime::{SteeringAgent, SteeringObstacle, SteeringQuery, SteeringVelocity};
use rquickjs::{Array, Ctx, Exception, Object, Value, object::Property};

fn object<'js>(ctx: &Ctx<'js>, value: Value<'js>) -> rquickjs::Result<Object<'js>> {
    value
        .into_object()
        .ok_or_else(|| Exception::throw_type(ctx, "expected steering object"))
}
fn array<'js>(
    ctx: &Ctx<'js>,
    value: Value<'js>,
    max: usize,
) -> rquickjs::Result<(Object<'js>, usize)> {
    if !value.is_array() {
        return Err(Exception::throw_type(
            ctx,
            "expected bounded steering array",
        ));
    }
    let object = object(ctx, value)?;
    let length = object
        .get::<_, Value>("length")?
        .as_number()
        .filter(|n| n.is_finite() && (0. ..=max as f64).contains(n) && n.fract() == 0.)
        .ok_or_else(|| Exception::throw_type(ctx, "steering array limit"))?
        as usize;
    Ok((object, length))
}
fn number(ctx: &Ctx<'_>, value: Value<'_>, min: f64, max: f64) -> rquickjs::Result<f32> {
    value
        .as_number()
        .filter(|n| n.is_finite() && (min..=max).contains(n))
        .map(|n| n as f32)
        .ok_or_else(|| Exception::throw_type(ctx, "invalid steering number"))
}
fn point<'js, const N: usize>(ctx: &Ctx<'js>, value: Value<'js>) -> rquickjs::Result<[f32; N]> {
    let values = commands::numbers::<N>(ctx, &value)?;
    if !values.iter().all(|n| n.is_finite() && n.abs() <= 10000.) {
        return Err(Exception::throw_type(ctx, "invalid steering position"));
    }
    Ok(values.map(|n| n as f32))
}
fn velocity<'js>(ctx: &Ctx<'js>, value: Value<'js>) -> rquickjs::Result<[f32; 2]> {
    let values = commands::numbers::<2>(ctx, &value)?;
    if !values.iter().all(|n| n.is_finite()) || values[0].hypot(values[1]) > 30. {
        return Err(Exception::throw_type(ctx, "invalid steering velocity"));
    }
    Ok(values.map(|n| n as f32))
}
pub(super) fn parse<'js>(
    ctx: &Ctx<'js>,
    args: &[Value<'js>],
    dt: f64,
) -> rquickjs::Result<Request> {
    let [value] = args else {
        return Err(Exception::throw_type(
            ctx,
            "steerAgents expects one bulk request",
        ));
    };
    let input = object(ctx, value.clone())?;
    if !input.get::<_, Value>("scene_id")?.is_undefined() {
        return Err(Exception::throw_type(
            ctx,
            "steering input is already world-space",
        ));
    }
    let time_horizon = number(ctx, input.get("time_horizon")?, dt, 10.)?;
    let obstacle_time_horizon = number(ctx, input.get("obstacle_time_horizon")?, dt, 10.)?;
    let margin = input.get::<_, Value>("margin")?;
    let margin = if margin.is_undefined() {
        0.05
    } else {
        number(ctx, margin, 0., 1.)?
    };
    let (array, length) = array(ctx, input.get("agents")?, 128)?;
    let mut agents = Vec::with_capacity(length);
    for i in 0..length {
        let a = object(ctx, array.get(i as u32)?)?;
        agents.push(SteeringAgent {
            id: commands::id(ctx, &a.get::<_, Value>("id")?)?,
            position: point(ctx, a.get("position")?)?,
            velocity: velocity(ctx, a.get("velocity")?)?,
            preferred_velocity: velocity(ctx, a.get("preferred_velocity")?)?,
            radius: number(ctx, a.get("radius")?, 0.05, 4.)?,
            height: number(ctx, a.get("height")?, 0.1, 10.)?,
            max_speed: number(ctx, a.get("max_speed")?, 0., 30.)?,
            responsibility: number(ctx, a.get("responsibility")?, 0.1, 10.)?,
        });
    }
    let mut obstacles = Vec::new();
    let value = input.get::<_, Value>("obstacles")?;
    if !value.is_undefined() {
        let (array, length) = self::array(ctx, value, 32)?;
        obstacles.reserve_exact(length);
        let mut edges = 0;
        for i in 0..length {
            let o = object(ctx, array.get(i as u32)?)?;
            let closed = o
                .get::<_, Value>("closed")?
                .as_bool()
                .ok_or_else(|| Exception::throw_type(ctx, "expected steering closed boolean"))?;
            let (points, count) = self::array(ctx, o.get("vertices")?, 32)?;
            if (closed && count < 3) || (!closed && count != 2) {
                return Err(Exception::throw_type(
                    ctx,
                    "invalid steering obstacle vertex count",
                ));
            }
            edges += if closed { count } else { 1 };
            if edges > 128 {
                return Err(Exception::throw_type(ctx, "steering obstacle edge limit"));
            }
            let mut vertices = Vec::with_capacity(count);
            for j in 0..count {
                vertices.push(point(ctx, points.get(j as u32)?)?);
            }
            obstacles.push(SteeringObstacle {
                id: commands::id(ctx, &o.get::<_, Value>("id")?)?,
                vertices,
                closed,
                min_y: number(ctx, o.get("min_y")?, -10000., 10000.)?,
                max_y: number(ctx, o.get("max_y")?, -10000., 10010.)?,
            });
        }
    }
    Ok(Request::Steering(SteeringQuery {
        agents,
        obstacles,
        time_horizon,
        obstacle_time_horizon,
        margin,
    }))
}

pub(super) fn materialize<'js>(
    ctx: &Ctx<'js>,
    velocities: Vec<SteeringVelocity>,
) -> rquickjs::Result<Value<'js>> {
    let result = Array::new(ctx.clone())?;
    for (i, output) in velocities.into_iter().enumerate() {
        let item = Object::new_proto(ctx.clone(), None)?;
        item.set("id", ulid::Ulid::from_bytes(output.id.0).to_string())?;
        item.set(
            "velocity",
            owned_vector(ctx, output.velocity.map(f64::from))?,
        )?;
        item.set("neighbors", output.neighbors)?;
        result.as_object().prop(
            i as u32,
            Property::from(item).enumerable().writable().configurable(),
        )?;
    }
    Ok(result.into_value())
}

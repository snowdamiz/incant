//! Fixed-field navigation conversion; no Rust cache loan spans guest getters.
use crate::{
    commands,
    query_values::{Request, owned_vector},
};
use incant_runtime::{GridPath, GridPathRequest, NavigationPath, PathRequest, StableId};
use rquickjs::{Array, Ctx, Exception, Object, Value, object::Property};

fn arguments<'js>(
    ctx: &Ctx<'js>,
    args: &[Value<'js>],
) -> rquickjs::Result<(StableId, Object<'js>)> {
    let [entity, value] = args else {
        return Err(Exception::throw_type(
            ctx,
            "navigation expects an entity and request",
        ));
    };
    let entity = commands::id(ctx, entity)?;
    let object = value
        .as_object()
        .ok_or_else(|| Exception::throw_type(ctx, "expected a navigation request"))?;
    if !object.get::<_, Value>("scene_id")?.is_undefined() {
        return Err(Exception::throw_type(
            ctx,
            "navigation uses the active scene only",
        ));
    }
    Ok((entity, object.clone()))
}
fn number(
    ctx: &Ctx<'_>,
    value: &Value<'_>,
    min: f64,
    max: f64,
    integer: bool,
) -> rquickjs::Result<f64> {
    let value = value
        .as_number()
        .filter(|n| n.is_finite() && (min..=max).contains(n) && (!integer || n.fract() == 0.));
    value.ok_or_else(|| Exception::throw_type(ctx, "invalid navigation number"))
}
fn point<'js>(ctx: &Ctx<'js>, value: &Value<'js>) -> rquickjs::Result<[f32; 3]> {
    let values = commands::numbers::<3>(ctx, value)?;
    // Validate the original doubles before narrowing to the native f32 surface.
    if !values.iter().all(|n| n.is_finite() && n.abs() <= 100_000.) {
        return Err(Exception::throw_type(ctx, "invalid navigation point"));
    }
    Ok(values.map(|n| n as f32))
}
fn cell<'js>(ctx: &Ctx<'js>, value: &Value<'js>) -> rquickjs::Result<[u16; 2]> {
    let values = commands::numbers::<2>(ctx, value)?;
    if !values
        .iter()
        .all(|n| n.is_finite() && (0. ..=u16::MAX as f64).contains(n) && n.fract() == 0.)
    {
        return Err(Exception::throw_type(
            ctx,
            "expected unsigned integer grid cells",
        ));
    }
    Ok(values.map(|n| n as u16))
}
pub(super) fn path<'js>(ctx: &Ctx<'js>, args: &[Value<'js>]) -> rquickjs::Result<Request> {
    let (entity, object) = arguments(ctx, args)?;
    Ok(Request::Navigation(
        entity,
        PathRequest {
            start: point(ctx, &object.get::<_, Value>("start")?)?,
            end: point(ctx, &object.get::<_, Value>("end")?)?,
            snap_distance: number(
                ctx,
                &object.get::<_, Value>("snap_distance")?,
                0.001,
                10.,
                false,
            )? as f32,
            max_visited: number(
                ctx,
                &object.get::<_, Value>("max_visited")?,
                1.,
                32_768.,
                true,
            )? as u32,
        },
    ))
}
pub(super) fn grid<'js>(ctx: &Ctx<'js>, args: &[Value<'js>]) -> rquickjs::Result<Request> {
    let (entity, object) = arguments(ctx, args)?;
    let start = cell(ctx, &object.get::<_, Value>("start")?)?;
    let end = cell(ctx, &object.get::<_, Value>("end")?)?;
    let diagonal = object.get::<_, Value>("diagonal")?;
    let diagonal = if diagonal.is_undefined() {
        false
    } else {
        diagonal
            .as_bool()
            .ok_or_else(|| Exception::throw_type(ctx, "expected a diagonal boolean"))?
    };
    let expansions = object.get::<_, Value>("max_expansions")?;
    let max_expansions = if expansions.is_undefined() {
        65_536
    } else {
        number(ctx, &expansions, 1., 65_536., true)? as u32
    };
    Ok(Request::Grid(
        entity,
        GridPathRequest {
            start,
            end,
            diagonal,
            max_expansions,
        },
    ))
}
fn safe_integer(ctx: &Ctx<'_>, value: u64) -> rquickjs::Result<f64> {
    if value > 9_007_199_254_740_991 {
        return Err(Exception::throw_type(
            ctx,
            "navigation metadata exceeds exact JavaScript integers",
        ));
    }
    Ok(value as f64)
}
fn append<'js, T: rquickjs::IntoJs<'js>>(
    array: &Array<'js>,
    index: usize,
    value: T,
) -> rquickjs::Result<()> {
    // Define own data properties, bypassing arbitrary inherited guest setters.
    array.as_object().prop(
        index as u32,
        Property::from(value).enumerable().writable().configurable(),
    )
}
pub(super) fn path_value<'js>(
    ctx: &Ctx<'js>,
    path: Option<NavigationPath>,
) -> rquickjs::Result<Value<'js>> {
    let Some(path) = path else {
        return Ok(Value::new_null(ctx.clone()));
    };
    let result = Object::new_proto(ctx.clone(), None)?;
    result.set("generation", safe_integer(ctx, path.generation)?)?;
    result.set("visited", path.visited)?;
    let points = Array::new(ctx.clone())?;
    for (i, point) in path.points.into_iter().enumerate() {
        append(&points, i, owned_vector(ctx, point.map(f64::from))?)?;
    }
    result.set("points", points)?;
    let corridor = Array::new(ctx.clone())?;
    for (i, polygon) in path.corridor.into_iter().enumerate() {
        append(&corridor, i, polygon)?;
    }
    result.set("corridor", corridor)?;
    let traversals = Array::new(ctx.clone())?;
    for (i, traversal) in path.traversals.into_iter().enumerate() {
        let item = Object::new_proto(ctx.clone(), None)?;
        item.set("link_id", traversal.link_id)?;
        item.set("from_index", traversal.from_index)?;
        item.set("to_index", traversal.to_index)?;
        item.set("reversed", traversal.reversed)?;
        append(&traversals, i, item)?;
    }
    result.set("traversals", traversals)?;
    Ok(result.into_value())
}
pub(super) fn grid_value<'js>(
    ctx: &Ctx<'js>,
    path: Option<GridPath>,
) -> rquickjs::Result<Value<'js>> {
    let Some(path) = path else {
        return Ok(Value::new_null(ctx.clone()));
    };
    let result = Object::new_proto(ctx.clone(), None)?;
    result.set("generation", safe_integer(ctx, path.generation)?)?;
    result.set("cost", safe_integer(ctx, path.cost)?)?;
    let cells = Array::new(ctx.clone())?;
    for (i, cell) in path.cells.into_iter().enumerate() {
        append(&cells, i, owned_vector(ctx, cell.map(f64::from))?)?;
    }
    result.set("cells", cells)?;
    Ok(result.into_value())
}

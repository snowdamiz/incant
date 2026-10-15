//! Strict active-tick punctual-light scalar controls, no world pointer exposure.
use crate::commands;
use incant_runtime::{LightControl, StructuralCommand};
use rquickjs::{Ctx, Exception, Value};
pub(crate) fn parse<'js>(
    ctx: &Ctx<'js>,
    name: &str,
    args: &[Value<'js>],
) -> rquickjs::Result<StructuralCommand> {
    let [entity, value] = args else {
        return Err(Exception::throw_type(
            ctx,
            "light control expects an entity and a value",
        ));
    };
    let control = match name {
        "setLightColor" => LightControl::Color(commands::numbers(ctx, value)?),
        "setLightIntensity" => LightControl::Intensity(
            value
                .as_number()
                .ok_or_else(|| Exception::throw_type(ctx, "light intensity must be a number"))?,
        ),
        _ => return Err(Exception::throw_type(ctx, "unsupported light control")),
    };
    control.validate().map_err(|_| {
        Exception::throw_type(ctx, "light color/intensity must be finite and in range")
    })?;
    Ok(StructuralCommand::ControlLight {
        entity: commands::id(ctx, entity)?,
        control,
    })
}

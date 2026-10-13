//! Owned pre-command declarations and primitive command arguments. No world
//! pointers, retained JS roots, JSON conversion or guest property access.
use incant_runtime::{NativeWorld, StructuralCommand, TextControl, UiControl};
use incant_types::UiKind;
use rquickjs::{Array, Ctx, Exception, IntoJs, Object, Value, object::Property};

fn field<'js>(o: &Object<'js>, key: &str, value: impl IntoJs<'js>) -> rquickjs::Result<()> {
    o.prop(key, Property::from(value).enumerable())
}
fn append<'js>(array: &Array<'js>, i: usize, row: Object<'js>) -> rquickjs::Result<()> {
    array
        .as_object()
        .prop(i as u32, Property::from(row).enumerable())
}
fn seal(array: &Array<'_>) -> rquickjs::Result<()> {
    array
        .as_object()
        .prop("length", crate::input::ReadonlyLength(array.len() as u32))
}

pub(crate) fn expose<'js>(
    ctx: &Ctx<'js>,
    world: &NativeWorld,
) -> Result<Object<'js>, crate::ScriptError> {
    let ui = world.ui_view()?;
    let text = world.text_view()?;
    let result = (|| -> rquickjs::Result<_> {
        let result = Object::new_proto(ctx.clone(), None)?;
        let widgets = Array::new(ctx.clone())?;
        for (i, node) in ui.nodes().enumerate() {
            let row = Object::new_proto(ctx.clone(), None)?;
            field(&row, "id", ulid::Ulid::from_bytes(node.id.0).to_string())?;
            field(&row, "disabled", node.node.disabled)?;
            field(
                &row,
                "role",
                match node.node.kind {
                    UiKind::Root => "root",
                    UiKind::Container => "container",
                    UiKind::Label { .. } => "label",
                    UiKind::Button { .. } => "button",
                    UiKind::TextInput { .. } => "text_input",
                },
            )?;
            field(
                &row,
                "text",
                node.node.kind.text().map(|t| t.content.as_str()),
            )?;
            append(&widgets, i, row)?;
        }
        seal(&widgets)?;
        field(&result, "widgets", widgets)?;
        let texts = Array::new(ctx.clone())?;
        for (i, (id, text)) in text.nodes().enumerate() {
            let row = Object::new_proto(ctx.clone(), None)?;
            field(&row, "id", ulid::Ulid::from_bytes(id.0).to_string())?;
            field(&row, "text", text.content.as_ref())?;
            append(&texts, i, row)?;
        }
        seal(&texts)?;
        field(&result, "texts", texts)?;
        Ok(result)
    })();
    result.map_err(crate::diagnostics::allocation)
}

pub(crate) fn parse<'js>(
    ctx: &Ctx<'js>,
    name: &str,
    args: &[Value<'js>],
) -> rquickjs::Result<StructuralCommand> {
    let invalid = || Exception::throw_type(ctx, "invalid HUD control arguments");
    let [id, value] = args else {
        return Err(invalid());
    };
    let entity = crate::commands::id(ctx, id)?;
    if name == "setUiDisabled" {
        return Ok(StructuralCommand::ControlUi {
            entity,
            control: UiControl::Disabled(value.as_bool().ok_or_else(invalid)?),
        });
    }
    let value = value.as_string().ok_or_else(invalid)?;
    let bytes = value.clone().to_cstring()?;
    if bytes.len() > incant_runtime::MAX_TEXT_BYTES {
        return Err(invalid());
    }
    drop(bytes);
    let value = value.to_string().map_err(|_| invalid())?;
    Ok(if name == "setUiText" {
        StructuralCommand::ControlUi {
            entity,
            control: UiControl::Text(value.into()),
        }
    } else {
        StructuralCommand::ControlText {
            entity,
            control: TextControl::Content(value.into()),
        }
    })
}

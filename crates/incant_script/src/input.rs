//! Bounded input copies, allocated inside the VM's memory/deadline limits. Only
//! engine-validated InputRuntime frames reach this module. No serialization,
//! guest calls, native closures or borrowed pointers enter the JS snapshot.
use incant_input::*;
use rquickjs::{Array, Ctx, IntoJs, Object, Value, object::Property};

fn object<'js>(ctx: &Ctx<'js>) -> rquickjs::Result<Object<'js>> {
    Object::new_proto(ctx.clone(), None)
}

fn field<'js>(object: &Object<'js>, name: &str, value: impl IntoJs<'js>) -> rquickjs::Result<()> {
    object.prop(name, Property::from(value).enumerable())
}

fn array<'js, T: IntoJs<'js>>(
    ctx: &Ctx<'js>,
    values: impl IntoIterator<Item = T>,
) -> rquickjs::Result<Array<'js>> {
    let array = Array::new(ctx.clone())?;
    for (index, value) in values.into_iter().enumerate() {
        // Define own data properties: Array.prototype may contain guest setters.
        array
            .as_object()
            .prop(index as u32, Property::from(value).enumerable())?;
    }
    Ok(array)
}

fn buttons<'js, T: Ord + Copy>(
    ctx: &Ctx<'js>,
    values: &Buttons<T>,
    label: impl Fn(T) -> String,
) -> rquickjs::Result<Object<'js>> {
    let result = object(ctx)?;
    for (name, values) in [
        ("held", &values.held),
        ("pressed", &values.pressed),
        ("released", &values.released),
    ] {
        field(
            &result,
            name,
            array(ctx, values.iter().map(|value| label(*value)))?,
        )?;
    }
    Ok(result)
}

fn mouse(value: MouseButton) -> String {
    match value {
        MouseButton::Left => "left",
        MouseButton::Right => "right",
        MouseButton::Middle => "middle",
        MouseButton::Back => "back",
        MouseButton::Forward => "forward",
    }
    .into()
}

fn pad(value: GamepadButton) -> String {
    match value {
        GamepadButton::South => "south",
        GamepadButton::East => "east",
        GamepadButton::West => "west",
        GamepadButton::North => "north",
        GamepadButton::LeftShoulder => "left_shoulder",
        GamepadButton::RightShoulder => "right_shoulder",
        GamepadButton::LeftTrigger => "left_trigger",
        GamepadButton::RightTrigger => "right_trigger",
        GamepadButton::Select => "select",
        GamepadButton::Start => "start",
        GamepadButton::Guide => "guide",
        GamepadButton::LeftStick => "left_stick",
        GamepadButton::RightStick => "right_stick",
        GamepadButton::DpadUp => "dpad_up",
        GamepadButton::DpadDown => "dpad_down",
        GamepadButton::DpadLeft => "dpad_left",
        GamepadButton::DpadRight => "dpad_right",
    }
    .into()
}

pub(crate) fn expose<'js>(ctx: &Ctx<'js>, frame: &InputFrame) -> rquickjs::Result<Object<'js>> {
    let result = object(ctx)?;
    field(&result, "focused", frame.focused)?;
    // KeyCode's public physical key spelling is its Rust/DOM variant name.
    field(
        &result,
        "keyboard",
        buttons(ctx, &frame.keyboard, |key| format!("{key:?}"))?,
    )?;
    let mouse_frame = object(ctx)?;
    field(
        &mouse_frame,
        "buttons",
        buttons(ctx, &frame.mouse.buttons, mouse)?,
    )?;
    field(
        &mouse_frame,
        "position",
        match frame.mouse.position {
            Some(value) => array(ctx, value)?.into_value(),
            None => Value::new_null(ctx.clone()),
        },
    )?;
    field(&mouse_frame, "motion", array(ctx, frame.mouse.motion)?)?;
    field(
        &mouse_frame,
        "wheel_lines",
        array(ctx, frame.mouse.wheel_lines)?,
    )?;
    field(
        &mouse_frame,
        "wheel_pixels",
        array(ctx, frame.mouse.wheel_pixels)?,
    )?;
    field(&result, "mouse", mouse_frame)?;
    let gamepads = object(ctx)?;
    for (id, value) in &frame.gamepads {
        let entry = object(ctx)?;
        field(&entry, "buttons", buttons(ctx, &value.buttons, pad)?)?;
        let values = object(ctx)?;
        for (button, value) in &value.values {
            field(&values, &pad(*button), *value)?;
        }
        field(&entry, "values", values)?;
        field(&entry, "left_stick", array(ctx, value.left_stick)?)?;
        field(&entry, "right_stick", array(ctx, value.right_stick)?)?;
        field(&gamepads, &id.to_string(), entry)?;
    }
    field(&result, "gamepads", gamepads)?;
    field(
        &result,
        "connected",
        array(ctx, frame.connected.iter().copied())?,
    )?;
    field(
        &result,
        "disconnected",
        array(ctx, frame.disconnected.iter().copied())?,
    )?;
    let touches = object(ctx)?;
    for (id, value) in &frame.touches {
        let entry = object(ctx)?;
        field(&entry, "position", array(ctx, value.position)?)?;
        field(&entry, "delta", array(ctx, value.delta)?)?;
        field(&touches, &id.to_string(), entry)?;
    }
    field(&result, "touches", touches)?;
    let changes = frame
        .touch_changes
        .iter()
        .map(|change| {
            let entry = object(ctx)?;
            field(&entry, "id", change.id)?;
            field(
                &entry,
                "phase",
                match change.phase {
                    TouchPhase::Down => "down",
                    TouchPhase::Move => "move",
                    TouchPhase::Up => "up",
                    TouchPhase::Cancel => "cancel",
                },
            )?;
            field(&entry, "position", array(ctx, change.position)?)?;
            Ok(entry)
        })
        .collect::<rquickjs::Result<Vec<_>>>()?;
    field(&result, "touch_changes", array(ctx, changes)?)?;
    let gestures = frame
        .gestures
        .iter()
        .map(|gesture| {
            let entry = object(ctx)?;
            match gesture {
                Gesture::Tap { id, position } | Gesture::LongPress { id, position } => {
                    field(
                        &entry,
                        "type",
                        if matches!(gesture, Gesture::Tap { .. }) {
                            "tap"
                        } else {
                            "long_press"
                        },
                    )?;
                    field(&entry, "id", *id)?;
                    field(&entry, "position", array(ctx, *position)?)?;
                }
                Gesture::Swipe {
                    id,
                    direction,
                    distance,
                    duration_seconds,
                } => {
                    field(&entry, "type", "swipe")?;
                    field(&entry, "id", *id)?;
                    field(&entry, "direction", array(ctx, *direction)?)?;
                    field(&entry, "distance", *distance)?;
                    field(&entry, "duration_seconds", *duration_seconds)?;
                }
                Gesture::Pinch {
                    ids,
                    center,
                    scale_delta,
                    rotation_delta,
                } => {
                    field(&entry, "type", "pinch")?;
                    field(&entry, "ids", array(ctx, *ids)?)?;
                    field(&entry, "center", array(ctx, *center)?)?;
                    field(&entry, "scale_delta", *scale_delta)?;
                    field(&entry, "rotation_delta", *rotation_delta)?;
                }
            }
            Ok(entry)
        })
        .collect::<rquickjs::Result<Vec<_>>>()?;
    field(&result, "gestures", array(ctx, gestures)?)?;
    let actions = object(ctx)?;
    for (name, state) in &frame.actions {
        let entry = object(ctx)?;
        field(
            &entry,
            "kind",
            match state.kind {
                ActionKind::Button => "button",
                ActionKind::Axis1 => "axis1",
                ActionKind::Axis2 => "axis2",
            },
        )?;
        field(&entry, "value", array(ctx, state.value)?)?;
        field(&entry, "active", state.active)?;
        field(&entry, "pressed", state.pressed)?;
        field(&entry, "released", state.released)?;
        field(&actions, name, entry)?;
    }
    field(&result, "actions", actions)?;
    Ok(result)
}

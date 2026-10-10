use incant_input::*;
use serde_json::{Value, json};

pub fn check() -> Result<Value, String> {
    let actions: InputActions = serde_json::from_value(json!({
        "forward":{"kind":"button","bindings":[{"type":"key","code":"KeyW"}]},
        "steer":{"kind":"axis1","dead_zone":0.2,"bindings":[{"type":"gamepad_axis","id":0,"axis":"left_x"}]}
    })).map_err(|e| e.to_string())?;
    let events = vec![
        InputEvent::Key {
            code: KeyCode::KeyW,
            down: true,
        },
        InputEvent::GamepadConnected { id: 0 },
        InputEvent::GamepadAxis {
            id: 0,
            axis: GamepadAxis::LeftX,
            value: 0.5,
        },
        InputEvent::Touch {
            id: 1,
            phase: TouchPhase::Down,
            position: [0., 0.],
        },
        InputEvent::Touch {
            id: 2,
            phase: TouchPhase::Down,
            position: [10., 0.],
        },
    ];
    let movement = vec![InputEvent::Touch {
        id: 2,
        phase: TouchPhase::Move,
        position: [0., 20.],
    }];
    let mut input = InputRuntime::default();
    input
        .advance_mapped(&events, 1. / 60., &actions)
        .map_err(|e| e.to_string())?;
    let first = input.frame().clone();
    input
        .advance_mapped(&movement, 1. / 60., &actions)
        .map_err(|e| e.to_string())?;
    let (scale, rotation) = match &input.frame().gestures[..] {
        [
            Gesture::Pinch {
                scale_delta,
                rotation_delta,
                ..
            },
        ] => (*scale_delta, *rotation_delta),
        _ => return Err("pinch recognition failed".into()),
    };
    if scale != 2.
        || (rotation - std::f64::consts::FRAC_PI_2).abs() > 1e-12
        || !first.keyboard.held.contains(&KeyCode::KeyW)
        || first.gamepads[&0].left_stick[0] != 0.5
        || !first.actions["forward"].pressed
        || input.frame().actions["forward"].pressed
        || (first.actions["steer"].value[0] - 0.375).abs() > 1e-12
    {
        return Err("physical input state assertion failed".into());
    }
    let clip = json!({"format":"incant-input","version":1,"tick_rate":60,"start_tick":0,"ticks":2,
        "frames":[{"tick":1,"events":events},{"tick":2,"events":movement}]})
    .to_string();
    let replay = InputReplay::from_text(&clip, 1, 60).map_err(|e| e.to_string())?;
    let mut resumed = replay.initial_state();
    resumed
        .advance_mapped(
            replay.events_at(2).map_err(|e| e.to_string())?,
            1. / 60.,
            &actions,
        )
        .map_err(|e| e.to_string())?;
    if resumed.frame() != input.frame() {
        return Err("input history replay differs".into());
    }
    input
        .advance_mapped(&[InputEvent::Focus { focused: false }], 1. / 60., &actions)
        .map_err(|e| e.to_string())?;
    if !input.frame().keyboard.held.is_empty()
        || !input.frame().touches.is_empty()
        || input.frame().gamepads[&0].left_stick != [0., 0.]
        || !input.frame().actions["forward"].released
        || input.frame().actions["forward"].active
    {
        return Err("focus loss left active controls".into());
    }
    Ok(
        json!({"pinch_scale":scale,"pinch_rotation":rotation,"replay_equal":true,"focus_releases_controls":true,
        "named_action_edges":true,"action_dead_zone":true,
        "scope":"normalized synthetic events; not a live device adapter"}),
    )
}

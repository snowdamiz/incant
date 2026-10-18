use super::super::tests::{Measure, accept, id, runtime};
use super::*;
use incant_input::{
    ActionGesture, Gesture, InputAction, InputActions, InputBinding, InputRuntime,
    MAX_EVENTS_PER_TICK,
};
use incant_ui::{UiAction, UiRouter};

fn native(id: u64, phase: NativePhase, position: [f64; 2]) -> WindowEvent<'static> {
    WindowEvent::Touch(Touch {
        // SAFETY: this inert ID is only compared; never passed into a Tao API.
        device_id: unsafe { DeviceId::dummy() },
        id,
        phase,
        location: PhysicalPosition::new(position[0], position[1]),
        force: None,
    })
}
fn deliver(
    input: &mut UiInput,
    physical: &mut PendingInput,
    ui: Option<&UiRuntime>,
    id: u64,
    phase: NativePhase,
    p: [f64; 2],
) {
    input
        .window(ui, physical, &native(id, phase, p), 1.)
        .unwrap();
}
fn actions() -> InputActions {
    [
        ActionGesture::Tap,
        ActionGesture::LongPress,
        ActionGesture::Swipe,
    ]
    .into_iter()
    .enumerate()
    .map(|(i, gesture)| {
        (
            format!("gesture{i}"),
            InputAction::Button {
                bindings: vec![InputBinding::Gesture {
                    gesture,
                    scale: [1., 0.],
                }],
                threshold: 0.5,
            },
        )
    })
    .collect()
}

mod contacts;
mod gestures;
mod lifecycle;
mod ownership;
mod transactions;

use super::*;
use incant_input::{InputEvent, InputRuntime, KeyCode as InputKey, MouseButton as InputButton};
use std::{cell::RefCell, rc::Rc};

#[derive(Default)]
struct State {
    grabbed: bool,
    visible: bool,
    fail_grab: bool,
    fail_release: bool,
    calls: Vec<(bool, bool)>,
}
#[derive(Clone, Default)]
struct Mock(Rc<RefCell<State>>);
impl Cursor for Mock {
    fn grab(&self, grabbed: bool) -> Result<(), Error> {
        let mut state = self.0.borrow_mut();
        // Model a backend that changes state before reporting acquisition failure.
        state.grabbed = grabbed;
        let visible = state.visible;
        state.calls.push((grabbed, visible));
        if (grabbed && state.fail_grab) || (!grabbed && state.fail_release) {
            Err(Error::PointerCapture(
                if grabbed { "acquire" } else { "release" }.into(),
            ))
        } else {
            Ok(())
        }
    }
    fn visible(&self, visible: bool) {
        self.0.borrow_mut().visible = visible;
    }
}
fn owner(scale: Option<f64>) -> (Pointer<Mock>, Mock) {
    let backend = Mock::default();
    backend.0.borrow_mut().visible = true;
    (
        Pointer::new(backend.clone(), scale.map(|s| Sensitivity::new(s).unwrap())),
        backend,
    )
}
fn press(pointer: &mut Pointer<Mock>, eligible: bool) -> Route {
    pointer
        .button(MouseButton::Left, ElementState::Pressed, eligible)
        .unwrap()
}
fn release(pointer: &mut Pointer<Mock>) -> Route {
    pointer
        .button(MouseButton::Left, ElementState::Released, true)
        .unwrap()
}

#[test]
fn default_and_inactive_mode_never_acquire_or_observe_global_motion() {
    for scale in [None, Some(1.)] {
        let (mut pointer, backend) = owner(scale);
        let mut input = PendingInput::new();
        assert_eq!(press(&mut pointer, false).delivery, Delivery::Ordinary);
        pointer
            .motion([f64::NAN, f64::INFINITY], false, &mut input)
            .unwrap();
        pointer.motion([9., 3.], true, &mut input).unwrap();
        assert!(input.events().is_empty());
        assert!(pointer.last_raw.is_none());
        if scale.is_none() {
            assert_eq!(press(&mut pointer, true).delivery, Delivery::Ordinary);
        }
        assert!(backend.0.borrow().calls.is_empty());
    }
}
#[test]
#[allow(deprecated)] // Tao requires the legacy modifiers field when constructing this event.
fn explicit_click_consumes_only_its_release_and_captured_mouse_bypasses_ui() {
    let (mut pointer, backend) = owner(Some(2.));
    let route = press(&mut pointer, true);
    assert_eq!(route.delivery, Delivery::Consume);
    assert!(route.reset && pointer.captured);
    assert!(backend.0.borrow().grabbed && !backend.0.borrow().visible);
    assert_eq!(
        pointer
            .button(MouseButton::Right, ElementState::Released, true)
            .unwrap()
            .delivery,
        Delivery::Gameplay
    );
    assert_eq!(release(&mut pointer).delivery, Delivery::Consume);
    assert_eq!(press(&mut pointer, true).delivery, Delivery::Gameplay);
    assert_eq!(release(&mut pointer).delivery, Delivery::Gameplay);
    assert_eq!(
        pointer
            .window(
                &WindowEvent::CursorMoved {
                    // SAFETY: used only as inert test event data in our router, never
                    // passed to any Tao API or native backend.
                    device_id: unsafe { tao::event::DeviceId::dummy() },
                    position: tao::dpi::PhysicalPosition::new(12., 13.),
                    modifiers: Default::default(),
                },
                true
            )
            .unwrap()
            .delivery,
        Delivery::Consume
    );
    assert_eq!(pointer.escape().unwrap().delivery, Delivery::Consume);
    assert!(!pointer.captured && backend.0.borrow().visible && !backend.0.borrow().grabbed);
    assert_eq!(pointer.escape().unwrap().delivery, Delivery::Ordinary);
    assert_eq!(
        pointer
            .button(MouseButton::Right, ElementState::Pressed, true)
            .unwrap()
            .delivery,
        Delivery::Ordinary
    );
    assert!(press(&mut pointer, true).reset);
}
#[test]
fn lifecycle_retirement_never_recaptures_and_lost_release_does_not_swallow_next_click() {
    let (mut pointer, backend) = owner(Some(1.));
    press(&mut pointer, true);
    assert!(pointer.release().unwrap());
    assert!(!pointer.release().unwrap());
    let mut input = PendingInput::new();
    pointer.motion([7., -3.], true, &mut input).unwrap();
    assert!(input.events().is_empty());
    assert_eq!(
        backend
            .0
            .borrow()
            .calls
            .iter()
            .filter(|(grabbed, _)| *grabbed)
            .count(),
        1
    );
    // Press/release while inactive never reacquires. Only the matching old left
    // release is swallowed; unrelated button releases remain ordinary.
    assert_eq!(
        pointer
            .button(MouseButton::Middle, ElementState::Released, false)
            .unwrap()
            .delivery,
        Delivery::Ordinary
    );
    assert_eq!(release(&mut pointer).delivery, Delivery::Consume);
    assert!(!press(&mut pointer, false).reset);
    assert!(press(&mut pointer, true).reset);
    pointer.release().unwrap(); // lost rearm release on focus loss
    assert!(press(&mut pointer, true).reset);
}
#[test]
fn raw_scaling_preserves_sum_discrete_order_and_never_invents_absolute_position() {
    let (mut pointer, _) = owner(Some(0.5));
    press(&mut pointer, true);
    release(&mut pointer);
    let mut input = PendingInput::new();
    input.reset_focus(true);
    for _ in 0..10_000 {
        pointer.motion([0.25, -0.5], true, &mut input).unwrap();
    }
    input
        .push(InputEvent::MouseButton {
            button: InputButton::Right,
            down: true,
        })
        .unwrap();
    pointer.motion([6., 8.], true, &mut input).unwrap();
    assert_eq!(input.events().len(), 5);
    assert_eq!(
        input.events()[2],
        InputEvent::PointerMotion {
            delta: [1250., -2500.]
        }
    );
    assert!(matches!(
        input.events()[3],
        InputEvent::MouseButton { down: true, .. }
    ));
    let mut runtime = InputRuntime::default();
    runtime
        .advance(
            &[
                InputEvent::PointerPosition {
                    position: [10., 20.],
                },
                InputEvent::Key {
                    code: InputKey::Space,
                    down: true,
                },
            ],
            1. / 60.,
        )
        .unwrap();
    runtime.advance(input.events(), 1. / 60.).unwrap();
    assert_eq!(runtime.frame().mouse.motion, [1253., -2496.]);
    assert_eq!(runtime.frame().mouse.position, None);
    assert!(!runtime.frame().keyboard.held.contains(&InputKey::Space));
    assert_eq!(pointer.last_raw, Some([6., 8.]));
    assert_eq!(pointer.last_scaled, Some([3., 4.]));
    assert_eq!(pointer.raw_events, 10_001);
}
#[test]
fn invalid_scale_delta_and_accumulation_refuse_without_partial_mutation() {
    for scale in [0., -1., f64::NAN, f64::INFINITY, 1. / 2048., 1025.] {
        assert!(Sensitivity::new(scale).is_err());
    }
    for scale in [1. / 1024., 1024.] {
        assert!(Sensitivity::new(scale).is_ok());
    }
    let (mut pointer, _) = owner(Some(1024.));
    press(&mut pointer, true);
    let mut input = PendingInput::new();
    pointer.motion([512., 0.], true, &mut input).unwrap();
    let before = input.events().to_vec();
    for raw in [
        [f64::NAN, 0.],
        [f64::INFINITY, 0.],
        [1_000_001., 0.],
        [1000., 0.],
        [512., 0.],
    ] {
        assert!(pointer.motion(raw, true, &mut input).is_err());
        assert_eq!(input.events(), before);
        assert_eq!(pointer.raw_events, 1);
        assert_eq!(pointer.last_raw, Some([512., 0.]));
    }
    input.clear();
    for _ in 0..incant_input::MAX_EVENTS_PER_TICK {
        input
            .push(InputEvent::Key {
                code: InputKey::Space,
                down: true,
            })
            .unwrap();
    }
    assert!(matches!(
        pointer.motion([1., 0.], true, &mut input),
        Err(Error::InputCapacity)
    ));
    assert_eq!(pointer.raw_events, 1);
}
#[test]
fn failed_grab_restores_visible_unconfined_cursor_and_preserves_acquisition_cause() {
    let (mut pointer, backend) = owner(Some(1.));
    backend.0.borrow_mut().fail_grab = true;
    let error = pointer
        .button(MouseButton::Left, ElementState::Pressed, true)
        .unwrap_err();
    assert!(matches!(error, Error::PointerCapture(ref cause) if cause == "acquire"));
    assert!(!pointer.captured && !backend.0.borrow().grabbed && backend.0.borrow().visible);
    assert_eq!(pointer.captures, 0);
    backend.0.borrow_mut().fail_release = true;
    let error = pointer
        .button(MouseButton::Left, ElementState::Pressed, true)
        .unwrap_err();
    assert!(matches!(error, Error::PointerCapture(ref cause) if cause == "acquire"));
    assert!(!pointer.captured && pointer.release_required && backend.0.borrow().visible);
    backend.0.borrow_mut().fail_release = false;
    drop(pointer);
    assert!(!backend.0.borrow().grabbed);
}
#[test]
fn drop_and_unwind_release_capture_and_release_failure_revokes_delivery() {
    let (mut pointer, backend) = owner(Some(1.));
    press(&mut pointer, true);
    backend.0.borrow_mut().fail_release = true;
    assert!(pointer.release().is_err());
    assert!(!pointer.captured && backend.0.borrow().visible);
    let mut pending = PendingInput::new();
    pointer.motion([2., 4.], true, &mut pending).unwrap();
    assert!(pending.events().is_empty());
    backend.0.borrow_mut().fail_release = false;
    drop(pointer);
    let other = backend.clone();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut pointer = Pointer::new(other, Some(Sensitivity::new(1.).unwrap()));
        press(&mut pointer, true);
        panic!("simulated callback unwind");
    }));
    assert!(result.is_err());
    assert!(!backend.0.borrow().grabbed && backend.0.borrow().visible);
}

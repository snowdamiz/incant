use super::*;
use incant_input::InputRuntime;

#[test]
fn cursor_and_pixel_wheel_use_logical_pixels_at_scale_one_and_two() {
    for scale in [1., 2.] {
        let mut input = PendingInput::new();
        input
            .pointer(PhysicalPosition::new(12.5 * scale, 20. * scale), scale)
            .unwrap();
        input
            .pointer(PhysicalPosition::new(18. * scale, 15.5 * scale), scale)
            .unwrap();
        input
            .wheel(
                MouseScrollDelta::PixelDelta(PhysicalPosition::new(-4.5 * scale, 7. * scale)),
                scale,
            )
            .unwrap();
        input
            .wheel(MouseScrollDelta::LineDelta(1.5, -3.25), scale)
            .unwrap();
        let mut runtime = InputRuntime::default();
        runtime.advance(input.events(), 1. / 60.).unwrap();
        let mouse = &runtime.frame().mouse;
        assert_eq!(mouse.position, Some([18., 15.5]));
        assert_eq!(mouse.motion, [5.5, -4.5]);
        assert_eq!(mouse.wheel_pixels, [-4.5, 7.]);
        assert_eq!(mouse.wheel_lines, [1.5, -3.25]);
    }
}

#[test]
fn scale_changes_use_each_samples_current_factor_without_inventing_motion() {
    let mut input = PendingInput::new();
    let mut runtime = InputRuntime::default();
    input.pointer(PhysicalPosition::new(80., 40.), 2.).unwrap();
    runtime.advance(input.events(), 1. / 60.).unwrap();
    assert_eq!(runtime.frame().mouse.position, Some([40., 20.]));
    input.clear();
    // The same logical position on a scale-one display has half the physical coordinates.
    input.pointer(PhysicalPosition::new(40., 20.), 1.).unwrap();
    runtime.advance(input.events(), 1. / 60.).unwrap();
    assert_eq!(runtime.frame().mouse.motion, [0.; 2]);
    input.clear();
    input.pointer(PhysicalPosition::new(45., 23.), 1.).unwrap();
    input.pointer(PhysicalPosition::new(90., 46.), 2.).unwrap();
    for (scale, position) in [(2., [-8., 12.]), (1., [-4., 6.])] {
        input
            .wheel(
                MouseScrollDelta::PixelDelta(PhysicalPosition::new(position[0], position[1])),
                scale,
            )
            .unwrap();
        input
            .wheel(MouseScrollDelta::LineDelta(-1.5, 3.25), scale)
            .unwrap();
    }
    runtime.advance(input.events(), 1. / 60.).unwrap();
    assert_eq!(runtime.frame().mouse.position, Some([45., 23.]));
    assert_eq!(runtime.frame().mouse.motion, [5., 3.]);
    assert_eq!(runtime.frame().mouse.wheel_pixels, [-8., 12.]);
    assert_eq!(runtime.frame().mouse.wheel_lines, [-3., 6.5]);
}

#[test]
fn invalid_scale_is_typed_and_does_not_mutate_the_pending_batch() {
    let mut input = PendingInput::new();
    input.pointer(PhysicalPosition::new(12., 24.), 2.).unwrap();
    let before = input.events().to_vec();
    for scale in [
        0.,
        -1.,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::from_bits(1),
    ] {
        assert!(matches!(
            input.pointer(PhysicalPosition::new(1., 2.), scale),
            Err(Error::InputScale)
        ));
        assert!(matches!(
            input.wheel(
                MouseScrollDelta::PixelDelta(PhysicalPosition::new(1., 2.)),
                scale
            ),
            Err(Error::InputScale)
        ));
        assert_eq!(input.events(), before);
    }
}

#[test]
fn scaled_cursor_coalescing_retains_discrete_order_and_logical_displacement() {
    let mut input = PendingInput::new();
    for i in 0..MAX_EVENTS_PER_TICK * 4 {
        input
            .pointer(PhysicalPosition::new(2. * i as f64, 4. * i as f64), 2.)
            .unwrap();
    }
    input
        .push(InputEvent::MouseButton {
            button: MouseButton::Left,
            down: true,
        })
        .unwrap();
    input.pointer(PhysicalPosition::new(20., 40.), 2.).unwrap();
    assert_eq!(input.events().len(), 4);
    assert!(matches!(
        input.events()[2],
        InputEvent::MouseButton { down: true, .. }
    ));
    let mut runtime = InputRuntime::default();
    runtime.advance(input.events(), 1. / 60.).unwrap();
    assert_eq!(runtime.frame().mouse.position, Some([10., 20.]));
    assert_eq!(runtime.frame().mouse.motion, [10., 20.]);
    assert!(
        runtime
            .frame()
            .mouse
            .buttons
            .pressed
            .contains(&MouseButton::Left)
    );
}

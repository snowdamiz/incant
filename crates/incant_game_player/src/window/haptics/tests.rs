use super::*;
use incant_input::haptics::{HapticError, HapticRequest, HapticTarget};
use std::{cell::RefCell, rc::Rc};
#[derive(Clone)]
struct Mock(Rc<RefCell<Vec<HapticEffect>>>, bool);
impl Device for Mock {
    fn rumble(&self, strong: f32, weak: f32, duration_ms: u32) -> Result<(), Error> {
        self.0.borrow_mut().push(HapticEffect::Rumble {
            strong,
            weak,
            duration_ms,
        });
        if self.1 {
            Err(Error::HapticBackend)
        } else {
            Ok(())
        }
    }
    fn stop(&self) -> Result<(), Error> {
        self.0.borrow_mut().push(HapticEffect::Stop);
        Ok(())
    }
}
fn play(target: HapticTarget) -> HapticRequest {
    HapticRequest {
        target,
        effect: HapticEffect::Rumble {
            strong: 0.25,
            weak: 0.75,
            duration_ms: 125,
        },
    }
}
#[test]
fn haptics_reuses_devices_and_expires_reconnect_and_session_targets() {
    let calls = Rc::new(RefCell::new(vec![]));
    let mut host = Haptics::default();
    host.connect(0, true, || Some(Mock(calls.clone(), false)))
        .unwrap();
    let cap = host.resume();
    let first = cap.targets().next().unwrap();
    let mut requests = HapticRequests::default();
    requests.push(play(first), &cap).unwrap();
    host.dispatch(&requests).unwrap();
    assert_eq!(calls.borrow().as_slice(), [play(first).effect]);
    host.pause().unwrap();
    let count = calls.borrow().len();
    host.pause().unwrap();
    assert_eq!(calls.borrow().len(), count);
    let cap = host.resume();
    assert_ne!(cap.targets().next().unwrap(), first);
    assert!(matches!(
        host.dispatch(&requests),
        Err(Error::HapticRequest(HapticError::Unavailable))
    ));
    host.disconnect(0).unwrap();
    assert!(host.resume().is_empty());
    host.connect(0, true, || {
        panic!("must reuse effects, not allocate a new backend ID")
    })
    .unwrap();
    assert_ne!(host.resume().targets().next().unwrap(), first);
    host.slots[0].generation = u32::MAX;
    assert!(matches!(host.pause(), Err(Error::HapticGeneration)));
    assert!(host.resume().is_empty());
    host.disconnect(0).unwrap();
    assert!(matches!(
        host.connect(0, true, || panic!()),
        Err(Error::HapticGeneration)
    ));
}
#[test]
fn haptics_validates_entire_batch_coalesces_and_cleans_up_on_driver_error() {
    let calls = Rc::new(RefCell::new(vec![]));
    let mut host = Haptics::default();
    host.connect(0, true, || Some(Mock(calls.clone(), false)))
        .unwrap();
    let cap = host.resume();
    let target = cap.targets().next().unwrap();
    let mut requests = HapticRequests::default();
    requests.push(play(target), &cap).unwrap();
    requests
        .push(
            HapticRequest {
                target,
                effect: HapticEffect::Stop,
            },
            &cap,
        )
        .unwrap();
    host.dispatch(&requests).unwrap();
    assert_eq!(calls.borrow().as_slice(), [HapticEffect::Stop]);
    let other = HapticCapabilities::default().with_gamepad(1, 1).unwrap();
    requests
        .push(play(other.targets().next().unwrap()), &other)
        .unwrap();
    calls.borrow_mut().clear();
    assert!(host.dispatch(&requests).is_err());
    assert!(
        !calls
            .borrow()
            .iter()
            .any(|v| matches!(v, HapticEffect::Rumble { .. }))
    );
    let failing = Rc::new(RefCell::new(vec![]));
    host.connect(1, true, || Some(Mock(failing.clone(), true)))
        .unwrap();
    let cap = host.resume();
    let mut requests = HapticRequests::default();
    for target in cap.targets() {
        requests.push(play(target), &cap).unwrap()
    }
    assert!(host.dispatch(&requests).is_err());
    assert!(host.capabilities().is_empty());
    assert_eq!(calls.borrow().last(), Some(&HapticEffect::Stop));
    assert_eq!(failing.borrow().last(), Some(&HapticEffect::Stop));
}
#[test]
fn haptics_unavailable_effect_creation_is_not_retried_or_reported_as_supported() {
    let mut host = Haptics::<Mock>::default();
    host.connect(0, false, || panic!()).unwrap();
    assert!(host.resume().is_empty());
    host.disconnect(0).unwrap();
    host.connect(0, true, || None).unwrap();
    assert!(host.resume().is_empty());
    host.disconnect(0).unwrap();
    host.connect(0, true, || {
        panic!("bounded one creation attempt per device")
    })
    .unwrap();
    assert!(host.resume().is_empty());
}

#[test]
fn haptics_failed_stop_remains_visible_through_repeated_exit_cleanup() {
    struct FailedStop(Rc<RefCell<usize>>);
    impl Device for FailedStop {
        fn rumble(&self, _: f32, _: f32, _: u32) -> Result<(), Error> {
            Ok(())
        }
        fn stop(&self) -> Result<(), Error> {
            *self.0.borrow_mut() += 1;
            Err(Error::HapticBackend)
        }
    }
    let calls = Rc::new(RefCell::new(0));
    let mut host = Haptics::default();
    host.connect(0, true, || Some(FailedStop(calls.clone())))
        .unwrap();
    host.resume();
    assert!(matches!(host.pause(), Err(Error::HapticBackend)));
    let count = *calls.borrow();
    assert!(matches!(host.pause(), Err(Error::HapticBackend)));
    assert_eq!(*calls.borrow(), count);
    assert!(host.resume().is_empty());
    assert!(matches!(
        host.dispatch(&HapticRequests::default()),
        Err(Error::HapticBackend)
    ));
}
#[test]
fn haptics_duration_quantization_matches_pinned_driver_without_device() {
    use gilrs::ff::Ticks;
    assert_eq!(Ticks::from_ms(1), Ticks::from_ms(50));
    assert_eq!(Ticks::from_ms(125), Ticks::from_ms(150));
    assert_ne!(Ticks::from_ms(125), Ticks::from_ms(100));
    assert_eq!(Ticks::from_ms(4_999), Ticks::from_ms(5_000));
}

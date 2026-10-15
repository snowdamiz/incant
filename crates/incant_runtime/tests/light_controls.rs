#[path = "support/lights.rs"]
mod fixture;
use fixture::*;
use incant_runtime::*;
use incant_types::Velocity;
#[test]
fn light_controls_order_with_numeric_batches_preserve_bindings_and_round_trip_saves() {
    let mut w = NativeWorld::new(scene());
    let before = declarations(&w);
    let mut q = queue([
        control(1, LightControl::Color([0.2, 0.3, 0.4])),
        control(1, LightControl::Intensity(0.)),
        control(2, LightControl::Intensity(1_000_000.)),
        control(3, LightControl::Intensity(2.)),
        control(3, LightControl::Intensity(4.)),
        StructuralCommand::SetVelocity {
            entity: id(4),
            velocity: Some(Velocity {
                linear: [1., 0., 0.],
            }),
        },
    ]);
    w.apply(&mut q).unwrap();
    assert!(q.is_empty());
    let now = declarations(&w);
    assert_eq!(now[0].directional.as_ref().unwrap().color, [0.2, 0.3, 0.4]);
    assert_eq!(now[0].directional.as_ref().unwrap().intensity, 0.);
    assert_eq!(
        now[0].directional.as_ref().unwrap().shadows,
        before[0].directional.as_ref().unwrap().shadows
    );
    assert_eq!(now[1].point.as_ref().unwrap().intensity, 1_000_000.);
    assert_eq!(now[1].point.as_ref().unwrap().range, 3.);
    assert_eq!(now[2].spot.as_ref().unwrap().intensity, 4.);
    assert_eq!(
        (
            w.spatial_view().unwrap().binding_generation(),
            w.spatial_view().unwrap().topology_generation()
        ),
        (0, 0)
    );
    let save = w.save_bytes().unwrap();
    assert_eq!(
        declarations(&NativeWorld::from_save_bytes(&save).unwrap()),
        now
    );
    w.apply(&mut queue([control(1, LightControl::Intensity(200.))]))
        .unwrap();
    w.restore_bytes(&save).unwrap();
    assert_eq!(declarations(&w), now);
}
#[test]
fn light_controls_use_final_declarations_and_late_refusal_keeps_every_mixed_prefix() {
    let mut w = NativeWorld::new(scene());
    let mut replacement = CookedSpatial::empty(id(4));
    replacement.point = scene().spatial()[1].point.clone();
    w.apply(&mut queue([
        control(4, LightControl::Intensity(9.)),
        StructuralCommand::SetSpatial {
            entity: id(4),
            spatial: Some(replacement),
        },
    ]))
    .unwrap();
    assert_eq!(
        declarations(&w)
            .last()
            .unwrap()
            .point
            .as_ref()
            .unwrap()
            .intensity,
        9.
    );
    for bad in [
        StructuralCommand::SetSpatial {
            entity: id(1),
            spatial: None,
        },
        StructuralCommand::Despawn(id(1)),
        StructuralCommand::SetParent {
            entity: id(4),
            parent: Some(id(555)),
        },
    ] {
        let before = w.save_bytes().unwrap();
        let mut q = queue([
            control(1, LightControl::Intensity(500.)),
            StructuralCommand::SetVelocity {
                entity: id(4),
                velocity: None,
            },
            bad,
        ]);
        assert!(w.apply(&mut q).is_err());
        assert_eq!(q.len(), 3);
        assert_eq!(w.save_bytes().unwrap(), before);
    }
    let before = w.save_bytes().unwrap();
    let mut q = queue([
        control(1, LightControl::Color([0.5; 3])),
        control(99, LightControl::Intensity(1.)),
    ]);
    assert!(w.apply(&mut q).is_err());
    assert_eq!(w.save_bytes().unwrap(), before);
}
#[test]
fn light_controls_reject_nonfinite_original_precision_and_nonlight_targets() {
    let mut w = NativeWorld::new(scene());
    let before = w.save_bytes().unwrap();
    for c in [
        LightControl::Intensity(f64::NAN),
        LightControl::Intensity(f64::INFINITY),
        LightControl::Intensity(-f64::EPSILON),
        LightControl::Intensity(1_000_000. + 0.00000001),
        LightControl::Color([1. + f64::EPSILON, 0., 0.]),
        LightControl::Color([0., f64::NAN, 0.]),
    ] {
        assert!(w.apply(&mut queue([control(1, c)])).is_err());
        assert_eq!(w.save_bytes().unwrap(), before);
    }
    assert!(
        w.apply(&mut queue([control(4, LightControl::Intensity(1.))]))
            .is_err()
    );
    assert_eq!(w.save_bytes().unwrap(), before);
    let mut q = FrameCommands::default();
    for _ in 0..MAX_FRAME_COMMANDS {
        q.push(control(1, LightControl::Intensity(0.))).unwrap();
    }
    assert!(q.push(control(1, LightControl::Intensity(1.))).is_err());
}

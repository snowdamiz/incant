#[path = "support/scene_format.rs"]
mod scene_format;
use incant_runtime::*;
use incant_types::{Animation, Transform};
use std::sync::Arc;

fn id(n: u8) -> StableId {
    StableId([n; 16])
}
fn definition() -> Arc<NativeAnimation> {
    Arc::new(NativeAnimation {
        graph: None,
        mesh_fingerprint: [4; 32],
        clip_durations: Arc::from([1.0, 0.0]),
        initial: Animation {
            clip: Some(0),
            seconds: 0.0,
            speed: 1.0,
            playing: true,
            looping: false,
        },
    })
}
fn node(n: u8, animation: Arc<NativeAnimation>) -> CookedSpatial {
    let mut node = CookedSpatial::empty(id(n));
    node.mesh = Some(NativeMeshRenderer {
        mesh: id(3),
        cast_shadows: true,
    });
    node.animation = Some(animation);
    node
}
fn scene() -> CookedScene {
    let animation = definition();
    CookedScene::new(
        id(0),
        60,
        [1, 2]
            .into_iter()
            .map(|n| CookedEntity {
                id: id(n),
                parent: None,
                transform: Transform::default(),
                velocity: None,
            })
            .collect(),
    )
    .unwrap()
    .with_spatial(
        vec![VisualAssetRef {
            id: id(3),
            kind: VisualAssetKind::Model,
            fingerprint: [4; 32],
        }],
        vec![node(1, animation.clone()), node(2, animation)],
    )
    .unwrap()
}
fn control(world: &mut NativeWorld, n: u8, value: AnimationControl) {
    let mut commands = FrameCommands::default();
    commands
        .push(StructuralCommand::ControlAnimation {
            entity: id(n),
            control: value,
        })
        .unwrap();
    world.apply(&mut commands).unwrap();
}
fn time(world: &NativeWorld, n: u8) -> f64 {
    world.animation_state(id(n)).unwrap().sample.seconds
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-12, "{a} != {b}");
}

#[test]
fn v11_scene_and_independent_native_clocks_round_trip_without_binding_changes() {
    let bytes = scene().to_bytes();
    assert_eq!(
        u32::from_le_bytes(bytes[8..12].try_into().unwrap()),
        crate::scene_format::scene_format(11)
    );
    assert_eq!(CookedScene::from_bytes(&bytes).unwrap().to_bytes(), bytes);
    let mut world = NativeWorld::from_bytes(&bytes).unwrap();
    let mut rows = Vec::with_capacity(2);
    let old: std::collections::BTreeMap<_, _> = world
        .spatial_view()
        .unwrap()
        .nodes()
        .map(|n| {
            (
                n.id,
                n.declaration.unwrap().animation.as_ref().unwrap().clone(),
            )
        })
        .collect();
    control(&mut world, 2, AnimationControl::SetPlaying(false));
    for _ in 0..30 {
        world.step().unwrap();
    }
    close(time(&world, 1), 0.5);
    close(time(&world, 2), 0.0);
    world.snapshot_animations(&mut rows).unwrap();
    assert_eq!(rows.len(), 2);
    for node in world.spatial_view().unwrap().nodes() {
        assert_eq!(
            node.animation,
            Some(world.animation_state(node.id).unwrap().sample)
        );
        assert!(Arc::ptr_eq(
            &old[&node.id],
            node.declaration.unwrap().animation.as_ref().unwrap()
        ));
    }
    assert_eq!(world.spatial_view().unwrap().binding_generation(), 0);
    assert_eq!(world.spatial_view().unwrap().topology_generation(), 0);
}
#[test]
fn pause_seek_reverse_loop_zero_length_and_rest_have_explicit_semantics() {
    let mut world = NativeWorld::new(scene());
    control(&mut world, 1, AnimationControl::SetPlaying(false));
    control(&mut world, 1, AnimationControl::Seek(0.75));
    world.step().unwrap();
    close(time(&world, 1), 0.75);
    control(&mut world, 1, AnimationControl::SetSpeed(-1.0));
    control(&mut world, 1, AnimationControl::SetPlaying(true));
    for _ in 0..60 {
        world.step().unwrap();
    }
    close(time(&world, 1), 0.0);
    let stopped = world.animation_state(id(1)).unwrap();
    assert!(stopped.completed && !stopped.playing);
    control(&mut world, 1, AnimationControl::SetLooping(true));
    control(&mut world, 1, AnimationControl::SetPlaying(true));
    world.step().unwrap();
    close(time(&world, 1), 59.0 / 60.0);
    control(
        &mut world,
        1,
        AnimationControl::Select(AnimationSample {
            clip: Some(1),
            seconds: 0.0,
        }),
    );
    world.step().unwrap();
    assert!(world.animation_state(id(1)).unwrap().playing);
    close(time(&world, 1), 0.0);
    control(&mut world, 1, AnimationControl::SetLooping(false));
    world.step().unwrap();
    assert!(world.animation_state(id(1)).unwrap().completed);
    control(
        &mut world,
        1,
        AnimationControl::Select(AnimationSample::REST),
    );
    control(&mut world, 1, AnimationControl::SetPlaying(true));
    world.step().unwrap();
    assert_eq!(
        world.animation_state(id(1)).unwrap().sample,
        AnimationSample::REST
    );
}
#[test]
fn invalid_controls_and_mixed_structural_batches_publish_no_prefix() {
    let mut world = NativeWorld::new(scene());
    world.step().unwrap();
    let before = world.save_bytes().unwrap();
    for bad in [
        AnimationControl::Seek(f64::NAN),
        AnimationControl::Seek(-1.0),
        AnimationControl::Seek(1.01),
        AnimationControl::SetSpeed(f64::INFINITY),
        AnimationControl::SetSpeed(16.1),
        AnimationControl::Select(AnimationSample {
            clip: Some(2),
            seconds: 0.0,
        }),
    ] {
        let mut commands = FrameCommands::default();
        commands
            .push(StructuralCommand::ControlAnimation {
                entity: id(1),
                control: AnimationControl::Seek(0.5),
            })
            .unwrap();
        commands
            .push(StructuralCommand::ControlAnimation {
                entity: id(2),
                control: bad,
            })
            .unwrap();
        assert!(world.apply(&mut commands).is_err());
        assert_eq!(commands.len(), 2);
        assert_eq!(world.save_bytes().unwrap(), before);
    }
    let mut commands = FrameCommands::default();
    commands
        .push(StructuralCommand::SetSpatial {
            entity: id(1),
            spatial: Some(node(1, definition())),
        })
        .unwrap();
    commands
        .push(StructuralCommand::ControlAnimation {
            entity: id(1),
            control: AnimationControl::Seek(0.5),
        })
        .unwrap();
    commands
        .push(StructuralCommand::ControlAnimation {
            entity: id(2),
            control: AnimationControl::Seek(2.0),
        })
        .unwrap();
    assert!(world.apply(&mut commands).is_err());
    assert_eq!(world.save_bytes().unwrap(), before);
    commands.clear();
    commands.push(StructuralCommand::Despawn(id(2))).unwrap();
    commands
        .push(StructuralCommand::ControlAnimation {
            entity: id(2),
            control: AnimationControl::SetPlaying(false),
        })
        .unwrap();
    assert!(world.apply(&mut commands).is_err());
    assert_eq!(world.save_bytes().unwrap(), before);
}
#[test]
fn changed_render_flags_retain_phase_replacement_resets_and_new_controls_use_final_binding() {
    let mut world = NativeWorld::new(scene());
    for _ in 0..12 {
        world.step().unwrap();
    }
    let mut spatial = node(1, definition());
    spatial.mesh.as_mut().unwrap().cast_shadows = false;
    let mut commands = FrameCommands::default();
    commands
        .push(StructuralCommand::SetSpatial {
            entity: id(1),
            spatial: Some(spatial),
        })
        .unwrap();
    world.apply(&mut commands).unwrap();
    close(time(&world, 1), 0.2);
    let mut changed = (*definition()).clone();
    changed.initial.speed = 2.0;
    commands
        .push(StructuralCommand::SetSpatial {
            entity: id(1),
            spatial: Some(node(1, Arc::new(changed))),
        })
        .unwrap();
    commands
        .push(StructuralCommand::ControlAnimation {
            entity: id(1),
            control: AnimationControl::Seek(0.4),
        })
        .unwrap();
    world.apply(&mut commands).unwrap();
    world.step().unwrap();
    close(time(&world, 1), 0.4 + 2.0 / 60.0);
}
#[test]
fn save_restore_continues_exact_phase_and_all_playback_flags() {
    let initial = scene();
    let mut uninterrupted = NativeWorld::new(initial.clone());
    for n in [1, 2] {
        control(&mut uninterrupted, n, AnimationControl::SetLooping(true));
    }
    control(&mut uninterrupted, 2, AnimationControl::Seek(0.75));
    control(&mut uninterrupted, 2, AnimationControl::SetSpeed(-0.75));
    for _ in 0..13 {
        uninterrupted.step().unwrap();
    }
    let saved = uninterrupted.save_bytes().unwrap();
    assert_eq!(u32::from_le_bytes(saved[8..12].try_into().unwrap()), 4);
    let mut resumed = NativeWorld::from_save_bytes_for_scene(&saved, &initial).unwrap();
    assert_eq!(resumed.save_bytes().unwrap(), saved);
    for _ in 0..137 {
        uninterrupted.step().unwrap();
        resumed.step().unwrap();
    }
    assert_eq!(
        resumed.save_bytes().unwrap(),
        uninterrupted.save_bytes().unwrap()
    );
}
#[test]
fn asset_binding_and_duration_tables_reject_before_world_publication() {
    for field in 0..5 {
        let mut bad = (*definition()).clone();
        match field {
            0 => bad.mesh_fingerprint = [5; 32],
            1 => bad.clip_durations = Arc::from([f32::NAN]),
            2 => bad.clip_durations = Arc::from([-0.0]),
            3 => bad.initial.seconds = 2.0,
            _ => bad.clip_durations = vec![1.0; 257].into(),
        }
        let mut nodes = scene().spatial().to_vec();
        nodes[0].animation = Some(Arc::new(bad));
        assert!(
            scene()
                .with_spatial(scene().visual_assets().to_vec(), nodes)
                .is_err()
        );
    }
}

#[test]
fn localized_v4_restores_in_place_and_resumes_exactly() {
    use incant_localization::LocaleSettings;
    let initial = scene()
        .with_localization(
            NativeLocalization::new(
                Default::default(),
                LocaleSettings {
                    locale: "fr".into(),
                    ..Default::default()
                },
            )
            .unwrap(),
        )
        .unwrap();
    let mut baseline = NativeWorld::new(initial.clone());
    control(&mut baseline, 1, AnimationControl::SetLooping(true));
    control(&mut baseline, 2, AnimationControl::Seek(0.75));
    control(&mut baseline, 2, AnimationControl::SetSpeed(-0.75));
    let locale = LocaleSettings {
        locale: "de".into(),
        fallbacks: vec!["fr".into()],
        pseudo: true,
    };
    let mut commands = FrameCommands::default();
    commands
        .push(StructuralCommand::SetLocale(locale.clone()))
        .unwrap();
    baseline.apply(&mut commands).unwrap();
    for _ in 0..13 {
        baseline.step().unwrap();
    }
    let saved = baseline.save_bytes().unwrap();
    assert_eq!(&saved[8..12], &4u32.to_le_bytes());
    let mut fresh = NativeWorld::from_save_bytes_for_scene(&saved, &initial).unwrap();
    let mut restored = NativeWorld::new(initial);
    let identity = restored.spatial_view().unwrap().identity().clone();
    let incarnation = restored.incarnation();
    restored.step().unwrap();
    restored.restore_bytes(&saved).unwrap();
    assert_eq!(restored.spatial_view().unwrap().identity(), &identity);
    assert_ne!(restored.incarnation(), incarnation);
    assert_eq!(restored.localization().settings(), &locale);
    assert_eq!(restored.save_bytes().unwrap(), saved);
    for _ in 0..137 {
        baseline.step().unwrap();
        fresh.step().unwrap();
        restored.step().unwrap();
        assert_eq!(
            restored.save_bytes().unwrap(),
            baseline.save_bytes().unwrap()
        );
        assert_eq!(fresh.save_bytes().unwrap(), baseline.save_bytes().unwrap());
    }
}

#[test]
fn localized_v4_restore_keeps_legacy_and_wrong_binding_refusals_atomic() {
    use incant_localization::LocaleSettings;
    use sha2::{Digest, Sha256};
    let initial = scene()
        .with_localization(
            NativeLocalization::new(
                Default::default(),
                LocaleSettings {
                    locale: "fr".into(),
                    ..Default::default()
                },
            )
            .unwrap(),
        )
        .unwrap();
    let mut live = NativeWorld::new(initial.clone());
    live.step().unwrap();
    let saved = live.save_bytes().unwrap();
    let incarnation = live.incarnation();
    let plain = CookedScene::new(id(0), 60, initial.entities().to_vec()).unwrap();
    let mut v2 = NativeWorld::new(plain.clone()).save_bytes().unwrap();
    v2[8..12].copy_from_slice(&2u32.to_le_bytes());
    let mut v1 = v2[..64].to_vec();
    v1[8..12].copy_from_slice(&1u32.to_le_bytes());
    v1.extend(plain.to_bytes());
    for mut legacy in [v1, v2] {
        let size = (legacy.len() - 64) as u64;
        legacy[24..32].copy_from_slice(&size.to_le_bytes());
        let mut h = Sha256::new();
        h.update(&legacy[..32]);
        h.update(&legacy[64..]);
        legacy[32..64].copy_from_slice(&h.finalize());
        assert!(NativeWorld::from_save_bytes(&legacy).is_ok());
        assert!(matches!(
            live.restore_bytes(&legacy),
            Err(SaveError::LocalizationUnsupported)
        ));
        assert_eq!(live.save_bytes().unwrap(), saved);
        assert_eq!(live.incarnation(), incarnation);
    }
    let mut changed = (*definition()).clone();
    changed.clip_durations = Arc::from([2., 0.]);
    let foreign = initial
        .clone()
        .with_spatial(
            initial.visual_assets().to_vec(),
            vec![node(1, Arc::new(changed)), node(2, definition())],
        )
        .unwrap();
    let wrong = NativeWorld::new(foreign).save_bytes().unwrap();
    assert!(matches!(
        live.restore_bytes(&wrong),
        Err(SaveError::WorldMismatch)
    ));
    assert_eq!(live.save_bytes().unwrap(), saved);
    assert_eq!(live.incarnation(), incarnation);
    let mut future = saved.clone();
    future[8..12].copy_from_slice(&12u32.to_le_bytes());
    let mut h = Sha256::new();
    h.update(&future[..32]);
    h.update(&future[64..]);
    future[32..64].copy_from_slice(&h.finalize());
    assert!(matches!(
        live.restore_bytes(&future),
        Err(SaveError::Version(12))
    ));
    assert_eq!(live.save_bytes().unwrap(), saved);
    assert_eq!(live.incarnation(), incarnation);
    live.step().unwrap();
    assert_eq!(live.tick(), 2);
}

#[test]
fn failed_tick_never_advances_animation() {
    let mut world = NativeWorld::new(scene());
    world.step().unwrap();
    let before = world.animation_state(id(1));
    let tick = world.tick();
    world.with_numeric_columns(|columns| columns.for_each_chunk(|c| c.transforms[0] = f64::NAN));
    assert!(world.step().is_err());
    assert_eq!(world.tick(), tick);
    assert_eq!(world.animation_state(id(1)), before);
}

#[test]
fn checksum_valid_malformed_animation_scene_and_save_data_refuse() {
    use sha2::{Digest, Sha256};
    let original = scene().to_bytes();
    // Two records of 71 fixed bytes and two f32 durations each.
    let start = original.len() - 2 * 79;
    for offset in [start - 4, start + 48, start + 71] {
        let mut bytes = original.clone();
        bytes[offset..offset + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        let mut hash = Sha256::new();
        hash.update(&bytes[..40]);
        hash.update(&bytes[72..]);
        bytes[40..72].copy_from_slice(&hash.finalize());
        assert!(CookedScene::from_bytes(&bytes).is_err());
    }
    let original = NativeWorld::new(scene()).save_bytes().unwrap();
    let start = original.len() - 2 * 36;
    for kind in 0..4 {
        let mut bytes = original.clone();
        match kind {
            0 => bytes[start + 20..start + 28].copy_from_slice(&f64::NAN.to_le_bytes()),
            1 => bytes[start + 28..start + 36].copy_from_slice(&17.0f64.to_le_bytes()),
            2 => bytes[start + 18] = 5, // playing and completed
            _ => bytes[start + 16..start + 18].copy_from_slice(&2u16.to_le_bytes()),
        }
        let mut hash = Sha256::new();
        hash.update(&bytes[..32]);
        hash.update(&bytes[64..]);
        bytes[32..64].copy_from_slice(&hash.finalize());
        assert!(NativeWorld::from_save_bytes(&bytes).is_err());
    }
}

#[path = "animation/crossfade.rs"]
mod crossfade;

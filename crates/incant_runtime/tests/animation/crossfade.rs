use super::*;

fn fade(target: AnimationSample, duration: f64) -> AnimationControl {
    AnimationControl::Crossfade { target, duration }
}
fn clip(seconds: f64) -> AnimationSample {
    AnimationSample {
        clip: Some(0),
        seconds,
    }
}
fn state(world: &NativeWorld) -> AnimationState {
    world.animation_state(id(1)).unwrap()
}

#[test]
fn independent_outgoing_clock_destination_weight_and_exact_endpoint() {
    let mut world = NativeWorld::new(scene());
    control(&mut world, 1, AnimationControl::Seek(0.9));
    control(&mut world, 1, AnimationControl::SetLooping(true));
    control(&mut world, 1, fade(clip(0.75), 0.5));
    assert_eq!(
        state(&world).blend_sample(),
        Some(AnimationBlendSample {
            sample: clip(0.9),
            weight: 0.0
        })
    );
    control(&mut world, 1, AnimationControl::SetSpeed(-0.5));
    control(&mut world, 1, AnimationControl::SetLooping(false));
    for _ in 0..15 {
        world.step().unwrap();
    }
    let sample = state(&world);
    close(sample.sample.seconds, 0.625);
    let transition = sample.transition.unwrap();
    close(transition.source.seconds, 0.15);
    assert_eq!(transition.speed, 1.0);
    assert!(transition.looping && transition.playing);
    close(sample.blend_sample().unwrap().weight, 0.5);
    assert_eq!(world.spatial_view().unwrap().binding_generation(), 0);
    for _ in 0..15 {
        world.step().unwrap();
    }
    close(state(&world).sample.seconds, 0.5);
    assert!(state(&world).transition.is_none());
    assert!(state(&world).playing);
}

#[test]
fn pause_freezes_both_clocks_and_seek_select_or_zero_duration_cancel() {
    let mut world = NativeWorld::new(scene());
    for cancel in [
        AnimationControl::Seek(0.25),
        AnimationControl::Select(clip(0.25)),
        fade(clip(0.25), 0.0),
    ] {
        control(&mut world, 1, AnimationControl::SetPlaying(true));
        control(&mut world, 1, fade(clip(0.5), 1.0));
        world.step().unwrap();
        control(&mut world, 1, AnimationControl::SetPlaying(false));
        let paused = state(&world);
        for _ in 0..10 {
            world.step().unwrap();
        }
        assert_eq!(state(&world), paused);
        control(&mut world, 1, AnimationControl::SetPlaying(true));
        world.step().unwrap();
        assert!(state(&world).transition.unwrap().elapsed > paused.transition.unwrap().elapsed);
        control(&mut world, 1, cancel);
        assert_eq!(state(&world).sample, clip(0.25));
        assert!(state(&world).transition.is_none());
    }
}

#[test]
fn ended_destination_does_not_freeze_fade_and_rest_remains_live() {
    for (target, completed) in [
        (clip(0.99), true),
        (
            AnimationSample {
                clip: Some(1),
                seconds: 0.0,
            },
            true,
        ),
        (AnimationSample::REST, false),
    ] {
        let mut world = NativeWorld::new(scene());
        control(&mut world, 1, fade(target, 0.5));
        for _ in 0..15 {
            world.step().unwrap();
        }
        assert!(state(&world).playing && !state(&world).completed);
        close(state(&world).blend_sample().unwrap().weight, 0.5);
        for _ in 0..15 {
            world.step().unwrap();
        }
        let value = state(&world);
        assert!(value.transition.is_none());
        assert_eq!(value.completed, completed);
        assert_eq!(value.playing, !completed);
    }
}

#[test]
fn outgoing_nonloop_end_freezes_its_pose_while_destination_runs() {
    let mut world = NativeWorld::new(scene());
    control(&mut world, 1, AnimationControl::Seek(0.99));
    control(&mut world, 1, fade(clip(0.0), 0.5));
    for _ in 0..3 {
        world.step().unwrap();
    }
    let value = state(&world);
    let source = value.transition.unwrap();
    assert_eq!(source.source, clip(1.0));
    assert!(!source.playing);
    assert!(value.playing && !value.completed);
    close(value.sample.seconds, 0.05);
    // Fade duration is real fixed-step time, independent of destination speed.
    control(&mut world, 1, AnimationControl::SetSpeed(0.0));
    for _ in 0..27 {
        world.step().unwrap();
    }
    assert!(state(&world).transition.is_none());
    close(state(&world).sample.seconds, 0.05);
}

#[test]
fn invalid_crossfades_and_active_retarget_refuse_entire_batch() {
    let mut world = NativeWorld::new(scene());
    control(&mut world, 1, fade(clip(0.25), 1.0));
    world.step().unwrap();
    let saved = world.save_bytes().unwrap();
    for bad in [
        fade(clip(0.5), 0.1),
        fade(clip(0.5), -1.0),
        fade(clip(0.5), 60.01),
        fade(clip(0.5), f64::NAN),
        fade(clip(0.5), f64::INFINITY),
        fade(clip(1.01), 0.0),
        fade(
            AnimationSample {
                clip: Some(2),
                seconds: 0.0,
            },
            0.0,
        ),
    ] {
        let mut commands = FrameCommands::default();
        commands
            .push(StructuralCommand::ControlAnimation {
                entity: id(2),
                control: AnimationControl::Seek(0.5),
            })
            .unwrap();
        commands
            .push(StructuralCommand::ControlAnimation {
                entity: id(1),
                control: bad,
            })
            .unwrap();
        assert!(world.apply(&mut commands).is_err());
        assert_eq!(commands.len(), 2);
        assert_eq!(world.save_bytes().unwrap(), saved);
    }
}

#[test]
fn v5_restores_active_and_inactive_rows_then_returns_to_unchanged_v4_encoding() {
    let initial = scene();
    let mut baseline = NativeWorld::new(initial.clone());
    control(&mut baseline, 1, AnimationControl::SetLooping(true));
    control(&mut baseline, 1, fade(clip(0.75), 0.5));
    control(&mut baseline, 1, AnimationControl::SetSpeed(-0.5));
    for _ in 0..7 {
        baseline.step().unwrap();
    }
    let saved = baseline.save_bytes().unwrap();
    assert_eq!(&saved[8..12], &5u32.to_le_bytes());
    let mut fresh = NativeWorld::from_save_bytes_for_scene(&saved, &initial).unwrap();
    let mut live = NativeWorld::new(initial);
    live.step().unwrap();
    let identity = live.spatial_view().unwrap().identity().clone();
    live.restore_bytes(&saved).unwrap();
    assert_eq!(live.spatial_view().unwrap().identity(), &identity);
    assert_eq!(live.save_bytes().unwrap(), saved);
    assert_eq!(fresh.save_bytes().unwrap(), saved);
    for _ in 0..50 {
        for world in [&mut baseline, &mut fresh, &mut live] {
            world.step().unwrap();
        }
        assert_eq!(fresh.save_bytes().unwrap(), baseline.save_bytes().unwrap());
        assert_eq!(live.save_bytes().unwrap(), baseline.save_bytes().unwrap());
    }
    assert_eq!(&baseline.save_bytes().unwrap()[8..12], &4u32.to_le_bytes());
}

fn rehash(bytes: &mut [u8]) {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(&bytes[..32]);
    h.update(&bytes[64..]);
    bytes[32..64].copy_from_slice(&h.finalize());
}

#[test]
fn paused_start_captures_a_still_source_and_failed_tick_preserves_the_fade() {
    let mut world = NativeWorld::new(scene());
    control(&mut world, 1, AnimationControl::SetPlaying(false));
    control(&mut world, 1, AnimationControl::Select(clip(0.4)));
    control(&mut world, 1, fade(clip(0.25), 1.0));
    control(&mut world, 1, AnimationControl::SetPlaying(true));
    world.step().unwrap();
    let before = state(&world);
    assert_eq!(before.transition.unwrap().source, clip(0.4));
    assert!(!before.transition.unwrap().playing);
    close(before.sample.seconds, 0.25 + 1.0 / 60.0);
    let tick = world.tick();
    world.with_numeric_columns(|columns| columns.for_each_chunk(|c| c.transforms[0] = f64::NAN));
    assert!(world.step().is_err());
    assert_eq!(world.tick(), tick);
    assert_eq!(state(&world), before);
}

#[test]
fn longest_transition_finishes_on_its_integral_tick_at_high_tick_rates() {
    let old = scene();
    for rate in [60, 120, 216, 240] {
        let scene = CookedScene::new(old.id(), rate, old.entities().to_vec())
            .unwrap()
            .with_spatial(old.visual_assets().to_vec(), old.spatial().to_vec())
            .unwrap();
        let mut world = NativeWorld::new(scene);
        control(&mut world, 1, AnimationControl::SetLooping(true));
        control(&mut world, 1, fade(clip(0.25), 60.0));
        for _ in 0..rate * 60 - 1 {
            world.step().unwrap();
        }
        assert!(state(&world).transition.is_some(), "{rate}Hz ended early");
        world.step().unwrap();
        assert!(state(&world).transition.is_none(), "{rate}Hz ended late");
    }
}

#[test]
fn checksum_valid_malformed_v5_transitions_refuse_atomically() {
    let initial = scene();
    let mut live = NativeWorld::new(initial.clone());
    control(&mut live, 1, fade(clip(0.75), 0.5));
    live.step().unwrap();
    let saved = live.save_bytes().unwrap();
    let incarnation = live.incarnation();
    let start = saved.len() - 2 * 72;
    for kind in 0..14 {
        let mut bad = saved.clone();
        match kind {
            0 => bad[start + 38] = 8,
            1 => bad[start + 39] = 1,
            2 => bad[start + 36..start + 38].copy_from_slice(&2u16.to_le_bytes()),
            3 => bad[start + 40..start + 48].copy_from_slice(&f64::NAN.to_le_bytes()),
            4 => bad[start + 48..start + 56].copy_from_slice(&17f64.to_le_bytes()),
            5 => bad[start + 56..start + 64].copy_from_slice(&(-1f64).to_le_bytes()),
            6 => bad[start + 56..start + 64].copy_from_slice(&0.5f64.to_le_bytes()),
            7 => bad[start + 64..start + 72].copy_from_slice(&0f64.to_le_bytes()),
            8 => bad[start + 64..start + 72].copy_from_slice(&61f64.to_le_bytes()),
            9 => bad[start + 64..start + 72].copy_from_slice(&f64::INFINITY.to_le_bytes()),
            10 => bad[start + 18] |= 4,
            11 => bad[start + 36..start + 72].fill(0), // v5 without a transition
            12 => bad[start + 72 + 40] = 1,            // inactive row must have a zero suffix
            _ => bad[start + 72..start + 88].fill(1),  // duplicate identity
        }
        rehash(&mut bad);
        assert!(NativeWorld::from_save_bytes(&bad).is_err(), "case {kind}");
        assert!(live.restore_bytes(&bad).is_err(), "case {kind}");
        assert_eq!(live.save_bytes().unwrap(), saved);
        assert_eq!(live.incarnation(), incarnation);
    }
    let mut other = initial.spatial().to_vec();
    let mut changed = (*definition()).clone();
    changed.clip_durations = Arc::from([2.0, 0.0]);
    other[0].animation = Some(Arc::new(changed));
    let foreign = initial
        .clone()
        .with_spatial(initial.visual_assets().to_vec(), other)
        .unwrap();
    assert!(matches!(
        NativeWorld::from_save_bytes_for_scene(&saved, &foreign),
        Err(SaveError::WorldMismatch)
    ));
}

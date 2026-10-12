use super::*;
use incant_types::PoseInterpolation;

#[test]
fn compound_fade_retains_both_motion_clocks_shared_parameters_pause_and_exact_restore() {
    let g = graph(vec![edge(30, 0, 2, 0, 0.5), edge(31, 2, 1, 0, 0.5)]);
    let scene = scene(g);
    let bytes = scene.to_bytes();
    assert_eq!(
        &bytes[8..12],
        &crate::scene_format::scene_format(18).to_le_bytes()
    );
    assert_eq!(CookedScene::from_bytes(&bytes).unwrap().to_bytes(), bytes);
    let mut world = NativeWorld::from_bytes(&bytes).unwrap();
    parameter(&mut world, 0.5);
    world.step().unwrap();
    let start = snapshot(&world);
    assert_eq!(start.state, id(22));
    assert_eq!(start.fade.unwrap().state, id(20));
    close(start.fade.unwrap().seconds, 0.125);
    let pose = start.pose.unwrap();
    assert_eq!(pose.outer_weight(), Some(0.0));
    assert_eq!(
        pose.group(0).unwrap().interpolation,
        PoseInterpolation::Clip
    );
    assert_eq!(
        pose.group(1).unwrap().interpolation,
        PoseInterpolation::SphericalPair
    );
    assert!(start.blend.is_none());
    assert_eq!(
        world.animation_state(id(1)).unwrap().sample,
        pose.representative()
    );
    master(&mut world, AnimationControl::SetSpeed(2.0));
    world.step().unwrap();
    let moving = snapshot(&world);
    close(moving.seconds, 0.25);
    close(moving.fade.unwrap().seconds, 0.25);
    assert_eq!(moving.fade.unwrap().speed, 1.0);
    assert_eq!(moving.pose.unwrap().outer_weight(), Some(0.25));
    parameter(&mut world, 0.75);
    close(
        snapshot(&world).pose.unwrap().group(1).unwrap().samples[1].weight,
        0.75,
    );
    master(&mut world, AnimationControl::SetPlaying(false));
    let paused = snapshot(&world);
    for _ in 0..3 {
        world.step().unwrap();
    }
    assert_eq!(snapshot(&world), paused);
    let save = world.save_bytes().unwrap();
    assert_eq!(&save[8..12], &10u32.to_le_bytes());
    let mut restored = NativeWorld::from_save_bytes(&save).unwrap();
    assert_eq!(restored.save_bytes().unwrap(), save);
    for w in [&mut world, &mut restored] {
        master(w, AnimationControl::SetPlaying(true));
    }
    for _ in 0..12 {
        world.step().unwrap();
        restored.step().unwrap();
        assert_eq!(world.save_bytes().unwrap(), restored.save_bytes().unwrap());
        assert_eq!(snapshot(&world), snapshot(&restored));
    }
    assert_eq!(snapshot(&world).state, id(21));
    assert!(snapshot(&world).fade.is_none());
}

#[test]
fn complete_and_reverse_sources_manual_cancel_and_failed_batch_preserve_compound_state() {
    let mut world = NativeWorld::new(scene(graph(vec![edge(30, 2, 1, 0, 0.5)])));
    select(&mut world, 22);
    parameter(&mut world, 0.75);
    master(&mut world, AnimationControl::SetSpeed(-1.0));
    world.step().unwrap();
    let initial = snapshot(&world);
    assert_eq!(initial.fade.unwrap().state, id(22));
    close(initial.fade.unwrap().seconds, 1.875);
    world.step().unwrap();
    close(snapshot(&world).fade.unwrap().seconds, 1.75);
    close(snapshot(&world).seconds, 3.875);
    let saved = world.save_bytes().unwrap();
    let mut commands = FrameCommands::default();
    for value in [0.25, 99.0] {
        commands
            .push(StructuralCommand::ControlAnimationGraph {
                entity: id(1),
                control: AnimationGraphControl::SetParameter {
                    parameter: id(10),
                    value,
                },
            })
            .unwrap();
    }
    assert!(world.apply(&mut commands).is_err());
    assert_eq!(world.save_bytes().unwrap(), saved);
    select(&mut world, 20);
    assert!(snapshot(&world).fade.is_none());
    assert!(snapshot(&world).pose.is_none());
    assert_eq!(snapshot(&world).seconds, 0.0);
}

#[test]
fn completed_incoming_motion_does_not_stop_fade_or_lose_its_checkpoint() {
    let template = graph(vec![]);
    let mut states = template.states().to_vec();
    states[1].looping = false;
    let mut outgoing = edge(30, 2, 1, 0, 0.5);
    outgoing.conditions = Arc::from([]);
    let g = NativeAnimationGraph::new(
        0,
        template.parameters().to_vec(),
        states,
        vec![outgoing],
        &[2.0, 4.0],
    )
    .unwrap();
    let mut world = NativeWorld::new(scene(g));
    select(&mut world, 22);
    master(&mut world, AnimationControl::SetSpeed(16.0));
    world.step().unwrap(); // Enter the nonlooping clip, outgoing tree continues.
    world.step().unwrap();
    world.step().unwrap(); // Incoming clip reaches its end in two ticks.
    assert!(world.animation_state(id(1)).unwrap().completed);
    assert!(!world.animation_state(id(1)).unwrap().playing);
    assert!(snapshot(&world).fade.is_some());
    let mut resumed = NativeWorld::from_save_bytes(&world.save_bytes().unwrap()).unwrap();
    for _ in 0..3 {
        world.step().unwrap();
        resumed.step().unwrap();
    }
    assert!(snapshot(&world).fade.is_none());
    assert!(world.animation_state(id(1)).unwrap().completed);
    assert_eq!(world.save_bytes().unwrap(), resumed.save_bytes().unwrap());
}

#[test]
fn recomputed_checksum_malformed_fades_and_downgraded_scene_refuse_without_changes() {
    let initial = scene(graph(vec![edge(30, 0, 2, 0, 0.5)]));
    let mut world = NativeWorld::new(initial.clone());
    parameter(&mut world, 0.5);
    world.step().unwrap();
    world.step().unwrap();
    let save = world.save_bytes().unwrap();
    let fade = save.len() - 40;
    for case in 0..12 {
        let mut bad = save.clone();
        match case {
            0 => bad[fade] = 0,
            1 => bad[fade] = 2,
            2 => bad[fade + 1] = 255,
            3 => bad[fade + 1] = 2, // Same state as incoming.
            4 => bad[fade + 2] = 2,
            5 => bad[fade + 3] = 1,
            6 => bad[fade + 8..fade + 16].copy_from_slice(&f64::NAN.to_le_bytes()),
            7 => bad[fade + 16..fade + 24].copy_from_slice(&65f64.to_le_bytes()),
            8 => bad[fade + 24..fade + 32].copy_from_slice(&0.5f64.to_le_bytes()),
            9 => bad[fade + 24..fade + 32].copy_from_slice(&0.1f64.to_le_bytes()),
            10 => bad[fade + 32..fade + 40].copy_from_slice(&0.75f64.to_le_bytes()),
            _ => bad[fade + 2] = 0, // Stopped looping source is impossible.
        }
        save_checksum(&mut bad);
        assert!(world.restore_bytes(&bad).is_err(), "case {case}");
        assert_eq!(world.save_bytes().unwrap(), save);
    }
    let scene = initial.to_bytes();
    for version in [14u32, 15, 16, 17] {
        let mut bad = scene.clone();
        bad[8..12].copy_from_slice(&version.to_le_bytes());
        scene_checksum(&mut bad);
        assert!(CookedScene::from_bytes(&bad).is_err());
    }
    for offset in [scene.len() - 8, scene.len() - 4] {
        let mut bad = scene.clone();
        bad[offset..offset + 4].copy_from_slice(&1u32.to_le_bytes());
        scene_checksum(&mut bad);
        assert!(CookedScene::from_bytes(&bad).is_err());
    }
}

#[test]
fn eight_sample_states_keep_independent_phase_and_normalization_through_fade() {
    let template = graph(vec![]);
    let parameters = template.parameters().to_vec();
    let mut nodes: Vec<_> = (0..8).map(|n| NativeBlendTreeNode::Clip(Some(n))).collect();
    for (a, b) in [(0, 1), (2, 3), (4, 5), (6, 7), (8, 9), (10, 11), (12, 13)] {
        nodes.push(NativeBlendTreeNode::Blend1d {
            parameter: 0,
            points: Arc::from([
                NativeBlendTreePoint {
                    threshold: 0.0,
                    child: a,
                },
                NativeBlendTreePoint {
                    threshold: 1.0,
                    child: b,
                },
            ]),
        });
    }
    let tree = Arc::new(
        NativeAnimationBlendTree::new(nodes, parameters.clone().into(), Arc::from([2.0; 8]))
            .unwrap(),
    );
    let mut states = template.states().to_vec();
    states[1].motion = NativeAnimationMotion::Tree {
        cycle_seconds: 2.0,
        tree: tree.clone(),
    };
    states[2].motion = NativeAnimationMotion::Tree {
        cycle_seconds: 4.0,
        tree,
    };
    let graph = NativeAnimationGraph::new(
        0,
        parameters,
        states,
        vec![edge(30, 1, 2, 0, 0.5)],
        &[2.0; 8],
    )
    .unwrap();
    let base = scene(super::graph(vec![]));
    let mut nodes = base.spatial().to_vec();
    let animation = Arc::make_mut(nodes[0].animation.as_mut().unwrap());
    animation.clip_durations = Arc::from([2.0; 8]);
    animation.graph = Some(Arc::new(graph));
    let scene = base
        .clone()
        .with_spatial(base.visual_assets().to_vec(), nodes)
        .unwrap();
    let mut world = NativeWorld::new(scene);
    select(&mut world, 21);
    parameter(&mut world, 0.5);
    world.step().unwrap();
    world.step().unwrap();
    let pose = snapshot(&world).pose.unwrap();
    assert_eq!(pose.samples().len(), 16);
    for group in 0..2 {
        let group = pose.group(group).unwrap();
        assert_eq!(group.interpolation, PoseInterpolation::Weighted);
        assert_eq!(group.samples.len(), 8);
        close(group.samples.iter().map(|s| s.weight).sum(), 1.0);
    }
    close(pose.group(0).unwrap().samples[0].sample.seconds, 0.25);
    close(pose.group(1).unwrap().samples[0].sample.seconds, 0.0625);
    let mut resumed = NativeWorld::from_save_bytes(&world.save_bytes().unwrap()).unwrap();
    for _ in 0..8 {
        world.step().unwrap();
        resumed.step().unwrap();
        assert_eq!(world.save_bytes().unwrap(), resumed.save_bytes().unwrap());
    }
}

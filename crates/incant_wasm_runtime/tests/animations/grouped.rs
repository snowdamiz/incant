use super::*;
use std::sync::Arc;
#[test]
fn legacy_pose_and_graph_protocols_refuse_compound_fades_atomically() {
    let base = graph_fixture::scene(0.5);
    let mut nodes = base.spatial().to_vec();
    for node in &mut nodes {
        let animation = Arc::make_mut(node.animation.as_mut().unwrap());
        let old = animation.graph.as_ref().unwrap();
        let mut states = old.states().to_vec();
        states.push(NativeAnimationGraphState {
            id: StableId([21; 16]),
            motion: NativeAnimationMotion::Clip(Some(0)),
            looping: true,
        });
        animation.graph = Some(Arc::new(
            NativeAnimationGraph::new(
                0,
                old.parameters().to_vec(),
                states,
                vec![NativeAnimationGraphTransition {
                    id: StableId([30; 16]),
                    from: 0,
                    to: 1,
                    priority: 0,
                    fade_seconds: 0.5,
                    after_seconds: None,
                    conditions: Arc::from([]),
                }],
                &[1.0, 2.0],
            )
            .unwrap(),
        ));
    }
    let initial = base
        .clone()
        .with_spatial(base.visual_assets().to_vec(), nodes)
        .unwrap();
    for call in [
        Call::AnimationLen,
        Call::ReadAnimations {
            ptr: 0,
            capacity: 8192,
        },
        Call::AnimationLenV2,
        Call::ReadAnimationsV2 {
            ptr: 0,
            capacity: 8192,
        },
        Call::AnimationLenV3,
        Call::ReadAnimationsV3 {
            ptr: 0,
            capacity: 8192,
        },
        Call::AnimationLenV4,
        Call::ReadAnimationsV4 {
            ptr: 0,
            capacity: 8192,
        },
        Call::AnimationIkLen,
        Call::ReadAnimationIk {
            ptr: 0,
            capacity: 8192,
        },
        Call::AnimationGraphLen,
        Call::ReadAnimationGraphs {
            ptr: 0,
            capacity: 8192,
        },
    ] {
        let mut world = NativeWorld::new(initial.clone());
        world.step().unwrap();
        let saved = world.save_bytes().unwrap();
        let mut cap = Capability::default();
        let mut memory = vec![0xa5; 8192];
        let epoch = cap.begin(&mut world, DT).unwrap();
        let untouched = memory.clone();
        assert!(matches!(
            cap.invoke(&mut memory, epoch, call),
            Err(BoundaryError::Abi(_))
        ));
        assert_eq!(memory, untouched);
        assert!(cap.finish(&mut world).is_err());
        assert_eq!(world.save_bytes().unwrap(), saved);
    }
}

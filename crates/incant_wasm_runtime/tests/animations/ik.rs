use super::*;
use std::sync::Arc;
#[test]
fn ik_pose_observations_refuse_without_memory_or_world_prefix() {
    let base = scene();
    let mut nodes = base.spatial().to_vec();
    let node = nodes.iter_mut().find(|n| n.animation.is_some()).unwrap();
    node.ik = Some(Arc::new(NativeIk {
        mesh_fingerprint: node.animation.as_ref().unwrap().mesh_fingerprint,
        initial: Arc::from([AnimationIkSample {
            id: id(9),
            chain: [0, 1, 2],
            target: [1., 1., 0.],
            pole: [0., 1., 0.],
            weight: 1.,
        }]),
    }));
    let initial = base
        .clone()
        .with_spatial(base.visual_assets().to_vec(), nodes)
        .unwrap();
    for call in pose_imports() {
        let mut world = NativeWorld::new(initial.clone());
        let before = world.save_bytes().unwrap();
        let mut cap = Capability::default();
        let mut memory = vec![0xa5; 2048];
        let epoch = cap.begin(&mut world, DT).unwrap();
        prefix(&mut cap, &mut memory, epoch);
        let untouched = memory.clone();
        assert!(matches!(
            cap.invoke(&mut memory, epoch, call),
            Err(BoundaryError::Abi(
                "animation snapshots cannot represent IK constraints"
            ))
        ));
        assert_eq!(memory, untouched);
        assert!(cap.finish(&mut world).is_err());
        assert_eq!(world.save_bytes().unwrap(), before);
    }
    // A module with no pose observation can still run against this world.
    let mut world = NativeWorld::new(initial);
    let mut cap = Capability::default();
    for _ in 0..2 {
        cap.begin(&mut world, DT).unwrap();
        cap.finish(&mut world).unwrap();
    }
    assert!(world.ik_state(id(1)).is_some());
}

// Extend this list together with the central admission for each newer pose ABI.
fn pose_imports() -> [Call; 8] {
    [
        Call::AnimationLen,
        Call::AnimationLenV2,
        Call::AnimationLenV3,
        Call::AnimationLenV4,
        Call::ReadAnimations {
            ptr: 7,
            capacity: 1024,
        },
        Call::ReadAnimationsV2 {
            ptr: 7,
            capacity: 1024,
        },
        Call::ReadAnimationsV3 {
            ptr: 7,
            capacity: 1024,
        },
        Call::ReadAnimationsV4 {
            ptr: 7,
            capacity: 1024,
        },
    ]
}

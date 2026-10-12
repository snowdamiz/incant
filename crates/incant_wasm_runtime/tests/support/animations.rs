use incant_runtime::*;
use incant_types::{Animation, Transform};
use std::sync::Arc;

pub fn id(n: u8) -> StableId {
    StableId([n; 16])
}

pub fn scene() -> CookedScene {
    let animation = Arc::new(NativeAnimation {
        mesh_fingerprint: [4; 32],
        clip_durations: Arc::from([1.0, 0.0]),
        initial: Animation {
            clip: Some(0),
            seconds: 0.0,
            speed: 1.0,
            playing: true,
            looping: false,
        },
    });
    let entities = [1, 2]
        .into_iter()
        .map(|n| CookedEntity {
            id: id(n),
            parent: None,
            transform: Transform::default(),
            velocity: None,
        })
        .collect();
    let nodes = [1, 2]
        .into_iter()
        .map(|n| {
            let mut node = CookedSpatial::empty(id(n));
            node.mesh = Some(NativeMeshRenderer {
                mesh: id(3),
                cast_shadows: true,
            });
            node.animation = Some(animation.clone());
            node
        })
        .collect();
    CookedScene::new(id(0), 60, entities)
        .unwrap()
        .with_spatial(
            vec![VisualAssetRef {
                id: id(3),
                kind: VisualAssetKind::Model,
                fingerprint: [4; 32],
            }],
            nodes,
        )
        .unwrap()
}

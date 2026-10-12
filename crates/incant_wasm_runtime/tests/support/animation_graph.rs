use incant_runtime::*;
use incant_types::{Animation, Transform};
use std::sync::Arc;
fn id(n: u8) -> StableId {
    StableId([n; 16])
}
pub fn scene(value: f64) -> CookedScene {
    let graph = NativeAnimationGraph::new(
        0,
        vec![NativeAnimationParameter {
            id: id(10),
            initial: value,
            min: 0.,
            max: 1.,
        }],
        vec![NativeAnimationGraphState {
            id: id(20),
            looping: true,
            motion: NativeAnimationMotion::Blend1d {
                parameter: 0,
                cycle_seconds: 1.,
                points: Arc::from([
                    NativeAnimationBlendPoint {
                        threshold: 0.,
                        clip: 0,
                    },
                    NativeAnimationBlendPoint {
                        threshold: 1.,
                        clip: 1,
                    },
                ]),
            },
        }],
        vec![],
        &[1., 2.],
    )
    .unwrap();
    let animation = Arc::new(NativeAnimation {
        graph: Some(Arc::new(graph)),
        mesh_fingerprint: [4; 32],
        clip_durations: Arc::from([1., 2.]),
        initial: Animation {
            clip: None,
            seconds: 0.,
            speed: 1.,
            playing: true,
            looping: true,
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
                cast_shadows: false,
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

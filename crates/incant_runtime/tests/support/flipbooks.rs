//! Shared typed data for actual QuickJS, Wasmtime and wasm2c conformance tests.
use incant_runtime::*;
use incant_types::Transform;
use std::sync::Arc;
pub fn id(n: u128) -> StableId {
    StableId(n.to_be_bytes())
}
pub fn scene(count: usize) -> CookedScene {
    let nodes = (1..=count)
        .map(|n| {
            let mut node = CookedSpatial::empty(id(n as u128));
            node.sprite = Some(Box::new(NativeSpriteRenderer {
                texture: id(u128::MAX),
                region: NativeSpriteRegion::Pixels([0, 0, 2, 2]),
                size: [1.; 2],
                pivot: [0.5; 2],
                tint: [1.; 4],
                flip_x: false,
                flip_y: false,
                layer: 0,
                order: 0,
                sampling: SpriteSampling::Nearest,
                mode: SpriteMode::Simple,
            }));
            node.flipbook = Some(Arc::new(NativeFlipbook {
                frames: (0..3)
                    .map(|x| NativeSpriteRegion::Pixels([x * 2, 0, 2, 2]))
                    .collect(),
                rate: FrameRate {
                    numerator: 16,
                    denominator: 1,
                },
                looping: n % 2 == 1,
                playing: true,
            }));
            node
        })
        .collect();
    CookedScene::new(
        id(0),
        64,
        (1..=count)
            .map(|n| CookedEntity {
                id: id(n as u128),
                parent: None,
                transform: Transform::default(),
                velocity: None,
            })
            .collect(),
    )
    .unwrap()
    .with_spatial(
        vec![VisualAssetRef {
            id: id(u128::MAX),
            kind: VisualAssetKind::Texture,
            fingerprint: [4; 32],
        }],
        nodes,
    )
    .unwrap()
}

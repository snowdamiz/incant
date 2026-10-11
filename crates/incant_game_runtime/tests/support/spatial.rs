use incant_runtime::{
    CookedEntity, CookedScene, CookedSpatial, StableId, VisualAssetKind, VisualAssetRef,
};
use incant_types::{Camera, CameraProjection, Transform};
pub fn scenes() -> [CookedScene; 3] {
    let id = StableId(1u128.to_be_bytes());
    let plain = CookedScene::new(
        StableId(99u128.to_be_bytes()),
        60,
        vec![CookedEntity {
            id,
            parent: None,
            transform: Transform::default(),
            velocity: None,
        }],
    )
    .unwrap();
    let mut camera = CookedSpatial::empty(id);
    camera.camera = Some(Camera {
        fov_degrees: 60.,
        near: 0.1,
        far: 100.,
        projection: CameraProjection::Perspective {},
    });
    [
        plain.clone(),
        plain.clone().with_spatial(vec![], vec![camera]).unwrap(),
        plain
            .with_spatial(
                vec![VisualAssetRef {
                    id: StableId(90u128.to_be_bytes()),
                    kind: VisualAssetKind::Model,
                    fingerprint: [7; 32],
                }],
                vec![],
            )
            .unwrap(),
    ]
}

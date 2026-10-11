#![allow(dead_code)]
use incant_render_native::{NativeRenderInput, VisualCaches};
use incant_runtime::{
    CookedEntity, CookedScene, CookedSpatial, FrameCommands, NativeWorld, StableId,
    StructuralCommand,
};
use incant_types::{Camera, CameraProjection, PointLight, Transform, Velocity};
use std::path::Path;
pub fn id(n: u128) -> StableId {
    StableId(n.to_be_bytes())
}
pub fn entity(n: u128, parent: Option<u128>) -> CookedEntity {
    CookedEntity {
        id: id(n),
        parent: parent.map(id),
        transform: Transform {
            translation: [n as f64 + 0.125, 0., 0.],
            ..Default::default()
        },
        velocity: Some(Velocity {
            linear: [60., 0., 0.],
        }),
    }
}
pub fn camera() -> Camera {
    Camera {
        fov_degrees: 60.,
        near: 0.1,
        far: 100.,
        projection: CameraProjection::Perspective {},
    }
}
pub fn declaration(n: u128) -> CookedSpatial {
    let mut d = CookedSpatial::empty(id(n));
    if n == 2 {
        d.camera = Some(camera());
    } else {
        d.point = Some(PointLight {
            color: [1.; 3],
            intensity: 2.,
            range: 4.,
        });
    }
    d
}
pub fn scene() -> CookedScene {
    CookedScene::new(
        id(99),
        60,
        vec![entity(1, None), entity(2, Some(1)), entity(3, None)],
    )
    .unwrap()
    .with_spatial(vec![], vec![declaration(2), declaration(3)])
    .unwrap()
}
pub fn world() -> NativeWorld {
    NativeWorld::from_bytes(&scene().to_bytes()).unwrap()
}
pub fn load(w: &NativeWorld) -> NativeRenderInput {
    NativeRenderInput::load(
        &w.spatial_view().unwrap(),
        VisualCaches {
            models: Path::new("absent-model-cache"),
            textures: Path::new("absent-texture-cache"),
        },
        id(2),
    )
    .unwrap()
}
pub fn queue(commands: impl IntoIterator<Item = StructuralCommand>) -> FrameCommands {
    let mut q = FrameCommands::default();
    for c in commands {
        q.push(c).unwrap();
    }
    q
}
pub fn matrices(a: &NativeRenderInput) -> Vec<[f64; 16]> {
    a.input()
        .nodes
        .iter()
        .map(|n| n.world.to_cols_array())
        .collect()
}
pub fn generations(a: &NativeRenderInput) -> (u64, u64, u64) {
    (a.tick(), a.topology_generation(), a.binding_generation())
}

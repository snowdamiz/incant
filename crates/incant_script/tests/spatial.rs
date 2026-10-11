#![cfg(feature = "compiler")]
use incant_runtime::{CookedEntity, CookedScene, CookedSpatial, NativeWorld, StableId};
use incant_script::{CompiledScript, ScriptHost};
use incant_types::{Camera, CameraProjection, Transform};
fn id(n: u128) -> StableId {
    StableId(n.to_be_bytes())
}
fn world() -> NativeWorld {
    let mut spatial = CookedSpatial::empty(id(2));
    spatial.camera = Some(Camera {
        fov_degrees: 60.,
        near: 0.1,
        far: 100.,
        projection: CameraProjection::Perspective {},
    });
    NativeWorld::new(
        CookedScene::new(
            id(99),
            60,
            (1..=2)
                .map(|n| CookedEntity {
                    id: id(n),
                    parent: (n == 2).then_some(id(1)),
                    transform: Transform::default(),
                    velocity: None,
                })
                .collect(),
        )
        .unwrap()
        .with_spatial(vec![], vec![spatial])
        .unwrap(),
    )
}
fn observe(w: &NativeWorld) -> Vec<(StableId, Transform, [f64; 16], Option<CookedSpatial>)> {
    w.spatial_view()
        .unwrap()
        .nodes()
        .map(|n| {
            (
                n.id,
                n.local.clone(),
                n.global.to_cols_array(),
                n.declaration.cloned(),
            )
        })
        .collect()
}
#[test]
fn script_failures_restore_committed_spatial_view_after_detachment_and_rejected_sync() {
    for tail in [
        "throw Error('late');",
        "new Promise(()=>{});",
        "commands.setParent('00000000000000000000000001','00000000000000000000000002');",
    ] {
        let code=CompiledScript::compile(&format!("exports.default=defineBehavior({{update({{chunks,commands}}){{for(const c of chunks)c.transforms[0]=42; {tail}}}}});")).unwrap();
        let mut host = ScriptHost::new(code).unwrap();
        let mut world = world();
        let before = observe(&world);
        assert!(host.tick(&mut world, 1. / 60.).is_err());
        assert_eq!(observe(&world), before);
        let view = world.spatial_view().unwrap();
        assert_eq!(view.topology_generation(), 0);
        assert_eq!(view.binding_generation(), 0);
    }
}
#[test]
fn script_writes_keep_f64_continuity_without_invalidating_binding_indices() {
    let code=CompiledScript::compile("exports.default=defineBehavior({update({chunks}){for(const c of chunks){for(let i=0;i<c.transforms.length;i+=c.transformStride)c.transforms[i]+=0.000000001;}}});").unwrap();
    let mut host = ScriptHost::new(code).unwrap();
    let mut world = world();
    for _ in 0..10 {
        host.tick(&mut world, 1. / 60.).unwrap();
    }
    let view = world.spatial_view().unwrap();
    let child = view.nodes().find(|n| n.id == id(2)).unwrap();
    assert!((child.local.translation[0] - 1e-8).abs() < 1e-22);
    assert!((child.global.w_axis.x - 2e-8).abs() < 1e-22);
    assert!(child.declaration.unwrap().camera.is_some());
    assert_eq!(view.topology_generation(), 0);
    assert_eq!(view.binding_generation(), 0);
}

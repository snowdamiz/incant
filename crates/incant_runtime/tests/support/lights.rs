use incant_runtime::*;
use incant_types::{
    DirectionalLight, DirectionalShadows, PointLight, SpotLight, Transform, Velocity,
};
pub fn id(n: u128) -> StableId {
    StableId(n.to_be_bytes())
}
pub fn scene() -> CookedScene {
    let entities = (1..=4)
        .map(|n| CookedEntity {
            id: id(n),
            parent: None,
            transform: Transform::default(),
            velocity: Some(Velocity { linear: [0.; 3] }),
        })
        .collect();
    let mut d = CookedSpatial::empty(id(1));
    d.directional = Some(DirectionalLight {
        color: [1.; 3],
        intensity: 1.,
        shadows: Some(DirectionalShadows { distance: 10. }),
    });
    let mut p = CookedSpatial::empty(id(2));
    p.point = Some(PointLight {
        color: [1.; 3],
        intensity: 2.,
        range: 3.,
    });
    let mut s = CookedSpatial::empty(id(3));
    s.spot = Some(SpotLight {
        color: [1.; 3],
        intensity: 3.,
        range: 4.,
        inner_degrees: 20.,
        outer_degrees: 30.,
    });
    CookedScene::new(id(99), 60, entities)
        .unwrap()
        .with_spatial(vec![], vec![d, p, s])
        .unwrap()
}
pub fn queue(commands: impl IntoIterator<Item = StructuralCommand>) -> FrameCommands {
    let mut q = FrameCommands::default();
    for c in commands {
        q.push(c).unwrap();
    }
    q
}
pub fn declarations(w: &NativeWorld) -> Vec<CookedSpatial> {
    w.spatial_view()
        .unwrap()
        .nodes()
        .filter_map(|n| n.declaration.cloned())
        .collect()
}
pub fn control(n: u128, control: LightControl) -> StructuralCommand {
    StructuralCommand::ControlLight {
        entity: id(n),
        control,
    }
}

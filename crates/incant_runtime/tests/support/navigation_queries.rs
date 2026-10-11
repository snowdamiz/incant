use incant_runtime::*;
use incant_types::Transform;
use std::sync::Arc;
pub fn id(n: u8) -> StableId {
    StableId(u128::from(n).to_be_bytes())
}
pub fn entity(n: u8) -> CookedEntity {
    CookedEntity {
        id: id(n),
        parent: None,
        transform: Transform::default(),
        velocity: None,
    }
}
pub fn settings() -> NavigationSettings {
    NavigationSettings {
        min: [-10., -2., -6.],
        max: [10., 5., 6.],
        cell_size: 0.25,
        cell_height: 0.1,
        tile_cells: 16,
        agent_radius: 0.5,
        agent_height: 1.8,
        max_climb: 0.3,
        max_slope_degrees: 45.,
    }
}
pub fn floor() -> Arc<NavigationGeometry> {
    Arc::new(NavigationGeometry {
        vertices: vec![
            [-10., 0., -6.],
            [10., 0., -6.],
            [10., 0., 6.],
            [-10., 0., 6.],
        ],
        triangles: vec![[0, 2, 1], [0, 3, 2]],
    })
}
pub fn asset() -> VisualAssetRef {
    VisualAssetRef {
        id: id(90),
        kind: VisualAssetKind::Model,
        fingerprint: [8; 32],
    }
}
pub fn config() -> CookedNavigation {
    CookedNavigation {
        entity: id(1),
        grid: Some(NavigationGrid {
            dimensions: [3, 2],
            costs: vec![1, 0, 1, 1, 1, 1],
        }),
        mesh: Some(NativeNavigationMesh {
            settings: settings(),
            sources: vec![
                NativeNavigationSource {
                    entity: id(2),
                    geometry: NativeNavigationGeometry::Mesh {
                        asset: asset(),
                        geometry: floor(),
                    },
                },
                NativeNavigationSource {
                    entity: id(3),
                    geometry: NativeNavigationGeometry::Collider,
                },
            ],
            links: vec![],
        }),
    }
}
pub fn scene() -> CookedScene {
    let mut model = CookedSpatial::empty(id(2));
    model.mesh = Some(NativeMeshRenderer {
        mesh: asset().id,
        cast_shadows: false,
    });
    let mut obstacle = entity(3);
    obstacle.transform.translation = [0., 1.5, 0.];
    CookedScene::new(id(0), 60, vec![entity(1), entity(2), obstacle, entity(4)])
        .unwrap()
        .with_spatial(vec![asset()], vec![model])
        .unwrap()
        .with_physics(vec![CookedPhysics {
            entity: id(3),
            body: None,
            collider: incant_types::Collider {
                shape: incant_types::ColliderShape::Box {
                    half_extents: [1., 1.5, 2.],
                },
                ..Default::default()
            },
            angular_velocity: None,
        }])
        .unwrap()
        .with_navigation(vec![config()])
        .unwrap()
}
pub fn path() -> PathRequest {
    PathRequest {
        start: [-8., 0., 0.],
        end: [8., 0., 0.],
        snap_distance: 1.,
        max_visited: 1000,
    }
}
pub fn grid() -> GridPathRequest {
    GridPathRequest {
        start: [0, 0],
        end: [2, 0],
        diagonal: true,
        max_expansions: 100,
    }
}

pub fn linked_scene(enabled: bool) -> CookedScene {
    let mut cfg = config();
    let mut geometry = NavigationGeometry {
        vertices: vec![],
        triangles: vec![],
    };
    for (min, max) in [(-10., -1.), (1., 10.)] {
        let base = geometry.vertices.len() as u32;
        geometry
            .vertices
            .extend([[min, 0., -6.], [max, 0., -6.], [max, 0., 6.], [min, 0., 6.]]);
        geometry
            .triangles
            .extend([[base, base + 2, base + 1], [base, base + 3, base + 2]]);
    }
    let mesh = cfg.mesh.as_mut().unwrap();
    mesh.sources = vec![NativeNavigationSource {
        entity: id(2),
        geometry: NativeNavigationGeometry::Mesh {
            asset: asset(),
            geometry: Arc::new(geometry),
        },
    }];
    mesh.links = vec![NativeNavigationLink {
        id: id(8),
        start: [-2., 0., 0.],
        end: [2., 0., 0.],
        snap_distance: 1.,
        bidirectional: true,
        enabled,
        extra_cost: 0.,
    }];
    scene().with_navigation(vec![cfg]).unwrap()
}

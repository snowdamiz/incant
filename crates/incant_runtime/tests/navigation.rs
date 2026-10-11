use incant_runtime::*;
use incant_types::{Transform, Velocity};
use std::sync::Arc;
fn id(n: u8) -> StableId {
    StableId([n; 16])
}
fn entity(n: u8) -> CookedEntity {
    CookedEntity {
        id: id(n),
        parent: None,
        transform: Transform::default(),
        velocity: None,
    }
}
fn settings() -> NavigationSettings {
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
fn floor() -> Arc<NavigationGeometry> {
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
fn asset() -> VisualAssetRef {
    VisualAssetRef {
        id: id(90),
        kind: VisualAssetKind::Model,
        fingerprint: [8; 32],
    }
}
fn config() -> CookedNavigation {
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
fn scene() -> CookedScene {
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
fn path() -> PathRequest {
    PathRequest {
        start: [-8., 0., 0.],
        end: [8., 0., 0.],
        snap_distance: 1.,
        max_visited: 1000,
    }
}
fn grid() -> GridPathRequest {
    GridPathRequest {
        start: [0, 0],
        end: [2, 0],
        diagonal: true,
        max_expansions: 100,
    }
}
fn apply(w: &mut NativeWorld, commands: Vec<StructuralCommand>) -> Result<(), RuntimeError> {
    let mut c = FrameCommands::default();
    for command in commands {
        c.push(command).unwrap();
    }
    w.apply(&mut c)
}
fn shift(w: &mut NativeWorld, n: u8, p: [f64; 3]) {
    w.with_numeric_columns(|cols| {
        cols.for_each_chunk(|c| {
            for (row, &eid) in c.ids.iter().enumerate() {
                if eid == id(n) {
                    c.transforms[row * TRANSFORM_STRIDE..row * TRANSFORM_STRIDE + 3]
                        .copy_from_slice(&p);
                }
            }
        })
    });
}
#[test]
fn source_free_roundtrip_paths_and_unchanged_ticks_retain_generations() {
    let cooked = scene();
    let bytes = cooked.to_bytes();
    assert_eq!(u32::from_le_bytes(bytes[8..12].try_into().unwrap()), 9);
    let loaded = CookedScene::from_bytes(&bytes).unwrap();
    assert_eq!(loaded.to_bytes(), bytes);
    drop(cooked);
    let mut world = NativeWorld::new(loaded);
    let original = world.navigation_path(id(1), &path()).unwrap().unwrap();
    assert!(original.points.len() > 2);
    let original_grid = world.grid_path(id(1), &grid()).unwrap().unwrap();
    assert_eq!(
        original_grid.cells,
        vec![[0, 0], [0, 1], [1, 1], [2, 1], [2, 0]]
    );
    for _ in 0..30 {
        world.step().unwrap();
    }
    assert_eq!(
        world.navigation_path(id(1), &path()).unwrap().unwrap(),
        original
    );
    assert_eq!(
        world.grid_path(id(1), &grid()).unwrap().unwrap(),
        original_grid
    );
    assert!(matches!(
        world.save_bytes(),
        Err(SaveError::PhysicsUnsupported)
    ));
}
#[test]
fn moving_obstacle_reuses_unaffected_tiles_and_queries_refuse_uncommitted_poses() {
    let mut world = NativeWorld::new(scene());
    let old = world.navigation_report(id(1)).unwrap().clone();
    shift(&mut world, 3, [0., 1.5, 4.]);
    assert!(world.navigation_path(id(1), &path()).is_err());
    apply(&mut world, vec![]).unwrap();
    let report = world.navigation_report(id(1)).unwrap();
    assert!(report.generation > old.generation);
    assert!(!report.rebuilt.is_empty());
    assert!(!report.reused.is_empty());
    assert_eq!(
        report.rebuilt.len() + report.reused.len(),
        old.rebuilt.len()
    );
    let path = world.navigation_path(id(1), &path()).unwrap().unwrap();
    assert_eq!(path.points.len(), 2);
}
#[test]
fn structural_and_numeric_failures_do_not_publish_or_consume_the_queue() {
    let mut world = NativeWorld::new(scene());
    let before = world.snapshot();
    let path_before = world.navigation_path(id(1), &path()).unwrap();
    let report = world.navigation_report(id(1)).unwrap().clone();
    let mut commands = FrameCommands::default();
    commands.push(StructuralCommand::Spawn(entity(8))).unwrap();
    commands.push(StructuralCommand::Despawn(id(2))).unwrap();
    assert!(world.apply(&mut commands).is_err());
    assert_eq!(commands.len(), 2);
    assert_eq!(world.snapshot(), before);
    assert_eq!(world.navigation_path(id(1), &path()).unwrap(), path_before);
    // A transform write that cannot produce bounded geometry is rejected before
    // navigation publication; a foreign host can restore its numeric undo safely.
    shift(&mut world, 2, [1e9, 0., 0.]);
    assert!(apply(&mut world, vec![]).is_err());
    shift(&mut world, 2, [0.; 3]);
    assert_eq!(world.navigation_report(id(1)).unwrap(), &report);
    assert_eq!(world.navigation_path(id(1), &path()).unwrap(), path_before);
    let tick = world.tick();
    shift(&mut world, 2, [1e9, 0., 0.]);
    assert!(world.step().is_err());
    assert_eq!(world.tick(), tick);
    shift(&mut world, 2, [0.; 3]);
    world.step().unwrap();
}
#[test]
fn source_ancestry_velocity_binding_and_lifetime_are_checked_atomically() {
    let mut world = NativeWorld::new(scene());
    let original = world.navigation_path(id(1), &path()).unwrap();
    assert!(
        apply(
            &mut world,
            vec![
                StructuralCommand::SetParent {
                    entity: id(2),
                    parent: Some(id(4))
                },
                StructuralCommand::SetVelocity {
                    entity: id(4),
                    velocity: Some(Velocity {
                        linear: [1., 0., 0.]
                    })
                }
            ]
        )
        .is_err()
    );
    apply(
        &mut world,
        vec![StructuralCommand::SetParent {
            entity: id(2),
            parent: Some(id(4)),
        }],
    )
    .unwrap();
    assert!(
        apply(
            &mut world,
            vec![StructuralCommand::SetVelocity {
                entity: id(4),
                velocity: Some(Velocity {
                    linear: [1., 0., 0.]
                })
            }]
        )
        .is_err()
    );
    assert_eq!(world.navigation_path(id(1), &path()).unwrap(), original);
    // Zero motion causes real archetype relocation without invalidating source IDs.
    apply(
        &mut world,
        vec![StructuralCommand::SetVelocity {
            entity: id(2),
            velocity: Some(Velocity { linear: [0.; 3] }),
        }],
    )
    .unwrap();
    assert_eq!(world.navigation_path(id(1), &path()).unwrap(), original);
    assert!(
        apply(
            &mut world,
            vec![StructuralCommand::SetSpatial {
                entity: id(2),
                spatial: None
            }]
        )
        .is_err()
    );
    apply(
        &mut world,
        vec![
            StructuralCommand::SetNavigation {
                entity: id(1),
                navigation: None,
            },
            StructuralCommand::Despawn(id(2)),
        ],
    )
    .unwrap();
    assert!(world.navigation_path(id(1), &path()).is_err());
    apply(&mut world, vec![StructuralCommand::Spawn(entity(2))]).unwrap();
    assert!(world.navigation_path(id(1), &path()).is_err());
    assert!(matches!(
        world.save_bytes(),
        Err(SaveError::PhysicsUnsupported)
    ));
}
#[test]
fn grid_replacement_and_late_bad_link_are_atomic() {
    let mut world = NativeWorld::new(scene());
    let before = world.grid_path(id(1), &grid()).unwrap().unwrap();
    let mut replacement = config();
    replacement.grid.as_mut().unwrap().costs[1] = 1;
    apply(
        &mut world,
        vec![StructuralCommand::SetNavigation {
            entity: id(1),
            navigation: Some(Box::new(replacement.clone())),
        }],
    )
    .unwrap();
    let direct = world.grid_path(id(1), &grid()).unwrap().unwrap();
    assert_eq!(direct.cells, vec![[0, 0], [1, 0], [2, 0]]);
    assert!(direct.generation > before.generation);
    replacement.grid.as_mut().unwrap().costs[1] = 0;
    replacement
        .mesh
        .as_mut()
        .unwrap()
        .links
        .push(NativeNavigationLink {
            id: id(8),
            start: [500., 0., 0.],
            end: [600., 0., 0.],
            snap_distance: 1.,
            bidirectional: true,
            enabled: true,
            extra_cost: 0.,
        });
    assert!(
        apply(
            &mut world,
            vec![StructuralCommand::SetNavigation {
                entity: id(1),
                navigation: Some(Box::new(replacement))
            }]
        )
        .is_err()
    );
    assert_eq!(world.grid_path(id(1), &grid()).unwrap().unwrap(), direct);
}
#[test]
fn v9_rejects_checksum_valid_bad_counts_tags_floats_duplicates_and_truncation() {
    use sha2::{Digest, Sha256};
    fn repair(b: &mut [u8]) {
        let n = (b.len() - 72) as u32;
        b[36..40].copy_from_slice(&n.to_le_bytes());
        let hash: [u8; 32] = Sha256::digest([&b[..40], &b[72..]].concat()).into();
        b[40..72].copy_from_slice(&hash);
    }
    let simple = CookedScene::new(id(0), 60, vec![entity(1)])
        .unwrap()
        .with_navigation(vec![CookedNavigation {
            entity: id(1),
            mesh: None,
            grid: Some(NavigationGrid {
                dimensions: [1, 1],
                costs: vec![1],
            }),
        }])
        .unwrap()
        .to_bytes();
    // one entity, six empty v2-v8 tables, then v9 count/id/mask/grid.
    let suffix = 72 + 104 + 24;
    for (offset, value) in [
        (suffix, 255),
        (suffix + 20, 0),
        (suffix + 21, 0),
        (suffix + 25, 255),
        (suffix + 30, 255),
    ] {
        let mut b = simple.clone();
        b[offset] = value;
        repair(&mut b);
        assert!(CookedScene::from_bytes(&b).is_err(), "offset {offset}");
    }
    for length in suffix..simple.len() {
        let mut b = simple[..length].to_vec();
        repair(&mut b);
        assert!(CookedScene::from_bytes(&b).is_err());
    }
    let mut bad = config();
    let duplicate = bad.mesh.as_ref().unwrap().sources[0].clone();
    bad.mesh.as_mut().unwrap().sources.push(duplicate);
    assert!(scene().with_navigation(vec![bad]).is_err());
    let mut bad = config();
    bad.mesh.as_mut().unwrap().settings.cell_size = f32::NAN;
    assert!(scene().with_navigation(vec![bad]).is_err());
    let mut b = simple;
    b[8..12].copy_from_slice(&8u32.to_le_bytes());
    repair(&mut b);
    assert!(CookedScene::from_bytes(&b).is_err());
}

#[test]
fn mirrored_parent_pose_and_retained_source_geometry_use_committed_f64_globals() {
    let mut cfg = config();
    cfg.mesh
        .as_mut()
        .unwrap()
        .sources
        .retain(|s| s.entity == id(2));
    let mut world = NativeWorld::new(scene().with_navigation(vec![cfg]).unwrap());
    apply(
        &mut world,
        vec![StructuralCommand::SetParent {
            entity: id(2),
            parent: Some(id(4)),
        }],
    )
    .unwrap();
    shift(&mut world, 4, [0., 2.0000000001, 0.]);
    world.with_numeric_columns(|columns| {
        columns.for_each_chunk(|c| {
            for (row, &eid) in c.ids.iter().enumerate() {
                if eid == id(2) {
                    c.transforms[row * TRANSFORM_STRIDE + 7] = -1.;
                }
            }
        })
    });
    world.step().unwrap();
    let q = PathRequest {
        start: [-8., 2., 0.],
        end: [8., 2., 0.],
        ..path()
    };
    let path = world.navigation_path(id(1), &q).unwrap().unwrap();
    assert_eq!(path.points.len(), 2);
    assert!(path.points.iter().all(|p| (p[1] - 2.).abs() < 0.2));
    assert_eq!(
        world.inspect(id(2)).unwrap().world_transform[3][1],
        2.0000000001
    );
}
#[test]
fn aggregate_geometry_and_grid_budgets_reject_before_publication() {
    let mut cfg = config();
    let mut geometry = (*floor()).clone();
    geometry.triangles = vec![[0, 2, 1]; 20_000];
    cfg.mesh.as_mut().unwrap().sources = vec![
        NativeNavigationSource {
            entity: id(1),
            geometry: NativeNavigationGeometry::Mesh {
                asset: asset(),
                geometry: Arc::new(geometry.clone()),
            },
        },
        NativeNavigationSource {
            entity: id(2),
            geometry: NativeNavigationGeometry::Mesh {
                asset: asset(),
                geometry: Arc::new(geometry),
            },
        },
    ];
    assert!(matches!(
        scene().with_navigation(vec![cfg]),
        Err(SceneError::Invalid(
            "aggregate navigation geometry capacity"
        ))
    ));
    let nodes = (1..=2)
        .map(|n| CookedNavigation {
            entity: id(n),
            mesh: None,
            grid: Some(NavigationGrid {
                dimensions: [256, 256],
                costs: vec![1; 65536],
            }),
        })
        .collect();
    assert!(matches!(
        scene().with_navigation(nodes),
        Err(SceneError::Invalid("aggregate navigation capacity"))
    ));
}

#[test]
fn off_mesh_links_roundtrip_binary_identity_and_disable_without_stale_paths() {
    let mut geometry = (*floor()).clone();
    geometry.vertices = vec![
        [-10., 0., -6.],
        [-2., 0., -6.],
        [-2., 0., 6.],
        [-10., 0., 6.],
        [2., 0., -6.],
        [10., 0., -6.],
        [10., 0., 6.],
        [2., 0., 6.],
    ];
    geometry.triangles = vec![[0, 2, 1], [0, 3, 2], [4, 6, 5], [4, 7, 6]];
    let mut cfg = config();
    let mesh = cfg.mesh.as_mut().unwrap();
    mesh.sources = vec![NativeNavigationSource {
        entity: id(2),
        geometry: NativeNavigationGeometry::Mesh {
            asset: asset(),
            geometry: Arc::new(geometry),
        },
    }];
    mesh.links = vec![NativeNavigationLink {
        id: StableId(1u128.to_be_bytes()),
        start: [-3., 0., 0.],
        end: [3., 0., 0.],
        snap_distance: 1.,
        bidirectional: true,
        enabled: true,
        extra_cost: 2.,
    }];
    let bytes = scene()
        .with_navigation(vec![cfg.clone()])
        .unwrap()
        .to_bytes();
    let mut world = NativeWorld::from_bytes(&bytes).unwrap();
    let linked = world.navigation_path(id(1), &path()).unwrap().unwrap();
    assert_eq!(linked.traversals.len(), 1);
    assert_eq!(linked.traversals[0].link_id, "00000000000000000000000001");
    cfg.mesh.as_mut().unwrap().links[0].enabled = false;
    apply(
        &mut world,
        vec![StructuralCommand::SetNavigation {
            entity: id(1),
            navigation: Some(Box::new(cfg)),
        }],
    )
    .unwrap();
    assert!(world.navigation_path(id(1), &path()).unwrap().is_none());
}

#[test]
fn decoder_checks_mesh_floats_source_tags_and_link_booleans_after_integrity() {
    use sha2::{Digest, Sha256};
    fn repaired(mut b: Vec<u8>) -> Vec<u8> {
        let hash: [u8; 32] = Sha256::digest([&b[..40], &b[72..]].concat()).into();
        b[40..72].copy_from_slice(&hash);
        b
    }
    // Empty geometry with one disabled link is valid and keeps offsets explicit.
    let node = CookedNavigation {
        entity: id(1),
        grid: None,
        mesh: Some(NativeNavigationMesh {
            settings: settings(),
            sources: vec![],
            links: vec![NativeNavigationLink {
                id: id(8),
                start: [0., 0., 0.],
                end: [2., 0., 0.],
                snap_distance: 1.,
                bidirectional: false,
                enabled: false,
                extra_cost: 0.,
            }],
        }),
    };
    let bytes = CookedScene::new(id(0), 60, vec![entity(1)])
        .unwrap()
        .with_navigation(vec![node])
        .unwrap()
        .to_bytes();
    let base = 72 + 104 + 24 + 4 + 16 + 1;
    for bits in [
        f32::NAN.to_bits(),
        f32::INFINITY.to_bits(),
        (-0.0f32).to_bits(),
    ] {
        let mut b = bytes.clone();
        b[base..base + 4].copy_from_slice(&bits.to_le_bytes());
        assert!(CookedScene::from_bytes(&repaired(b)).is_err());
    }
    for offset in [base + 58 + 44, base + 58 + 45] {
        let mut b = bytes.clone();
        b[offset] = 2;
        assert!(CookedScene::from_bytes(&repaired(b)).is_err());
    }
    // For the real mixed scene, locate the canonical v9 suffix through its size.
    let bytes = scene().to_bytes();
    let needle = [1u32.to_le_bytes().as_slice(), &id(1).0, [3].as_slice()].concat();
    let suffix = bytes
        .windows(needle.len())
        .position(|v| v == needle)
        .unwrap();
    let first_tag = suffix + 4 + 16 + 1 + 50 + 4 + 16;
    let mut b = bytes;
    b[first_tag] = 255;
    assert!(CookedScene::from_bytes(&repaired(b)).is_err());
}

#[test]
fn navigation_save_envelopes_refuse_without_replacing_either_world() {
    use sha2::{Digest, Sha256};
    let plain = CookedScene::new(id(0), 60, vec![entity(1)]).unwrap();
    let mut destination = NativeWorld::new(plain);
    destination.step().unwrap();
    let original = destination.save_bytes().unwrap();
    let payload = scene().to_bytes();
    let mut envelope = original[..64].to_vec();
    envelope[8..12].copy_from_slice(&1u32.to_le_bytes());
    envelope[24..32].copy_from_slice(&(payload.len() as u64).to_le_bytes());
    envelope.extend(payload);
    let mut hash = Sha256::new();
    hash.update(&envelope[..32]);
    hash.update(&envelope[64..]);
    envelope[32..64].copy_from_slice(&hash.finalize());
    assert!(matches!(
        NativeWorld::from_save_bytes(&envelope),
        Err(SaveError::NavigationUnsupported)
    ));
    assert!(matches!(
        destination.restore_bytes(&envelope),
        Err(SaveError::NavigationUnsupported)
    ));
    assert_eq!(destination.save_bytes().unwrap(), original);
    let mut navigation = NativeWorld::new(scene());
    let before = navigation.snapshot();
    let query = navigation.navigation_path(id(1), &path()).unwrap();
    assert!(matches!(
        navigation.restore_bytes(&original),
        Err(SaveError::PhysicsUnsupported)
    ));
    assert_eq!(navigation.snapshot(), before);
    assert_eq!(navigation.navigation_path(id(1), &path()).unwrap(), query);
    navigation.step().unwrap();
}

#[test]
fn aggregate_tile_link_and_ancestor_budgets_reject_before_publication() {
    let mesh = || NativeNavigationMesh {
        settings: NavigationSettings {
            min: [0., -1., 0.],
            max: [64., 4., 64.],
            ..settings()
        },
        sources: vec![],
        links: vec![],
    };
    // Each declaration fits the algorithm's 256-tile budget; their sum does not.
    assert!(
        CookedScene::new(id(0), 60, vec![entity(1), entity(2)])
            .unwrap()
            .with_navigation(vec![
                CookedNavigation {
                    entity: id(1),
                    mesh: Some(mesh()),
                    grid: None
                },
                CookedNavigation {
                    entity: id(2),
                    mesh: Some(mesh()),
                    grid: None
                }
            ])
            .is_err()
    );
    let mut one = NativeNavigationMesh {
        settings: settings(),
        sources: vec![],
        links: vec![],
    };
    for n in 0u128..65 {
        one.links.push(NativeNavigationLink {
            id: StableId(n.to_be_bytes()),
            start: [0., 0., 0.],
            end: [1., 0., 0.],
            snap_distance: 1.,
            bidirectional: true,
            enabled: false,
            extra_cost: 0.,
        });
    }
    assert!(
        CookedScene::new(id(0), 60, vec![entity(1), entity(2)])
            .unwrap()
            .with_navigation(vec![
                CookedNavigation {
                    entity: id(1),
                    mesh: Some(one.clone()),
                    grid: None
                },
                CookedNavigation {
                    entity: id(2),
                    mesh: Some(one),
                    grid: None
                }
            ])
            .is_err()
    );
    let mut world = NativeWorld::new(scene());
    let original = world.navigation_path(id(1), &path()).unwrap();
    let mut commands = vec![];
    let chain_id = |n: usize| StableId((10_000 + n as u128).to_be_bytes());
    for n in 0..MAX_NAVIGATION_DEPENDENCIES {
        commands.push(StructuralCommand::Spawn(CookedEntity {
            id: chain_id(n),
            parent: n.checked_sub(1).map(chain_id),
            transform: Transform::default(),
            velocity: None,
        }));
    }
    commands.push(StructuralCommand::SetParent {
        entity: id(2),
        parent: Some(chain_id(MAX_NAVIGATION_DEPENDENCIES - 1)),
    });
    assert!(apply(&mut world, commands).is_err());
    assert!(world.inspect(chain_id(0)).is_none());
    assert_eq!(world.navigation_path(id(1), &path()).unwrap(), original);
}

#[test]
fn removing_one_source_rebuilds_its_old_tiles_and_retains_unrelated_tiles() {
    let mut world = NativeWorld::new(scene());
    let before = world.navigation_report(id(1)).unwrap().clone();
    let mut next = config();
    next.mesh
        .as_mut()
        .unwrap()
        .sources
        .retain(|s| s.entity != id(3));
    apply(
        &mut world,
        vec![
            StructuralCommand::SetNavigation {
                entity: id(1),
                navigation: Some(Box::new(next)),
            },
            StructuralCommand::Despawn(id(3)),
        ],
    )
    .unwrap();
    let report = world.navigation_report(id(1)).unwrap();
    assert!(report.generation > before.generation);
    assert!(!report.rebuilt.is_empty());
    assert!(!report.reused.is_empty());
    assert_eq!(
        report.rebuilt.len() + report.reused.len(),
        before.rebuilt.len()
    );
    assert_eq!(
        world
            .navigation_path(id(1), &path())
            .unwrap()
            .unwrap()
            .points
            .len(),
        2
    );
}

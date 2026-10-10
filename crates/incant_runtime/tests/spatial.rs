use incant_runtime::{
    CookedEntity, CookedScene, CookedSpatial, FrameCommands, NativeEnvironmentLight,
    NativeMeshRenderer, NativeWorld, SaveError, StableId, StructuralCommand as Cmd,
    VisualAssetKind, VisualAssetRef,
};
use incant_types::{
    Camera, CameraProjection, DirectionalLight, DirectionalShadows, PointLight, SpotLight,
    Transform, Velocity,
};
use sha2::{Digest, Sha256};
fn id(n: u128) -> StableId {
    StableId(n.to_be_bytes())
}
fn entity(n: u128, parent: Option<u128>) -> CookedEntity {
    CookedEntity {
        id: id(n),
        parent: parent.map(id),
        transform: Transform {
            translation: [n as f64 + 0.125, 0., 0.],
            ..Default::default()
        },
        velocity: None,
    }
}
fn scene() -> CookedScene {
    CookedScene::new(
        id(99),
        60,
        (1..=4).map(|n| entity(n, (n == 2).then_some(1))).collect(),
    )
    .unwrap()
}
fn camera(projection: CameraProjection) -> Camera {
    Camera {
        fov_degrees: 60.,
        near: 0.1,
        far: 100.,
        projection,
    }
}
fn assets() -> Vec<VisualAssetRef> {
    vec![
        VisualAssetRef {
            id: id(90),
            kind: VisualAssetKind::Model,
            fingerprint: [7; 32],
        },
        VisualAssetRef {
            id: id(91),
            kind: VisualAssetKind::Texture,
            fingerprint: [8; 32],
        },
    ]
}
fn nodes() -> Vec<CookedSpatial> {
    let mut a = CookedSpatial::empty(id(1));
    a.camera = Some(camera(CameraProjection::PixelPerfect {
        pixels_per_unit: 32.,
        reference_resolution: [320, 180],
    }));
    a.mesh = Some(NativeMeshRenderer {
        mesh: id(90),
        cast_shadows: true,
    });
    a.directional = Some(DirectionalLight {
        color: [1., 0.5, 0.25],
        intensity: 100.,
        shadows: Some(DirectionalShadows { distance: 20. }),
    });
    let mut b = CookedSpatial::empty(id(2));
    b.camera = Some(camera(CameraProjection::Orthographic { vertical_size: 3. }));
    b.point = Some(PointLight {
        color: [0., 1., 0.],
        intensity: 2.,
        range: 4.,
    });
    let mut c = CookedSpatial::empty(id(3));
    c.camera = Some(camera(CameraProjection::Perspective {}));
    c.spot = Some(SpotLight {
        color: [1.; 3],
        intensity: 4.,
        range: 5.,
        inner_degrees: 10.,
        outer_degrees: 30.,
    });
    let mut d = CookedSpatial::empty(id(4));
    d.environment = Some(NativeEnvironmentLight {
        texture: id(91),
        intensity: 2.,
        rotation_degrees: -30.,
    });
    vec![a, b, c, d]
}
fn queue(commands: impl IntoIterator<Item = Cmd>) -> FrameCommands {
    let mut q = FrameCommands::default();
    for c in commands {
        q.push(c).unwrap();
    }
    q
}
fn generations(w: &NativeWorld) -> (u64, u64) {
    let v = w.spatial_view().unwrap();
    (v.topology_generation(), v.binding_generation())
}
type ObservedNode = (
    StableId,
    Option<usize>,
    Transform,
    [f64; 16],
    Option<CookedSpatial>,
);
fn observe(w: &NativeWorld) -> Vec<ObservedNode> {
    w.spatial_view()
        .unwrap()
        .nodes()
        .map(|n| {
            (
                n.id,
                n.parent_index,
                n.local.clone(),
                n.global.to_cols_array(),
                n.declaration.cloned(),
            )
        })
        .collect()
}
fn rehash(bytes: &mut [u8]) {
    let len = (bytes.len() - 72) as u32;
    bytes[36..40].copy_from_slice(&len.to_le_bytes());
    let hash = Sha256::new()
        .chain_update(&bytes[..40])
        .chain_update(&bytes[72..])
        .finalize();
    bytes[40..72].copy_from_slice(&hash);
}

#[test]
fn source_free_v4_roundtrip_is_canonical_and_preserves_all_typed_declarations() {
    let plain = scene().to_bytes();
    assert_eq!(
        scene().with_spatial(vec![], vec![]).unwrap().to_bytes(),
        plain
    );
    assert_eq!(&plain[8..12], &1u32.to_le_bytes());
    let expected = scene().with_spatial(assets(), nodes()).unwrap().to_bytes();
    assert_eq!(&expected[8..12], &4u32.to_le_bytes());
    let mut a = assets();
    a.reverse();
    let mut n = nodes();
    n.reverse();
    assert_eq!(scene().with_spatial(a, n).unwrap().to_bytes(), expected);
    let loaded = CookedScene::from_bytes(&expected).unwrap();
    assert_eq!(loaded.spatial(), nodes());
    assert_eq!(loaded.visual_assets(), assets());
    assert_eq!(loaded.to_bytes(), expected);
    let world = NativeWorld::new(loaded);
    let view = world.spatial_view().unwrap();
    assert_eq!(view.nodes().len(), 4);
    assert_eq!(view.assets(), assets());
    let child = view.nodes().find(|n| n.id == id(2)).unwrap();
    assert_eq!(child.global.w_axis.x, 3.25);
    assert_eq!(
        view.nodes().nth(child.parent_index.unwrap()).unwrap().id,
        id(1)
    );
}

#[test]
fn checked_decoder_rejects_unknown_tags_counts_flags_and_every_truncation() {
    let original = scene().with_spatial(assets(), nodes()).unwrap().to_bytes();
    // Fixed baseline: four 104-byte entities, empty physics/audio, two 49-byte assets.
    let asset_count = 72 + 4 * 104 + 8;
    let first_asset = asset_count + 4;
    let first_node = first_asset + 2 * 49 + 4;
    for (offset, value) in [
        (first_asset + 16, 2),
        (first_node + 16, 0),
        (first_node + 16, 64),
        (first_node + 17 + 24, 3),
        (first_node + 17 + 41 + 16, 2),
    ] {
        let mut bad = original.clone();
        bad[offset] = value;
        rehash(&mut bad);
        assert!(CookedScene::from_bytes(&bad).is_err(), "offset {offset}");
    }
    let mut bad = original.clone();
    bad[asset_count..asset_count + 4].copy_from_slice(&u32::MAX.to_le_bytes());
    rehash(&mut bad);
    assert!(CookedScene::from_bytes(&bad).is_err());
    let mut bad = original.clone();
    bad[first_asset + 49..first_asset + 65].copy_from_slice(&id(90).0);
    rehash(&mut bad);
    assert!(CookedScene::from_bytes(&bad).is_err());
    for len in 72..original.len() {
        let mut bad = original[..len].to_vec();
        rehash(&mut bad);
        assert!(CookedScene::from_bytes(&bad).is_err(), "length {len}");
    }
    let mut bad = original.clone();
    bad.push(0);
    rehash(&mut bad);
    assert!(CookedScene::from_bytes(&bad).is_err());
}

#[test]
fn cook_boundary_rejects_invalid_ranges_references_and_excessive_lights() {
    let mut invalid = nodes();
    invalid[0].camera.as_mut().unwrap().near = f64::NAN;
    assert!(scene().with_spatial(assets(), invalid).is_err());
    let mut invalid = nodes();
    invalid[1].point.as_mut().unwrap().range = f64::INFINITY;
    assert!(scene().with_spatial(assets(), invalid).is_err());
    let mut invalid = nodes();
    invalid[0].point = nodes()[1].point.clone();
    assert!(scene().with_spatial(assets(), invalid).is_err());
    let mut invalid = assets();
    invalid[0].kind = VisualAssetKind::Texture;
    assert!(scene().with_spatial(invalid, nodes()).is_err());
    let mut invalid = assets();
    invalid[0].id = id(1);
    assert!(scene().with_spatial(invalid, nodes()).is_err());
    let mut invalid = nodes();
    invalid.push(invalid[0].clone());
    assert!(scene().with_spatial(assets(), invalid).is_err());
    assert!(
        scene()
            .with_spatial(vec![], vec![CookedSpatial::empty(id(1))])
            .is_err()
    );
    let light = nodes()[1].point.clone();
    let entities = (1..=4097).map(|n| entity(n, None)).collect();
    let too_many = (1..=4097)
        .map(|n| {
            let mut s = CookedSpatial::empty(id(n));
            s.point = light.clone();
            s
        })
        .collect();
    assert!(
        CookedScene::new(id(9999), 60, entities)
            .unwrap()
            .with_spatial(vec![], too_many)
            .is_err()
    );
}

#[test]
fn final_structural_state_is_atomic_and_generations_ignore_noops_and_motion_relocation() {
    let mut world = NativeWorld::new(scene().with_spatial(assets(), nodes()).unwrap());
    let before = observe(&world);
    let mut invalid = nodes()[1].clone();
    invalid.environment = nodes()[3].environment.clone();
    let mut commands = queue([
        Cmd::SetParent {
            entity: id(2),
            parent: None,
        },
        Cmd::SetSpatial {
            entity: id(2),
            spatial: Some(invalid),
        },
    ]);
    assert!(world.apply(&mut commands).is_err());
    assert_eq!(commands.len(), 2);
    assert_eq!(observe(&world), before);
    assert_eq!(generations(&world), (0, 0));
    let mut noop = queue([Cmd::SetSpatial {
        entity: id(2),
        spatial: Some(nodes()[1].clone()),
    }]);
    world.apply(&mut noop).unwrap();
    assert_eq!(generations(&world), (0, 0));
    let mut motion = queue([Cmd::SetVelocity {
        entity: id(2),
        velocity: Some(Velocity { linear: [0.; 3] }),
    }]);
    world.apply(&mut motion).unwrap();
    assert_eq!(observe(&world), before);
    assert_eq!(generations(&world), (0, 0));
    let mut replacement = nodes()[1].clone();
    replacement.point.as_mut().unwrap().intensity = 42.;
    let mut changes = queue([
        Cmd::SetSpatial {
            entity: id(2),
            spatial: Some(replacement.clone()),
        },
        Cmd::SetParent {
            entity: id(2),
            parent: Some(id(3)),
        },
        Cmd::Despawn(id(1)),
        Cmd::Spawn(entity(5, Some(2))),
    ]);
    world.apply(&mut changes).unwrap();
    assert!(changes.is_empty());
    assert_eq!(generations(&world), (1, 1));
    let view = world.spatial_view().unwrap();
    let child = view.nodes().find(|n| n.id == id(2)).unwrap();
    assert_eq!(child.declaration, Some(&replacement));
    assert_eq!(child.global.w_axis.x, 5.25);
    assert_eq!(
        view.nodes().nth(child.parent_index.unwrap()).unwrap().id,
        id(3)
    );
    let before = observe(&world);
    let mut cycle = queue([
        Cmd::SetSpatial {
            entity: id(2),
            spatial: None,
        },
        Cmd::SetParent {
            entity: id(3),
            parent: Some(id(5)),
        },
    ]);
    assert!(world.apply(&mut cycle).is_err());
    assert_eq!(observe(&world), before);
    assert_eq!(generations(&world), (1, 1));
    assert!(
        world
            .apply(&mut queue([Cmd::Spawn(entity(90, None))]))
            .is_err()
    );
}

#[test]
fn declaration_only_batches_check_final_values_and_bounds_before_any_installation() {
    let mut world = NativeWorld::new(scene().with_spatial(assets(), nodes()).unwrap());
    let before = observe(&world);
    let mut bad = nodes()[0].clone();
    bad.camera.as_mut().unwrap().far = 0.;
    let mut q = queue([
        Cmd::SetSpatial {
            entity: id(2),
            spatial: None,
        },
        Cmd::SetSpatial {
            entity: id(1),
            spatial: Some(bad.clone()),
        },
    ]);
    assert!(world.apply(&mut q).is_err());
    assert_eq!(observe(&world), before);
    // Last value wins, but removing another declaration is a real binding change.
    q.push(Cmd::SetSpatial {
        entity: id(1),
        spatial: Some(nodes()[0].clone()),
    })
    .unwrap();
    world.apply(&mut q).unwrap();
    assert_eq!(generations(&world), (0, 1));
    let mut wrong = nodes()[0].clone();
    wrong.entity = id(3);
    assert!(
        world
            .apply(&mut queue([Cmd::SetSpatial {
                entity: id(1),
                spatial: Some(wrong)
            }]))
            .is_err()
    );
}

#[test]
fn failed_integration_does_not_publish_candidate_globals_or_tick() {
    let mut root = entity(1, None);
    root.transform.scale = [1e100; 3];
    root.velocity = Some(Velocity {
        linear: [60., 0., 0.],
    });
    let mut child = entity(2, Some(1));
    child.velocity = Some(Velocity {
        linear: [1e300, 0., 0.],
    });
    let mut world = NativeWorld::new(CookedScene::new(id(99), 60, vec![root, child]).unwrap());
    let before = observe(&world);
    assert!(world.step().is_err());
    assert_eq!(observe(&world), before);
    assert_eq!(world.tick(), 0);
    assert_eq!(generations(&world), (0, 0));
}

#[test]
fn spatial_saves_refuse_sticky_usage_and_plain_restore_invalidates_consumer_generations() {
    let mut world = NativeWorld::new(scene().with_spatial(assets(), nodes()).unwrap());
    assert_eq!(world.save_bytes(), Err(SaveError::SpatialUnsupported));
    let mut commands = queue((1..=4).map(|n| Cmd::SetSpatial {
        entity: id(n),
        spatial: None,
    }));
    world.apply(&mut commands).unwrap();
    let mut plain = NativeWorld::new(scene());
    let save = plain.save_bytes().unwrap();
    assert_eq!(world.save_bytes(), Err(SaveError::SpatialUnsupported));
    assert_eq!(
        world.restore_bytes(&save),
        Err(SaveError::SpatialUnsupported)
    );
    plain.restore_bytes(&save).unwrap();
    assert_eq!(generations(&plain), (1, 1));
    plain.restore_bytes(&save).unwrap();
    assert_eq!(generations(&plain), (2, 2));
}

#[test]
fn declaration_caps_and_removal_then_reuse_are_final_state_rules() {
    for (count, shadow) in [(17, false), (5, true)] {
        let entities = (1..=count).map(|n| entity(n, None)).collect();
        let declarations = (1..=count)
            .map(|n| {
                let mut s = CookedSpatial::empty(id(n));
                s.directional = Some(DirectionalLight {
                    color: [1.; 3],
                    intensity: 1.,
                    shadows: shadow.then_some(DirectionalShadows { distance: 1. }),
                });
                s
            })
            .collect();
        assert!(
            CookedScene::new(id(999), 60, entities)
                .unwrap()
                .with_spatial(vec![], declarations)
                .is_err()
        );
    }
    let asset = assets()[0];
    assert!(
        scene()
            .with_spatial(vec![asset; incant_runtime::MAX_VISUAL_ASSETS + 1], vec![])
            .is_err()
    );
    let mut world = NativeWorld::new(scene().with_spatial(assets(), nodes()).unwrap());
    let mut replacement = CookedSpatial::empty(id(2));
    replacement.environment = nodes()[3].environment.clone();
    world
        .apply(&mut queue([
            Cmd::SetSpatial {
                entity: id(2),
                spatial: Some(replacement),
            },
            Cmd::Despawn(id(4)),
        ]))
        .unwrap();
    assert_eq!(generations(&world), (1, 1));
    assert_eq!(
        world
            .spatial_view()
            .unwrap()
            .nodes()
            .filter(|n| n.declaration.is_some_and(|s| s.environment.is_some()))
            .count(),
        1
    );
}

#[test]
fn retained_world_identity_survives_moves_and_restore_but_never_a_new_world() {
    let original = NativeWorld::new(scene());
    let identity = original.spatial_view().unwrap().identity().clone();
    let mut moved = Box::new(original);
    assert_eq!(moved.spatial_view().unwrap().identity(), &identity);
    let save = moved.save_bytes().unwrap();
    let other = NativeWorld::from_save_bytes(&save).unwrap();
    assert_ne!(other.spatial_view().unwrap().identity(), &identity);
    assert_eq!(other.scene_id(), moved.scene_id());
    assert_eq!(generations(&other), generations(&moved));
    moved.restore_bytes(&save).unwrap();
    assert_eq!(moved.spatial_view().unwrap().identity(), &identity);
    assert_eq!(generations(&moved), (1, 1));
    drop(moved);
    // The retained opaque identity keeps its allocation live across later worlds.
    for _ in 0..32 {
        assert_ne!(
            NativeWorld::new(scene()).spatial_view().unwrap().identity(),
            &identity
        );
    }
}

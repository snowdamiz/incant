#[path = "common/allocations.rs"]
mod allocations;
#[path = "common/model_refresh.rs"]
mod fixture;
use fixture::*;
use incant_assets_runtime::RuntimeAssetData;
use incant_render_native::{NativeRenderError, NativeRenderInput, VisualCaches};
use incant_runtime::*;
use incant_types::{Camera, CameraProjection, Transform};
use std::{fs, sync::Arc};

fn world(references: Vec<VisualAssetRef>) -> NativeWorld {
    let mut mesh = CookedSpatial::empty(id(1));
    mesh.mesh = Some(NativeMeshRenderer {
        mesh: id(90),
        cast_shadows: true,
    });
    let mut camera = CookedSpatial::empty(id(2));
    camera.camera = Some(Camera {
        fov_degrees: 60.,
        near: 0.1,
        far: 100.,
        projection: CameraProjection::Perspective {},
    });
    if references
        .iter()
        .any(|r| r.kind == VisualAssetKind::Texture)
    {
        camera.environment = Some(NativeEnvironmentLight {
            texture: id(91),
            intensity: 1.,
            rotation_degrees: 0.,
        });
    }
    NativeWorld::new(
        CookedScene::new(
            id(99),
            60,
            (1..=2)
                .map(|n| CookedEntity {
                    id: id(n),
                    parent: None,
                    transform: Transform::default(),
                    velocity: None,
                })
                .collect(),
        )
        .unwrap()
        .with_spatial(references, vec![mesh, camera])
        .unwrap(),
    )
}
fn load(root: &std::path::Path, world: &NativeWorld) -> NativeRenderInput {
    NativeRenderInput::load(
        &world.spatial_view().unwrap(),
        VisualCaches {
            models: &root.join(".incant/cache/models"),
            textures: &root.join(".incant/cache/textures"),
        },
        id(2),
    )
    .unwrap()
}
fn prepared(
    input: &NativeRenderInput,
    root: &std::path::Path,
    references: &[VisualAssetRef],
) -> incant_render_native::PreparedTextureRefresh {
    input
        .texture_refresh_ticket()
        .prepare_visuals(
            references,
            VisualCaches {
                models: &root.join(".incant/cache/models"),
                textures: &root.join(".incant/cache/textures"),
            },
        )
        .unwrap()
}
fn model_asset(input: &NativeRenderInput) -> Arc<incant_assets_runtime::RuntimeAsset> {
    input
        .input()
        .nodes
        .iter()
        .find_map(|n| n.mesh.as_ref().map(|m| m.source.clone()))
        .unwrap()
}
fn nav(reference: VisualAssetRef) -> CookedNavigation {
    CookedNavigation {
        entity: id(2),
        grid: None,
        mesh: Some(NativeNavigationMesh {
            settings: NavigationSettings {
                min: [-4., -1., -4.],
                max: [4., 4., 4.],
                cell_size: 0.25,
                cell_height: 0.1,
                tile_cells: 16,
                agent_radius: 0.1,
                agent_height: 1.,
                max_climb: 0.2,
                max_slope_degrees: 45.,
            },
            sources: vec![NativeNavigationSource {
                entity: id(1),
                geometry: NativeNavigationGeometry::Mesh {
                    asset: reference,
                    geometry: Arc::new(NavigationGeometry {
                        vertices: vec![[-2., 0., -2.], [2., 0., -2.], [0., 0., 2.]],
                        triangles: vec![[0, 2, 1]],
                    }),
                },
            }],
            links: vec![],
        }),
    }
}

#[test]
fn imported_geometry_material_and_both_image_catalogs_publish_together_without_final_allocation() {
    let root = tempfile::tempdir().unwrap();
    let first = model(root.path(), 1., RED, [1.; 4], Variant::Static);
    let texture_first = texture(root.path(), RED);
    let world = world(vec![first, texture_first]);
    let mut input = load(root.path(), &world);
    let old = model_asset(&input);
    let next = model(
        root.path(),
        2.,
        DARK,
        [0.2, 0.4, 0.6, 1.],
        Variant::LargerGeometry,
    );
    let texture_next = texture(root.path(), DARK);
    assert!(
        input
            .texture_refresh_ticket()
            .prepare(
                &[next, texture_next],
                &root.path().join(".incant/cache/textures")
            )
            .is_err()
    );
    let mut stage = input
        .stage_texture_refresh(
            &world.spatial_view().unwrap(),
            prepared(&input, root.path(), &[next, texture_next]),
        )
        .unwrap();
    let (receipt, count, disposals) = allocations::record_lifecycle(|| {
        input
            .try_publish_textures(&world.spatial_view().unwrap(), &mut stage, || {
                Ok::<_, ()>(())
            })
            .unwrap()
    });
    assert_eq!((count, disposals), (0, 0));
    assert_eq!(receipt.changed, 2);
    assert_eq!(world.tick(), 0);
    assert_eq!(
        input
            .input()
            .environment
            .as_ref()
            .unwrap()
            .source
            .info()
            .fingerprint,
        key(texture_next)
    );
    let current = model_asset(&input);
    assert_ne!(current.version_id(), old.version_id());
    assert_eq!(current.info().fingerprint, key(next));
    let (RuntimeAssetData::Model(old_data), RuntimeAssetData::Model(new_data)) =
        (old.data(), current.data())
    else {
        panic!("model versions")
    };
    assert_eq!(old_data.materials[0].base_color, [1.; 4]);
    assert_eq!(new_data.materials[0].base_color, [0.2, 0.4, 0.6, 1.]);
    assert_ne!(old_data.meshes, new_data.meshes);
    assert_eq!(
        (
            old_data.meshes[0].vertices.len(),
            new_data.meshes[0].vertices.len()
        ),
        (3, 6)
    );
    assert_ne!(old_data.images[0].levels, new_data.images[0].levels);
    assert!(
        input
            .try_publish_textures(&world.spatial_view().unwrap(), &mut stage, || Ok::<_, ()>(
                ()
            ))
            .is_err()
    );
}

#[test]
fn one_bad_changed_resource_or_payload_budget_retains_the_complete_prior_catalog() {
    let root = tempfile::tempdir().unwrap();
    let first = model(root.path(), 1., RED, [1.; 4], Variant::Static);
    let old_texture = texture(root.path(), RED);
    let world = world(vec![first, old_texture]);
    let input = load(root.path(), &world);
    let old = model_asset(&input);
    let next = model(root.path(), 2., DARK, [0.2; 4], Variant::Static);
    let next_texture = texture(root.path(), DARK);
    for (kind, cache_key) in [("textures", key(next_texture)), ("models", key(next))] {
        let suffix = if kind == "models" { "incmodel" } else { "json" };
        let file = root
            .path()
            .join(format!(".incant/cache/{kind}/{cache_key}.{suffix}"));
        let bytes = fs::read(&file).unwrap();
        fs::write(&file, b"corrupt owned cooked fixture").unwrap();
        assert!(
            input
                .texture_refresh_ticket()
                .prepare_visuals(
                    &[next, next_texture],
                    VisualCaches {
                        models: &root.path().join(".incant/cache/models"),
                        textures: &root.path().join(".incant/cache/textures")
                    }
                )
                .is_err()
        );
        fs::write(file, bytes).unwrap();
        assert_eq!(model_asset(&input).version_id(), old.version_id());
        assert_eq!(
            input
                .input()
                .environment
                .as_ref()
                .unwrap()
                .source
                .info()
                .fingerprint,
            key(old_texture)
        );
    }
    assert!(matches!(
        input.texture_refresh_ticket().prepare_visuals_with_budget(
            &[next, next_texture],
            VisualCaches {
                models: &root.path().join(".incant/cache/models"),
                textures: &root.path().join(".incant/cache/textures")
            },
            0
        ),
        Err(NativeRenderError::PayloadLimit)
    ));
}

#[test]
fn internal_topology_material_binding_skinning_and_late_consumer_refusal_preserve_old_version() {
    let root = tempfile::tempdir().unwrap();
    let first = model(root.path(), 1., RED, [1.; 4], Variant::Static);
    let world = world(vec![first]);
    let mut input = load(root.path(), &world);
    let old = model_asset(&input);
    for variant in [
        Variant::TransformedNode,
        Variant::DetachedPrimitive,
        Variant::ExtraPrimitive,
        Variant::MaterialBinding,
        Variant::Skinned,
    ] {
        let next = model(root.path(), 2., DARK, [0.2; 4], variant);
        if matches!(variant, Variant::DetachedPrimitive) {
            let changed = incant_assets_runtime::load_model(
                &root.path().join(".incant/cache/models"),
                &key(next),
            )
            .unwrap();
            let RuntimeAssetData::Model(prior) = old.data() else {
                panic!("model")
            };
            assert_eq!(
                (
                    prior.metadata.nodes.len(),
                    prior.meshes.len(),
                    prior.materials.len()
                ),
                (
                    changed.metadata.nodes.len(),
                    changed.meshes.len(),
                    changed.materials.len()
                )
            );
            assert_ne!(
                prior.metadata.nodes[0].meshes,
                changed.metadata.nodes[0].meshes
            );
        }
        assert!(
            input
                .texture_refresh_ticket()
                .prepare_visuals(
                    &[next],
                    VisualCaches {
                        models: &root.path().join(".incant/cache/models"),
                        textures: &root.path().join(".incant/cache/textures")
                    }
                )
                .is_err()
        );
        assert_eq!(model_asset(&input).version_id(), old.version_id());
    }
    let next = model(root.path(), 2., DARK, [0.2; 4], Variant::Static);
    let mut stage = input
        .stage_texture_refresh(
            &world.spatial_view().unwrap(),
            prepared(&input, root.path(), &[next]),
        )
        .unwrap();
    assert!(
        input
            .try_publish_textures(&world.spatial_view().unwrap(), &mut stage, || Err(
                "consumer refusal"
            ))
            .is_err()
    );
    assert_eq!(model_asset(&input).version_id(), old.version_id());
    assert_eq!(world.tick(), 0);
}

#[test]
fn formerly_skinned_model_cannot_be_replaced_by_static_content() {
    let root = tempfile::tempdir().unwrap();
    let first = model(root.path(), 1., RED, [1.; 4], Variant::Skinned);
    let world = world(vec![first]);
    let input = load(root.path(), &world);
    let old = model_asset(&input);
    let next = model(root.path(), 2., DARK, [0.2; 4], Variant::Static);
    assert!(
        input
            .texture_refresh_ticket()
            .prepare_visuals(
                &[next],
                VisualCaches {
                    models: &root.path().join(".incant/cache/models"),
                    textures: &root.path().join(".incant/cache/textures"),
                }
            )
            .is_err()
    );
    assert_eq!(model_asset(&input).version_id(), old.version_id());
}

#[test]
fn navigation_added_after_staging_refuses_final_publication_without_calling_consumer() {
    let root = tempfile::tempdir().unwrap();
    let first = model(root.path(), 1., RED, [1.; 4], Variant::Static);
    let mut world = world(vec![first]);
    let mut input = load(root.path(), &world);
    let next = model(root.path(), 2., DARK, [0.2; 4], Variant::Static);
    let mut stage = input
        .stage_texture_refresh(
            &world.spatial_view().unwrap(),
            prepared(&input, root.path(), &[next]),
        )
        .unwrap();
    let old = model_asset(&input);
    let mut commands = FrameCommands::default();
    commands
        .push(StructuralCommand::SetNavigation {
            entity: id(2),
            navigation: Some(Box::new(nav(first))),
        })
        .unwrap();
    world.apply(&mut commands).unwrap();
    assert_eq!(
        world
            .spatial_view()
            .unwrap()
            .navigation_model_assets()
            .collect::<Vec<_>>(),
        vec![id(90)]
    );
    let mut called = false;
    assert!(
        input
            .try_publish_textures(&world.spatial_view().unwrap(), &mut stage, || {
                called = true;
                Ok::<_, ()>(())
            })
            .is_err()
    );
    assert!(!called);
    assert_eq!(model_asset(&input).version_id(), old.version_id());
    assert!(
        input
            .stage_texture_refresh(
                &world.spatial_view().unwrap(),
                prepared(&input, root.path(), &[next])
            )
            .is_err()
    );
}

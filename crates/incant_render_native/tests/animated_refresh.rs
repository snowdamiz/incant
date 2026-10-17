#[path = "common/allocations.rs"]
mod allocations;
#[path = "common/animated_refresh.rs"]
mod fixture;
use fixture::*;
use incant_assets_runtime::{RuntimeAsset, RuntimeAssetData, animation::AnimationPlayer};
use incant_render_native::{
    NativeRenderError, NativeRenderInput, SyncError, UpdateKind, VisualCaches,
};
use incant_runtime::*;
use incant_types::{Animation, Camera, CameraProjection, Transform};
use std::{fs, path::Path, sync::Arc};

fn scene(root: &Path, references: Vec<VisualAssetRef>, root_motion: bool) -> CookedScene {
    let reference = references[0];
    let mut mesh = CookedSpatial::empty(id(1));
    mesh.mesh = Some(NativeMeshRenderer {
        mesh: reference.id,
        cast_shadows: true,
    });
    mesh.animation = Some(Arc::new(NativeAnimation {
        mesh_fingerprint: reference.fingerprint,
        clip_durations: Arc::from([2.; 3]),
        initial: Animation {
            clip: Some(0),
            seconds: 0.,
            speed: 1.,
            playing: true,
            looping: true,
        },
        graph: None,
    }));
    if root_motion {
        let model =
            incant_assets_runtime::load_model(&root.join(".incant/cache/models"), &key(reference))
                .unwrap();
        mesh.root_motion = Some(Arc::new(NativeRootMotion {
            catalog: Arc::new(
                incant_assets_runtime::animation::compile_root_motion_catalog(
                    &model,
                    reference.fingerprint,
                    1,
                )
                .unwrap(),
            ),
            application: incant_types::RootMotionApplication::Transform,
            applying: true,
        }));
    }
    let mut camera = CookedSpatial::empty(id(2));
    camera.camera = Some(Camera {
        fov_degrees: 60.,
        near: 0.1,
        far: 100.,
        projection: CameraProjection::Perspective {},
    });
    if references.len() == 2 {
        camera.environment = Some(NativeEnvironmentLight {
            texture: references[1].id,
            intensity: 1.,
            rotation_degrees: 0.,
        });
    }
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
    .unwrap()
}
fn caches(root: &Path) -> (std::path::PathBuf, std::path::PathBuf) {
    (
        root.join(".incant/cache/models"),
        root.join(".incant/cache/textures"),
    )
}
fn load(root: &Path, world: &NativeWorld) -> Result<NativeRenderInput, NativeRenderError> {
    let (models, textures) = caches(root);
    NativeRenderInput::load(
        &world.spatial_view().unwrap(),
        VisualCaches {
            models: &models,
            textures: &textures,
        },
        id(2),
    )
}
fn prepared(
    input: &NativeRenderInput,
    root: &Path,
    references: &[VisualAssetRef],
) -> incant_render_native::PreparedTextureRefresh {
    let (models, textures) = caches(root);
    input
        .texture_refresh_ticket()
        .prepare_visuals(
            references,
            VisualCaches {
                models: &models,
                textures: &textures,
            },
        )
        .unwrap()
}
fn mesh(input: &NativeRenderInput) -> &incant_render_data::MeshInput {
    input
        .input()
        .nodes
        .iter()
        .find_map(|n| n.mesh.as_ref())
        .unwrap()
}
fn position(asset: &RuntimeAsset, sample: AnimationSample) -> [f64; 3] {
    let RuntimeAssetData::Model(model) = asset.data() else {
        panic!("model")
    };
    let mut player = AnimationPlayer::new(model).unwrap();
    player
        .sample(sample.clip.map(usize::from), sample.seconds)
        .unwrap();
    let v = model.meshes[0].vertices[0];
    player.skin_position(3, 0, 0, [v[0], v[1], v[2]]).unwrap()
}
fn apply(world: &mut NativeWorld, command: StructuralCommand) {
    let mut commands = FrameCommands::default();
    commands.push(command).unwrap();
    world.apply(&mut commands).unwrap();
}
fn sync(input: &mut NativeRenderInput, world: &NativeWorld) {
    input
        .try_sync(&world.spatial_view().unwrap(), |_, _, _| Ok::<_, ()>(()))
        .unwrap();
}

#[test]
fn clip_keys_material_images_and_texture_publish_atomically_at_current_clock_without_allocation() {
    let root = tempfile::tempdir().unwrap();
    let original = model(root.path(), Change::Original);
    let original_texture = texture(root.path(), RED);
    let mut world = NativeWorld::new(scene(root.path(), vec![original, original_texture], false));
    let identity = world.spatial_view().unwrap().identity().clone();
    let mut input = load(root.path(), &world).unwrap();
    for _ in 0..30 {
        world.step().unwrap();
    }
    sync(&mut input, &world);
    let old = mesh(&input).source.clone();
    let next = model(root.path(), Change::Keys);
    let next_texture = texture(root.path(), DARK);
    assert!(
        input
            .texture_refresh_ticket()
            .prepare(&[next, next_texture], &caches(root.path()).1)
            .is_err()
    );
    let mut staged = input
        .stage_texture_refresh(
            &world.spatial_view().unwrap(),
            prepared(&input, root.path(), &[next, next_texture]),
        )
        .unwrap();
    // Simulation may advance during cold CPU/GPU preparation. Final publication
    // must copy the newest complete sample rather than the staged old clock.
    for _ in 0..2 {
        world.step().unwrap();
    }
    sync(&mut input, &world);
    let state = world.animation_state(id(1)).unwrap();
    let (receipt, count, disposals) = allocations::record_lifecycle(|| {
        input
            .try_publish_textures(&world.spatial_view().unwrap(), &mut staged, || {
                Ok::<_, ()>(())
            })
            .unwrap()
    });
    assert_eq!((count, disposals), (0, 0));
    assert_eq!((receipt.changed, receipt.generation), (2, 1));
    assert_eq!(world.tick(), 32);
    assert_eq!(world.spatial_view().unwrap().identity(), &identity);
    assert_eq!(world.animation_state(id(1)), Some(state));
    assert_eq!(mesh(&input).animation, Some(state.sample));
    assert_eq!(mesh(&input).source.info().fingerprint, key(next));
    assert_ne!(mesh(&input).source.version_id(), old.version_id());
    assert_eq!(
        input
            .input()
            .environment
            .as_ref()
            .unwrap()
            .source
            .info()
            .fingerprint,
        key(next_texture)
    );
    let RuntimeAssetData::Model(old_model) = old.data() else {
        panic!()
    };
    let RuntimeAssetData::Model(next_model) = mesh(&input).source.data() else {
        panic!()
    };
    assert_eq!(old_model.meshes, next_model.meshes);
    assert_eq!(
        old_model.animation.as_ref().unwrap().clips[0].channels[0]
            .times
            .len(),
        2
    );
    assert_eq!(
        next_model.animation.as_ref().unwrap().clips[0].channels[0]
            .times
            .len(),
        3
    );
    assert_ne!(old_model.images[0].levels, next_model.images[0].levels);
    assert_eq!(next_model.materials[0].base_color, [0.2, 0.4, 0.6, 1.]);
    let old_position = position(&old, state.sample);
    let new_position = position(&mesh(&input).source, state.sample);
    assert!((old_position[0] - (11. + state.sample.seconds)).abs() < 1e-6);
    assert!((new_position[0] - (11. + 2. * state.sample.seconds)).abs() < 1e-6);
    assert_ne!(old_position, new_position);
    root.close().unwrap();
    for _ in 0..10 {
        world.step().unwrap();
        let (result, count) = allocations::record(|| {
            input.try_sync(&world.spatial_view().unwrap(), |_, _, kind| {
                assert_eq!(kind, UpdateKind::Motion);
                Ok::<_, ()>(())
            })
        });
        result.unwrap();
        assert_eq!(count, 0);
    }
    assert_eq!(
        mesh(&input).animation,
        world.animation_state(id(1)).map(|s| s.sample)
    );
    assert_eq!(old.info().fingerprint, key(original));
}

#[test]
fn consumer_refusal_unwind_and_stale_catalog_preserve_the_complete_prior_animation() {
    let root = tempfile::tempdir().unwrap();
    let original = model(root.path(), Change::Original);
    let world = NativeWorld::new(scene(root.path(), vec![original], false));
    let mut input = load(root.path(), &world).unwrap();
    let old = mesh(&input).source.clone();
    let next = model(root.path(), Change::Keys);
    let stale = input.texture_refresh_ticket();
    let mut stage = input
        .stage_texture_refresh(
            &world.spatial_view().unwrap(),
            prepared(&input, root.path(), &[next]),
        )
        .unwrap();
    assert!(matches!(
        input.try_publish_textures(&world.spatial_view().unwrap(), &mut stage, || Err(
            "late consumer"
        )),
        Err(SyncError::Consumer("late consumer"))
    ));
    assert!(Arc::ptr_eq(&old, &mesh(&input).source));
    let mut stage = input
        .stage_texture_refresh(
            &world.spatial_view().unwrap(),
            prepared(&input, root.path(), &[next]),
        )
        .unwrap();
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = input.try_publish_textures(
                &world.spatial_view().unwrap(),
                &mut stage,
                || -> Result<(), ()> { panic!("late publication") },
            );
        }))
        .is_err()
    );
    assert!(Arc::ptr_eq(&old, &mesh(&input).source));
    assert_eq!((input.asset_generation(), world.tick()), (0, 0));
    let mut stage = input
        .stage_texture_refresh(
            &world.spatial_view().unwrap(),
            prepared(&input, root.path(), &[next]),
        )
        .unwrap();
    input
        .try_publish_textures(&world.spatial_view().unwrap(), &mut stage, || {
            Ok::<_, ()>(())
        })
        .unwrap();
    let (models, textures) = caches(root.path());
    let prepared = stale
        .prepare_visuals(
            &[next],
            VisualCaches {
                models: &models,
                textures: &textures,
            },
        )
        .unwrap();
    assert!(
        input
            .stage_texture_refresh(&world.spatial_view().unwrap(), prepared)
            .is_err()
    );
    assert_eq!(mesh(&input).source.info().fingerprint, key(next));
    assert_eq!(old.info().fingerprint, key(original));
}

#[path = "animated_refresh/refusals.rs"]
mod refusals;

#[path = "animated_refresh/continuity.rs"]
mod continuity;

#[path = "animated_refresh/vertex_content.rs"]
mod vertex_content;

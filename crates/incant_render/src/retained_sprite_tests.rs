//! Numeric native sprite agreement and lifetime/rollback checks, not look-dev.
use super::*;
use crate::hdr_output::{read, texture};
use incant_doc::{Asset, Entity, Project, Scene, Transform, new_id};
use incant_render_native::{NativeRenderInput, UpdateKind, VisualCaches};
use incant_runtime::{FrameCommands, NativeWorld, StableId, StructuralCommand};
use serde_json::json;
use std::path::Path;
const A: &str = "00000000000000000000000001";
const B: &str = "00000000000000000000000002";
pub(super) fn fixture() -> (NativeWorld, NativeRenderInput, StableId) {
    let (world, input, moving, _) = fixture_kind(false);
    (world, input, moving)
}
pub(super) fn tile_fixture() -> (NativeWorld, NativeRenderInput, StableId) {
    let (world, input, moving, _) = fixture_kind(true);
    (world, input, moving)
}
pub(super) fn tile_fixture_with_authoring() -> (NativeWorld, NativeRenderInput, StableId, SceneInput)
{
    fixture_kind(true)
}
fn fixture_kind(tiles: bool) -> (NativeWorld, NativeRenderInput, StableId, SceneInput) {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    let mut encoder = png::Encoder::new(
        std::fs::File::create(root.join("pixels.png")).unwrap(),
        4,
        4,
    );
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let pixels: Vec<u8> = (0..16)
        .flat_map(|n| {
            [
                if n % 2 == 0 { 255 } else { 0 },
                if n % 3 == 0 { 255 } else { 0 },
                128,
                if n % 2 == 0 { 255 } else { 128 },
            ]
        })
        .collect();
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&pixels)
        .unwrap();
    let descriptor = json!({"version":1,"id":"00000000000000000000000009","maximum_size":[32,32],"padding":2,"frames":{(A):{"name":"Whole","path":"pixels.png"},(B):{"name":"Part","path":"pixels.png","region":[1,1,2,2]}}});
    std::fs::write(root.join("sheet.atlas.json"), descriptor.to_string()).unwrap();
    let cooked = incant_assets::cook_texture(
        root,
        Path::new("sheet.atlas.json"),
        &root.join("textures"),
        incant_assets::TextureUsage::Color,
    )
    .unwrap();
    let mut project = Project::empty("Native numerical sprite fixture");
    let texture_id = new_id();
    project.assets.insert(
        texture_id.clone(),
        Asset {
            id: texture_id.clone(),
            name: "Atlas".into(),
            path: "sheet.atlas.json".into(),
            kind: "texture".into(),
            sha256: cooked.metadata.fingerprint,
            import_settings: None,
        },
    );
    let mut scene = Scene::new("Main");
    let mut parent = Entity::new("Moving parent");
    let parent_id = parent.id.clone();
    parent
        .components
        .insert("Transform".into(), json!(Transform::default()));
    parent
        .components
        .insert("Velocity".into(), json!({"linear":[1,0,0]}));
    scene.entities.insert(parent.id.clone(), parent);
    let mut camera = Entity::new("Camera");
    let camera_id = incant_cook::stable_id(&camera.id).unwrap();
    camera.components.insert(
        "Transform".into(),
        json!(Transform {
            translation: [0., 0., 8.],
            ..Default::default()
        }),
    );
    camera.components.insert("Camera".into(),json!({"fov_degrees":60,"near":0.1,"far":100,"projection":{"kind":"pixel_perfect","pixels_per_unit":8,"reference_resolution":[64,64]}}));
    scene.entities.insert(camera.id.clone(), camera);
    let mut moving = None;
    for n in 0..3 {
        let mut entity = Entity::new("Sprite");
        entity.parent = Some(parent_id.clone());
        entity.components.insert(
            "Transform".into(),
            json!(Transform {
                translation: [-1. + n as f64 * 0.75, n as f64 * 0.25, 0.],
                ..Default::default()
            }),
        );
        entity.components.insert("SpriteRenderer".into(),json!({"texture":texture_id,"region":{"frame":if n==1{B}else{A}},"size":[2.5,2.0],"pivot":[0.5,0.5],"tint":[1,0.75,1,0.8],"flip_x":n==1,"flip_y":n==2,"layer":n-1,"order":3-n,"sampling":if n==1{"linear"}else{"nearest"},"mode":if n==2{json!({"kind":"nine_slice","border":[1,1,1,1],"pixels_per_unit":4})}else{json!({"kind":"simple"})}}));
        if n == 0 {
            moving = Some(incant_cook::stable_id(&entity.id).unwrap());
        }
        scene.entities.insert(entity.id.clone(), entity);
    }
    if tiles {
        for (id, offset) in [
            ("00000000000000000000000006", 0.),
            ("00000000000000000000000007", 0.75),
        ] {
            let mut entity = Entity::new("Numerical tile map");
            entity.id = id.into();
            entity.parent = Some(parent_id.clone());
            entity.components.insert(
                "Transform".into(),
                json!(Transform {
                    translation: [-2. + offset, -1. + offset, 0.],
                    ..Default::default()
                }),
            );
            entity.components.insert("Tilemap".into(),json!({"tileset":texture_id,"size":[3,2],"cell_size":[0.75,0.5],"layers":[
                {"id":"00000000000000000000000004","layer":2,"order":-1,"cells":[null,A,null,B,null,null]},
                {"id":"00000000000000000000000003","layer":-2,"order":3,"cells":[A,B,null,null,A,B]}
            ]}));
            scene.entities.insert(entity.id.clone(), entity);
        }
    }
    let scene_id = scene.id.clone();
    project.scenes.insert(scene_id.clone(), scene);
    let bytes = incant_cook::cook_scene(&project, &scene_id).unwrap();
    assert_eq!(
        &bytes[8..12],
        &(if tiles { 6u32 } else { 5u32 }).to_le_bytes()
    );
    let world = NativeWorld::from_bytes(&bytes).unwrap();
    let mut assets = incant_assets::AssetStore::default();
    assets.sync(&project, root).unwrap();
    let authoring = incant_render_authoring::project_input(&project, Some(&assets)).unwrap();
    drop(assets);
    drop(project);
    drop(bytes);
    let input = NativeRenderInput::load(
        &world.spatial_view().unwrap(),
        VisualCaches {
            models: &root.join("models"),
            textures: &root.join("textures"),
        },
        camera_id,
    )
    .unwrap();
    directory.close().unwrap();
    (world, input, moving.unwrap(), authoring)
}
#[test]
#[ignore = "requires a native GPU; run by desktop workflows"]
fn source_free_native_atlas_sprites_match_fresh_frames_across_motion_reflection_and_rebuild() {
    let (mut world, mut input, moving) = fixture();
    let renderer = Renderer::headless().unwrap();
    let mut gpu = renderer
        .prepare_retained(input.input(), input.camera_label())
        .unwrap();
    assert_eq!(gpu.stats().sprite_entities, 3);
    let (vertices, storage, upload_bytes) = gpu
        .live
        .sprites
        .as_ref()
        .unwrap()
        .retained_storage(gpu.staging.sprites.as_ref().unwrap());
    // The old two complete 64-byte CPU/GPU streams plus two 16-byte local
    // position arrays required at least 288 bytes/vertex. Measure actual buffer
    // sizes and CPU capacities here; sharing must save more than half even for
    // this small, unevenly sized fixture with vector capacity slack.
    assert!(
        storage < vertices * 288 / 2,
        "{vertices} vertices use {storage} bytes"
    );
    assert_eq!(upload_bytes, vertices * 12);
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let output = texture(&renderer, format, 64, 64);
    let view = output.create_view(&Default::default());
    let target = |width| RetainedTarget {
        view: &view,
        format,
        width,
        height: 64,
        viewport: None,
    };
    gpu.submit(&renderer, input.input(), target(64)).unwrap();
    let initial = read(&renderer, &output);
    let mut changed = false;
    let mut rebuilt = 0;
    for frame in 0..8 {
        world.with_numeric_columns(|columns| {
            columns.for_each_chunk(|chunk| {
                for (id, t) in chunk
                    .ids
                    .iter()
                    .zip(chunk.transforms.as_chunks_mut::<10>().0)
                {
                    if *id == moving {
                        t[7] = if frame % 2 == 0 { -1. } else { 1. };
                        t[8] = 1. + frame as f64 * 0.0625;
                    }
                }
            })
        });
        if frame == 4 {
            let mut node = world
                .spatial_view()
                .unwrap()
                .nodes()
                .find(|n| n.id == moving)
                .unwrap()
                .declaration
                .unwrap()
                .clone();
            node.sprite.as_mut().unwrap().layer = 3;
            let mut commands = FrameCommands::default();
            commands
                .push(StructuralCommand::SetSpatial {
                    entity: moving,
                    spatial: Some(node),
                })
                .unwrap();
            world.apply(&mut commands).unwrap();
        }
        world.step().unwrap();
        let spatial = world.spatial_view().unwrap();
        if frame == 3 {
            let old_tick = input.tick();
            let old = renderer.screenshot_scene_png(&gpu.live, 64, 64).unwrap();
            assert!(
                input
                    .try_sync(&spatial, |scene, _, _| gpu.submit(
                        &renderer,
                        scene,
                        target(0)
                    ))
                    .is_err()
            );
            assert_eq!(input.tick(), old_tick);
            assert_eq!(
                renderer.screenshot_scene_png(&gpu.live, 64, 64).unwrap(),
                old
            );
        }
        let replacement = input
            .try_sync(
                &spatial,
                |scene, camera, kind| -> Result<Option<RetainedScene>> {
                    if kind == UpdateKind::Rebuilt {
                        rebuilt += 1;
                        let mut candidate = renderer.prepare_retained(scene, camera)?;
                        candidate.submit(&renderer, scene, target(64))?;
                        Ok(Some(candidate))
                    } else {
                        gpu.submit(&renderer, scene, target(64))?;
                        Ok(None)
                    }
                },
            )
            .unwrap();
        if let Some(candidate) = replacement {
            gpu = candidate;
        }
        let actual = read(&renderer, &output);
        changed |= actual != initial;
        let reference = renderer
            .prepare(input.input())
            .unwrap()
            .with_camera(input.camera_label())
            .unwrap();
        renderer.queue.submit([renderer
            .draw_scene(&reference, &view, format, 64, 64, None)
            .unwrap()]);
        assert_eq!(
            actual,
            read(&renderer, &output),
            "native sprite frame {frame}"
        );
    }
    assert!(changed);
    assert_eq!(rebuilt, 1);
}

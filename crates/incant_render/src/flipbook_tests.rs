//! Exact retained-frame comparisons against the existing static sprite path.
use super::*;
use crate::hdr_output::{read, texture};
use incant_render_data::{EnvironmentInput, MeshInput, SceneNode, SpriteInput};
use incant_render_native::UpdateKind;
use incant_runtime::{
    FlipbookControl, FrameCommands, FrameRate, NativeFlipbook, NativeSpriteRegion,
    StructuralCommand,
};
use std::sync::Arc;

fn static_selection(input: &SceneInput) -> SceneInput {
    SceneInput {
        text: None,
        nodes: input
            .nodes
            .iter()
            .map(|node| SceneNode {
                id: node.id.clone(),
                world: node.world,
                camera: node.camera.clone(),
                light: node.light.clone(),
                mesh: node.mesh.as_ref().map(|m| MeshInput {
                    source: Arc::clone(&m.source),
                    cast_shadows: m.cast_shadows,
                    animation: m.animation,
                    animation_blend: m.animation_blend,
                }),
                sprite: node.sprite.as_ref().map(|s| {
                    let mut sprite = s.sprite.clone();
                    if let Some(frames) = &s.frames {
                        sprite.region = frames.regions()[usize::from(s.frame)].into();
                    }
                    SpriteInput {
                        source: Arc::clone(&s.source),
                        sprite,
                        frames: None,
                        frame: 0,
                    }
                }),
                tilemap: node.tilemap.clone(),
                diagnostic: node.diagnostic,
            })
            .collect(),
        environment: input.environment.as_ref().map(|e| EnvironmentInput {
            source: Arc::clone(&e.source),
            intensity: e.intensity,
            rotation_degrees: e.rotation_degrees,
        }),
    }
}
#[test]
#[ignore = "requires a native GPU; run by desktop workflows"]
fn retained_flipbooks_match_static_frames_without_rebuilding_after_cache_deletion() {
    let (mut world, mut input, _) = super::sprite_tests::tile_fixture();
    let declarations: Vec<_> = world
        .spatial_view()
        .unwrap()
        .nodes()
        .filter_map(|node| {
            let mut declared = node.declaration?.clone();
            declared.sprite.as_ref()?;
            declared.flipbook = Some(Arc::new(NativeFlipbook {
                frames: vec![
                    NativeSpriteRegion::Atlas {
                        frame: incant_cook::stable_id("00000000000000000000000001").unwrap(),
                    },
                    NativeSpriteRegion::Atlas {
                        frame: incant_cook::stable_id("00000000000000000000000002").unwrap(),
                    },
                ]
                .into(),
                rate: FrameRate {
                    numerator: 13,
                    denominator: 3,
                },
                looping: true,
                playing: true,
            }));
            Some(declared)
        })
        .collect();
    let ids: Vec<_> = declarations.iter().map(|d| d.entity).collect();
    let mut commands = FrameCommands::default();
    for declaration in declarations {
        commands
            .push(StructuralCommand::SetSpatial {
                entity: declaration.entity,
                spatial: Some(declaration),
            })
            .unwrap();
    }
    world.apply(&mut commands).unwrap();
    input
        .try_sync(&world.spatial_view().unwrap(), |_, _, kind| {
            assert_eq!(kind, UpdateKind::Rebuilt);
            Ok::<_, ()>(())
        })
        .unwrap();
    let renderer = Renderer::headless().unwrap();
    let mut gpu = renderer
        .prepare_retained(input.input(), input.camera_label())
        .unwrap();
    let initial_storage = gpu
        .live
        .sprites
        .as_ref()
        .unwrap()
        .retained_storage(gpu.staging.sprites.as_ref().unwrap());
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
    for tick in 1..=32 {
        if tick == 7 {
            for (index, &entity) in ids.iter().enumerate() {
                commands
                    .push(StructuralCommand::ControlFlipbook {
                        entity,
                        control: FlipbookControl::Seek((index % 2) as u16),
                    })
                    .unwrap();
            }
            world.apply(&mut commands).unwrap();
        }
        world.step().unwrap();
        let spatial = world.spatial_view().unwrap();
        if tick == 14 {
            let old_tick = input.tick();
            let old_frame = input
                .input()
                .nodes
                .iter()
                .filter_map(|n| n.sprite.as_ref())
                .map(|s| s.frame)
                .collect::<Vec<_>>();
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
                old_frame,
                input
                    .input()
                    .nodes
                    .iter()
                    .filter_map(|n| n.sprite.as_ref())
                    .map(|s| s.frame)
                    .collect::<Vec<_>>()
            );
        }
        input
            .try_sync(&spatial, |scene, _, kind| {
                assert_eq!(kind, UpdateKind::Motion);
                gpu.submit(&renderer, scene, target(64))
            })
            .unwrap();
        assert_eq!(
            gpu.live
                .sprites
                .as_ref()
                .unwrap()
                .retained_storage(gpu.staging.sprites.as_ref().unwrap()),
            initial_storage
        );
        let actual = read(&renderer, &output);
        changed |= actual != initial;
        let reference = renderer
            .prepare(&static_selection(input.input()))
            .unwrap()
            .with_camera(input.camera_label())
            .unwrap();
        renderer.queue.submit([renderer
            .draw_scene(&reference, &view, format, 64, 64, None)
            .unwrap()]);
        assert_eq!(actual, read(&renderer, &output), "flipbook tick {tick}");
    }
    assert!(changed);
    // Public SceneInput is checked even when an external consumer changes frame numbers.
    let mut malformed = static_selection(input.input());
    for (bad, source) in malformed.nodes.iter_mut().zip(&input.input().nodes) {
        if let (Some(bad), Some(source)) = (&mut bad.sprite, &source.sprite) {
            bad.sprite = source.sprite.clone();
            bad.frames = source.frames.clone();
            bad.frame = 2;
        }
    }
    assert!(gpu.stage(&malformed).is_err());
    assert!(renderer.prepare(&malformed).is_err());
    gpu.submit(&renderer, input.input(), target(64)).unwrap();
}

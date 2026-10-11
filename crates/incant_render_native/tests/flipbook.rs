#[path = "common/allocations.rs"]
mod allocation_counter;
mod common;
use common::*;
use incant_assets_runtime::TextureUsage;
use incant_render_data::SpriteInput;
use incant_render_native::{NativeRenderInput, UpdateKind, VisualCaches};
use incant_runtime::*;
use std::{fs, path::Path, sync::Arc};

fn declaration(frames: Vec<NativeSpriteRegion>) -> CookedSpatial {
    let mut node = CookedSpatial::empty(id(3));
    node.sprite = Some(Box::new(NativeSpriteRenderer {
        texture: id(90),
        region: NativeSpriteRegion::Pixels([0, 0, 1, 1]),
        size: [1.; 2],
        pivot: [0.5; 2],
        tint: [1.; 4],
        flip_x: false,
        flip_y: false,
        layer: 0,
        order: 0,
        sampling: SpriteSampling::Nearest,
        mode: SpriteMode::Simple,
    }));
    node.flipbook = Some(Arc::new(NativeFlipbook {
        frames: frames.into(),
        rate: FrameRate {
            numerator: 24,
            denominator: 1,
        },
        looping: true,
        playing: true,
    }));
    node
}
fn regions() -> Vec<NativeSpriteRegion> {
    vec![
        NativeSpriteRegion::Pixels([0, 0, 1, 1]),
        NativeSpriteRegion::Pixels([1, 0, 1, 1]),
    ]
}
fn prepare(frames: Vec<NativeSpriteRegion>) -> (tempfile::TempDir, NativeWorld) {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    let mut encoder = png::Encoder::new(fs::File::create(root.join("pixels.png")).unwrap(), 2, 1);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&[255, 0, 0, 255, 0, 0, 255, 255])
        .unwrap();
    let cooked = incant_assets::cook_texture(
        root,
        Path::new("pixels.png"),
        &root.join("textures"),
        TextureUsage::Color,
    )
    .unwrap();
    let key = cooked.metadata.fingerprint;
    let scene = scene()
        .with_spatial(
            vec![VisualAssetRef {
                id: id(90),
                kind: VisualAssetKind::Texture,
                fingerprint: std::array::from_fn(|i| {
                    u8::from_str_radix(&key[i * 2..i * 2 + 2], 16).unwrap()
                }),
            }],
            vec![common::declaration(2), declaration(frames)],
        )
        .unwrap();
    fs::remove_file(root.join("pixels.png")).unwrap();
    (
        directory,
        NativeWorld::from_bytes(&scene.to_bytes()).unwrap(),
    )
}
fn load(
    directory: &tempfile::TempDir,
    world: &NativeWorld,
) -> Result<NativeRenderInput, incant_render_native::NativeRenderError> {
    NativeRenderInput::load(
        &world.spatial_view().unwrap(),
        VisualCaches {
            models: &directory.path().join("models"),
            textures: &directory.path().join("textures"),
        },
        id(2),
    )
}
fn sprite(input: &NativeRenderInput) -> &SpriteInput {
    input
        .input()
        .nodes
        .iter()
        .find_map(|n| n.sprite.as_ref())
        .unwrap()
}
#[test]
fn source_free_frames_reuse_bindings_and_numeric_storage_through_rational_timing_and_controls() {
    let (directory, mut world) = prepare(regions());
    let mut input = load(&directory, &world).unwrap();
    let frames = Arc::clone(sprite(&input).frames.as_ref().unwrap());
    let source = Arc::clone(&sprite(&input).source);
    let nodes = input.input().nodes.as_ptr();
    let references = Arc::strong_count(&frames);
    directory.close().unwrap();
    for tick in 1..=100 {
        world.step().unwrap();
        let (result, allocations) = allocation_counter::record(|| {
            input.try_sync(&world.spatial_view().unwrap(), |_, _, kind| {
                assert_eq!(kind, UpdateKind::Motion);
                Ok::<_, ()>(())
            })
        });
        result.unwrap();
        assert_eq!(allocations, 0);
        assert_eq!(sprite(&input).frame, ((tick * 24 / 60) % 2) as u16);
        assert_eq!(input.input().nodes.as_ptr(), nodes);
        assert!(Arc::ptr_eq(
            &frames,
            sprite(&input).frames.as_ref().unwrap()
        ));
        assert!(Arc::ptr_eq(&source, &sprite(&input).source));
        assert_eq!(Arc::strong_count(&frames), references);
    }
    world
        .apply(&mut queue([
            StructuralCommand::ControlFlipbook {
                entity: id(3),
                control: FlipbookControl::Seek(1),
            },
            StructuralCommand::ControlFlipbook {
                entity: id(3),
                control: FlipbookControl::Pause,
            },
        ]))
        .unwrap();
    for _ in 0..3 {
        world.step().unwrap();
    }
    input
        .try_sync(&world.spatial_view().unwrap(), |_, _, kind| {
            assert_eq!(kind, UpdateKind::Motion);
            Ok::<_, ()>(())
        })
        .unwrap();
    assert_eq!(sprite(&input).frame, 1);
    assert!(Arc::ptr_eq(
        &frames,
        sprite(&input).frames.as_ref().unwrap()
    ));
    let mut replacement = declaration(regions().into_iter().rev().collect());
    Arc::make_mut(replacement.flipbook.as_mut().unwrap()).playing = false;
    world
        .apply(&mut queue([StructuralCommand::SetSpatial {
            entity: id(3),
            spatial: Some(replacement),
        }]))
        .unwrap();
    input
        .try_sync(&world.spatial_view().unwrap(), |_, _, kind| {
            assert_eq!(kind, UpdateKind::Rebuilt);
            Ok::<_, ()>(())
        })
        .unwrap();
    assert_eq!(sprite(&input).frame, 0);
    assert_eq!(
        sprite(&input).frames.as_ref().unwrap().regions()[0],
        [1, 0, 1, 1]
    );
    assert!(Arc::ptr_eq(&source, &sprite(&input).source));
}
#[test]
fn rejected_or_unwound_frame_consumer_preserves_complete_prior_numeric_input() {
    let (directory, mut world) = prepare(regions());
    let mut input = load(&directory, &world).unwrap();
    directory.close().unwrap();
    for _ in 0..3 {
        world.step().unwrap();
    }
    let before = matrices(&input);
    let generation = generations(&input);
    assert!(
        input
            .try_sync(&world.spatial_view().unwrap(), |scene, _, _| {
                assert_eq!(
                    scene
                        .nodes
                        .iter()
                        .find_map(|n| n.sprite.as_ref())
                        .unwrap()
                        .frame,
                    1
                );
                Err::<(), _>("declined")
            })
            .is_err()
    );
    assert_eq!(sprite(&input).frame, 0);
    assert_eq!(matrices(&input), before);
    assert_eq!(generations(&input), generation);
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _: Result<(), _> = input.try_sync(
                &world.spatial_view().unwrap(),
                |_, _, _| -> Result<(), ()> { panic!("consumer") },
            );
        }))
        .is_err()
    );
    assert_eq!(sprite(&input).frame, 0);
    assert_eq!(matrices(&input), before);
    assert_eq!(generations(&input), generation);
    input
        .try_sync(&world.spatial_view().unwrap(), |_, _, _| Ok::<_, ()>(()))
        .unwrap();
    assert_eq!(sprite(&input).frame, 1);
    assert_eq!(input.tick(), 3);
}
#[test]
fn every_frame_is_resolved_before_first_publication_and_bad_rebind_keeps_prior_frame() {
    for bad in [
        NativeSpriteRegion::Pixels([2, 0, 1, 1]),
        NativeSpriteRegion::Atlas { frame: id(100) },
    ] {
        let (directory, world) = prepare(vec![regions()[0], bad]);
        assert!(load(&directory, &world).is_err()); // Invalid noncurrent frame.
    }
    let (directory, mut world) = prepare(regions());
    let mut input = load(&directory, &world).unwrap();
    directory.close().unwrap();
    let old = Arc::clone(sprite(&input).frames.as_ref().unwrap());
    let bad = declaration(vec![regions()[0], NativeSpriteRegion::Pixels([2, 0, 1, 1])]);
    world
        .apply(&mut queue([StructuralCommand::SetSpatial {
            entity: id(3),
            spatial: Some(bad),
        }]))
        .unwrap();
    assert!(
        input
            .try_sync(
                &world.spatial_view().unwrap(),
                |_, _, _| -> Result<(), ()> { panic!("invalid candidate reached consumer") }
            )
            .is_err()
    );
    assert!(Arc::ptr_eq(&old, sprite(&input).frames.as_ref().unwrap()));
    assert_eq!(input.binding_generation(), 0);
}

#[test]
fn animation_replaces_unused_static_region_and_a_removal_revalidates_the_static_binding() {
    let (directory, mut world) = prepare(regions());
    let mut node = declaration(regions());
    node.sprite.as_mut().unwrap().region = NativeSpriteRegion::Atlas { frame: id(100) };
    world
        .apply(&mut queue([StructuralCommand::SetSpatial {
            entity: id(3),
            spatial: Some(node.clone()),
        }]))
        .unwrap();
    let mut input = load(&directory, &world).unwrap();
    assert_eq!(sprite(&input).sprite.region.pixels(), Some([0, 0, 1, 1]));
    directory.close().unwrap();
    node.flipbook = None;
    world
        .apply(&mut queue([StructuralCommand::SetSpatial {
            entity: id(3),
            spatial: Some(node),
        }]))
        .unwrap();
    assert!(
        input
            .try_sync(&world.spatial_view().unwrap(), |_, _, _| Ok::<_, ()>(()))
            .is_err()
    );
    assert!(sprite(&input).frames.is_some());
}

mod common;
use common::*;
use incant_assets_runtime::{RuntimeAssetData, TextureUsage};
use incant_render_native::{
    NativeRenderError, NativeRenderInput, TilemapAtlasBinding, VisualCaches,
};
use incant_runtime::{
    CookedSpatial, NativeTilemap, NativeTilemapLayer, NativeWorld, StructuralCommand,
    VisualAssetKind, VisualAssetRef,
};
use serde_json::json;
use std::{
    fs,
    path::{Path, PathBuf},
};

struct Fixture {
    directory: tempfile::TempDir,
    cache: PathBuf,
    atlas: VisualAssetRef,
    ordinary: VisualAssetRef,
}
impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source");
        fs::create_dir(&source).unwrap();
        let cache = directory.path().join("cache");
        let mut encoder =
            png::Encoder::new(fs::File::create(source.join("tile.png")).unwrap(), 2, 1);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&[255, 0, 0, 255, 0, 255, 0, 255])
            .unwrap();
        fs::write(source.join("tiles.atlas.json"),json!({"version":1,"id":"00000000000000000000000009","maximum_size":[16,16],"padding":1,"frames":{"00000000000000000000000008":{"name":"Tile","path":"tile.png"}}}).to_string()).unwrap();
        let reference = |key: &str| VisualAssetRef {
            id: id(90),
            kind: VisualAssetKind::Texture,
            fingerprint: std::array::from_fn(|i| {
                u8::from_str_radix(&key[i * 2..i * 2 + 2], 16).unwrap()
            }),
        };
        let atlas = reference(
            &incant_assets::cook_texture(
                &source,
                Path::new("tiles.atlas.json"),
                &cache,
                TextureUsage::Color,
            )
            .unwrap()
            .metadata
            .fingerprint,
        );
        let ordinary = reference(
            &incant_assets::cook_texture(
                &source,
                Path::new("tile.png"),
                &cache,
                TextureUsage::Color,
            )
            .unwrap()
            .metadata
            .fingerprint,
        );
        fs::remove_dir_all(source).unwrap();
        Self {
            directory,
            cache,
            atlas,
            ordinary,
        }
    }
    fn map(&self) -> NativeTilemap {
        NativeTilemap {
            tileset: self.atlas.id,
            size: [2, 1],
            cell_size: [0.5, 1.],
            layers: vec![NativeTilemapLayer {
                id: id(7),
                cells: vec![Some(id(8)), Some(id(8))],
                layer: 0,
                order: 0,
                solid: false,
            }],
        }
    }
    fn caches(&self) -> VisualCaches<'_> {
        VisualCaches {
            models: Path::new("absent-models"),
            textures: &self.cache,
        }
    }
}
#[test]
fn checked_tile_atlas_bindings_survive_source_and_cache_removal_and_reject_wrong_references() {
    let fixture = Fixture::new();
    let map = fixture.map();
    let retained = TilemapAtlasBinding::load(&map, &fixture.atlas, &fixture.cache).unwrap();
    assert_eq!(retained.regions().len(), 1);
    assert_eq!(&retained.regions()[&id(8)][2..], &[2, 1]);
    let prepared = map.prepare().unwrap();
    assert_eq!(prepared.cell_bounds([0, 0]), [[0., 0.], [0.5, 1.]]); // Atlas packing cannot offset local grid geometry.
    assert!(matches!(
        TilemapAtlasBinding::load(&map, &fixture.ordinary, &fixture.cache),
        Err(NativeRenderError::TilemapBinding)
    ));
    for invalid in [
        VisualAssetRef {
            id: id(91),
            ..fixture.atlas
        },
        VisualAssetRef {
            kind: VisualAssetKind::Model,
            ..fixture.atlas
        },
    ] {
        assert!(matches!(
            TilemapAtlasBinding::load(&map, &invalid, &fixture.cache),
            Err(NativeRenderError::TilemapBinding)
        ));
    }
    let mut stale = fixture.atlas;
    stale.fingerprint[0] ^= 1;
    assert!(matches!(
        TilemapAtlasBinding::load(&map, &stale, &fixture.cache),
        Err(NativeRenderError::Asset { .. })
    ));
    let mut missing = map.clone();
    missing.layers[0].cells[1] = Some(id(9));
    assert!(
        matches!(TilemapAtlasBinding::load(&missing,&fixture.atlas,&fixture.cache),Err(NativeRenderError::TilemapFrame(frame)) if frame==id(9))
    );
    fixture.directory.close().unwrap();
    let RuntimeAssetData::Texture(texture) = retained.source().data() else {
        panic!("texture")
    };
    assert!(!texture.texture.levels[0].is_empty());
    assert_eq!(retained.regions().len(), 1);
    assert_eq!(prepared.placements().len(), 2);
}
#[test]
fn tile_input_binds_once_and_failed_rebuild_preserves_previous_input() {
    let fixture = Fixture::new();
    let base = scene()
        .with_spatial(vec![fixture.atlas], vec![declaration(2), declaration(3)])
        .unwrap();
    let mut world = NativeWorld::new(base);
    let mut input =
        NativeRenderInput::load(&world.spatial_view().unwrap(), fixture.caches(), id(2)).unwrap();

    let mut map = CookedSpatial::empty(id(3));
    map.tilemap = Some(Box::new(fixture.map()));
    world
        .apply(&mut queue([StructuralCommand::SetSpatial {
            entity: id(3),
            spatial: Some(map.clone()),
        }]))
        .unwrap();
    let initial =
        NativeRenderInput::load(&world.spatial_view().unwrap(), fixture.caches(), id(2)).unwrap();
    assert_eq!(
        initial
            .input()
            .nodes
            .iter()
            .find(|n| n.tilemap.is_some())
            .unwrap()
            .tilemap
            .as_ref()
            .unwrap()
            .cells()
            .len(),
        2
    );
    input
        .try_sync(&world.spatial_view().unwrap(), |_, _, kind| {
            assert_eq!(kind, incant_render_native::UpdateKind::Rebuilt);
            Ok::<_, ()>(())
        })
        .unwrap();
    let binding = std::sync::Arc::clone(
        input
            .input()
            .nodes
            .iter()
            .find(|n| n.tilemap.is_some())
            .unwrap()
            .tilemap
            .as_ref()
            .unwrap(),
    );
    fixture.directory.close().unwrap(); // No reads while motion or a candidate binding is prepared.
    for _ in 0..20 {
        world.step().unwrap();
        input
            .try_sync(&world.spatial_view().unwrap(), |candidate, _, kind| {
                assert_eq!(kind, incant_render_native::UpdateKind::Motion);
                assert!(std::sync::Arc::ptr_eq(
                    candidate
                        .nodes
                        .iter()
                        .find(|n| n.tilemap.is_some())
                        .unwrap()
                        .tilemap
                        .as_ref()
                        .unwrap(),
                    &binding
                ));
                Ok::<_, ()>(())
            })
            .unwrap();
    }
    let before = matrices(&input);
    let generations = generations(&input);
    map.tilemap.as_mut().unwrap().layers[0].cells[1] = Some(id(9));
    world
        .apply(&mut queue([StructuralCommand::SetSpatial {
            entity: id(3),
            spatial: Some(map),
        }]))
        .unwrap();
    let mut called = false;
    assert!(
        input
            .try_sync(&world.spatial_view().unwrap(), |_, _, _| {
                called = true;
                Ok::<_, ()>(())
            })
            .is_err()
    );
    assert!(!called);
    assert_eq!(matrices(&input), before);
    assert_eq!(common::generations(&input), generations);
    world
        .apply(&mut queue([StructuralCommand::SetSpatial {
            entity: id(3),
            spatial: None,
        }]))
        .unwrap();
    input
        .try_sync(&world.spatial_view().unwrap(), |_, _, _| Ok::<_, ()>(()))
        .unwrap();
}

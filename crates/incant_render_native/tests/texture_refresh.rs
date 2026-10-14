#[path = "common/allocations.rs"]
mod allocations;
mod common;
use common::*;
use incant_assets_runtime::TextureUsage;
use incant_render_native::{NativeRenderError, NativeRenderInput, SyncError, VisualCaches};
use incant_runtime::{
    CookedSpatial, NativeEnvironmentLight, NativeSpriteRegion, NativeSpriteRenderer, NativeWorld,
    SpriteMode, SpriteSampling, StructuralCommand, VisualAssetKind, VisualAssetRef,
};
use std::{
    fs,
    panic::{AssertUnwindSafe, catch_unwind},
    path::Path,
};

struct Fixture {
    root: tempfile::TempDir,
    references: Vec<VisualAssetRef>,
}
impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let mut result = Self {
            root,
            references: vec![],
        };
        result.references = vec![
            result.cook(90, 255, 2, TextureUsage::Color),
            result.cook(91, 127, 2, TextureUsage::Color),
        ];
        result
    }
    fn cook(&self, n: u128, color: u8, width: u32, usage: TextureUsage) -> VisualAssetRef {
        let name = format!("{n}.png");
        let mut encoder = png::Encoder::new(
            fs::File::create(self.root.path().join(&name)).unwrap(),
            width,
            1,
        );
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&[color, 0, 0, 255].repeat(width as usize))
            .unwrap();
        let key = incant_assets::cook_texture(
            self.root.path(),
            Path::new(&name),
            &self.root.path().join("textures"),
            usage,
        )
        .unwrap()
        .metadata
        .fingerprint;
        VisualAssetRef {
            id: id(n),
            kind: VisualAssetKind::Texture,
            fingerprint: std::array::from_fn(|i| {
                u8::from_str_radix(&key[i * 2..i * 2 + 2], 16).unwrap()
            }),
        }
    }
    fn world(&self) -> NativeWorld {
        let mut environment = CookedSpatial::empty(id(4));
        environment.environment = Some(NativeEnvironmentLight {
            texture: id(90),
            intensity: 1.,
            rotation_degrees: 0.,
        });
        let mut sprite = CookedSpatial::empty(id(3));
        sprite.sprite = Some(Box::new(NativeSpriteRenderer {
            texture: id(91),
            region: NativeSpriteRegion::Pixels([1, 0, 1, 1]),
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
        NativeWorld::new(
            incant_runtime::CookedScene::new(
                id(99),
                60,
                vec![
                    entity(1, None),
                    entity(2, Some(1)),
                    entity(3, None),
                    entity(4, None),
                ],
            )
            .unwrap()
            .with_spatial(
                self.references.clone(),
                vec![declaration(2), sprite, environment],
            )
            .unwrap(),
        )
    }
    fn load(&self, world: &NativeWorld) -> NativeRenderInput {
        NativeRenderInput::load(
            &world.spatial_view().unwrap(),
            VisualCaches {
                models: Path::new("absent"),
                textures: &self.root.path().join("textures"),
            },
            id(2),
        )
        .unwrap()
    }
    fn next(&self) -> Vec<VisualAssetRef> {
        vec![
            self.cook(90, 33, 2, TextureUsage::Color),
            self.cook(91, 66, 2, TextureUsage::Color),
        ]
    }
}
fn versions(input: &NativeRenderInput) -> Vec<incant_assets_runtime::AssetVersionId> {
    vec![
        input
            .input()
            .environment
            .as_ref()
            .unwrap()
            .source
            .version_id(),
        input
            .input()
            .nodes
            .iter()
            .find_map(|n| n.sprite.as_ref().map(|s| s.source.version_id()))
            .unwrap(),
    ]
}
#[test]
fn staged_replacement_publishes_without_allocations_preserves_world_and_old_handles() {
    let f = Fixture::new();
    let mut world = f.world();
    let identity = world.spatial_view().unwrap().identity().clone();
    let mut input = f.load(&world);
    let old = input.input().environment.as_ref().unwrap().source.clone();
    let before = versions(&input);
    let prepared = input
        .texture_refresh_ticket()
        .prepare(&f.next(), &f.root.path().join("textures"))
        .unwrap();
    let stage = input
        .stage_texture_refresh(&world.spatial_view().unwrap(), prepared)
        .unwrap();
    assert_eq!(versions(&input), before);
    world.step().unwrap();
    input
        .try_sync(&world.spatial_view().unwrap(), |_, _, _| Ok::<_, ()>(()))
        .unwrap();
    let poses = matrices(&input);
    let tick = world.tick();
    let view = world.spatial_view().unwrap();
    let (published, count) =
        allocations::record(|| input.try_publish_textures(&view, stage, || Ok::<_, ()>(())));
    let (publication, _) = published.unwrap();
    assert_eq!(count, 0);
    assert_eq!(publication.receipt.changed, 2);
    assert_eq!(input.asset_generation(), 1);
    assert_ne!(versions(&input), before);
    assert_eq!(old.version_id(), before[0]);
    assert_eq!(world.tick(), tick);
    assert_eq!(view.identity(), &identity);
    assert_eq!(matrices(&input), poses);
    drop(publication);
    assert_eq!(old.version_id(), before[0]);
}
#[test]
fn consumer_refusal_and_unwind_keep_complete_catalog_and_generation() {
    let f = Fixture::new();
    let world = f.world();
    let mut input = f.load(&world);
    let before = versions(&input);
    let prepare = |input: &NativeRenderInput| {
        input
            .stage_texture_refresh(
                &world.spatial_view().unwrap(),
                input
                    .texture_refresh_ticket()
                    .prepare(&f.next(), &f.root.path().join("textures"))
                    .unwrap(),
            )
            .unwrap()
    };
    let stage = prepare(&input);
    assert!(matches!(
        input.try_publish_textures(&world.spatial_view().unwrap(), stage, || Err::<(), _>(
            "late refusal"
        )),
        Err(SyncError::Consumer("late refusal"))
    ));
    assert_eq!(versions(&input), before);
    assert_eq!(input.asset_generation(), 0);
    let stage = prepare(&input);
    assert!(
        catch_unwind(AssertUnwindSafe(|| input.try_publish_textures(
            &world.spatial_view().unwrap(),
            stage,
            || -> Result<(), ()> { panic!("late unwind") }
        )))
        .is_err()
    );
    assert_eq!(versions(&input), before);
    assert_eq!(input.asset_generation(), 0);
}
#[test]
fn stale_catalog_and_foreign_world_refuse_before_publication() {
    let f = Fixture::new();
    let world = f.world();
    let mut input = f.load(&world);
    let ticket = input.texture_refresh_ticket();
    let refs = f.next();
    let a = input
        .stage_texture_refresh(
            &world.spatial_view().unwrap(),
            ticket
                .clone()
                .prepare(&refs, &f.root.path().join("textures"))
                .unwrap(),
        )
        .unwrap();
    let b = input
        .stage_texture_refresh(
            &world.spatial_view().unwrap(),
            ticket
                .prepare(&refs, &f.root.path().join("textures"))
                .unwrap(),
        )
        .unwrap();
    input
        .try_publish_textures(&world.spatial_view().unwrap(), a, || Ok::<_, ()>(()))
        .unwrap();
    let before = versions(&input);
    assert!(
        input
            .try_publish_textures(&world.spatial_view().unwrap(), b, || -> Result<(), ()> {
                panic!("stale callback")
            })
            .is_err()
    );
    let foreign = f.world();
    let prepared = input
        .texture_refresh_ticket()
        .prepare(&refs, &f.root.path().join("textures"))
        .unwrap();
    assert!(matches!(
        input.stage_texture_refresh(&foreign.spatial_view().unwrap(), prepared),
        Err(NativeRenderError::DifferentWorld)
    ));
    assert_eq!(versions(&input), before);
}
#[test]
fn topology_change_between_staging_and_publication_preserves_complete_old_input() {
    let f = Fixture::new();
    let mut world = f.world();
    let mut input = f.load(&world);
    let before = versions(&input);
    let prepared = input
        .texture_refresh_ticket()
        .prepare(&f.next(), &f.root.path().join("textures"))
        .unwrap();
    let stage = input
        .stage_texture_refresh(&world.spatial_view().unwrap(), prepared)
        .unwrap();
    world
        .apply(&mut queue([StructuralCommand::SetParent {
            entity: id(2),
            parent: Some(id(3)),
        }]))
        .unwrap();
    assert!(
        input
            .try_publish_textures(
                &world.spatial_view().unwrap(),
                stage,
                || -> Result<(), ()> { panic!("changed topology callback") }
            )
            .is_err()
    );
    assert_eq!(versions(&input), before);
    assert_eq!(input.asset_generation(), 0);
}
#[test]
fn malformed_late_asset_usage_change_and_invalid_atlas_publish_no_prefix() {
    let f = Fixture::new();
    let world = f.world();
    let input = f.load(&world);
    let before = versions(&input);
    let mut refs = f.next();
    refs[1].fingerprint = [7; 32];
    assert!(
        input
            .texture_refresh_ticket()
            .prepare(&refs, &f.root.path().join("textures"))
            .is_err()
    );
    let refs = vec![
        f.cook(90, 44, 2, TextureUsage::Color),
        f.cook(91, 99, 2, TextureUsage::Normal),
    ];
    assert!(
        input
            .texture_refresh_ticket()
            .prepare(&refs, &f.root.path().join("textures"))
            .is_err()
    );
    let refs = vec![
        f.cook(90, 55, 2, TextureUsage::Color),
        f.cook(91, 88, 1, TextureUsage::Color),
    ];
    let prepared = input
        .texture_refresh_ticket()
        .prepare(&refs, &f.root.path().join("textures"))
        .unwrap();
    assert!(
        input
            .stage_texture_refresh(&world.spatial_view().unwrap(), prepared)
            .is_err()
    );
    assert_eq!(versions(&input), before);
    assert_eq!(input.asset_generation(), 0);
}
#[test]
fn catalog_ids_kinds_and_smaller_budget_refuse_without_replacing_assets() {
    let f = Fixture::new();
    let world = f.world();
    let input = f.load(&world);
    let before = versions(&input);
    let mut refs = f.next();
    assert!(
        input
            .texture_refresh_ticket()
            .prepare_with_budget(&refs, &f.root.path().join("textures"), 1)
            .is_err()
    );
    refs[1].id = id(92);
    assert!(
        input
            .texture_refresh_ticket()
            .prepare(&refs, &f.root.path().join("textures"))
            .is_err()
    );
    refs[1] = f.references[1];
    refs[1].kind = VisualAssetKind::Model;
    assert!(
        input
            .texture_refresh_ticket()
            .prepare(&refs, &f.root.path().join("textures"))
            .is_err()
    );
    assert_eq!(versions(&input), before);
}

mod common;
#[path = "common/text.rs"]
mod text_fixture;
use common::*;
use incant_render_native::{
    NativeRenderError, NativeRenderInput, SyncError, TextSettings, UpdateKind, VisualCaches,
};
use incant_runtime::*;
use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    path::Path,
    sync::Arc,
};
use text_fixture::{fixture, text};
fn sync(input: &mut NativeRenderInput, world: &NativeWorld) {
    input
        .try_sync_with_text(
            &world.spatial_view().unwrap(),
            &world.text_view().unwrap(),
            |_, _, _| Ok::<_, ()>(()),
        )
        .unwrap();
}
#[test]
fn real_layout_atlas_motion_and_topology_reuse_survive_deleted_sources_and_cache() {
    let (temp, mut world, mut input) = fixture();
    temp.close().unwrap();
    let layout = input.input().text.as_ref().unwrap().nodes[0].layout.clone();
    let atlas = input.input().text.as_ref().unwrap().atlas.clone();
    assert!(layout.glyphs().len() > 3);
    assert!(layout.glyphs().len() < "ffi Hello".len());
    assert!(layout.missing_clusters().is_empty());
    assert!(atlas.pixels().iter().any(|&p| p > 0));
    let ptr = input.input().text.as_ref().unwrap().nodes.as_ptr();
    let pixels = atlas.pixels().as_ptr();
    for tick in 1..=32 {
        world.step().unwrap();
        sync(&mut input, &world);
        let t = input.input().text.as_ref().unwrap();
        assert_eq!(t.nodes.as_ptr(), ptr);
        assert_eq!(t.atlas.pixels().as_ptr(), pixels);
        assert!(Arc::ptr_eq(&t.nodes[0].layout, &layout));
        assert_eq!(t.nodes[0].world.w_axis.x, 1.125 + f64::from(tick));
        assert!(t.nodes[0].glyphs.iter().all(|h| t.atlas.get(h).is_some()));
    }
    world
        .apply(&mut queue([StructuralCommand::SetParent {
            entity: id(1),
            parent: Some(id(3)),
        }]))
        .unwrap();
    sync(&mut input, &world);
    let t = input.input().text.as_ref().unwrap();
    assert!(Arc::ptr_eq(&t.atlas, &atlas));
    assert!(Arc::ptr_eq(&t.nodes[0].layout, &layout));
    // A changed string is prepared from already retained font bytes.
    world
        .apply(&mut queue([StructuralCommand::SetText {
            entity: id(1),
            text: Some(text("Changed")),
        }]))
        .unwrap();
    sync(&mut input, &world);
    let changed = input.input().text.as_ref().unwrap();
    assert_eq!(changed.nodes[0].layout.source(), "Changed");
    assert!(!Arc::ptr_eq(&changed.atlas, &atlas));
    assert!(
        changed
            .atlas
            .get(&input.input().text.as_ref().unwrap().nodes[0].glyphs[0])
            .is_some()
    );
}
#[test]
fn text_and_spatial_publication_is_atomic_on_callback_error_unwind_and_late_preparation_refusal() {
    let (_temp, mut world, mut input) = fixture();
    let old = input.input().text.as_ref().unwrap().clone();
    let tick = input.tick();
    world.step().unwrap();
    for panic in [false, true] {
        let result = catch_unwind(AssertUnwindSafe(|| {
            input.try_sync_with_text(
                &world.spatial_view().unwrap(),
                &world.text_view().unwrap(),
                |new, _, kind| {
                    assert_eq!(kind, UpdateKind::Motion);
                    assert_ne!(
                        new.text.as_ref().unwrap().nodes[0].world,
                        old.nodes[0].world
                    );
                    if panic {
                        panic!("consumer");
                    }
                    Err::<(), _>(())
                },
            )
        }));
        if panic {
            assert!(result.is_err());
        } else {
            assert!(matches!(result.unwrap(), Err(SyncError::Consumer(()))));
        }
        assert_eq!(input.tick(), tick);
        assert_eq!(
            input.input().text.as_ref().unwrap().nodes[0].world,
            old.nodes[0].world
        );
    }
    let old_spatial = matrices(&input);
    world
        .apply(&mut queue([StructuralCommand::SetText {
            entity: id(1),
            text: Some(text("Other")),
        }]))
        .unwrap();
    assert!(
        input
            .try_sync_with_text(
                &world.spatial_view().unwrap(),
                &world.text_view().unwrap(),
                |_, _, _| Err::<(), _>(())
            )
            .is_err()
    );
    assert_eq!(matrices(&input), old_spatial);
    assert!(Arc::ptr_eq(
        &input.input().text.as_ref().unwrap().atlas,
        &old.atlas
    ));
    // Valid world layout exceeds the configured atlas, after spatial preparation.
    let mut huge = (*text("W")).clone();
    huge.layout.font_size = 512.;
    world
        .apply(&mut queue([StructuralCommand::SetText {
            entity: id(1),
            text: Some(text("i")),
        }]))
        .unwrap();
    let mut limited = TextSettings::new(_temp.path().join("fonts"));
    limited.atlas.width = 32;
    limited.atlas.height = 32;
    let small = NativeRenderInput::load_with_text(
        &world.spatial_view().unwrap(),
        &world.text_view().unwrap(),
        VisualCaches {
            models: Path::new("absent"),
            textures: Path::new("absent"),
        },
        id(2),
        limited,
    );
    let mut small = small.unwrap();
    let prior = small.input().text.as_ref().unwrap().atlas.clone();
    world
        .apply(&mut queue([StructuralCommand::SetText {
            entity: id(1),
            text: Some(Arc::new(huge)),
        }]))
        .unwrap();
    assert!(matches!(
        small.try_sync_with_text(
            &world.spatial_view().unwrap(),
            &world.text_view().unwrap(),
            |_, _, _| -> Result<(), ()> { panic!("must not publish") }
        ),
        Err(SyncError::Native(NativeRenderError::Text(_)))
    ));
    assert!(Arc::ptr_eq(
        &small.input().text.as_ref().unwrap().atlas,
        &prior
    ));
}
#[test]
fn wrong_world_restore_and_despawn_reuse_keep_text_handles_scoped() {
    let (_temp, mut world, mut input) = fixture();
    let (_other, foreign, _) = fixture();
    assert!(matches!(
        input.try_sync_with_text(
            &world.spatial_view().unwrap(),
            &foreign.text_view().unwrap(),
            |_, _, _| Ok::<_, ()>(())
        ),
        Err(SyncError::Native(NativeRenderError::DifferentWorld))
    ));
    let save = world.save_bytes().unwrap();
    let old = input.input().text.as_ref().unwrap().clone();
    world.restore_bytes(&save).unwrap();
    sync(&mut input, &world);
    let new = input.input().text.as_ref().unwrap();
    assert!(!Arc::ptr_eq(&old.atlas, &new.atlas));
    assert!(new.atlas.get(&old.nodes[0].glyphs[0]).is_none());
    // Remove child's parent first; ordinary despawn still follows hierarchy rules.
    world
        .apply(&mut queue([
            StructuralCommand::SetParent {
                entity: id(2),
                parent: None,
            },
            StructuralCommand::Despawn(id(1)),
        ]))
        .unwrap();
    sync(&mut input, &world);
    assert!(input.input().text.is_none());
    world
        .apply(&mut queue([StructuralCommand::Spawn(entity(1, None))]))
        .unwrap();
    sync(&mut input, &world);
    assert!(input.input().text.is_none());
}

#[test]
fn aggregate_layout_cap_refuses_before_raster_and_preserves_the_old_candidate() {
    let (_temp, mut world, mut input) = fixture();
    let old = input.input().text.as_ref().unwrap().atlas.clone();
    let mut long = (*text(&"i".repeat(16_384))).clone();
    long.layout.wrap = TextWrap::None;
    let long = Arc::new(long);
    let mut commands = queue([StructuralCommand::SetText {
        entity: id(1),
        text: Some(Arc::clone(&long)),
    }]);
    for n in 4..8 {
        commands
            .push(StructuralCommand::Spawn(entity(n, None)))
            .unwrap();
        commands
            .push(StructuralCommand::SetText {
                entity: id(n),
                text: Some(Arc::clone(&long)),
            })
            .unwrap();
    }
    world.apply(&mut commands).unwrap();
    assert!(matches!(
        input.try_sync_with_text(
            &world.spatial_view().unwrap(),
            &world.text_view().unwrap(),
            |_, _, _| -> Result<(), ()> { panic!("must not publish") }
        ),
        Err(SyncError::Native(NativeRenderError::TextLimit(
            "glyph placements"
        )))
    ));
    assert!(Arc::ptr_eq(
        &input.input().text.as_ref().unwrap().atlas,
        &old
    ));
}

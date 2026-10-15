//! Composition with the real native material-table publication route. Recording
//! observes only published input; it is not granted live-refresh authority.
#[path = "../../../incant_render_native/tests/common/model_refresh.rs"]
mod fixture;
#[path = "../../../incant_render_native/tests/common/material_refresh.rs"]
#[allow(dead_code)]
mod tables;
use fixture::id;
use incant_render_data::prewarm::{MaterialIdentity, MaterialRecorder};
use incant_render_native::{NativeRenderInput, VisualCaches};
use incant_runtime::{CookedEntity, CookedScene, CookedSpatial, NativeMeshRenderer, NativeWorld};

#[test]
fn published_material_tables_update_keys_and_receipts_without_inventing_ticks() {
    let root = tempfile::tempdir().unwrap();
    let first = tables::material_model(root.path(), tables::Tables::Initial);
    let mut model = CookedSpatial::empty(id(1));
    model.mesh = Some(NativeMeshRenderer {
        mesh: first.id,
        cast_shadows: false,
    });
    let mut camera = CookedSpatial::empty(id(2));
    camera.camera = Some(super::super::camera());
    let world = NativeWorld::new(
        CookedScene::new(
            id(99),
            64,
            (1..=2)
                .map(|n| CookedEntity {
                    id: id(n),
                    parent: None,
                    transform: Default::default(),
                    velocity: None,
                })
                .collect(),
        )
        .unwrap()
        .with_spatial(vec![first], vec![model, camera])
        .unwrap(),
    );
    let models = root.path().join(".incant/cache/models");
    let textures = root.path().join(".incant/cache/textures");
    let caches = || VisualCaches {
        models: &models,
        textures: &textures,
    };
    let mut input =
        NativeRenderInput::load(&world.spatial_view().unwrap(), caches(), id(2)).unwrap();
    let mut recorder = MaterialRecorder::new(
        input.input(),
        input.camera_label(),
        world.tick(),
        super::context(),
    )
    .unwrap();
    let initial = recorder.manifest().unwrap();
    for tables in [tables::Tables::Rebound, tables::Tables::Expanded] {
        let next = tables::material_model(root.path(), tables);
        let prepared = input
            .texture_refresh_ticket()
            .prepare_visuals(&[next], caches())
            .unwrap();
        let mut candidate = input
            .stage_texture_refresh(&world.spatial_view().unwrap(), prepared)
            .unwrap();
        // A refused candidate leaves both the accepted input and observations
        // intact. Re-observing the same committed tick must not increment counts.
        assert!(
            input
                .try_publish_textures(&world.spatial_view().unwrap(), &mut candidate, || Err(
                    "refused candidate"
                ))
                .is_err()
        );
        let before = recorder.manifest().unwrap().to_bytes().unwrap();
        recorder
            .accept(input.input(), input.camera_label(), world.tick())
            .unwrap();
        assert_eq!(before, recorder.manifest().unwrap().to_bytes().unwrap());
        input
            .try_publish_textures(&world.spatial_view().unwrap(), &mut candidate, || {
                Ok::<_, ()>(())
            })
            .unwrap();
        recorder
            .accept(input.input(), input.camera_label(), world.tick())
            .unwrap();
    }
    root.close().unwrap();
    recorder
        .accept(input.input(), input.camera_label(), world.tick())
        .unwrap();
    let final_manifest = recorder.manifest().unwrap();
    assert_eq!(final_manifest.identity, initial.identity);
    assert_eq!(final_manifest.recording.observed_models.len(), 3);
    assert_eq!(final_manifest.recording.first_tick, 0);
    assert_eq!(final_manifest.recording.last_tick, 0);
    assert_eq!(
        final_manifest
            .observed
            .iter()
            .map(|o| o.key.index())
            .collect::<Vec<_>>(),
        [0, 6, 8]
    );
    assert!(final_manifest.observed.iter().all(|o| o.ticks == 1));
    let loaded = MaterialIdentity::loaded(
        initial.identity.scene,
        initial.identity.cooked_scene_sha256,
        initial.identity.profile,
        input.input(),
    )
    .unwrap();
    assert_ne!(loaded.initial_models, initial.identity.initial_models);
    assert!(
        initial.admit(&loaded).is_err(),
        "changed actual content admitted an old recording"
    );
}

#[path = "../../incant_render_native/tests/common/model_refresh.rs"]
mod fixture;
#[path = "../../incant_render_native/tests/common/material_refresh.rs"]
mod material_fixture;
#[path = "model_refresh/materials.rs"]
mod materials;
use fixture::*;
use incant_audio_runtime::{AudioManagerSettings, OfflineBackend};
use incant_play::{PlaySession, PreparedPlay, TexturePublishAttempt, TextureRefreshBasis};
use incant_render_data::SceneInput;
use incant_render_game::{NativeRenderSettings, RenderInputConsumer, TextureRefreshConsumer};
use incant_runtime::*;
use incant_types::{Camera, CameraProjection, Transform};
use std::{convert::Infallible, path::Path, sync::Arc};

struct Observer {
    model: Arc<incant_assets::RuntimeAsset>,
    texture: Option<Arc<incant_assets::RuntimeAsset>>,
}
fn source(input: &SceneInput) -> Arc<incant_assets::RuntimeAsset> {
    input
        .nodes
        .iter()
        .find_map(|n| n.mesh.as_ref().map(|m| m.source.clone()))
        .unwrap()
}
fn observed(input: &SceneInput) -> Observer {
    Observer {
        model: source(input),
        texture: input.environment.as_ref().map(|e| e.source.clone()),
    }
}
impl RenderInputConsumer for Observer {
    type Settings = ();
    type Error = Infallible;
    fn prepare(
        input: &SceneInput,
        _: &str,
        _: Option<&Arc<incant_render_game::UiInput>>,
        (): (),
    ) -> Result<Self, Infallible> {
        Ok(observed(input))
    }
    fn accept(
        &mut self,
        input: &SceneInput,
        _: &str,
        _: incant_render_native::UpdateKind,
        _: Option<&Arc<incant_render_game::UiInput>>,
    ) -> Result<(), Infallible> {
        *self = observed(input);
        Ok(())
    }
    fn admit_ui(&mut self, _: &Arc<incant_render_game::UiInput>) -> Result<(), Infallible> {
        Ok(())
    }
}
impl TextureRefreshConsumer for Observer {
    type Prepared = Observer;
    fn prepare_textures(&self, input: &SceneInput, _: &str) -> Result<Self::Prepared, Infallible> {
        Ok(observed(input))
    }
    fn validate_textures(&self, _: &Self::Prepared) -> Result<(), Infallible> {
        Ok(())
    }
    fn publish_textures(&mut self, next: &mut Self::Prepared) {
        std::mem::swap(self, next)
    }
}
fn game(root: &Path, references: Vec<VisualAssetRef>) -> PlaySession<OfflineBackend, Observer> {
    let mut mesh = CookedSpatial::empty(id(1));
    mesh.mesh = Some(NativeMeshRenderer {
        mesh: references
            .iter()
            .find(|r| r.kind == VisualAssetKind::Model)
            .unwrap()
            .id,
        cast_shadows: true,
    });
    let mut camera = CookedSpatial::empty(id(2));
    camera.camera = Some(Camera {
        fov_degrees: 60.,
        near: 0.1,
        far: 100.,
        projection: CameraProjection::Perspective {},
    });
    if let Some(texture) = references
        .iter()
        .find(|r| r.kind == VisualAssetKind::Texture)
    {
        camera.environment = Some(NativeEnvironmentLight {
            texture: texture.id,
            intensity: 1.,
            rotation_degrees: 0.,
        });
    }
    let code = incant_script::CompiledScript::compile(
        "let calls=0;exports.default={update({chunks}){calls++;for(const c of chunks)for(let i=0;i<c.ids.length;i++)c.transforms[i*c.transformStride]+=calls;}};",
    ).unwrap().into();
    let cooked = CookedScene::new(
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
    .unwrap();
    let prepared = PreparedPlay {
        scene: ulid::Ulid::from_bytes(id(99).0).to_string(),
        script_source_sha256: "fixture".into(),
        authored_scripts: false,
        cooked,
        code,
        actions: Default::default(),
        clips: vec![],
        sample_rate: 48000,
        render: Some(NativeRenderSettings {
            model_cache: root.join(".incant/cache/models"),
            texture_cache: root.join(".incant/cache/textures"),
            camera: id(2),
            text: None,
            ui: None,
            consumer: (),
        }),
    };
    PlaySession::new(
        prepared,
        AudioManagerSettings {
            backend_settings: 48000,
            ..Default::default()
        },
        (),
    )
    .unwrap()
}
fn publish(
    game: &mut PlaySession<OfflineBackend, Observer>,
    prepared: incant_play::PreparedTextureRefresh,
) {
    let candidate = game.prepare_texture_refresh(prepared).unwrap();
    let mut ready = match game.prepare_texture_publication(candidate) {
        Ok(ready) => ready,
        Err(_) => panic!("ready"),
    };
    assert!(matches!(
        ready.publish(),
        TexturePublishAttempt::Published(_)
    ));
}
#[test]
fn actual_model_import_history_keeps_world_and_vm_counter_and_incompatible_edits_refuse() {
    use incant_cmd::{Actor, Command, CommandBus};
    let root = tempfile::tempdir().unwrap();
    let first = model(root.path(), 1., RED, [1.; 4], Variant::Static);
    let next = model(
        root.path(),
        2.,
        DARK,
        [0.2, 0.4, 0.6, 1.],
        Variant::LargerGeometry,
    );
    let asset = incant_doc::Asset {
        id: ulid::Ulid::from_bytes(first.id.0).to_string(),
        name: "Model".into(),
        path: "tri.gltf".into(),
        kind: "model".into(),
        sha256: key(first),
        import_settings: None,
    };
    let mut project = incant_doc::Project::empty("Model refresh");
    project.assets.insert(asset.id.clone(), asset.clone());
    let basis = TextureRefreshBasis::capture(&project, &[first]).unwrap();
    let mut bus = CommandBus::new(project).unwrap();
    let mut game = game(root.path(), vec![first]);
    let identity = game
        .world()
        .unwrap()
        .spatial_view()
        .unwrap()
        .identity()
        .clone();
    game.step(&[]).unwrap();
    let old = game.consumer().unwrap().unwrap().model.clone();
    let mut replacement = asset.clone();
    replacement.sha256 = key(next);
    bus.execute(
        vec![Command::UpsertAsset { asset: replacement }],
        Actor::user("import"),
        "replace static model",
        Some(0),
    )
    .unwrap();
    for expected in [next, first, next] {
        let prepared = basis
            .prepare(
                bus.project(),
                root.path(),
                game.texture_refresh_ticket().unwrap(),
            )
            .unwrap()
            .unwrap();
        publish(&mut game, prepared);
        assert_eq!(
            game.consumer().unwrap().unwrap().model.info().fingerprint,
            key(expected)
        );
        assert_eq!(game.world().unwrap().tick(), 1);
        assert_eq!(
            game.world().unwrap().spatial_view().unwrap().identity(),
            &identity
        );
        if bus.revision() == 1 {
            bus.undo().unwrap();
        } else if bus.revision() == 2 {
            bus.redo().unwrap();
        }
    }
    assert_eq!(old.info().fingerprint, key(first));
    game.step(&[]).unwrap();
    assert_eq!(
        game.world()
            .unwrap()
            .inspect(id(1))
            .unwrap()
            .transform
            .translation[0],
        3.
    ); // calls1+2, not reset1
    bus.execute(
        vec![Command::SetMemory {
            section: "other".into(),
            text: "incompatible edit".into(),
        }],
        Actor::user("test"),
        "other",
        Some(bus.revision()),
    )
    .unwrap();
    assert!(
        basis
            .prepare(
                bus.project(),
                root.path(),
                game.texture_refresh_ticket().unwrap()
            )
            .unwrap()
            .is_none()
    );
    bus.execute(
        vec![Command::UpsertAsset { asset }],
        Actor::user("import"),
        "model after incompatible edit",
        Some(bus.revision()),
    )
    .unwrap();
    assert!(
        basis
            .prepare(
                bus.project(),
                root.path(),
                game.texture_refresh_ticket().unwrap()
            )
            .is_err()
    );
    assert_eq!(
        game.consumer().unwrap().unwrap().model.info().fingerprint,
        key(next)
    );
}

fn imports() -> [incant_import::ImportRequest; 2] {
    [
        incant_import::ImportRequest {
            source: "tri.gltf".into(),
            texture_usage: None,
        },
        incant_import::ImportRequest {
            source: "pixel.png".into(),
            texture_usage: Some(incant_assets::TextureUsage::Color),
        },
    ]
}
fn imported_references(bus: &incant_cmd::CommandBus) -> Vec<VisualAssetRef> {
    let mut references: Vec<_> = bus
        .project()
        .assets
        .values()
        .map(|asset| {
            let kind = if asset.kind == "model" {
                VisualAssetKind::Model
            } else {
                VisualAssetKind::Texture
            };
            let mut next = reference(0, kind, &asset.sha256);
            next.id = StableId(asset.id.parse::<ulid::Ulid>().unwrap().to_bytes());
            next
        })
        .collect();
    references.sort_by_key(|reference| reference.id);
    references
}
#[test]
fn real_mixed_import_batch_and_cooked_refusal_preserve_the_whole_catalog_and_history() {
    use incant_cmd::{Actor, CommandBus};
    use incant_import::ImportSnapshot;
    let root = tempfile::tempdir().unwrap();
    model(root.path(), 1., RED, [1.; 4], Variant::Static);
    texture(root.path(), RED);
    let mut bus = CommandBus::new(incant_doc::Project::empty("Mixed refresh")).unwrap();
    ImportSnapshot::capture(&bus)
        .prepare(root.path(), None, &imports())
        .unwrap()
        .commit(&mut bus, Actor::user("import"), "initial visuals")
        .unwrap();
    let first = imported_references(&bus);
    let basis = TextureRefreshBasis::capture(bus.project(), &first).unwrap();
    let mut game = game(root.path(), first.clone());
    game.step(&[]).unwrap();
    let old_model = game.consumer().unwrap().unwrap().model.clone();
    let old_texture = game
        .consumer()
        .unwrap()
        .unwrap()
        .texture
        .as_ref()
        .unwrap()
        .clone();
    let identity = game
        .world()
        .unwrap()
        .spatial_view()
        .unwrap()
        .identity()
        .clone();
    model(
        root.path(),
        2.,
        DARK,
        [0.2, 0.4, 0.6, 1.],
        Variant::LargerGeometry,
    );
    texture(root.path(), DARK);
    // A failed source in an actual importer batch commits neither resource.
    std::fs::write(root.path().join("pixel.png"), b"bad owned source").unwrap();
    assert!(
        ImportSnapshot::capture(&bus)
            .prepare(root.path(), None, &imports())
            .is_err()
    );
    assert_eq!(bus.revision(), 1);
    assert_eq!(imported_references(&bus), first);
    texture(root.path(), DARK);
    ImportSnapshot::capture(&bus)
        .prepare(root.path(), None, &imports())
        .unwrap()
        .commit(&mut bus, Actor::user("import"), "replace both visuals")
        .unwrap();
    let next = imported_references(&bus);
    assert_eq!(
        next.iter().map(|r| r.id).collect::<Vec<_>>(),
        first.iter().map(|r| r.id).collect::<Vec<_>>()
    );
    assert!(
        next.iter()
            .zip(&first)
            .all(|(a, b)| a.fingerprint != b.fingerprint)
    );
    let texture = next
        .iter()
        .find(|r| r.kind == VisualAssetKind::Texture)
        .unwrap();
    let cache = root
        .path()
        .join(format!(".incant/cache/textures/{}.json", key(*texture)));
    let bytes = std::fs::read(&cache).unwrap();
    std::fs::write(&cache, b"bad owned cooked texture").unwrap();
    assert!(
        basis
            .prepare(
                bus.project(),
                root.path(),
                game.texture_refresh_ticket().unwrap()
            )
            .is_err()
    );
    assert_eq!(
        game.consumer().unwrap().unwrap().model.version_id(),
        old_model.version_id()
    );
    assert_eq!(
        game.consumer()
            .unwrap()
            .unwrap()
            .texture
            .as_ref()
            .unwrap()
            .version_id(),
        old_texture.version_id()
    );
    std::fs::write(cache, bytes).unwrap();
    let prepared = basis
        .prepare(
            bus.project(),
            root.path(),
            game.texture_refresh_ticket().unwrap(),
        )
        .unwrap()
        .unwrap();
    publish(&mut game, prepared);
    assert_ne!(
        game.consumer().unwrap().unwrap().model.version_id(),
        old_model.version_id()
    );
    assert_ne!(
        game.consumer()
            .unwrap()
            .unwrap()
            .texture
            .as_ref()
            .unwrap()
            .version_id(),
        old_texture.version_id()
    );
    assert_eq!(game.world().unwrap().tick(), 1);
    assert_eq!(
        game.world().unwrap().spatial_view().unwrap().identity(),
        &identity
    );
    bus.undo().unwrap();
    let prepared = basis
        .prepare(
            bus.project(),
            root.path(),
            game.texture_refresh_ticket().unwrap(),
        )
        .unwrap()
        .unwrap();
    publish(&mut game, prepared);
    assert_eq!(
        game.consumer().unwrap().unwrap().model.info().fingerprint,
        old_model.info().fingerprint
    );
    assert_eq!(
        game.consumer()
            .unwrap()
            .unwrap()
            .texture
            .as_ref()
            .unwrap()
            .info()
            .fingerprint,
        old_texture.info().fingerprint
    );
    game.step(&[]).unwrap();
    assert_eq!(
        game.world()
            .unwrap()
            .inspect(id(1))
            .unwrap()
            .transform
            .translation[0],
        3.
    );
}

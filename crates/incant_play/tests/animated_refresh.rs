#[path = "../../incant_render_native/tests/common/animated_refresh.rs"]
mod fixture;
use fixture::*;
use incant_audio_runtime::{AudioManagerSettings, OfflineBackend};
use incant_cmd::{Actor, CommandBus};
use incant_import::{ImportRequest, ImportSnapshot};
use incant_play::{PlaySession, PreparedPlay, TexturePublishAttempt, TextureRefreshBasis};
use incant_render_data::SceneInput;
use incant_render_game::{NativeRenderSettings, RenderInputConsumer, TextureRefreshConsumer};
use incant_runtime::*;
use incant_types::{Animation, Camera, CameraProjection, Transform};
use std::{convert::Infallible, path::Path, sync::Arc};

struct Observer {
    model: Arc<incant_assets::RuntimeAsset>,
    texture: Arc<incant_assets::RuntimeAsset>,
}
fn observed(input: &SceneInput) -> Observer {
    Observer {
        model: input
            .nodes
            .iter()
            .find_map(|n| n.mesh.as_ref().map(|m| m.source.clone()))
            .unwrap(),
        texture: input.environment.as_ref().unwrap().source.clone(),
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
fn references(bus: &CommandBus) -> Vec<VisualAssetRef> {
    let mut refs: Vec<_> = bus
        .project()
        .assets
        .values()
        .map(|a| {
            let mut reference = reference(
                0,
                if a.kind == "model" {
                    VisualAssetKind::Model
                } else {
                    VisualAssetKind::Texture
                },
                &a.sha256,
            );
            reference.id = StableId(a.id.parse::<ulid::Ulid>().unwrap().to_bytes());
            reference
        })
        .collect();
    refs.sort_by_key(|r| r.id);
    refs
}
fn import(bus: &mut CommandBus, root: &Path) {
    ImportSnapshot::capture(bus)
        .prepare(
            root,
            None,
            &[
                ImportRequest {
                    source: "model.gltf".into(),
                    texture_usage: None,
                },
                ImportRequest {
                    source: "pixel.png".into(),
                    texture_usage: Some(incant_assets::TextureUsage::Color),
                },
            ],
        )
        .unwrap()
        .commit(bus, Actor::user("import"), "animated visuals")
        .unwrap();
}
fn game(root: &Path, references: Vec<VisualAssetRef>) -> PlaySession<OfflineBackend, Observer> {
    let model = references
        .iter()
        .find(|r| r.kind == VisualAssetKind::Model)
        .unwrap();
    let texture = references
        .iter()
        .find(|r| r.kind == VisualAssetKind::Texture)
        .unwrap();
    let mut mesh = CookedSpatial::empty(id(1));
    mesh.mesh = Some(NativeMeshRenderer {
        mesh: model.id,
        cast_shadows: true,
    });
    mesh.animation = Some(Arc::new(NativeAnimation {
        mesh_fingerprint: model.fingerprint,
        clip_durations: Arc::from([2.; 3]),
        graph: None,
        initial: Animation {
            clip: Some(0),
            seconds: 0.,
            speed: 1.,
            playing: true,
            looping: true,
        },
    }));
    let mut camera = CookedSpatial::empty(id(2));
    camera.camera = Some(Camera {
        fov_degrees: 60.,
        near: 0.1,
        far: 100.,
        projection: CameraProjection::Perspective {},
    });
    camera.environment = Some(NativeEnvironmentLight {
        texture: texture.id,
        intensity: 1.,
        rotation_degrees: 0.,
    });
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
    let code = incant_script::CompiledScript::compile("let calls=0;exports.default={update({chunks}){calls++;for(const c of chunks)for(let i=0;i<c.ids.length;i++)c.transforms[i*c.transformStride]+=calls;}};").unwrap().into();
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
fn real_import_undo_redo_keep_world_vm_and_animation_clock_then_reject_incompatible_clips() {
    imported_refresh(Change::Keys);
}

#[test]
fn real_vertex_import_undo_redo_preserves_the_running_script_and_animation() {
    imported_refresh(Change::Geometry);
}

#[test]
fn real_generated_tangent_import_undo_redo_preserves_the_running_script_and_animation() {
    imported_refresh(Change::GeneratedTangents);
}

fn imported_refresh(change: Change) {
    let root = tempfile::tempdir().unwrap();
    model(root.path(), Change::Original);
    texture(root.path(), RED);
    let mut bus = CommandBus::new(incant_doc::Project::empty("Animated refresh")).unwrap();
    import(&mut bus, root.path());
    let first = references(&bus);
    let basis = TextureRefreshBasis::capture(bus.project(), &first).unwrap();
    let mut game = game(root.path(), first.clone());
    game.step(&[]).unwrap();
    game.step(&[]).unwrap();
    let identity = game
        .world()
        .unwrap()
        .spatial_view()
        .unwrap()
        .identity()
        .clone();
    let old_model = game.consumer().unwrap().unwrap().model.clone();
    let old_texture = game.consumer().unwrap().unwrap().texture.clone();
    let state = game.world().unwrap().animation_state(id(1)).unwrap();
    model(root.path(), change);
    texture(root.path(), DARK);
    import(&mut bus, root.path());
    let next = references(&bus);
    assert_eq!(
        first.iter().map(|r| r.id).collect::<Vec<_>>(),
        next.iter().map(|r| r.id).collect::<Vec<_>>()
    );
    for expected in [&next, &first, &next] {
        let prepared = basis
            .prepare(
                bus.project(),
                root.path(),
                game.texture_refresh_ticket().unwrap(),
            )
            .unwrap()
            .unwrap();
        publish(&mut game, prepared);
        let observed = game.consumer().unwrap().unwrap();
        assert_eq!(
            observed.model.info().fingerprint,
            key(*expected
                .iter()
                .find(|r| r.kind == VisualAssetKind::Model)
                .unwrap())
        );
        assert_eq!(
            observed.texture.info().fingerprint,
            key(*expected
                .iter()
                .find(|r| r.kind == VisualAssetKind::Texture)
                .unwrap())
        );
        assert_eq!(game.world().unwrap().tick(), 2);
        assert_eq!(
            game.world().unwrap().spatial_view().unwrap().identity(),
            &identity
        );
        assert_eq!(game.world().unwrap().animation_state(id(1)), Some(state));
        if bus.revision() == 2 {
            bus.undo().unwrap();
        } else if bus.revision() == 3 {
            bus.redo().unwrap();
        }
    }
    assert_eq!(
        old_model.info().fingerprint,
        key(*first
            .iter()
            .find(|r| r.kind == VisualAssetKind::Model)
            .unwrap())
    );
    assert_eq!(
        old_texture.info().fingerprint,
        key(*first
            .iter()
            .find(|r| r.kind == VisualAssetKind::Texture)
            .unwrap())
    );
    game.step(&[]).unwrap();
    assert_eq!(
        game.world()
            .unwrap()
            .inspect(id(1))
            .unwrap()
            .transform
            .translation[0],
        6.
    ); // VM calls1+2+3.
    assert!(
        (game
            .world()
            .unwrap()
            .animation_state(id(1))
            .unwrap()
            .sample
            .seconds
            - 3. / 60.)
            .abs()
            < 1e-12
    );
    let accepted = game.consumer().unwrap().unwrap().model.clone();
    model(root.path(), Change::Duration);
    import(&mut bus, root.path()); // Authoring succeeds; the live catalog explicitly refuses.
    assert!(
        basis
            .prepare(
                bus.project(),
                root.path(),
                game.texture_refresh_ticket().unwrap()
            )
            .is_err()
    );
    assert!(Arc::ptr_eq(
        &accepted,
        &game.consumer().unwrap().unwrap().model
    ));
    bus.undo().unwrap();
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
    root.close().unwrap();
    game.step(&[]).unwrap();
    assert_eq!(
        game.world()
            .unwrap()
            .inspect(id(1))
            .unwrap()
            .transform
            .translation[0],
        10.
    );
    assert!(
        (game
            .world()
            .unwrap()
            .animation_state(id(1))
            .unwrap()
            .sample
            .seconds
            - 4. / 60.)
            .abs()
            < 1e-12
    );
}

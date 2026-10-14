use incant_audio_runtime::{AudioManagerSettings, OfflineBackend};
use incant_play::{
    PlayMode, PlaySession, PreparedPlay, TexturePublishAttempt, TextureRefreshBasis,
};
use incant_render_data::SceneInput;
use incant_render_game::{NativeRenderSettings, RenderInputConsumer, TextureRefreshConsumer};
use incant_render_native::UpdateKind;
use incant_runtime::{
    CookedEntity, CookedScene, CookedSpatial, NativeEnvironmentLight, StableId, VisualAssetKind,
    VisualAssetRef,
};
use incant_types::{Camera, CameraProjection, Transform};
use std::{
    fs,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};
const RED: &[u8] = &[
    137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0,
    0, 0, 31, 21, 196, 137, 0, 0, 0, 13, 73, 68, 65, 84, 120, 156, 99, 248, 207, 192, 240, 31, 0,
    5, 0, 1, 255, 137, 153, 61, 29, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
];
const DARK: &[u8] = &[
    137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0,
    0, 0, 31, 21, 196, 137, 0, 0, 0, 13, 73, 68, 65, 84, 120, 156, 99, 80, 100, 96, 248, 15, 0, 1,
    136, 1, 33, 206, 188, 226, 197, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
];
fn id(n: u128) -> StableId {
    StableId(n.to_be_bytes())
}
fn key(reference: VisualAssetRef) -> String {
    reference
        .fingerprint
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn cook(root: &Path, bytes: &[u8]) -> VisualAssetRef {
    fs::write(root.join("pixel.png"), bytes).unwrap();
    let key = incant_assets::cook_texture(
        root,
        Path::new("pixel.png"),
        &root.join(".incant/cache/textures"),
        incant_assets::TextureUsage::Color,
    )
    .unwrap()
    .metadata
    .fingerprint;
    VisualAssetRef {
        id: id(90),
        kind: VisualAssetKind::Texture,
        fingerprint: std::array::from_fn(|i| {
            u8::from_str_radix(&key[i * 2..i * 2 + 2], 16).unwrap()
        }),
    }
}
#[derive(Default)]
struct Probe {
    refuse: AtomicUsize,
    accepts: AtomicUsize,
    publications: AtomicUsize,
    drops: AtomicUsize,
    candidate_drops: AtomicUsize,
    drops_under_lock: AtomicUsize,
    publication_lock: std::sync::Mutex<()>,
}
struct Candidate {
    key: String,
    probe: Arc<Probe>,
}
impl Drop for Candidate {
    fn drop(&mut self) {
        self.probe.candidate_drops.fetch_add(1, Ordering::SeqCst);
        if self.probe.publication_lock.try_lock().is_err() {
            self.probe.drops_under_lock.fetch_add(1, Ordering::SeqCst);
        }
    }
}
struct Consumer {
    current: Candidate,
    probe: Arc<Probe>,
}
#[derive(Debug, thiserror::Error)]
#[error("texture consumer control refusal")]
struct Refused;
impl Drop for Consumer {
    fn drop(&mut self) {
        self.probe.drops.fetch_add(1, Ordering::SeqCst);
    }
}
impl RenderInputConsumer for Consumer {
    type Settings = Arc<Probe>;
    type Error = Refused;
    fn prepare(
        input: &SceneInput,
        _: &str,
        _: Option<&Arc<incant_render_game::UiInput>>,
        probe: Arc<Probe>,
    ) -> Result<Self, Refused> {
        Ok(Self {
            current: Candidate {
                key: input
                    .environment
                    .as_ref()
                    .unwrap()
                    .source
                    .info()
                    .fingerprint
                    .clone(),
                probe: probe.clone(),
            },
            probe,
        })
    }
    fn accept(
        &mut self,
        input: &SceneInput,
        _: &str,
        _: UpdateKind,
        _: Option<&Arc<incant_render_game::UiInput>>,
    ) -> Result<(), Refused> {
        self.current.key.clone_from(
            &input
                .environment
                .as_ref()
                .unwrap()
                .source
                .info()
                .fingerprint,
        );
        self.probe.accepts.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
    fn admit_ui(&mut self, _: &Arc<incant_render_game::UiInput>) -> Result<(), Refused> {
        Err(Refused)
    }
}
impl TextureRefreshConsumer for Consumer {
    type Prepared = Candidate;
    fn prepare_textures(&self, input: &SceneInput, _: &str) -> Result<Candidate, Refused> {
        if self.probe.refuse.load(Ordering::SeqCst) == 1 {
            return Err(Refused);
        }
        Ok(Candidate {
            key: input
                .environment
                .as_ref()
                .unwrap()
                .source
                .info()
                .fingerprint
                .clone(),
            probe: self.probe.clone(),
        })
    }
    fn validate_textures(&self, _: &Candidate) -> Result<(), Refused> {
        if self.probe.refuse.load(Ordering::SeqCst) == 2 {
            Err(Refused)
        } else {
            Ok(())
        }
    }
    fn publish_textures(&mut self, prepared: &mut Candidate) {
        if self.probe.refuse.load(Ordering::SeqCst) == 3 {
            panic!("before handle swap");
        }
        self.probe.publications.fetch_add(1, Ordering::SeqCst);
        std::mem::swap(&mut self.current, prepared);
        if self.probe.refuse.load(Ordering::SeqCst) == 4 {
            panic!("after handle swap");
        }
    }
}
fn publish(
    game: &mut PlaySession<OfflineBackend, Consumer>,
    candidate: incant_render_game::PreparedHostTextureRefresh<Consumer>,
) -> TexturePublishAttempt {
    let mut ready = match game.prepare_texture_publication(candidate) {
        Ok(ready) => ready,
        Err(_) => panic!("expected ready native game"),
    };
    let attempt = ready.publish();
    drop(ready);
    attempt
}
fn game(
    root: &Path,
    reference: VisualAssetRef,
    probe: Arc<Probe>,
) -> PlaySession<OfflineBackend, Consumer> {
    game_body(
        root,
        reference,
        probe,
        "let calls=0;exports.default={update({chunks}){calls++;for(const c of chunks)for(let i=0;i<c.ids.length;i++)c.transforms[i*c.transformStride]+=calls;}};",
    )
}
fn game_body(
    root: &Path,
    reference: VisualAssetRef,
    probe: Arc<Probe>,
    source: &str,
) -> PlaySession<OfflineBackend, Consumer> {
    let mut camera = CookedSpatial::empty(id(2));
    camera.camera = Some(Camera {
        fov_degrees: 60.,
        near: 0.1,
        far: 100.,
        projection: CameraProjection::Perspective {},
    });
    let mut environment = CookedSpatial::empty(id(1));
    environment.environment = Some(NativeEnvironmentLight {
        texture: reference.id,
        intensity: 1.,
        rotation_degrees: 0.,
    });
    let scene = CookedScene::new(
        id(99),
        60,
        vec![1, 2]
            .into_iter()
            .map(|n| CookedEntity {
                id: id(n),
                parent: None,
                transform: Transform::default(),
                velocity: None,
            })
            .collect(),
    )
    .unwrap()
    .with_spatial(vec![reference], vec![environment, camera])
    .unwrap();
    let prepared = PreparedPlay {
        scene: ulid::Ulid::from_bytes(id(99).0).to_string(),
        script_source_sha256: "fixture".into(),
        authored_scripts: false,
        cooked: scene,
        code: incant_script::CompiledScript::compile(source)
            .unwrap()
            .into(),
        actions: Default::default(),
        clips: vec![],
        sample_rate: 48000,
        render: Some(NativeRenderSettings {
            model_cache: root.join("models"),
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
        probe,
    )
    .unwrap()
}
#[test]
fn prepare_and_late_consumer_refusal_keep_world_vm_clock_input_and_live_host() {
    let root = tempfile::tempdir().unwrap();
    let first = cook(root.path(), RED);
    let next = cook(root.path(), DARK);
    let probe = Arc::new(Probe::default());
    let mut game = game(root.path(), first, probe.clone());
    let identity = game
        .world()
        .unwrap()
        .spatial_view()
        .unwrap()
        .identity()
        .clone();
    let prepared = |game: &PlaySession<OfflineBackend, Consumer>| {
        game.texture_refresh_ticket()
            .unwrap()
            .prepare(&[next], &root.path().join(".incant/cache/textures"))
            .unwrap()
    };
    probe.refuse.store(1, Ordering::SeqCst);
    assert!(game.prepare_texture_refresh(prepared(&game)).is_err());
    assert_eq!(game.mode(), PlayMode::Paused);
    assert_eq!(game.consumer().unwrap().unwrap().current.key, key(first));
    assert_eq!(probe.drops.load(Ordering::SeqCst), 0);
    probe.refuse.store(0, Ordering::SeqCst);
    let stage = game.prepare_texture_refresh(prepared(&game)).unwrap();
    game.step(&[]).unwrap();
    assert_eq!(game.world().unwrap().tick(), 1);
    probe.refuse.store(2, Ordering::SeqCst);
    assert!(matches!(
        publish(&mut game, stage),
        TexturePublishAttempt::Refused(_)
    ));
    assert_eq!(game.consumer().unwrap().unwrap().current.key, key(first));
    assert_eq!(probe.publications.load(Ordering::SeqCst), 0);
    assert_eq!(game.mode(), PlayMode::Paused);
    assert_eq!(game.world().unwrap().tick(), 1);
    assert_eq!(probe.drops.load(Ordering::SeqCst), 0);
    probe.refuse.store(0, Ordering::SeqCst);
    let stage = game.prepare_texture_refresh(prepared(&game)).unwrap();
    let publication = match publish(&mut game, stage) {
        TexturePublishAttempt::Published(p) => p,
        _ => panic!("expected publication"),
    };
    assert_eq!(publication.changed, 1);
    assert_eq!(game.consumer().unwrap().unwrap().current.key, key(next));
    assert_eq!(game.world().unwrap().tick(), 1);
    assert_eq!(probe.accepts.load(Ordering::SeqCst), 1);
    assert_eq!(
        game.world().unwrap().spatial_view().unwrap().identity(),
        &identity
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
    ); // same VM's calls=2
    assert_eq!(probe.accepts.load(Ordering::SeqCst), 2);
    game.stop();
    assert_eq!(probe.drops.load(Ordering::SeqCst), 1);
}
#[test]
fn authoring_undo_redo_uses_the_same_basis_and_nontexture_edits_refuse() {
    use incant_cmd::{Actor, Command, CommandBus};
    let root = tempfile::tempdir().unwrap();
    let first = cook(root.path(), RED);
    let next = cook(root.path(), DARK);
    let asset = incant_doc::Asset {
        id: ulid::Ulid::from_bytes(first.id.0).to_string(),
        name: "Texture".into(),
        path: "pixel.png".into(),
        kind: "texture".into(),
        sha256: key(first),
        import_settings: None,
    };
    let mut project = incant_doc::Project::empty("Refresh");
    project.assets.insert(asset.id.clone(), asset.clone());
    let basis = TextureRefreshBasis::capture(&project, &[first]).unwrap();
    let mut bus = CommandBus::new(project).unwrap();
    let probe = Arc::new(Probe::default());
    let mut game = game(root.path(), first, probe);
    let mut replacement = asset.clone();
    replacement.sha256 = key(next);
    bus.execute(
        vec![Command::UpsertAsset { asset: replacement }],
        Actor::user("import"),
        "replace texture",
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
        let stage = game.prepare_texture_refresh(prepared).unwrap();
        assert!(matches!(
            publish(&mut game, stage),
            TexturePublishAttempt::Published(_)
        ));
        assert_eq!(game.consumer().unwrap().unwrap().current.key, key(expected));
        if expected == next && bus.revision() == 1 {
            bus.undo().unwrap();
        } else if expected == first {
            bus.redo().unwrap();
        }
    }
    bus.execute(
        vec![Command::SetMemory {
            section: "unrelated".into(),
            text: "incompatible edit".into(),
        }],
        Actor::user("test"),
        "unrelated",
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
        "texture changed after incompatible edit",
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
    assert_eq!(game.consumer().unwrap().unwrap().current.key, key(next));
    assert_eq!(game.world().unwrap().tick(), 0);
}

#[test]
fn stopped_failed_and_plain_readiness_refusals_return_owned_candidates_before_the_lock() {
    let root = tempfile::tempdir().unwrap();
    let first = cook(root.path(), RED);
    let next = cook(root.path(), DARK);
    let probe = Arc::new(Probe::default());
    let mut source = game(root.path(), first, probe.clone());
    for mode in ["stopped", "failed", "plain"] {
        let prepared = source
            .texture_refresh_ticket()
            .unwrap()
            .prepare(&[next], &root.path().join(".incant/cache/textures"))
            .unwrap();
        let candidate = source.prepare_texture_refresh(prepared).unwrap();
        let mut receiver = if mode == "plain" {
            let prepared = PreparedPlay {
                scene: ulid::Ulid::from_bytes(id(99).0).to_string(),
                script_source_sha256: "plain".into(),
                authored_scripts: false,
                cooked: CookedScene::new(
                    id(99),
                    60,
                    vec![CookedEntity {
                        id: id(1),
                        parent: None,
                        transform: Transform::default(),
                        velocity: None,
                    }],
                )
                .unwrap(),
                code: incant_script::CompiledScript::compile("exports.default={update(){}};")
                    .unwrap()
                    .into(),
                actions: Default::default(),
                clips: vec![],
                sample_rate: 48000,
                render: None,
            };
            PlaySession::new(
                prepared,
                AudioManagerSettings {
                    backend_settings: 48000,
                    ..Default::default()
                },
                probe.clone(),
            )
            .unwrap()
        } else {
            game_body(
                root.path(),
                first,
                probe.clone(),
                "exports.default={update(){throw Error('failed readiness control')}};",
            )
        };
        if mode == "stopped" {
            receiver.stop();
        }
        if mode == "failed" {
            assert!(receiver.step(&[]).is_err());
            assert_eq!(receiver.mode(), PlayMode::Failed);
        }
        let before = probe.candidate_drops.load(Ordering::SeqCst);
        let refusal = match receiver.prepare_texture_publication(candidate) {
            Err(refusal) => refusal,
            Ok(_) => panic!("readiness should refuse {mode}"),
        };
        let guard = probe.publication_lock.lock().unwrap();
        assert_eq!(probe.candidate_drops.load(Ordering::SeqCst), before);
        drop(guard);
        drop(refusal);
        assert_eq!(probe.candidate_drops.load(Ordering::SeqCst), before + 1);
        assert_eq!(probe.drops_under_lock.load(Ordering::SeqCst), 0);
    }
}
#[test]
fn late_validation_refusal_and_success_retire_only_after_the_publication_lock() {
    let root = tempfile::tempdir().unwrap();
    let first = cook(root.path(), RED);
    let next = cook(root.path(), DARK);
    let probe = Arc::new(Probe::default());
    let mut game = game(root.path(), first, probe.clone());
    for refuse in [true, false] {
        let prepared = game
            .texture_refresh_ticket()
            .unwrap()
            .prepare(&[next], &root.path().join(".incant/cache/textures"))
            .unwrap();
        let candidate = game.prepare_texture_refresh(prepared).unwrap();
        let mut ready = match game.prepare_texture_publication(candidate) {
            Ok(ready) => ready,
            Err(_) => panic!("ready game"),
        };
        let before = probe.candidate_drops.load(Ordering::SeqCst);
        probe
            .refuse
            .store(if refuse { 2 } else { 0 }, Ordering::SeqCst);
        let guard = probe.publication_lock.lock().unwrap();
        let attempt = ready.publish();
        assert!(matches!(ready.publish(), TexturePublishAttempt::Refused(_)));
        assert_eq!(probe.candidate_drops.load(Ordering::SeqCst), before);
        drop(guard);
        drop(ready);
        assert_eq!(
            game.consumer().unwrap().unwrap().current.key,
            if refuse { key(first) } else { key(next) }
        );
        assert!(matches!(&attempt, TexturePublishAttempt::Refused(_)) == refuse);
        assert_eq!(probe.candidate_drops.load(Ordering::SeqCst), before + 1);
        assert_eq!(probe.drops_under_lock.load(Ordering::SeqCst), 0);
    }
}

#[test]
fn publication_unwind_before_and_after_swap_retains_all_owners_until_unlock() {
    use std::panic::{AssertUnwindSafe, catch_unwind};
    let root = tempfile::tempdir().unwrap();
    let first = cook(root.path(), RED);
    let next = cook(root.path(), DARK);
    for panic_at in [3, 4] {
        let probe = Arc::new(Probe::default());
        let mut game = game(root.path(), first, probe.clone());
        let prepared = game
            .texture_refresh_ticket()
            .unwrap()
            .prepare(&[next], &root.path().join(".incant/cache/textures"))
            .unwrap();
        let candidate = game.prepare_texture_refresh(prepared).unwrap();
        let mut ready = match game.prepare_texture_publication(candidate) {
            Ok(ready) => ready,
            Err(_) => panic!("ready game"),
        };
        probe.refuse.store(panic_at, Ordering::SeqCst);
        let before = probe.candidate_drops.load(Ordering::SeqCst);
        let guard = probe.publication_lock.lock().unwrap();
        assert!(catch_unwind(AssertUnwindSafe(|| ready.publish())).is_err());
        assert_eq!(probe.candidate_drops.load(Ordering::SeqCst), before);
        assert_eq!(probe.drops.load(Ordering::SeqCst), 0);
        assert_eq!(probe.drops_under_lock.load(Ordering::SeqCst), 0);
        drop(guard);
        drop(ready);
        // An unexpected trusted native panic is terminal, as for other host
        // operations. Both the live composite and staged/retired candidate are
        // destroyed after unlocking, regardless of which side of the swap ran.
        assert_eq!(game.mode(), PlayMode::Failed);
        assert_eq!(
            probe.publications.load(Ordering::SeqCst),
            usize::from(panic_at == 4)
        );
        assert_eq!(probe.candidate_drops.load(Ordering::SeqCst), before + 2);
        assert_eq!(probe.drops.load(Ordering::SeqCst), 1);
        assert_eq!(probe.drops_under_lock.load(Ordering::SeqCst), 0);
    }
}

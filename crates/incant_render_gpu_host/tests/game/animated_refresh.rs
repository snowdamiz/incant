//! Same-clock skeletal pixels and host continuity; actual authoring history is
//! separately exercised by the CPU Play/import/Undo/Redo control.
use super::*;
#[path = "../../../incant_render_native/tests/common/animated_refresh.rs"]
mod fixture;
use fixture::{Change, key, reference};
use incant_assets_runtime::animation::AnimationPlayer;
use incant_audio_runtime::{AudioManagerSettings, OfflineBackend};
use incant_game_runtime::HostedGameSession;
use incant_render_game::{
    PreparedHostTextureRefresh, TextureRefreshOperation, TextureRefreshReply, VisualCaches,
};
use incant_runtime::NativeAnimation;
use incant_types::{Animation, CameraProjection};
use std::{fs, path::Path, sync::Mutex};

struct AnimationTargets {
    inner: Targets,
    epoch: Arc<AtomicUsize>,
}
impl TargetProvider for AnimationTargets {
    type Error = TargetError;
    type Frame = Frame;
    fn acquire(&mut self, renderer: &Renderer) -> Result<Frame, TargetError> {
        self.inner.acquire(renderer)
    }
    fn motion_epoch(&self) -> Option<u64> {
        Some(self.epoch.load(Ordering::SeqCst) as u64)
    }
}
type AnimationGame =
    HostedGameSession<OfflineBackend, NativeRenderHost<GpuRenderConsumer<AnimationTargets>>>;
type Candidate = PreparedHostTextureRefresh<GpuRenderConsumer<AnimationTargets>>;

fn model(root: &Path, change: Change) -> VisualAssetRef {
    fixture::model(root, change);
    let mut document: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("model.gltf")).unwrap()).unwrap();
    // One unobscured skin instance and identical red emission isolate clip keys
    // from material/image changes and from the second reflected skin instance.
    document["scenes"][0]["nodes"] = json!([0, 3]);
    document["materials"][0]["emissiveTexture"] = json!({"index":0});
    document["materials"][0]["emissiveFactor"] = json!([1, 1, 1]);
    document["materials"][0]["doubleSided"] = json!(true);
    fs::write(root.join("model.gltf"), document.to_string()).unwrap();
    let cooked = incant_assets::cook_gltf(
        root,
        Path::new("model.gltf"),
        &root.join(".incant/cache/models"),
    )
    .unwrap();
    reference(90, VisualAssetKind::Model, &cooked.metadata.fingerprint)
}

fn sampled_position(model: &incant_assets_runtime::CookedModel, seconds: f64) -> [f64; 3] {
    let mut player = AnimationPlayer::new(model).unwrap();
    player.sample(Some(0), seconds).unwrap();
    let vertex = model.meshes[0].vertices[0];
    player
        .skin_position(3, 0, 0, [vertex[0], vertex[1], vertex[2]])
        .unwrap()
}

#[test]
fn keys_only_fixture_changes_skin_sample_without_geometry_material_or_image_changes() {
    let root = tempfile::tempdir().unwrap();
    let old = model(root.path(), Change::Original);
    let next = model(root.path(), Change::KeysOnly);
    let cache = root.path().join(".incant/cache/models");
    let before = incant_assets_runtime::load_model(&cache, &key(old)).unwrap();
    let after = incant_assets_runtime::load_model(&cache, &key(next)).unwrap();
    assert_ne!(old.fingerprint, next.fingerprint);
    assert_eq!(before.meshes, after.meshes);
    assert_eq!(before.materials, after.materials);
    assert_eq!(before.images, after.images);
    let a = sampled_position(&before, 0.5);
    let b = sampled_position(&after, 0.5);
    assert_eq!((a, b), ([11.5, 0., 0.], [12., 0., 0.]));
}

fn game(root: &Path, reference: VisualAssetRef, epoch: Arc<AtomicUsize>) -> (AnimationGame, Probe) {
    let mut mesh = CookedSpatial::empty(id(1));
    mesh.mesh = Some(NativeMeshRenderer {
        mesh: reference.id,
        cast_shadows: false,
    });
    mesh.animation = Some(Arc::new(NativeAnimation {
        mesh_fingerprint: reference.fingerprint,
        clip_durations: Arc::from([2.; 3]),
        initial: Animation {
            clip: Some(0),
            seconds: 0.5 - 1. / 60.,
            speed: 1.,
            playing: true,
            looping: true,
        },
        graph: None,
    }));
    let mut camera = CookedSpatial::empty(id(2));
    camera.camera = Some(Camera {
        fov_degrees: 60.,
        near: 0.1,
        far: 100.,
        projection: CameraProjection::Orthographic { vertical_size: 4. },
    });
    let scene = CookedScene::new(
        id(99),
        60,
        vec![
            CookedEntity {
                id: id(1),
                parent: None,
                transform: Transform {
                    translation: [-11.5, 0., 0.],
                    ..Default::default()
                },
                velocity: None,
            },
            CookedEntity {
                id: id(2),
                parent: None,
                transform: Transform {
                    translation: [0.5, 0.5, 5.],
                    ..Default::default()
                },
                velocity: None,
            },
            CookedEntity {
                id: id(3),
                parent: None,
                transform: Transform::default(),
                velocity: None,
            },
        ],
    )
    .unwrap()
    .with_spatial(vec![reference], vec![mesh, camera])
    .unwrap();
    let renderer = Arc::new(models::checked(Renderer::headless().unwrap()));
    eprintln!(
        "animated-refresh adapter: {:?}",
        renderer.adapter.get_info()
    );
    let texture = renderer.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Animated model refresh control"),
        size: wgpu::Extent3d {
            width: 160,
            height: 96,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let presents = Arc::new(AtomicUsize::new(0));
    let fault = Arc::new(AtomicUsize::new(0));
    let dropped = Arc::new(AtomicUsize::new(0));
    let targets = AnimationTargets {
        inner: Targets {
            texture: texture.clone(),
            presents: presents.clone(),
            fault: fault.clone(),
            dropped: dropped.clone(),
        },
        epoch,
    };
    // The invisible numeric entity proves persistent guest module state without
    // moving the visible geometry or the camera between same-clock captures.
    let code = CompiledScript::compile("let calls=0;exports.default={update({chunks}){calls++;for(const c of chunks)for(let i=0;i<c.ids.length;i++)if(c.ids[i]==='00000000000000000000000003')c.transforms[i*c.transformStride]+=calls;}};").unwrap();
    let game = AnimationGame::new(
        CookedScene::from_bytes(&scene.to_bytes()).unwrap(),
        code,
        Default::default(),
        vec![],
        AudioManagerSettings {
            backend_settings: 48000,
            ..Default::default()
        },
        NativeRenderSettings {
            ui: None,
            text: None,
            model_cache: root.join(".incant/cache/models"),
            texture_cache: root.join(".incant/cache/textures"),
            camera: id(2),
            consumer: GpuRenderSettings {
                renderer: renderer.clone(),
                targets,
            },
        },
    )
    .unwrap();
    (
        game,
        Probe {
            renderer,
            texture,
            presents,
            fault,
            dropped,
        },
    )
}

fn stage(game: &mut AnimationGame, reference: VisualAssetRef, root: &Path) -> Candidate {
    let prepared = game
        .host()
        .unwrap()
        .texture_refresh_ticket()
        .prepare_visuals(
            &[reference],
            VisualCaches {
                models: &root.join(".incant/cache/models"),
                textures: &root.join(".incant/cache/textures"),
            },
        )
        .unwrap();
    match game
        .update_host(TextureRefreshOperation::Prepare(Some(prepared)))
        .unwrap()
    {
        TextureRefreshReply::Prepared(candidate) => candidate,
        _ => panic!("GPU staging must succeed"),
    }
}
fn source(game: &AnimationGame) -> &Arc<incant_assets_runtime::RuntimeAsset> {
    &game
        .host()
        .unwrap()
        .render_input()
        .input()
        .nodes
        .iter()
        .find_map(|node| node.mesh.as_ref())
        .unwrap()
        .source
}
fn red_mask(bytes: &[u8]) -> Vec<bool> {
    bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|p| {
            p[0] > 150
                && i16::from(p[0]) - i16::from(p[1]) > 128
                && i16::from(p[0]) - i16::from(p[2]) > 128
                && p[3] > 200
        })
        .collect()
}

#[test]
#[ignore = "requires a native GPU; run explicitly after resource grant"]
fn compatible_clip_keys_deform_pixels_at_same_clock_and_keep_refused_pixels_and_vm() {
    let temporary = tempfile::tempdir().unwrap();
    let output = std::env::var_os("INCANT_ANIMATED_REFRESH_PIXELS")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| temporary.path().join("pixels"));
    assert!(output.is_absolute());
    fs::create_dir(&output).unwrap();
    let root = tempfile::tempdir().unwrap();
    let original = model(root.path(), Change::Original);
    let next = model(root.path(), Change::KeysOnly);
    let invalid = model(root.path(), Change::Duration);
    let epoch = Arc::new(AtomicUsize::new(1));
    let (mut game, probe) = game(root.path(), original, epoch.clone());
    let old_asset = source(&game).clone();
    let lock = Mutex::new(());
    drop(lock.lock().unwrap()); // Editor authoring locks initialize before publication.
    let identity = game
        .world()
        .unwrap()
        .spatial_view()
        .unwrap()
        .identity()
        .clone();
    game.tick(&[]).unwrap();
    let state = game.world().unwrap().animation_state(id(1)).unwrap();
    assert!((state.sample.seconds - 0.5).abs() < 1e-12);
    assert_eq!(
        game.world()
            .unwrap()
            .inspect(id(3))
            .unwrap()
            .transform
            .translation[0],
        1.
    );
    let first_motion = game.host().unwrap().consumer().motion_frame().unwrap();
    assert!(!first_motion.has_previous());
    let initial = pixels(&probe);
    super::model_refresh::retain_pixels(&output, "initial", &initial);
    game.refresh_host().unwrap();
    let old_motion = game.host().unwrap().consumer().motion_frame().unwrap();
    assert!(old_motion.has_previous());
    assert!(first_motion.same_owner(&old_motion));
    assert_eq!(pixels(&probe), initial);
    assert!(
        game.host()
            .unwrap()
            .texture_refresh_ticket()
            .prepare_visuals(
                &[invalid],
                VisualCaches {
                    models: &root.path().join(".incant/cache/models"),
                    textures: &root.path().join(".incant/cache/textures"),
                },
            )
            .is_err()
    );
    game.refresh_host().unwrap();
    let invalid_pixels = pixels(&probe);
    super::model_refresh::retain_pixels(&output, "invalid_layout_retained", &invalid_pixels);
    let mut refused = TextureRefreshOperation::Publish(stage(&mut game, next, root.path()));
    epoch.store(2, Ordering::SeqCst);
    let mut ready = game.prepare_host_update().unwrap();
    let guard = lock.lock().unwrap();
    assert!(matches!(
        ready.update(&mut refused).unwrap(),
        TextureRefreshReply::Refused(_)
    ));
    assert_eq!(probe.presents.load(Ordering::SeqCst), 3);
    drop(guard);
    drop(ready);
    drop(refused);
    epoch.store(1, Ordering::SeqCst);
    game.refresh_host().unwrap();
    let refused_pixels = pixels(&probe);
    super::model_refresh::retain_pixels(&output, "late_refusal_retained", &refused_pixels);
    assert_eq!(game.host().unwrap().render_input().asset_generation(), 0);
    let mut publication = TextureRefreshOperation::Publish(stage(&mut game, next, root.path()));
    root.close().unwrap(); // Delete only this fixture, after the candidate owns its complete assets.
    let mut ready = game.prepare_host_update().unwrap();
    let guard = lock.lock().unwrap();
    let receipt = match ready.update(&mut publication).unwrap() {
        TextureRefreshReply::Published(receipt) => receipt,
        _ => panic!("publication must succeed"),
    };
    assert_eq!((receipt.changed, receipt.generation), (1, 1));
    assert_eq!(probe.presents.load(Ordering::SeqCst), 4);
    assert_eq!(probe.dropped.load(Ordering::SeqCst), 0);
    drop(guard);
    drop(ready);
    assert_eq!(game.world().unwrap().tick(), 1);
    assert_eq!(game.world().unwrap().animation_state(id(1)), Some(state));
    assert_eq!(
        game.world().unwrap().spatial_view().unwrap().identity(),
        &identity
    );
    assert_eq!(game.host().unwrap().consumer().accepted_frames(), 4);
    game.refresh_host().unwrap();
    let replacement = pixels(&probe);
    super::model_refresh::retain_pixels(&output, "replacement", &replacement);
    assert_eq!(source(&game).info().fingerprint, key(next));
    assert_ne!(source(&game).version_id(), old_asset.version_id());
    assert_eq!(old_asset.info().fingerprint, key(original));
    let incant_assets_runtime::RuntimeAssetData::Model(old_model) = old_asset.data() else {
        panic!("retained model")
    };
    assert_eq!(sampled_position(old_model, 0.5), [11.5, 0., 0.]);
    let motion = game.host().unwrap().consumer().motion_frame().unwrap();
    assert!(!motion.has_previous());
    assert!(!motion.same_owner(&old_motion));
    let before = red_mask(&initial);
    let after = red_mask(&replacement);
    let count = before.iter().filter(|&&p| p).count();
    eprintln!(
        "animated-refresh same-clock captures: seconds={} tick={} old_red={count} new_red={}; unchanged material/image; expected translation=12px",
        state.sample.seconds,
        game.world().unwrap().tick(),
        after.iter().filter(|&&p| p).count()
    );
    // 0.5 world units at 96 pixels / 4 orthographic units = exactly 12 pixels.
    // Equal material/image/geometry makes this a deformation assertion rather
    // than a color-change proxy. Raw buffers are retained before this check.
    assert!(count > 0, "the fixed skin triangle must be visible");
    assert_eq!(count, after.iter().filter(|&&p| p).count());
    for y in 0..96 {
        for x in 0..160 {
            assert_eq!(after[y * 160 + x], x >= 12 && before[y * 160 + x - 12]);
        }
    }
    assert_eq!(invalid_pixels, initial);
    assert_eq!(refused_pixels, initial);
    assert_ne!(initial, replacement);
    let mut ready = game.prepare_host_update().unwrap();
    let guard = lock.lock().unwrap();
    assert!(matches!(
        ready.update(&mut publication).unwrap(),
        TextureRefreshReply::Refused(_)
    ));
    drop(guard);
    drop(ready);
    drop(publication);
    game.refresh_host().unwrap();
    assert_eq!(pixels(&probe), replacement);
    assert!(
        game.host()
            .unwrap()
            .consumer()
            .motion_frame()
            .unwrap()
            .has_previous()
    );
    assert_eq!(game.world().unwrap().tick(), 1);
    assert_eq!(game.world().unwrap().animation_state(id(1)), Some(state));
    game.tick(&[]).unwrap();
    let next_clock = game
        .world()
        .unwrap()
        .animation_state(id(1))
        .unwrap()
        .sample
        .seconds;
    assert!((next_clock - (0.5 + 1. / 60.)).abs() < 1e-12);
    assert_eq!(
        game.world()
            .unwrap()
            .inspect(id(3))
            .unwrap()
            .transform
            .translation[0],
        3.
    );
    assert_eq!(game.world().unwrap().tick(), 2);
    assert_eq!(
        game.world().unwrap().spatial_view().unwrap().identity(),
        &identity
    );
    let continued = pixels(&probe);
    super::model_refresh::retain_pixels(&output, "continued", &continued);
    use std::io::Write;
    let mut proof = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output.join("proof.json"))
        .unwrap();
    proof
        .write_all(
            &serde_json::to_vec_pretty(&json!({
                "source_scope":"same-clock numerical skeletal refresh; no window, appearance, authoring GUI or performance gate",
                "width":160,"height":96,"old_red":count,"replacement_red":count,"shift_pixels":12,
                "same_clock_seconds":state.sample.seconds,"continued_clock_seconds":next_clock,
                "world_identity_preserved":true,"catalog_generation":1,"tick":2,
                "vm_counter_before":1,"vm_counter_after":3,
                "fresh_motion_owner":!motion.same_owner(&old_motion),"first_replacement_has_previous":motion.has_previous(),
                "invalid_layout_pixels_equal":invalid_pixels==initial,"late_refusal_pixels_equal":refused_pixels==initial,
                "frames":game.host().unwrap().consumer().accepted_frames(),"presents":probe.presents.load(Ordering::SeqCst)
            }))
            .unwrap(),
        )
        .unwrap();
    eprintln!(
        "animated-refresh pixels: red={count} shift=12; clock={} -> {next_clock}; vm=1 -> 3; tick=2 frames={} presents={}",
        state.sample.seconds,
        game.host().unwrap().consumer().accepted_frames(),
        probe.presents.load(Ordering::SeqCst)
    );
    game.stop();
    assert_eq!(probe.dropped.load(Ordering::SeqCst), 1);
}

//! Numerical GPU admission control, not a native-window or visual design gate.
use super::*;
use incant_audio_runtime::{AudioManagerSettings, OfflineBackend};
use incant_game_runtime::HostedGameSession;
use incant_render_game::{
    PreparedHostTextureRefresh, TextureRefreshOperation, TextureRefreshReply,
};
use incant_runtime::{NativeSpriteRegion, NativeSpriteRenderer, SpriteMode, SpriteSampling};
use incant_types::CameraProjection;
use std::{
    fs,
    path::Path,
    sync::{Mutex, atomic::AtomicBool},
};

struct RefreshTargets {
    inner: Targets,
    policy_changed: Arc<AtomicBool>,
}
impl TargetProvider for RefreshTargets {
    type Error = TargetError;
    type Frame = Frame;
    fn acquire(&mut self, renderer: &Renderer) -> Result<Frame, TargetError> {
        self.inner.acquire(renderer)
    }
    fn motion_epoch(&self) -> Option<u64> {
        self.policy_changed.load(Ordering::SeqCst).then_some(1)
    }
}
type RefreshGame =
    HostedGameSession<OfflineBackend, NativeRenderHost<GpuRenderConsumer<RefreshTargets>>>;
type Candidate = PreparedHostTextureRefresh<GpuRenderConsumer<RefreshTargets>>;
fn cooked_texture(root: &Path, rgba: [u8; 4]) -> VisualAssetRef {
    let mut encoder = png::Encoder::new(fs::File::create(root.join("pixel.png")).unwrap(), 1, 1);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&rgba)
        .unwrap();
    let key = incant_assets::cook_texture(
        root,
        Path::new("pixel.png"),
        &root.join("textures"),
        incant_assets_runtime::TextureUsage::Color,
    )
    .unwrap()
    .metadata
    .fingerprint;
    VisualAssetRef {
        id: id(90),
        kind: VisualAssetKind::Texture,
        fingerprint: std::array::from_fn(|i| {
            u8::from_str_radix(&key[2 * i..2 * i + 2], 16).unwrap()
        }),
    }
}
fn fixture(
    root: &Path,
    reference: VisualAssetRef,
    policy_changed: Arc<AtomicBool>,
) -> (RefreshGame, Probe) {
    let mut sprite = CookedSpatial::empty(id(1));
    sprite.sprite = Some(Box::new(NativeSpriteRenderer {
        texture: reference.id,
        region: NativeSpriteRegion::Pixels([0, 0, 1, 1]),
        size: [2.; 2],
        pivot: [0.5; 2],
        tint: [1.; 4],
        flip_x: false,
        flip_y: false,
        layer: 0,
        order: 0,
        sampling: SpriteSampling::Nearest,
        mode: SpriteMode::Simple,
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
                transform: Transform::default(),
                velocity: None,
            },
            CookedEntity {
                id: id(2),
                parent: None,
                transform: Transform {
                    translation: [0., 0., 5.],
                    ..Default::default()
                },
                velocity: None,
            },
        ],
    )
    .unwrap()
    .with_spatial(vec![reference], vec![sprite, camera])
    .unwrap();
    let renderer = Arc::new(models::checked(Renderer::headless().unwrap()));
    eprintln!("texture-refresh adapter: {:?}", renderer.adapter.get_info());
    let texture = renderer.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Texture refresh admission control"),
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
    let targets = RefreshTargets {
        inner: Targets {
            texture: texture.clone(),
            presents: presents.clone(),
            fault: fault.clone(),
            dropped: dropped.clone(),
        },
        policy_changed,
    };
    let code=CompiledScript::compile("let calls=0;exports.default={update({chunks}){calls++;for(const c of chunks)for(let i=0;i<c.ids.length;i++)if(c.ids[i]==='00000000000000000000000001')c.transforms[i*c.transformStride]+=calls/10;}};").unwrap();
    let game = RefreshGame::new(
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
            model_cache: root.join("models"),
            texture_cache: root.join("textures"),
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
fn stage(game: &mut RefreshGame, reference: VisualAssetRef, cache: &Path) -> Candidate {
    let prepared = game
        .host()
        .unwrap()
        .texture_refresh_ticket()
        .prepare(&[reference], cache)
        .unwrap();
    match game
        .update_host(TextureRefreshOperation::Prepare(Some(prepared)))
        .unwrap()
    {
        TextureRefreshReply::Prepared(candidate) => candidate,
        _ => panic!("GPU preparation must succeed"),
    }
}
fn sprite_source(game: &RefreshGame) -> &Arc<incant_assets_runtime::RuntimeAsset> {
    &game
        .host()
        .unwrap()
        .render_input()
        .input()
        .nodes
        .iter()
        .find_map(|n| n.sprite.as_ref())
        .unwrap()
        .source
}
fn dominant(bytes: &[u8], channel: usize) -> usize {
    bytes
        .chunks_exact(4)
        .filter(|p| p[channel] > 150 && p[2 - channel] < 50 && p[3] > 200)
        .count()
}

#[test]
#[ignore = "requires a native GPU; run explicitly after resource grant"]
fn existing_texture_refresh_keeps_live_game_and_publishes_complete_gpu_pixels() {
    let root = tempfile::tempdir().unwrap();
    let first = cooked_texture(root.path(), [255, 0, 0, 255]);
    let next = cooked_texture(root.path(), [0, 0, 255, 255]);
    let policy_changed = Arc::new(AtomicBool::new(false));
    let (mut game, probe) = fixture(root.path(), first, policy_changed.clone());
    let lock = Mutex::new(());
    drop(lock.lock().unwrap()); // Matches prior editor startup locking.
    let identity = game
        .world()
        .unwrap()
        .spatial_view()
        .unwrap()
        .identity()
        .clone();
    let old = sprite_source(&game).clone();
    let old_version = old.version_id();
    game.refresh_host().unwrap();
    let red = pixels(&probe);
    assert!(dominant(&red, 0) >= 400);
    assert_eq!(dominant(&red, 2), 0);
    let candidate = stage(&mut game, next, &root.path().join("textures"));
    let mut refused = TextureRefreshOperation::Publish(candidate);
    policy_changed.store(true, Ordering::SeqCst);
    let mut ready = game.prepare_host_update().unwrap();
    let guard = lock.lock().unwrap();
    assert!(matches!(
        ready.update(&mut refused).unwrap(),
        TextureRefreshReply::Refused(_)
    ));
    assert_eq!(probe.presents.load(Ordering::SeqCst), 1);
    assert_eq!(probe.dropped.load(Ordering::SeqCst), 0);
    drop(guard);
    drop(ready);
    drop(refused);
    // Same world/tick/catalog generation: GPU policy refusal cannot lose its
    // prior scene or target provider, and another redraw still shows old pixels.
    assert!(!game.is_failed());
    assert_eq!(game.world().unwrap().tick(), 0);
    assert_eq!(game.host().unwrap().render_input().asset_generation(), 0);
    assert_eq!(sprite_source(&game).version_id(), old_version);
    assert_eq!(game.host().unwrap().consumer().accepted_frames(), 1);
    policy_changed.store(false, Ordering::SeqCst);
    game.refresh_host().unwrap();
    assert!(dominant(&pixels(&probe), 0) >= 400);
    let candidate = stage(&mut game, next, &root.path().join("textures"));
    // Remove only this test's own temporary source/cooked directory. All redraws
    // after this point use already prepared native CPU/GPU owners.
    root.close().unwrap();
    game.tick(&[]).unwrap();
    let tick = game.world().unwrap().tick();
    let pose = game.world().unwrap().inspect(id(1)).unwrap().transform;
    let frames = game.host().unwrap().consumer().accepted_frames();
    let presents = probe.presents.load(Ordering::SeqCst);
    let mut publication = TextureRefreshOperation::Publish(candidate);
    let mut ready = game.prepare_host_update().unwrap();
    let guard = lock.lock().unwrap();
    let receipt = match ready.update(&mut publication).unwrap() {
        TextureRefreshReply::Published(receipt) => receipt,
        _ => panic!("publication must succeed"),
    };
    assert_eq!((receipt.changed, receipt.generation), (1, 1));
    assert_eq!(probe.presents.load(Ordering::SeqCst), presents);
    assert_eq!(probe.dropped.load(Ordering::SeqCst), 0);
    drop(guard);
    drop(ready);
    assert_eq!(game.host().unwrap().consumer().accepted_frames(), frames);
    assert_eq!(game.world().unwrap().tick(), tick);
    assert_eq!(
        game.world().unwrap().inspect(id(1)).unwrap().transform,
        pose
    );
    assert_eq!(
        game.world().unwrap().spatial_view().unwrap().identity(),
        &identity
    );
    assert_ne!(sprite_source(&game).version_id(), old_version);
    game.refresh_host().unwrap();
    let blue = pixels(&probe);
    assert!(dominant(&blue, 2) >= 400);
    assert_eq!(dominant(&blue, 0), 0);
    assert_ne!(red, blue);
    assert_eq!(game.world().unwrap().tick(), tick);
    assert_eq!(
        game.host().unwrap().consumer().accepted_frames(),
        frames + 1
    );
    // An already used authority must refuse without another swap or draw.
    let mut ready = game.prepare_host_update().unwrap();
    let guard = lock.lock().unwrap();
    assert!(matches!(
        ready.update(&mut publication).unwrap(),
        TextureRefreshReply::Refused(_)
    ));
    assert_eq!(probe.presents.load(Ordering::SeqCst), presents + 1);
    drop(guard);
    drop(ready);
    drop(publication);
    assert_eq!(game.host().unwrap().render_input().asset_generation(), 1);
    assert_eq!(old.version_id(), old_version);
    game.refresh_host().unwrap();
    assert!(dominant(&pixels(&probe), 2) >= 400);
    game.tick(&[]).unwrap();
    assert_eq!(game.world().unwrap().tick(), 2);
    assert!(
        (game
            .world()
            .unwrap()
            .inspect(id(1))
            .unwrap()
            .transform
            .translation[0]
            - 0.3)
            .abs()
            < 1e-12,
        "same guest VM must retain calls=2"
    );
    assert_eq!(probe.dropped.load(Ordering::SeqCst), 0);
    eprintln!(
        "texture-refresh pixels: old_red={} new_blue={}; final_tick=2 frames={} presents={}",
        dominant(&red, 0),
        dominant(&blue, 2),
        game.host().unwrap().consumer().accepted_frames(),
        probe.presents.load(Ordering::SeqCst)
    );
    game.stop();
    assert_eq!(probe.dropped.load(Ordering::SeqCst), 1);
}

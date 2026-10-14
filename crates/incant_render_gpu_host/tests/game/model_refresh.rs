//! Numerical static-model publication; no native-window or appearance gate.
use super::*;
use incant_audio_runtime::{AudioManagerSettings, OfflineBackend};
use incant_game_runtime::HostedGameSession;
use incant_render_game::{
    PreparedHostTextureRefresh, TextureRefreshOperation, TextureRefreshReply, VisualCaches,
};
use incant_types::CameraProjection;
use std::{
    fs,
    path::Path,
    sync::{Mutex, atomic::AtomicBool},
};

struct ModelTargets {
    inner: Targets,
    policy_changed: Arc<AtomicBool>,
}
impl TargetProvider for ModelTargets {
    type Error = TargetError;
    type Frame = Frame;
    fn acquire(&mut self, renderer: &Renderer) -> Result<Frame, TargetError> {
        self.inner.acquire(renderer)
    }
    fn motion_epoch(&self) -> Option<u64> {
        self.policy_changed.load(Ordering::SeqCst).then_some(1)
    }
}
type ModelGame =
    HostedGameSession<OfflineBackend, NativeRenderHost<GpuRenderConsumer<ModelTargets>>>;
type Candidate = PreparedHostTextureRefresh<GpuRenderConsumer<ModelTargets>>;

fn model(root: &Path, replacement: bool) -> VisualAssetRef {
    // The second world instance starts beyond the right clip plane. Replacement
    // geometry/bounds move its visible coverage into the right half of the frame.
    let positions = if replacement {
        [-3f32, -1., 0., -1., -1., 0., -2., 1., 0.]
    } else {
        [-0.4, -0.4, 0., 0.4, -0.4, 0., 0., 0.4, 0.]
    };
    let uv = [0f32, 0., 1., 0., 0.5, 1.];
    fs::write(
        root.join("triangle.bin"),
        positions
            .into_iter()
            .chain(uv)
            .flat_map(f32::to_le_bytes)
            .collect::<Vec<_>>(),
    )
    .unwrap();
    let mut encoder = png::Encoder::new(fs::File::create(root.join("image.png")).unwrap(), 1, 1);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(if replacement {
            &[0, 0, 255, 255]
        } else {
            &[255, 0, 0, 255]
        })
        .unwrap();
    let (min, max, factor, roughness) = if replacement {
        ([-3., -1., 0.], [-1., 1., 0.], [0.4, 0.7, 1., 1.], 0.2)
    } else {
        ([-0.4, -0.4, 0.], [0.4, 0.4, 0.], [1.; 4], 0.8)
    };
    fs::write(root.join("triangle.gltf"), serde_json::to_vec(&json!({
        "asset":{"version":"2.0"},"scene":0,"scenes":[{"nodes":[0]}],
        "nodes":[{"mesh":0}],"meshes":[{"primitives":[{"attributes":{"POSITION":0,"TEXCOORD_0":1},"material":0}]}],
        "materials":[{"pbrMetallicRoughness":{"baseColorFactor":factor,"baseColorTexture":{"index":0},"metallicFactor":0,"roughnessFactor":roughness},"emissiveTexture":{"index":0},"emissiveFactor":[1,1,1],"doubleSided":true}],
        "textures":[{"source":0}],"images":[{"uri":"image.png"}],
        "buffers":[{"uri":"triangle.bin","byteLength":60}],
        "bufferViews":[{"buffer":0,"byteOffset":0,"byteLength":36},{"buffer":0,"byteOffset":36,"byteLength":24}],
        "accessors":[{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3","min":min,"max":max},
        {"bufferView":1,"componentType":5126,"count":3,"type":"VEC2"}]
    })).unwrap()).unwrap();
    let key = incant_assets::cook_gltf(
        root,
        Path::new("triangle.gltf"),
        &root.join(".incant/cache/models"),
    )
    .unwrap()
    .metadata
    .fingerprint;
    VisualAssetRef {
        id: id(90),
        kind: VisualAssetKind::Model,
        fingerprint: std::array::from_fn(|i| {
            u8::from_str_radix(&key[2 * i..2 * i + 2], 16).unwrap()
        }),
    }
}
fn fixture(
    root: &Path,
    reference: VisualAssetRef,
    policy_changed: Arc<AtomicBool>,
) -> (ModelGame, Probe) {
    let mut mesh = CookedSpatial::empty(id(1));
    mesh.mesh = Some(NativeMeshRenderer {
        mesh: reference.id,
        cast_shadows: true,
    });
    let mut second = CookedSpatial::empty(id(3));
    second.mesh = mesh.mesh.clone();
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
            CookedEntity {
                id: id(3),
                parent: None,
                transform: Transform {
                    translation: [4., 0., 0.],
                    ..Default::default()
                },
                velocity: None,
            },
        ],
    )
    .unwrap()
    .with_spatial(vec![reference], vec![mesh, camera, second])
    .unwrap();
    let renderer = Arc::new(models::checked(Renderer::headless().unwrap()));
    eprintln!("model-refresh adapter: {:?}", renderer.adapter.get_info());
    let texture = renderer.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Static model refresh control"),
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
    let targets = ModelTargets {
        inner: Targets {
            texture: texture.clone(),
            presents: presents.clone(),
            fault: fault.clone(),
            dropped: dropped.clone(),
        },
        policy_changed,
    };
    let code=CompiledScript::compile("let calls=0;exports.default={update({chunks}){calls++;for(const c of chunks)for(let i=0;i<c.ids.length;i++)if(c.ids[i]==='00000000000000000000000001')c.transforms[i*c.transformStride]+=calls/10;}};").unwrap();
    let game = ModelGame::new(
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
fn stage(game: &mut ModelGame, reference: VisualAssetRef, root: &Path) -> Candidate {
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
fn source(game: &ModelGame) -> &Arc<incant_assets_runtime::RuntimeAsset> {
    &game
        .host()
        .unwrap()
        .render_input()
        .input()
        .nodes
        .iter()
        .find_map(|n| n.mesh.as_ref())
        .unwrap()
        .source
}
fn colored(bytes: &[u8], channel: usize, region: std::ops::Range<usize>) -> usize {
    bytes
        .as_chunks::<4>()
        .0
        .iter()
        .enumerate()
        .filter(|(i, p)| {
            region.contains(&(i % 160)) && p[channel] > 150 && p[2 - channel] < 50 && p[3] > 200
        })
        .count()
}

#[test]
#[ignore = "requires a native GPU; run explicitly after resource grant"]
fn compatible_static_model_rebuilds_bounds_geometry_material_images_and_retains_rejected_pixels() {
    let root = tempfile::tempdir().unwrap();
    let first = model(root.path(), false);
    let next = model(root.path(), true);
    let policy_changed = Arc::new(AtomicBool::new(false));
    let (mut game, probe) = fixture(root.path(), first, policy_changed.clone());
    let lock = Mutex::new(());
    drop(lock.lock().unwrap()); // Actual editor's authoring lock is initialized at startup.
    let identity = game
        .world()
        .unwrap()
        .spatial_view()
        .unwrap()
        .identity()
        .clone();
    let old = source(&game).clone();
    let old_version = old.version_id();
    game.refresh_host().unwrap();
    let red = pixels(&probe);
    let old_red = colored(&red, 0, 0..160);
    assert!(old_red >= 80);
    assert_eq!(colored(&red, 0, 100..160), 0); // Second instance starts outside the view.
    assert_eq!(colored(&red, 2, 0..160), 0);
    let mut refused = TextureRefreshOperation::Publish(stage(&mut game, next, root.path()));
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
    assert!(!game.is_failed());
    assert_eq!(game.world().unwrap().tick(), 0);
    assert_eq!(game.host().unwrap().render_input().asset_generation(), 0);
    assert_eq!(source(&game).version_id(), old_version);
    policy_changed.store(false, Ordering::SeqCst);
    game.refresh_host().unwrap();
    assert_eq!(pixels(&probe), red); // Rejected candidate leaves the complete old pixels.
    let candidate = stage(&mut game, next, root.path());
    root.close().unwrap(); // Only this owned temporary source/cache; owners are already staged.
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
    let current = source(&game);
    assert_ne!(current.version_id(), old_version);
    let (
        incant_assets_runtime::RuntimeAssetData::Model(before),
        incant_assets_runtime::RuntimeAssetData::Model(after),
    ) = (old.data(), current.data())
    else {
        panic!("model versions")
    };
    assert_eq!(before.meshes[0].vertices[0][0], -0.4);
    assert_eq!(after.meshes[0].vertices[0][0], -3.);
    assert_eq!(after.materials[0].base_color, [0.4, 0.7, 1., 1.]);
    assert_eq!(after.materials[0].roughness, 0.2);
    assert_ne!(before.images[0].levels, after.images[0].levels);
    game.refresh_host().unwrap();
    let blue = pixels(&probe);
    let left = colored(&blue, 2, 0..70);
    let right = colored(&blue, 2, 100..160);
    assert!(
        left >= 500 && right >= 500,
        "new geometry and formerly offscreen instance must both draw"
    );
    assert!(left + right > old_red * 5);
    assert_eq!(colored(&blue, 0, 0..160), 0);
    assert_ne!(red, blue);
    assert_eq!(game.world().unwrap().tick(), tick);
    assert_eq!(
        game.host().unwrap().consumer().accepted_frames(),
        frames + 1
    );
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
    game.refresh_host().unwrap();
    assert_eq!(pixels(&probe), blue);
    assert_eq!(old.version_id(), old_version);
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
        "same guest module must retain calls=2"
    );
    assert_eq!(probe.dropped.load(Ordering::SeqCst), 0);
    eprintln!(
        "model-refresh pixels: old_red={old_red} new_left_blue={left} new_right_blue={right}; final_tick=2 frames={} presents={}",
        game.host().unwrap().consumer().accepted_frames(),
        probe.presents.load(Ordering::SeqCst)
    );
    game.stop();
    assert_eq!(probe.dropped.load(Ordering::SeqCst), 1);
}

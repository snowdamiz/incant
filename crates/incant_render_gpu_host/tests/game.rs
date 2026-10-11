#[path = "../../incant_render/tests/support/mod.rs"]
#[allow(dead_code)]
mod models;
#[path = "../../incant_assets_runtime/tests/support/mod.rs"]
mod pcm;
use incant_audio_runtime::ClipBinding;
use incant_game_runtime::HostedOfflineGameSession;
use incant_input::{InputActions, InputEvent, KeyCode};
use incant_render::{Renderer, RetainedTarget, wgpu};
use incant_render_game::{NativeRenderHost, NativeRenderSettings};
use incant_render_gpu_host::{GpuRenderConsumer, GpuRenderSettings, RenderFrame, TargetProvider};
use incant_runtime::{
    CookedAudio, CookedEntity, CookedPhysics, CookedScene, CookedSpatial, NativeAudioSource,
    NativeMeshRenderer, StableId, VisualAssetKind, VisualAssetRef,
};
use incant_script::CompiledScript;
use incant_types::{BodyMotion, Camera, Collider, RigidBody, Transform, Velocity};
use serde_json::json;
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
type Game = HostedOfflineGameSession<NativeRenderHost<GpuRenderConsumer<Targets>>>;
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
fn id(n: u128) -> StableId {
    StableId(n.to_be_bytes())
}
#[derive(Debug, thiserror::Error)]
#[error("test target unavailable")]
struct TargetError;
struct Targets {
    texture: wgpu::Texture,
    presents: Arc<AtomicUsize>,
    fault: Arc<AtomicUsize>,
    dropped: Arc<AtomicUsize>,
}
struct Frame {
    view: wgpu::TextureView,
    presents: Arc<AtomicUsize>,
    fault: Arc<AtomicUsize>,
}
impl Drop for Targets {
    fn drop(&mut self) {
        self.dropped.fetch_add(1, Ordering::SeqCst);
    }
}
impl TargetProvider for Targets {
    type Error = TargetError;
    type Frame = Frame;
    fn acquire(&mut self, _: &Renderer) -> Result<Frame, TargetError> {
        if self.fault.load(Ordering::SeqCst) == 1 {
            return Err(TargetError);
        }
        Ok(Frame {
            view: self.texture.create_view(&Default::default()),
            presents: Arc::clone(&self.presents),
            fault: Arc::clone(&self.fault),
        })
    }
}
impl RenderFrame for Frame {
    type Error = TargetError;
    fn target(&self) -> RetainedTarget<'_> {
        RetainedTarget {
            view: &self.view,
            format: FORMAT,
            width: 160,
            height: 96,
            viewport: None,
        }
    }
    fn present(self) -> Result<(), TargetError> {
        match self.fault.load(Ordering::SeqCst) {
            2 => return Err(TargetError),
            3 => panic!("target presentation panic"),
            _ => {}
        }
        self.presents.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}
struct Probe {
    renderer: Arc<Renderer>,
    texture: wgpu::Texture,
    presents: Arc<AtomicUsize>,
    fault: Arc<AtomicUsize>,
    dropped: Arc<AtomicUsize>,
}
fn game() -> (Game, Probe) {
    let temp = tempfile::tempdir().unwrap();
    let asset = models::model(temp.path(), 0.);
    let fingerprint =
        std::array::from_fn(|i| u8::from_str_radix(&asset.sha256[i * 2..i * 2 + 2], 16).unwrap());
    let reference = VisualAssetRef {
        id: id(91),
        kind: VisualAssetKind::Model,
        fingerprint,
    };
    let key = pcm::cooked(&temp.path().join("audio"), &[[0.2; 2]; 24000]);
    let clip = ClipBinding::load(id(90), &temp.path().join("audio"), &key).unwrap();
    let mut mesh = CookedSpatial::empty(id(1));
    mesh.mesh = Some(NativeMeshRenderer {
        mesh: id(91),
        cast_shadows: true,
    });
    let mut camera = CookedSpatial::empty(id(2));
    camera.camera = Some(Camera {
        fov_degrees: 55.,
        near: 0.1,
        far: 80.,
        projection: Default::default(),
    });
    let scene = CookedScene::new(
        id(99),
        64,
        vec![
            CookedEntity {
                id: id(1),
                parent: None,
                transform: Transform {
                    translation: [-2., 0., 0.],
                    ..Default::default()
                },
                velocity: Some(Velocity {
                    linear: [1., 0., 0.],
                }),
            },
            CookedEntity {
                id: id(2),
                parent: None,
                transform: Transform {
                    translation: [0., 2., 9.],
                    ..Default::default()
                },
                velocity: None,
            },
        ],
    )
    .unwrap()
    .with_physics(vec![CookedPhysics {
        entity: id(1),
        collider: Collider::default(),
        body: Some(RigidBody {
            motion: BodyMotion::Kinematic,
            ..Default::default()
        }),
        angular_velocity: None,
    }])
    .unwrap()
    .with_audio(vec![CookedAudio {
        entity: id(1),
        bus: None,
        listener: false,
        source: Some(NativeAudioSource {
            clip: id(90),
            bus: None,
            gain_db: 0.,
            pan: 0.,
            rate: 1.,
            start_seconds: 0.,
            playing: true,
            looping: true,
            streaming: false,
            spatial: None,
        }),
    }])
    .unwrap()
    .with_spatial(vec![reference], vec![mesh, camera])
    .unwrap();
    let scene = CookedScene::from_bytes(&scene.to_bytes()).unwrap();
    let renderer = Arc::new(Renderer::headless().unwrap());
    let texture = renderer.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Native hosted game probe"),
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
    let targets = Targets {
        texture: texture.clone(),
        presents: Arc::clone(&presents),
        fault: Arc::clone(&fault),
        dropped: Arc::clone(&dropped),
    };
    let code=CompiledScript::compile("exports.default={update({chunks,input}) { for(const c of chunks) for(let i=0;i<c.ids.length;i++) if(c.ids[i]==='00000000000000000000000001' && input.actions.move.pressed) c.transforms[i*10]+=0.5; }};").unwrap();
    let actions: InputActions = serde_json::from_value(
        json!({"move":{"kind":"button","bindings":[{"type":"key","code":"Space"}]}}),
    )
    .unwrap();
    let game = Game::new(
        scene,
        code,
        actions,
        vec![clip],
        48000,
        NativeRenderSettings {
            model_cache: temp.path().join(".incant/cache/models"),
            texture_cache: temp.path().join("textures"),
            camera: id(2),
            consumer: GpuRenderSettings {
                renderer: Arc::clone(&renderer),
                targets,
            },
        },
    )
    .unwrap();
    temp.close().unwrap(); // Source and all cooked files are absent throughout ticks.
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
fn pixels(probe: &Probe) -> Vec<u8> {
    let stride = 768;
    let size = stride * 96;
    let buffer = probe
        .renderer
        .device
        .create_buffer(&wgpu::BufferDescriptor {
            label: Some("Hosted game readback"),
            size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
    let mut encoder = probe
        .renderer
        .device
        .create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        probe.texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(stride as u32),
                rows_per_image: Some(96),
            },
        },
        probe.texture.size(),
    );
    probe.renderer.queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| tx.send(result).unwrap());
    probe
        .renderer
        .device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(Duration::from_secs(10)),
        })
        .unwrap();
    rx.recv_timeout(Duration::from_secs(10)).unwrap().unwrap();
    let mapped = buffer.slice(..).get_mapped_range();
    let bytes = mapped
        .chunks(stride as usize)
        .flat_map(|row| row[..160 * 4].iter().copied())
        .collect();
    drop(mapped);
    buffer.unmap();
    bytes
}
#[test]
#[ignore = "requires a native GPU; run by desktop workflows"]
fn script_physics_audio_and_submitted_frames_share_the_source_free_native_tick() {
    let (mut game, probe) = game();
    assert_eq!(game.host().unwrap().consumer().accepted_frames(), 0);
    assert_eq!(
        probe.presents.load(Ordering::SeqCst),
        0,
        "startup must not present before guest/audio readiness"
    );
    let mut first = None;
    for tick in 1..=8 {
        let mut pcm = vec![0.; game.next_output_frames().unwrap() * 2];
        let events = if tick == 2 {
            vec![InputEvent::Key {
                code: KeyCode::Space,
                down: true,
            }]
        } else {
            vec![]
        };
        game.tick(&events, &mut pcm).unwrap();
        assert!(pcm.iter().any(|v| v.abs() > 0.01));
        assert_eq!(game.host().unwrap().render_input().tick(), tick);
        assert_eq!(game.host().unwrap().consumer().accepted_frames(), tick);
        assert_eq!(probe.presents.load(Ordering::SeqCst), tick as usize);
        if tick == 1 {
            first = Some(pixels(&probe));
        }
    }
    assert_ne!(first.unwrap(), pixels(&probe));
    let native = game.world().unwrap().inspect(id(1)).unwrap();
    assert!((native.transform.translation[0] - (-2. + 8. / 64. + 0.5)).abs() < 0.00001);
    assert_eq!(game.rendered_frames().unwrap(), 6000);
    drop(game);
    assert_eq!(probe.dropped.load(Ordering::SeqCst), 1);
}
#[test]
#[ignore = "requires a native GPU; run by desktop workflows"]
fn target_errors_and_presentation_unwind_terminalize_game_and_silence_offline_pcm() {
    for mode in [1, 2, 3] {
        let (mut game, probe) = game();
        let mut output = vec![0.; game.next_output_frames().unwrap() * 2];
        game.tick(&[], &mut output).unwrap();
        assert!(output.iter().any(|v| v.abs() > 0.01));
        probe.fault.store(mode, Ordering::SeqCst);
        output.fill(0.7);
        let result =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| game.tick(&[], &mut output)));
        if mode == 3 {
            assert!(result.is_err());
        } else {
            assert!(result.unwrap().is_err());
        }
        assert!(game.is_failed());
        assert!(game.host().is_err());
        assert!(game.world().is_err());
        assert!(game.input().is_err());
        assert!(output.iter().all(|v| *v == 0.));
        assert_eq!(probe.dropped.load(Ordering::SeqCst), 1);
        assert!(game.tick(&[], &mut output).is_err());
    }
}

#[test]
#[ignore = "requires a native GPU; run by desktop workflows"]
fn observed_device_loss_stops_the_host_before_another_target_is_acquired() {
    let (mut game, probe) = game();
    let mut output = vec![0.; game.next_output_frames().unwrap() * 2];
    game.tick(&[], &mut output).unwrap();
    probe.renderer.device.destroy();
    // wgpu reports an explicit destroy only once pending submissions complete.
    // Production checks remain nonblocking; this test waits for the observation.
    probe
        .renderer
        .device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(Duration::from_secs(10)),
        })
        .unwrap();
    assert!(probe.renderer.check_device().is_err());
    output.fill(0.8);
    assert!(game.tick(&[], &mut output).is_err());
    assert!(game.is_failed());
    assert!(output.iter().all(|v| *v == 0.));
    assert_eq!(probe.presents.load(Ordering::SeqCst), 1);
    assert_eq!(probe.dropped.load(Ordering::SeqCst), 1);
}

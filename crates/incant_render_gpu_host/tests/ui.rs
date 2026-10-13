//! Screen-space UI through the ordinary composed game: routed input, candidate
//! publication, the GPU consumer and actual pixels over the moving world.
#[path = "../../incant_render/tests/support/mod.rs"]
#[allow(dead_code)]
mod models;
use incant_game_runtime::HostedOfflineGameSession;
use incant_input::{InputEvent, KeyCode};
use incant_render::{Renderer, RetainedTarget, ui_theme, wgpu};
use incant_render_game::{NativeRenderHost, NativeRenderSettings, UiSettings};
use incant_render_gpu_host::{GpuRenderConsumer, GpuRenderSettings, RenderFrame, TargetProvider};
use incant_runtime::{
    CookedEntity, CookedScene, CookedSpatial, CookedUi, FontAssetRef, NativeMeshRenderer, StableId,
    VisualAssetKind, VisualAssetRef,
};
use incant_script::CompiledScript;
use incant_types::{
    Camera, TextWrap, Transform, UiAlign, UiDirection, UiKind, UiLength, UiNode, UiStyle, UiText,
    Velocity,
};
use incant_ui::UiEvent;
use std::{
    sync::{Arc, Mutex},
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
/// A replaceable test-owned texture standing in for a resizable surface.
struct Targets(Arc<Mutex<wgpu::Texture>>);
struct Frame {
    view: wgpu::TextureView,
    size: [u32; 2],
}
impl TargetProvider for Targets {
    type Error = TargetError;
    type Frame = Frame;
    fn acquire(&mut self, _: &Renderer) -> Result<Frame, TargetError> {
        let texture = self.0.lock().unwrap();
        Ok(Frame {
            view: texture.create_view(&Default::default()),
            size: [texture.width(), texture.height()],
        })
    }
}
impl RenderFrame for Frame {
    type Error = TargetError;
    fn target(&self) -> RetainedTarget<'_> {
        RetainedTarget {
            view: &self.view,
            format: FORMAT,
            width: self.size[0],
            height: self.size[1],
            viewport: None,
        }
    }
    fn present(self) -> Result<(), TargetError> {
        Ok(())
    }
}
fn texture(renderer: &Renderer, size: [u32; 2]) -> wgpu::Texture {
    renderer.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Hosted UI probe"),
        size: wgpu::Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}
fn read(renderer: &Renderer, texture: &wgpu::Texture) -> Vec<u8> {
    let [w, h] = [texture.width(), texture.height()];
    let stride = (w * 4).div_ceil(256) * 256;
    let buffer = renderer.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Hosted UI readback"),
        size: u64::from(stride * h),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = renderer.device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(stride),
                rows_per_image: Some(h),
            },
        },
        texture.size(),
    );
    renderer.queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
    renderer
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
        .flat_map(|row| row[..w as usize * 4].iter().copied())
        .collect();
    drop(mapped);
    buffer.unmap();
    bytes
}
fn save(name: &str, pixels: &[u8], size: [u32; 2]) {
    if let Some(directory) = std::env::var_os("INCANT_UI_EVIDENCE") {
        std::fs::create_dir_all(&directory).unwrap();
        let file =
            std::fs::File::create(std::path::Path::new(&directory).join(format!("{name}.png")))
                .unwrap();
        let mut encoder = png::Encoder::new(file, size[0], size[1]);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(pixels)
            .unwrap();
    }
}

const ROOT: u128 = 10;
const LABEL: u128 = 11;
const BUTTON: u128 = 12;
const FIELD: u128 = 13;
const OFF: u128 = 14;
fn style(size: [UiLength; 2], padding: f32) -> UiStyle {
    UiStyle {
        size,
        padding: [padding; 4],
        ..Default::default()
    }
}
fn ui_nodes(fonts: &[FontAssetRef]) -> Vec<(u128, UiNode<StableId>)> {
    let text = |content: &str| UiText {
        content: content.into(),
        fonts: fonts.iter().map(|f| f.id).collect(),
        font_size: 16.,
        line_height: 20.,
        wrap: TextWrap::Word,
    };
    let node = |kind, style, order| UiNode {
        kind,
        style,
        order,
        disabled: false,
    };
    let px = UiLength::Pixels;
    let mut off = node(
        UiKind::Button { text: text("Off") },
        style([px(72.), px(32.)], 6.),
        4,
    );
    off.disabled = true;
    vec![
        (
            ROOT,
            node(
                UiKind::Root,
                UiStyle {
                    padding: [12.; 4],
                    gap: [8.; 2],
                    direction: UiDirection::Column,
                    align: UiAlign::Start,
                    ..Default::default()
                },
                0,
            ),
        ),
        (
            LABEL,
            node(
                UiKind::Label {
                    text: text("Score ffi سلام 中文"),
                },
                style([UiLength::Fraction(1.), UiLength::Auto], 0.),
                1,
            ),
        ),
        (
            BUTTON,
            node(
                UiKind::Button {
                    text: text("Start"),
                },
                style([px(96.), px(32.)], 6.),
                2,
            ),
        ),
        (
            FIELD,
            node(
                UiKind::TextInput { text: text("Name") },
                style([px(180.), px(32.)], 6.),
                3,
            ),
        ),
        (OFF, off),
    ]
}
struct Probe {
    renderer: Arc<Renderer>,
    texture: Arc<Mutex<wgpu::Texture>>,
}
impl Probe {
    fn pixels(&self) -> (Vec<u8>, [u32; 2]) {
        let texture = self.texture.lock().unwrap();
        (
            read(&self.renderer, &texture),
            [texture.width(), texture.height()],
        )
    }
    fn resize(&self, size: [u32; 2]) {
        *self.texture.lock().unwrap() = texture(&self.renderer, size);
    }
}
fn game(with_ui: bool, size: [u32; 2]) -> (Game, Probe) {
    let temp = tempfile::tempdir().unwrap();
    let asset = models::model(temp.path(), 0.);
    let fingerprint =
        std::array::from_fn(|i| u8::from_str_radix(&asset.sha256[i * 2..i * 2 + 2], 16).unwrap());
    let mut mesh = CookedSpatial::empty(id(1));
    mesh.mesh = Some(NativeMeshRenderer {
        mesh: id(91),
        cast_shadows: false,
    });
    let mut camera = CookedSpatial::empty(id(2));
    camera.camera = Some(Camera {
        fov_degrees: 55.,
        near: 0.1,
        far: 80.,
        projection: Default::default(),
    });
    let mut fonts = Vec::new();
    for (n, name) in [
        "NotoSans-Regular.ttf",
        "NotoSansArabic.ttf",
        "IncantCJKFixture.ttf",
    ]
    .into_iter()
    .enumerate()
    {
        let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../incant_text/tests/fonts")
            .join(name);
        std::fs::copy(source, temp.path().join(name)).unwrap();
        let font = incant_assets::cook_font(
            temp.path(),
            std::path::Path::new(name),
            &temp.path().join("fonts"),
        )
        .unwrap();
        fonts.push(FontAssetRef {
            id: id(80 + n as u128),
            fingerprint: std::array::from_fn(|i| {
                u8::from_str_radix(&font.metadata().fingerprint[2 * i..2 * i + 2], 16).unwrap()
            }),
        });
    }
    let nodes = if with_ui { ui_nodes(&fonts) } else { vec![] };
    let mut entities = vec![
        CookedEntity {
            id: id(1),
            parent: None,
            transform: Transform {
                translation: [-1.5, 0.5, 0.],
                ..Default::default()
            },
            velocity: Some(Velocity {
                linear: [6., 0., 0.],
            }),
        },
        CookedEntity {
            id: id(2),
            parent: None,
            transform: Transform {
                translation: [0., 1., 6.],
                ..Default::default()
            },
            velocity: None,
        },
    ];
    entities.extend(nodes.iter().map(|(n, _)| CookedEntity {
        id: id(*n),
        parent: (*n != ROOT).then(|| id(ROOT)),
        transform: Default::default(),
        velocity: None,
    }));
    let scene = CookedScene::new(id(99), 60, entities)
        .unwrap()
        .with_spatial(
            vec![VisualAssetRef {
                id: id(91),
                kind: VisualAssetKind::Model,
                fingerprint,
            }],
            vec![mesh, camera],
        )
        .unwrap()
        .with_text(fonts, vec![])
        .unwrap()
        .with_ui(
            nodes
                .into_iter()
                .map(|(n, node)| CookedUi {
                    entity: id(n),
                    node: Arc::new(node),
                })
                .collect(),
        )
        .unwrap();
    let scene = CookedScene::from_bytes(&scene.to_bytes()).unwrap();
    let renderer = Arc::new(models::checked(Renderer::headless().unwrap()));
    let shared = Arc::new(Mutex::new(texture(&renderer, size)));
    let script = CompiledScript::compile(
        "exports.default={update({input,log}){for(const e of input.ui.events)log(e.kind+('value' in e?':'+e.value:''));}};",
    )
    .unwrap();
    let game = Game::new(
        scene,
        script,
        Default::default(),
        vec![],
        48000,
        NativeRenderSettings {
            ui: with_ui.then(|| UiSettings::new([size[0] as f32, size[1] as f32])),
            text: Some(incant_render_native::TextSettings::new(
                temp.path().join("fonts"),
            )),
            model_cache: temp.path().join(".incant/cache/models"),
            texture_cache: temp.path().join("textures"),
            camera: id(2),
            consumer: GpuRenderSettings {
                renderer: Arc::clone(&renderer),
                targets: Targets(Arc::clone(&shared)),
            },
        },
    )
    .unwrap();
    temp.close().unwrap(); // Fonts and every cooked file are absent from here on.
    (
        game,
        Probe {
            renderer,
            texture: shared,
        },
    )
}
fn tick(game: &mut Game, keys: &[InputEvent], ui: &[UiEvent]) {
    let mut pcm = vec![0.; game.next_output_frames().unwrap() * 2];
    game.tick_with_ui(keys, ui, &mut pcm).unwrap();
}
fn key(code: KeyCode, down: bool) -> InputEvent {
    InputEvent::Key { code, down }
}
/// Pixels within `rect` (physical, [l, t, r, b]) that differ between frames.
fn changed(a: &[u8], b: &[u8], size: [u32; 2], rect: [u32; 4]) -> usize {
    (rect[1]..rect[3])
        .flat_map(|y| (rect[0]..rect[2]).map(move |x| ((y * size[0] + x) * 4) as usize))
        .filter(|&i| a[i..i + 3] != b[i..i + 3])
        .count()
}
fn accent(pixels: &[u8]) -> usize {
    pixels
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|p| (0..3).all(|c| p[c].abs_diff(ui_theme::ACCENT[c]) <= 2))
        .count()
}
fn widget_box(game: &Game, n: u128) -> [u32; 4] {
    let input = game.host().unwrap().ui_input().unwrap();
    let w = input.widgets.iter().find(|w| w.id == id(n)).unwrap();
    w.visible.map(|v| (v + 0.5).floor() as u32)
}

#[test]
#[ignore = "requires a native GPU; run by desktop workflows"]
fn composed_ui_draws_over_the_moving_world_and_reuses_unchanged_resources() {
    let size = [320, 192];
    let (mut plain, plain_probe) = game(false, size);
    let (mut game, probe) = game(true, size);
    let host = game.host().unwrap();
    assert!(host.ui_input().is_some());
    let startup = probe.renderer.ui_uploads();
    assert_eq!((startup.atlas_textures, startup.quad_buffers), (1, 1));
    let stats = host.consumer().ui_stats().unwrap();
    assert_eq!(stats.widgets, 4);
    assert!(stats.glyph_quads > 20 && stats.gpu_bytes > 0);
    let input = Arc::clone(host.ui_input().unwrap());
    let mut previous: Option<Vec<u8>> = None;
    for frame in 1..=4 {
        tick(&mut game, &[], &[]);
        plain
            .tick(&[], &mut vec![0.; plain.next_output_frames().unwrap() * 2])
            .unwrap();
        // The world moves every tick; the UI input and its GPU resources do not.
        assert!(Arc::ptr_eq(
            &input,
            game.host().unwrap().ui_input().unwrap()
        ));
        assert_eq!(probe.renderer.ui_uploads().atlas_textures, 1, "{frame}");
        assert_eq!(probe.renderer.ui_uploads().quad_buffers, 1, "{frame}");
        let (pixels, _) = probe.pixels();
        let (world, _) = plain_probe.pixels();
        // Outside every widget the composed frame is exactly the world frame.
        let boxes: Vec<_> = [LABEL, BUTTON, FIELD, OFF]
            .map(|n| widget_box(&game, n))
            .into();
        for (i, (a, b)) in pixels.chunks(4).zip(world.chunks(4)).enumerate() {
            let (x, y) = (i as u32 % size[0], i as u32 / size[0]);
            if !boxes
                .iter()
                .any(|r| x >= r[0] && x < r[2] && y >= r[1] && y < r[3])
            {
                assert_eq!(a, b, "frame {frame}: world pixel {x},{y} changed");
            }
        }
        for n in [LABEL, BUTTON, FIELD, OFF] {
            assert!(
                changed(&pixels, &world, size, widget_box(&game, n)) > 50,
                "{n}"
            );
        }
        if let Some(previous) = &previous {
            assert_ne!(previous, &pixels, "the world behind the UI moves");
        }
        if frame == 1 {
            save("hosted-ui-tick1", &pixels, size);
            save("hosted-world-tick1", &world, size);
        }
        previous = Some(pixels);
    }
    assert_eq!(
        accent(previous.as_ref().unwrap()),
        0,
        "nothing is focused yet"
    );
    // Tab focuses the button through routed physical input: a new input Arc,
    // one new instance buffer, the same atlas, and a visible focus ring.
    tick(&mut game, &[key(KeyCode::Tab, true)], &[]);
    tick(&mut game, &[key(KeyCode::Tab, false)], &[]);
    let focused = Arc::clone(game.host().unwrap().ui_input().unwrap());
    assert!(!Arc::ptr_eq(&input, &focused));
    assert!(Arc::ptr_eq(&input.atlas, &focused.atlas));
    let uploads = probe.renderer.ui_uploads();
    assert_eq!((uploads.atlas_textures, uploads.quad_buffers), (1, 2));
    let (pixels, _) = probe.pixels();
    save("hosted-ui-button-focus", &pixels, size);
    let ring = accent(&pixels);
    assert!(ring > 100, "{ring}");
    // Enter activates the focused button exactly once.
    tick(&mut game, &[key(KeyCode::Enter, true)], &[]);
    tick(&mut game, &[key(KeyCode::Enter, false)], &[]);
    let logs: Vec<_> = game
        .logs()
        .unwrap()
        .entries()
        .iter()
        .map(|e| e.message.clone())
        .collect();
    assert_eq!(logs, ["activated"]);
    // Focus the field, commit text and compose: changed text reshapes once.
    tick(&mut game, &[], &[UiEvent::Focus(id(FIELD))]);
    let target = game.host().unwrap().ui().unwrap().focus().unwrap();
    tick(
        &mut game,
        &[],
        &[UiEvent::Commit {
            target: target.clone(),
            text: " 日本".into(),
        }],
    );
    let typed = probe.renderer.ui_uploads();
    assert_eq!(typed.atlas_textures, 2);
    let (committed, _) = probe.pixels();
    save("hosted-ui-field-commit", &committed, size);
    tick(
        &mut game,
        &[],
        &[UiEvent::Preedit {
            target,
            text: "語".into(),
            cursor: Some(3..3),
        }],
    );
    let field = game.host().unwrap().ui_input().unwrap();
    let edit = field
        .widgets
        .iter()
        .find(|w| w.id == id(FIELD))
        .unwrap()
        .edit
        .clone()
        .unwrap();
    assert_eq!(edit.preedit, Some("Name 日本".len().."Name 日本語".len()));
    let (composing, _) = probe.pixels();
    save("hosted-ui-field-preedit", &composing, size);
    assert!(changed(&committed, &composing, size, widget_box(&game, FIELD)) > 20);
    assert_eq!(
        game.host()
            .unwrap()
            .ui()
            .unwrap()
            .text(id(FIELD))
            .unwrap()
            .text(),
        "Name 日本"
    );
}

#[test]
#[ignore = "requires a native GPU; run by desktop workflows"]
fn refresh_and_viewport_relayout_use_the_same_ui_without_advancing_the_game() {
    let (mut game, probe) = game(true, [320, 192]);
    tick(&mut game, &[key(KeyCode::Tab, true)], &[]);
    let (before, _) = probe.pixels();
    let uploads = probe.renderer.ui_uploads();
    let frames = game.host().unwrap().consumer().accepted_frames();
    let world = game.world().unwrap().snapshot();
    game.set_suspended(true).unwrap();
    game.refresh_host().unwrap();
    assert_eq!(probe.pixels().0, before);
    assert_eq!(probe.renderer.ui_uploads(), uploads);
    // A narrow logical viewport (the host's 2x scale of a 240x320 surface).
    assert_eq!(
        game.host().unwrap().consumer().accepted_frames(),
        frames + 1
    );
    // Relayout admits the candidate's GPU resources but presents nothing.
    game.resize_ui([120., 160.]).unwrap();
    assert_eq!(
        game.host().unwrap().consumer().accepted_frames(),
        frames + 1
    );
    let admitted = probe.renderer.ui_uploads();
    assert_eq!(admitted.quad_buffers, uploads.quad_buffers + 1);
    probe.resize([240, 320]);
    game.refresh_host().unwrap();
    assert_eq!(
        probe.renderer.ui_uploads().quad_buffers,
        admitted.quad_buffers
    );
    let (narrow, size) = probe.pixels();
    save("hosted-ui-narrow-2x", &narrow, size);
    assert_eq!(game.world().unwrap().snapshot(), world);
    assert_eq!(
        game.host().unwrap().consumer().accepted_frames(),
        frames + 2
    );
    let input = game.host().unwrap().ui_input().unwrap();
    assert_eq!(input.viewport, [120., 160.]);
    // Narrow layout wraps the label; the 2x frame keeps the focus ring 4 px wide.
    let label = input.widgets.iter().find(|w| w.id == id(LABEL)).unwrap();
    assert!(label.layout.lines().len() >= 2);
    assert!(accent(&narrow) > 100);
    let malformed = game.resize_ui([f32::NAN, 10.]);
    assert!(malformed.is_err() && !game.is_failed());
    assert_eq!(
        game.host().unwrap().ui_input().unwrap().viewport,
        [120., 160.]
    );
}

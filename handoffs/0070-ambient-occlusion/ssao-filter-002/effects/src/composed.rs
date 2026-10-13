//! Composed-host review scene for the bounded appearance pairs: the canonical
//! native route (CookedScene -> HostedOfflineGameSession -> NativeRenderHost ->
//! GpuRenderConsumer) with a helper-owned offscreen target. Fonts, UI tree,
//! script and input actions are handoff 0067's reviewed fixture; the static
//! geometry adds contact creases, a smooth sphere, a MASK card, opaque/BLEND
//! emissive surfaces and a sprite. Review content, not game art or a preset.
use crate::common::{FORMAT, Result, write_png};
use crate::gltf::{self, Mesh};
use glam::DQuat;
use incant_game_runtime::HostedOfflineGameSession;
use incant_input::InputActions;
use incant_render::{Renderer, RetainedTarget, Viewport, wgpu};
use incant_render_game::{NativeRenderHost, NativeRenderSettings, TextSettings, UiSettings};
use incant_render_gpu_host::{GpuRenderConsumer, GpuRenderSettings, RenderFrame, TargetProvider};
use incant_runtime::{
    CookedEntity, CookedScene, CookedSpatial, CookedText, CookedUi, FontAssetRef,
    NativeMeshRenderer, NativeSpriteRegion, NativeSpriteRenderer, NativeText, SpriteMode,
    SpriteSampling, StableId, TextLayout, TextWrap, VisualAssetKind, VisualAssetRef,
};
use incant_script::CompiledScript;
use incant_types::{
    Camera, CameraProjection, DirectionalLight, TextureUsage, Transform, UiAlign, UiDirection,
    UiKind, UiLength, UiNode, UiStyle, UiText,
};
use serde_json::{Value, json};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

pub type Game<P> = HostedOfflineGameSession<NativeRenderHost<GpuRenderConsumer<P>>>;

pub const CAMERA: u128 = 0x10;
pub const LIGHT: u128 = 0x11;
pub const FLOOR: u128 = 0x12;
pub const CRATE: u128 = 0x13;
pub const PILLAR: u128 = 0x14;
pub const WORLD_TEXT: u128 = 0x15;
pub const SPHERE: u128 = 0x16;
pub const PANEL: u128 = 0x17;
pub const PANE: u128 = 0x18;
pub const CARD: u128 = 0x19;
pub const SPRITE: u128 = 0x1A;
pub const ROOT: u128 = 0x20;
pub const TITLE: u128 = 0x21;
pub const BODY: u128 = 0x22;
pub const ACTIONS: u128 = 0x23;
pub const RESUME: u128 = 0x24;
pub const OPTIONS: u128 = 0x25;
pub const ONLINE: u128 = 0x26;
pub const CONTINUE: u128 = 0x27;
pub const FIELDS: u128 = 0x28;
const FONTS: [(u128, &str); 3] = [
    (0x80, "NotoSans-Regular.ttf"),
    (0x81, "NotoSansArabic.ttf"),
    (0x82, "IncantCJKFixture.ttf"),
];
const SPRITE_TEXTURE: u128 = 0xA0;

pub fn id(n: u128) -> StableId {
    StableId(n.to_be_bytes())
}
/// Crockford base32 ULID text of a stable ID (as the script API spells it).
pub fn ulid_text(id: StableId) -> String {
    const DIGITS: &[u8] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
    let value = u128::from_be_bytes(id.0);
    (0..26)
        .rev()
        .map(|i| DIGITS[((value >> (5 * i)) & 31) as usize] as char)
        .collect()
}
fn fingerprint(hex: &str) -> Result<[u8; 32]> {
    let mut out = [0; 32];
    for (i, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(hex.get(2 * i..2 * i + 2).ok_or("fingerprint")?, 16)?;
    }
    Ok(out)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum View {
    Perspective,
    Orthographic,
    CloseUp,
}
impl View {
    pub fn name(self) -> &'static str {
        match self {
            Self::Perspective => "perspective",
            Self::Orthographic => "orthographic",
            Self::CloseUp => "close-up",
        }
    }
    fn camera(self) -> (Transform, CameraProjection) {
        let pitch = DQuat::from_rotation_x((-8f64).to_radians()).to_array();
        match self {
            Self::Perspective => (
                transform([0., 0.9, 7.], pitch, [1.; 3]),
                CameraProjection::Perspective {},
            ),
            Self::Orthographic => (
                transform([0., 0.9, 7.], pitch, [1.; 3]),
                CameraProjection::Orthographic { vertical_size: 6. },
            ),
            // Looks down at the sphere/crate contact creases on the floor.
            Self::CloseUp => {
                let (eye, at): ([f64; 3], [f64; 3]) = ([1.0, -0.55, 2.6], [1.25, -1.0, 0.4]);
                let d = [at[0] - eye[0], at[1] - eye[1], at[2] - eye[2]];
                let length = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
                let yaw = (-d[0]).atan2(-d[2]);
                let pitch = (d[1] / length).asin();
                let rotation =
                    (DQuat::from_rotation_y(yaw) * DQuat::from_rotation_x(pitch)).to_array();
                (
                    transform(eye, rotation, [1.; 3]),
                    CameraProjection::Perspective {},
                )
            }
        }
    }
}

fn transform(translation: [f64; 3], rotation: [f64; 4], scale: [f64; 3]) -> Transform {
    Transform {
        translation,
        rotation,
        scale,
    }
}

/// Cook one static mesh into the model cache and return its fingerprint.
fn cook(
    root: &Path,
    name: &str,
    mesh: &Mesh,
    material: Value,
    image: Option<(&[u8], [u32; 2])>,
) -> Result<[u8; 32]> {
    let image_name = format!("{name}.png");
    if let Some((pixels, size)) = image {
        write_png(&root.join(&image_name), pixels, size)?;
    }
    let (doc, bin) = gltf::document(
        name,
        mesh,
        material,
        image.map(|_| image_name.as_str()),
        None,
    );
    std::fs::write(root.join(format!("{name}.bin")), bin)?;
    std::fs::write(root.join(format!("{name}.gltf")), serde_json::to_vec(&doc)?)?;
    let cooked = incant_assets::cook_gltf(
        root,
        Path::new(&format!("{name}.gltf")),
        &root.join(".incant/cache/models"),
    )?;
    fingerprint(&cooked.metadata.fingerprint)
}

/// Cooked caches live in `dir` until the fixture is dropped; every game loads
/// them at construction only.
pub struct Fixture {
    dir: tempfile::TempDir,
    bytes: Vec<u8>,
    pub view: View,
    pub ui: bool,
}

/// 0067's style() base: auto size, zero gaps, column, stretch, start, no grow
/// or shrink. Callers override fields as 0067 did.
fn style(edit: impl FnOnce(&mut UiStyle)) -> UiStyle {
    let mut style = UiStyle {
        shrink: 0.,
        ..UiStyle::default()
    };
    edit(&mut style);
    style
}

impl Fixture {
    pub fn new(view: View, ui: bool) -> Result<Self> {
        let dir = tempfile::tempdir()?;
        let root = dir.path();
        let fonts_source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../../crates/incant_text/tests/fonts");
        let mut fonts = Vec::new();
        for (n, name) in FONTS {
            std::fs::copy(fonts_source.join(name), root.join(name))?;
            let font = incant_assets::cook_font(root, Path::new(name), &root.join("fonts"))?;
            fonts.push(FontAssetRef {
                id: id(n),
                fingerprint: fingerprint(&font.metadata().fingerprint)?,
            });
        }
        let font_ids: Vec<StableId> = fonts.iter().map(|f| f.id).collect();
        let solid = |color: [f32; 4], roughness: f32| gltf::pbr(color, 0., roughness);
        let mut panel_material = solid([0.9, 0.9, 0.9, 1.], 0.5);
        panel_material["emissiveFactor"] = json!([1., 0.55, 0.25]);
        let mut pane_material = solid([0.35, 0.8, 1., 0.5], 0.3);
        pane_material["emissiveFactor"] = json!([0.3, 0.8, 1.]);
        pane_material["alphaMode"] = json!("BLEND");
        let mut card_material = solid([0.95, 0.85, 0.6, 1.], 0.7);
        card_material["alphaMode"] = json!("MASK");
        card_material["alphaCutoff"] = json!(0.5);
        card_material["doubleSided"] = json!(true);
        // Vertical alpha stripes, 8x8: alternate columns cut out.
        let stripes: Vec<u8> = (0..64)
            .flat_map(|i| [255, 255, 255, if i % 2 == 0 { 255 } else { 0 }])
            .collect();
        let cube = gltf::cuboid();
        let models = [
            (
                0x90u128,
                cook(
                    root,
                    "slate",
                    &cube,
                    solid([0.15, 0.17, 0.2, 1.], 0.6),
                    None,
                )?,
            ),
            (
                0x91,
                cook(root, "amber", &cube, solid([0.8, 0.52, 0.2, 1.], 0.6), None)?,
            ),
            (
                0x92,
                cook(root, "teal", &cube, solid([0.18, 0.46, 0.5, 1.], 0.6), None)?,
            ),
            (
                0x93,
                cook(
                    root,
                    "ivory",
                    &gltf::sphere(24, 48),
                    solid([0.85, 0.85, 0.82, 1.], 0.35),
                    None,
                )?,
            ),
            (0x94, cook(root, "panel", &cube, panel_material, None)?),
            (0x95, cook(root, "pane", &cube, pane_material, None)?),
            (
                0x96,
                cook(
                    root,
                    "card",
                    &gltf::unit_quad(1., 1., true),
                    card_material,
                    Some((stripes.as_slice(), [8, 8])),
                )?,
            ),
        ];
        let mut visuals: Vec<VisualAssetRef> = models
            .iter()
            .map(|(n, print)| VisualAssetRef {
                id: id(*n),
                kind: VisualAssetKind::Model,
                fingerprint: *print,
            })
            .collect();
        let checker: Vec<u8> = (0..64)
            .flat_map(|i| {
                if (i % 8 + i / 8) % 2 == 0 {
                    [240, 200, 60, 255]
                } else {
                    [40, 40, 52, 255]
                }
            })
            .collect();
        write_png(&root.join("sprite.png"), &checker, [8, 8])?;
        let texture = incant_assets::cook_texture(
            root,
            Path::new("sprite.png"),
            &root.join("textures"),
            TextureUsage::Color,
        )?;
        visuals.push(VisualAssetRef {
            id: id(SPRITE_TEXTURE),
            kind: VisualAssetKind::Texture,
            fingerprint: fingerprint(&texture.metadata.fingerprint)?,
        });
        // Key light from the upper left, a little behind the camera (0067),
        // bright enough that lit radiance exceeds the bloom threshold of one.
        let half = |degrees: f64| (degrees.to_radians() / 2.).sin_cos();
        let ((sy, cy), (sx, cx)) = (half(30.), half(-40.));
        let key = [cy * sx, sy * cx, -sy * sx, cy * cx];
        let (camera_pose, projection) = view.camera();
        let entity = |n: u128, parent: Option<u128>, transform: Transform| CookedEntity {
            id: id(n),
            parent: parent.map(id),
            transform,
            velocity: None,
        };
        let identity = [0., 0., 0., 1.];
        let one = [1.; 3];
        let mut entities = vec![
            entity(CAMERA, None, camera_pose),
            entity(LIGHT, None, transform([0.; 3], key, one)),
            entity(
                FLOOR,
                None,
                transform([0., -1.25, -2.], identity, [16., 0.1, 12.]),
            ),
            entity(CRATE, None, transform([1.9, -0.7, 0.2], identity, one)),
            entity(
                PILLAR,
                None,
                transform([-2.6, 0.3, -2.5], identity, [0.9, 3., 0.9]),
            ),
            entity(
                SPHERE,
                None,
                transform([0.6, -0.6, 0.6], identity, [1.2; 3]),
            ),
            entity(
                PANEL,
                None,
                transform([-2.6, 0.6, -2.02], identity, [0.9, 0.5, 0.06]),
            ),
            entity(
                PANE,
                None,
                transform([-0.6, 0.2, -0.8], identity, [1., 0.8, 0.04]),
            ),
            entity(
                CARD,
                None,
                transform([-1.1, -0.6, 1.2], identity, [1.2, 1.2, 1.]),
            ),
            entity(SPRITE, None, transform([2.6, 0.4, -1.], identity, one)),
            entity(
                WORLD_TEXT,
                None,
                transform([1.2, 1.8, -0.5], identity, [3.; 3]),
            ),
        ];
        let mut camera = CookedSpatial::empty(id(CAMERA));
        camera.camera = Some(Camera {
            fov_degrees: 45.,
            near: 0.1,
            far: 100.,
            projection,
        });
        let mut light = CookedSpatial::empty(id(LIGHT));
        light.directional = Some(DirectionalLight {
            color: [1., 0.97, 0.92],
            intensity: 6.,
            shadows: None,
        });
        let mesh = |n: u128, model: u128| {
            let mut spatial = CookedSpatial::empty(id(n));
            spatial.mesh = Some(NativeMeshRenderer {
                mesh: id(model),
                cast_shadows: false,
            });
            spatial
        };
        let mut sprite = CookedSpatial::empty(id(SPRITE));
        sprite.sprite = Some(Box::new(NativeSpriteRenderer {
            texture: id(SPRITE_TEXTURE),
            region: NativeSpriteRegion::Pixels([0, 0, 8, 8]),
            size: [0.8, 0.8],
            pivot: [0.5, 0.5],
            tint: [1.; 4],
            flip_x: false,
            flip_y: false,
            layer: 0,
            order: 0,
            sampling: SpriteSampling::Nearest,
            mode: SpriteMode::Simple,
        }));
        let spatial = vec![
            camera,
            light,
            mesh(FLOOR, 0x90),
            mesh(CRATE, 0x91),
            mesh(PILLAR, 0x92),
            mesh(SPHERE, 0x93),
            mesh(PANEL, 0x94),
            mesh(PANE, 0x95),
            mesh(CARD, 0x96),
            sprite,
        ];
        let world_text = CookedText {
            entity: id(WORLD_TEXT),
            text: Arc::new(NativeText {
                content: "ffi سلام 中文".into(),
                fonts: font_ids.iter().copied().collect(),
                layout: TextLayout {
                    font_size: 20.,
                    line_height: 28.,
                    width: 500.,
                    wrap: TextWrap::WordOrGlyph,
                },
            }),
        };
        let nodes = if ui { ui_nodes(&font_ids) } else { vec![] };
        entities.extend(
            nodes
                .iter()
                .map(|(n, parent, _)| entity(*n, *parent, Transform::default())),
        );
        let scene = CookedScene::new(id(0x99), 60, entities)?
            .with_spatial(visuals, spatial)?
            .with_text(fonts, vec![world_text])?
            .with_ui(
                nodes
                    .into_iter()
                    .map(|(n, _, node)| CookedUi {
                        entity: id(n),
                        node: Arc::new(node),
                    })
                    .collect(),
            )?;
        Ok(Self {
            bytes: scene.to_bytes(),
            dir,
            view,
            ui,
        })
    }
    pub fn scene_sha256(&self) -> String {
        incant_assets_runtime::sha256(&self.bytes)
    }
    /// The canonical composed game: script, input, audio clock, native render
    /// host and the GPU consumer. `size` is the physical UI viewport.
    pub fn game<P: TargetProvider>(
        &self,
        size: [u32; 2],
        consumer: GpuRenderSettings<P>,
    ) -> Result<Game<P>> {
        let root = self.dir.path();
        let settings = NativeRenderSettings {
            ui: self
                .ui
                .then(|| UiSettings::new([size[0] as f32, size[1] as f32])),
            text: Some(TextSettings::new(root.join("fonts"))),
            model_cache: root.join(".incant/cache/models"),
            texture_cache: root.join("textures"),
            camera: id(CAMERA),
            consumer,
        };
        Game::new(
            CookedScene::from_bytes(&self.bytes)?,
            script()?,
            actions()?,
            vec![],
            48000,
            settings,
        )
        .map_err(|e| e.to_string().into())
    }
}

/// 0067's script and action map (UI event logging; Space despawns the pillar).
fn script() -> Result<CompiledScript> {
    let source = format!(
        "exports.default={{update({{input,commands,log}}){{if(input.actions.drop.pressed)commands.despawn('{}');for(const e of input.ui.events)log(e.kind+('value' in e?':'+e.value:''));}}}};",
        ulid_text(id(PILLAR))
    );
    Ok(CompiledScript::compile(&source)?)
}
fn actions() -> Result<InputActions> {
    Ok(serde_json::from_value(serde_json::json!(
        {"drop": {"kind": "button", "bindings": [{"type": "key", "code": "Space"}]}}
    ))?)
}

type UiEntry = (u128, Option<u128>, UiNode<StableId>);
/// 0067's UI tree and strings (originally 0056), in its order and styles.
fn ui_nodes(fonts: &[StableId]) -> Vec<UiEntry> {
    let text = |content: &str, size: f32, line: f32, wrap| UiText {
        content: content.into(),
        fonts: fonts.to_vec(),
        font_size: size,
        line_height: line,
        wrap,
    };
    let node = |kind, style, order| UiNode {
        kind,
        style,
        order,
        disabled: false,
    };
    let full_width = |max: f32| {
        move |s: &mut UiStyle| {
            s.size = [UiLength::Fraction(1.), UiLength::Auto];
            s.max_size = [Some(max), None];
        }
    };
    let button = |n: u128, label: &str, order: i32, disabled: bool| {
        let mut b = node(
            UiKind::Button {
                text: text(label, 16., 20., TextWrap::Word),
            },
            style(|s| s.padding = [16., 9., 16., 9.]),
            order,
        );
        b.disabled = disabled;
        (n, Some(ACTIONS), b)
    };
    let group = |key: u128, caption: &str, value: &str, order: i32| {
        [
            (
                key + 0x08,
                Some(FIELDS),
                node(UiKind::Container, style(|s| s.gap = [0., 4.]), order),
            ),
            (
                key - 0x01,
                Some(key + 0x08),
                node(
                    UiKind::Label {
                        text: text(caption, 13., 18., TextWrap::Word),
                    },
                    style(|_| {}),
                    1,
                ),
            ),
            (
                key,
                Some(key + 0x08),
                node(
                    UiKind::TextInput {
                        text: text(value, 16., 22., TextWrap::None),
                    },
                    style(|s| {
                        s.size = [UiLength::Fraction(1.), UiLength::Auto];
                        s.padding = [10., 7., 10., 7.];
                    }),
                    2,
                ),
            ),
        ]
    };
    let mut nodes = vec![
        (
            ROOT,
            None,
            node(
                UiKind::Root,
                style(|s| {
                    s.padding = [32., 28., 32., 28.];
                    s.gap = [0., 14.];
                    s.align = UiAlign::Start;
                    s.shrink = 1.;
                }),
                0,
            ),
        ),
        (
            TITLE,
            Some(ROOT),
            node(
                UiKind::Label {
                    text: text("Paused", 30., 36., TextWrap::None),
                },
                style(|_| {}),
                1,
            ),
        ),
        (
            BODY,
            Some(ROOT),
            node(
                UiKind::Label {
                    text: text(
                        "Progress is saved at every checkpoint. يتم حفظ التقدم تلقائيا عند كل نقطة. 日本語 中文",
                        16.,
                        23.,
                        TextWrap::Word,
                    ),
                },
                style(full_width(520.)),
                2,
            ),
        ),
        (
            ACTIONS,
            Some(ROOT),
            node(
                UiKind::Container,
                style(|s| {
                    full_width(560.)(s);
                    s.direction = UiDirection::Row;
                    s.wrap = true;
                    s.gap = [10., 10.];
                    s.align = UiAlign::Start;
                }),
                3,
            ),
        ),
        button(RESUME, "Resume", 1, false),
        button(OPTIONS, "Options", 2, false),
        button(ONLINE, "Online play", 3, true),
        button(CONTINUE, "متابعة", 4, false),
        (
            FIELDS,
            Some(ROOT),
            node(
                UiKind::Container,
                style(|s| {
                    full_width(360.)(s);
                    s.gap = [0., 12.];
                }),
                4,
            ),
        ),
    ];
    nodes.extend(group(0x2A, "Player name", "Ada Lovelace", 1));
    nodes.extend(group(0x2C, "الاسم", "سلام عليكم", 2));
    nodes.extend(group(0x2E, "日本語", "日本", 3));
    nodes
}

// ------------------------------------------------------------------ target provider

/// Helper-owned offscreen output. No window, surface, capture or injection.
pub struct Probe {
    texture: wgpu::Texture,
    size: [u32; 2],
    viewport: Option<Viewport>,
    pub presents: AtomicU64,
}
impl Probe {
    pub fn new(renderer: &Renderer, size: [u32; 2], viewport: Option<Viewport>) -> Arc<Self> {
        Arc::new(Self {
            texture: crate::common::target(renderer, size),
            size,
            viewport,
            presents: AtomicU64::new(0),
        })
    }
    pub fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }
}
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct TargetError(&'static str);

pub struct Targets(pub Arc<Probe>);
pub struct Frame {
    probe: Arc<Probe>,
    view: wgpu::TextureView,
}
impl TargetProvider for Targets {
    type Error = TargetError;
    type Frame = Frame;
    fn acquire(&mut self, _renderer: &Renderer) -> std::result::Result<Frame, TargetError> {
        Ok(Frame {
            probe: Arc::clone(&self.0),
            view: self.0.texture.create_view(&Default::default()),
        })
    }
}
impl RenderFrame for Frame {
    type Error = TargetError;
    fn target(&self) -> RetainedTarget<'_> {
        RetainedTarget {
            view: &self.view,
            format: FORMAT,
            width: self.probe.size[0],
            height: self.probe.size[1],
            viewport: self.probe.viewport,
        }
    }
    fn present(self) -> std::result::Result<(), TargetError> {
        self.probe.presents.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

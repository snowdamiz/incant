//! Review scene: handoff 0056's committed game-UI composition (lit slate floor,
//! moving amber crate, teal pillar, key light, multi-script UI panel), rebuilt as
//! an in-code CookedScene from the same generated cubes and packaged test fonts,
//! plus the GPU host game test's world-space text string. Review content, not
//! game art; no product scene, style, filter or shader is changed.
use crate::Result;
use incant_game_runtime::HostedOfflineGameSession;
use incant_input::InputActions;
use incant_render_game::{
    CpuInputObserver, NativeRenderHost, NativeRenderSettings, TextSettings, UiSettings,
};
use incant_render_gpu_host::{GpuRenderConsumer, GpuRenderSettings, TargetProvider};
use incant_runtime::{
    CookedEntity, CookedScene, CookedSpatial, CookedText, CookedUi, FontAssetRef,
    NativeMeshRenderer, NativeText, StableId, TextLayout, TextWrap, VisualAssetKind,
    VisualAssetRef,
};
use incant_script::CompiledScript;
use incant_types::{
    Camera, CameraProjection, DirectionalLight, Transform, UiAlign, UiDirection, UiKind, UiLength,
    UiNode, UiStyle, UiText, Velocity,
};
use std::{path::Path, sync::Arc};

pub type Game<P> = HostedOfflineGameSession<NativeRenderHost<GpuRenderConsumer<P>>>;
pub type CpuGame = HostedOfflineGameSession<NativeRenderHost<CpuInputObserver>>;

pub const CAMERA: u128 = 0x10;
pub const LIGHT: u128 = 0x11;
pub const FLOOR: u128 = 0x12;
pub const CRATE: u128 = 0x13;
pub const PILLAR: u128 = 0x14;
pub const WORLD_TEXT: u128 = 0x15;
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
const MESHES: [(u128, &str, [f64; 4]); 3] = [
    (0x90, "slate", [0.15, 0.17, 0.20, 1.]),
    (0x91, "amber", [0.80, 0.52, 0.20, 1.]),
    (0x92, "teal", [0.18, 0.46, 0.50, 1.]),
];

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Projection {
    Perspective,
    Orthographic,
    PixelPerfect,
}
impl Projection {
    pub fn name(self) -> &'static str {
        match self {
            Self::Perspective => "perspective",
            Self::Orthographic => "orthographic",
            Self::PixelPerfect => "pixel-perfect",
        }
    }
    pub fn parse(text: &str) -> Option<Self> {
        [Self::Perspective, Self::Orthographic, Self::PixelPerfect]
            .into_iter()
            .find(|p| p.name() == text)
    }
    fn camera(self) -> CameraProjection {
        match self {
            Self::Perspective => CameraProjection::Perspective {},
            Self::Orthographic => CameraProjection::Orthographic { vertical_size: 6. },
            Self::PixelPerfect => CameraProjection::PixelPerfect {
                pixels_per_unit: 32.,
                reference_resolution: [240, 135],
            },
        }
    }
}

/// Cooked caches live in `dir` until the fixture is dropped; every game loads
/// them at construction only.
pub struct Fixture {
    dir: tempfile::TempDir,
    bytes: Vec<u8>,
    pub projection: Projection,
    pub ui: bool,
}

/// 0056's style() base: auto size, zero gaps, column, stretch, start, no grow
/// or shrink. Callers override fields as 0056 did.
fn style(edit: impl FnOnce(&mut UiStyle)) -> UiStyle {
    let mut style = UiStyle {
        shrink: 0.,
        ..UiStyle::default()
    };
    edit(&mut style);
    style
}

fn fingerprint(hex: &str) -> Result<[u8; 32]> {
    let mut out = [0; 32];
    for (i, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(hex.get(2 * i..2 * i + 2).ok_or("fingerprint")?, 16)?;
    }
    Ok(out)
}

/// 0056's cube(): 24 vertices with flat normals, 36 u16 indices, one
/// metallic-0/roughness-0.6 base colour material.
fn cube(root: &Path, name: &str, color: [f64; 4]) -> Result<[u8; 32]> {
    let (mut positions, mut normals, mut indices) = (Vec::new(), Vec::new(), Vec::<u16>::new());
    for axis in 0..3 {
        for sign in [-1f32, 1.] {
            let mut normal = [0f32; 3];
            normal[axis] = sign;
            let (u, v) = [(1, 2), (2, 0), (0, 1)][axis];
            let base = positions.len() as u16;
            for (a, b) in [(-1f32, -1f32), (1., -1.), (1., 1.), (-1., 1.)] {
                let mut p = [0f32; 3];
                p[axis] = 0.5 * sign;
                p[u] = 0.5 * a;
                p[v] = 0.5 * b;
                positions.push(p);
                normals.push(normal);
            }
            let quad: [u16; 6] = if sign > 0. {
                [0, 1, 2, 0, 2, 3]
            } else {
                [0, 2, 1, 0, 3, 2]
            };
            indices.extend(quad.map(|i| base + i));
        }
    }
    let mut data = Vec::new();
    for p in positions.iter().chain(&normals) {
        for c in p {
            data.extend(c.to_le_bytes());
        }
    }
    for i in &indices {
        data.extend(i.to_le_bytes());
    }
    let count = positions.len();
    std::fs::write(root.join(format!("{name}.bin")), &data)?;
    let gltf = serde_json::json!({
        "asset": {"version": "2.0"}, "scene": 0, "scenes": [{"nodes": [0]}], "nodes": [{"mesh": 0}],
        "materials": [{"pbrMetallicRoughness": {"baseColorFactor": color,
            "metallicFactor": 0, "roughnessFactor": 0.6}}],
        "meshes": [{"primitives": [{"attributes": {"POSITION": 0, "NORMAL": 1}, "indices": 2, "material": 0}]}],
        "buffers": [{"byteLength": data.len(), "uri": format!("{name}.bin")}],
        "bufferViews": [{"buffer": 0, "byteOffset": 0, "byteLength": count * 12},
            {"buffer": 0, "byteOffset": count * 12, "byteLength": count * 12},
            {"buffer": 0, "byteOffset": count * 24, "byteLength": indices.len() * 2}],
        "accessors": [{"bufferView": 0, "componentType": 5126, "count": count, "type": "VEC3",
                "min": [-0.5, -0.5, -0.5], "max": [0.5, 0.5, 0.5]},
            {"bufferView": 1, "componentType": 5126, "count": count, "type": "VEC3"},
            {"bufferView": 2, "componentType": 5123, "count": indices.len(), "type": "SCALAR"}]
    });
    std::fs::write(
        root.join(format!("{name}.gltf")),
        serde_json::to_vec(&gltf)?,
    )?;
    let cooked = incant_assets::cook_gltf(
        root,
        Path::new(&format!("{name}.gltf")),
        &root.join(".incant/cache/models"),
    )?;
    fingerprint(&cooked.metadata.fingerprint)
}

fn transform(at: [f64; 3], rotation: [f64; 4], scale: [f64; 3]) -> Transform {
    Transform {
        translation: at,
        rotation,
        scale,
    }
}

impl Fixture {
    pub fn new(projection: Projection, ui: bool) -> Result<Self> {
        let dir = tempfile::tempdir()?;
        let root = dir.path();
        let fonts_source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../../crates/incant_text/tests/fonts");
        let mut fonts = Vec::new();
        for (n, name) in FONTS {
            std::fs::copy(fonts_source.join(name), root.join(name))?;
            let font = incant_assets::cook_font(root, Path::new(name), &root.join("fonts"))?;
            fonts.push(FontAssetRef {
                id: crate::fixture::id(n),
                fingerprint: fingerprint(&font.metadata().fingerprint)?,
            });
        }
        let font_ids: Vec<StableId> = fonts.iter().map(|f| f.id).collect();
        let mut visuals = Vec::new();
        for (n, name, color) in MESHES {
            visuals.push(VisualAssetRef {
                id: id(n),
                kind: VisualAssetKind::Model,
                fingerprint: cube(root, name, color)?,
            });
        }
        // Key light from the upper left, a little behind the camera (0056).
        let half = |degrees: f64| (degrees.to_radians() / 2.).sin_cos();
        let ((sy, cy), (sx, cx)) = (half(30.), half(-40.));
        let key = [cy * sx, sy * cx, -sy * sx, cy * cx];
        let (sp, cp) = half(-8.);
        let entity = |n: u128, parent: Option<u128>, transform: Transform| CookedEntity {
            id: id(n),
            parent: parent.map(id),
            transform,
            velocity: None,
        };
        let one = [1.; 3];
        let mut entities = vec![
            entity(
                CAMERA,
                None,
                transform([0., 0.9, 7.], [sp, 0., 0., cp], one),
            ),
            entity(LIGHT, None, transform([0.; 3], key, one)),
            entity(
                FLOOR,
                None,
                transform([0., -1.25, -2.], [0., 0., 0., 1.], [16., 0.1, 12.]),
            ),
            CookedEntity {
                velocity: Some(Velocity {
                    linear: [-0.4, 0., 0.],
                }),
                ..entity(
                    CRATE,
                    None,
                    transform([2.2, -0.6, 0.4], [0., 0., 0., 1.], one),
                )
            },
            entity(
                PILLAR,
                None,
                transform([-2.6, 0.2, -2.5], [0., 0., 0., 1.], [0.9, 3., 0.9]),
            ),
            // The GPU host game test's world text, placed in the open upper right.
            entity(
                WORLD_TEXT,
                None,
                transform([1.2, 1.8, -0.5], [0., 0., 0., 1.], [3.; 3]),
            ),
        ];
        let mut camera = CookedSpatial::empty(id(CAMERA));
        camera.camera = Some(Camera {
            fov_degrees: 45.,
            near: 0.1,
            far: 100.,
            projection: projection.camera(),
        });
        let mut light = CookedSpatial::empty(id(LIGHT));
        light.directional = Some(DirectionalLight {
            color: [1., 0.97, 0.92],
            intensity: 2.4,
            shadows: None,
        });
        let mesh = |n: u128, mesh: u128| {
            let mut spatial = CookedSpatial::empty(id(n));
            spatial.mesh = Some(NativeMeshRenderer {
                mesh: id(mesh),
                cast_shadows: false,
            });
            spatial
        };
        let spatial = vec![
            camera,
            light,
            mesh(FLOOR, 0x90),
            mesh(CRATE, 0x91),
            mesh(PILLAR, 0x92),
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
            projection,
            ui,
        })
    }
    /// A fresh scene decoded from the cooked bytes, as a package player loads it.
    pub fn scene(&self) -> Result<CookedScene> {
        Ok(CookedScene::from_bytes(&self.bytes)?)
    }
    pub fn scene_sha256(&self) -> String {
        incant_assets_runtime::sha256(&self.bytes)
    }
    fn settings<S>(&self, size: [u32; 2], consumer: S) -> NativeRenderSettings<S> {
        let root = self.dir.path();
        NativeRenderSettings {
            ui: self
                .ui
                .then(|| UiSettings::new([size[0] as f32, size[1] as f32])),
            text: Some(TextSettings::new(root.join("fonts"))),
            model_cache: root.join(".incant/cache/models"),
            texture_cache: root.join("textures"),
            camera: id(CAMERA),
            consumer,
        }
    }
    /// The canonical composed game: script, input, audio clock, native render
    /// host and the GPU consumer. `size` is the physical UI viewport.
    pub fn game<P: TargetProvider>(
        &self,
        size: [u32; 2],
        consumer: GpuRenderSettings<P>,
    ) -> Result<Game<P>> {
        Game::new(
            self.scene()?,
            script()?,
            actions()?,
            vec![],
            48000,
            self.settings(size, consumer),
        )
        .map_err(|e| e.to_string().into())
    }
    /// The same composition with the CPU input observer: SceneInput only, no GPU.
    pub fn cpu_game(&self, size: [u32; 2]) -> Result<CpuGame> {
        CpuGame::new(
            self.scene()?,
            script()?,
            actions()?,
            vec![],
            48000,
            self.settings(size, ()),
        )
        .map_err(|e| e.to_string().into())
    }
}

/// Logs routed UI events (as the GPU host UI test guest does) and despawns the
/// pillar once when the `drop` action (Space) is pressed: a topology change
/// that makes the next sync a scene replacement.
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
/// 0056's UI tree and strings, in its order and styles.
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
    // Field IDs 0x2A/0x2C/0x2E, captions one lower, groups 0x32/0x34/0x36.
    nodes.extend(group(0x2A, "Player name", "Ada Lovelace", 1));
    nodes.extend(group(0x2C, "الاسم", "سلام عليكم", 2));
    nodes.extend(group(0x2E, "日本語", "日本", 3));
    nodes
}

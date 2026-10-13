//! Numerical SSAO and bloom cases on the direct typed scene route:
//! cooked glTF -> SceneInput -> Renderer::prepare / prepare_retained ->
//! draw_scene / RetainedScene::submit. Every attempt is charged before it is
//! encoded; every frame is retained as raw RGBA with the geometry and renderer
//! metadata that tools/analyze.py needs for its independent reference.
//! In-run checks are exact invariants (identity, monotonicity, receipts);
//! accessibility values are inferred offline from the retained readbacks.
use crate::common::*;
use crate::gltf::{self, Mesh, Skin};
use glam::{DMat4, DQuat, DVec3};
use incant_assets_runtime::RuntimeAsset;
use incant_render::resolution::{ResolutionConfig, ResolutionScale};
use incant_render::{AmbientOcclusion, Bloom, RetainedScene, RetainedTarget, Viewport, wgpu};
use incant_render_data::{EnvironmentInput, MeshInput, SceneInput, SceneLight, SceneNode};
use incant_types::{AnimationSample, Camera, CameraProjection, DirectionalLight, TextureUsage};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub const SIZE: [u32; 2] = [321, 193];
pub const RECT: [f32; 4] = [13.25, 17.5, 280.5, 160.5];
pub const CANVAS: [u8; 3] = [31, 63, 95];
/// Exactly one radian, as in the CPU reference (ambient_occlusion::tests).
pub const FOV_DEGREES: f64 = 180. / std::f64::consts::PI;
/// Uniform linear environment radiance 179/255: below the tone-map shoulder.
pub const ENVIRONMENT_BYTE: u8 = 179;
/// (centre, scale) of the unit quad; view space equals world space.
pub type Place = ([f64; 3], [f64; 3]);
pub const RECEIVER: Place = ([0., 0., -4.], [16., 12., 1.]);
/// x 0.4..3.0, y -1.5..1.5 at z = -3: the CPU reference's foreground side boundary.
pub const SLAB: Place = ([1.7, 0., -3.], [2.6, 3., 1.]);
/// Same slab two units nearer the eye: three units from the receiver > radius 2.
pub const FAR_SLAB: Place = ([1.7, 0., -1.], [2.6, 3., 1.]);
/// x 2.2..6.0 crosses the right frustum edge (half width 2.86 at z = -3).
pub const EDGE_SLAB: Place = ([4.1, 0., -3.], [3.8, 3., 1.]);
pub const PANE: Place = ([0., 1.25, -2.5], [1.6, 1.5, 1.]);
/// Bloom configuration for LDR emissive numerics (the importer caps emissive at 1).
pub const NUMERIC_BLOOM: Bloom = Bloom {
    threshold: 0.25,
    intensity: 0.5,
    levels: 6,
};

pub fn viewport() -> Viewport {
    Viewport {
        rect: RECT,
        corner_radii: [0.; 4],
        canvas_srgb: CANVAS,
    }
}
/// The same rectangle at an exact binary scale, with square corners and a
/// backdrop-coloured canvas so chrome mixing is inert in internal references.
pub fn scaled_viewport(sixteenths: u32) -> Viewport {
    let factor = sixteenths as f32 / 16.;
    Viewport {
        rect: RECT.map(|v| v * factor),
        corner_radii: [0.; 4],
        canvas_srgb: BACKDROP_SRGB,
    }
}
pub fn internal(output: [u32; 2], sixteenths: u32) -> [u32; 2] {
    output.map(|v| (v * sixteenths).div_ceil(16))
}
/// Identical policy fields with minimum = initial = maximum (handoff 0067).
pub fn fixed(sixteenths: u8) -> Result<ResolutionConfig> {
    let scale = ResolutionScale::new(sixteenths)?;
    Ok(ResolutionConfig {
        target_ms: 16.,
        deadband: 0.125,
        minimum: scale,
        maximum: scale,
        initial: scale,
        overload_samples: 2,
        recovery_samples: 4,
        cooldown_samples: 2,
        slew: 1,
    })
}

/// Cooked fixture assets in a private temporary directory. Dropping it deletes
/// every source and cache file; loaded versions stay valid in memory.
pub struct Assets {
    dir: tempfile::TempDir,
}
impl Assets {
    pub fn new() -> Result<Self> {
        Ok(Self {
            dir: tempfile::tempdir()?,
        })
    }
    fn cache(&self, kind: &str) -> PathBuf {
        self.dir.path().join(".incant/cache").join(kind)
    }
    pub fn model(
        &self,
        name: &str,
        mesh: &Mesh,
        material: Value,
        image: Option<(&[u8], [u32; 2])>,
        skin: Option<&Skin>,
    ) -> Result<Arc<RuntimeAsset>> {
        let root = self.dir.path();
        let image_name = format!("{name}.png");
        if let Some((pixels, size)) = image {
            write_png(&root.join(&image_name), pixels, size)?;
        }
        let (doc, bin) = gltf::document(
            name,
            mesh,
            material,
            image.map(|_| image_name.as_str()),
            skin,
        );
        std::fs::write(root.join(format!("{name}.bin")), bin)?;
        std::fs::write(root.join(format!("{name}.gltf")), serde_json::to_vec(&doc)?)?;
        let cooked = incant_assets::cook_gltf(
            root,
            Path::new(&format!("{name}.gltf")),
            &self.cache("models"),
        )?;
        Ok(Arc::new(RuntimeAsset::load_model(
            name,
            0,
            &self.cache("models"),
            &cooked.metadata.fingerprint,
        )?))
    }
    /// 2:1 equirectangular constant radiance, linear usage: a uniform furnace.
    pub fn environment(&self) -> Result<Arc<RuntimeAsset>> {
        let root = self.dir.path();
        let v = ENVIRONMENT_BYTE;
        write_png(
            &root.join("environment.png"),
            &[v, v, v, 255, v, v, v, 255],
            [2, 1],
        )?;
        let cooked = incant_assets::cook_texture(
            root,
            Path::new("environment.png"),
            &self.cache("textures"),
            TextureUsage::Linear,
        )?;
        Ok(Arc::new(RuntimeAsset::load_texture(
            "environment",
            0,
            &self.cache("textures"),
            &cooked.metadata.fingerprint,
            Some(TextureUsage::Linear),
        )?))
    }
}

pub fn camera(orthographic: bool) -> SceneNode {
    SceneNode {
        id: "camera".into(),
        world: DMat4::IDENTITY,
        camera: Some(Camera {
            fov_degrees: FOV_DEGREES,
            near: 0.1,
            far: 100.,
            projection: if orthographic {
                CameraProjection::Orthographic { vertical_size: 8. }
            } else {
                CameraProjection::Perspective {}
            },
        }),
        light: None,
        mesh: None,
        sprite: None,
        tilemap: None,
        diagnostic: false,
    }
}
pub fn mesh(id: &str, source: &Arc<RuntimeAsset>, world: DMat4) -> SceneNode {
    SceneNode {
        id: id.into(),
        world,
        camera: None,
        light: None,
        mesh: Some(MeshInput {
            source: Arc::clone(source),
            cast_shadows: false,
            animation: None,
            animation_blend: None,
            animation_pose: None,
            animation_ik: None,
            animation_root: None,
        }),
        sprite: None,
        tilemap: None,
        diagnostic: false,
    }
}
/// Directional key travelling along -Z (straight into the receiver).
pub fn key_light(intensity: f64) -> SceneNode {
    SceneNode {
        id: "key".into(),
        world: DMat4::IDENTITY,
        camera: None,
        light: Some(SceneLight::Directional(DirectionalLight {
            color: [1.; 3],
            intensity,
            shadows: None,
        })),
        mesh: None,
        sprite: None,
        tilemap: None,
        diagnostic: false,
    }
}
pub fn place((center, scale): Place) -> DMat4 {
    DMat4::from_translation(DVec3::from_array(center)) * DMat4::from_scale(DVec3::from_array(scale))
}
pub fn input(nodes: Vec<SceneNode>, environment: &Arc<RuntimeAsset>, intensity: f64) -> SceneInput {
    SceneInput {
        nodes,
        text: None,
        environment: Some(EnvironmentInput {
            source: Arc::clone(environment),
            intensity,
            rotation_degrees: 0.,
        }),
    }
}
fn viewport_json(viewport: Option<Viewport>) -> Value {
    viewport.map_or(
        Value::Null,
        |v| json!({"rect": v.rect, "corner_radii": v.corner_radii, "canvas_srgb": v.canvas_srgb}),
    )
}
/// Pixel centre of a view-space point for the default perspective camera.
pub fn project(point: [f64; 3], rect: [f32; 4]) -> [f64; 2] {
    let [x, y, w, h] = rect.map(f64::from);
    let f = 1. / 0.5f64.tan();
    let ndc = [point[0] / -point[2] * f / (w / h), point[1] / -point[2] * f];
    [x + (ndc[0] * 0.5 + 0.5) * w, y + (0.5 - ndc[1] * 0.5) * h]
}

#[derive(Clone, Copy)]
pub struct Shot {
    pub size: [u32; 2],
    pub viewport: Option<Viewport>,
}
pub fn shot() -> Shot {
    Shot {
        size: SIZE,
        viewport: Some(viewport()),
    }
}

/// A renderer variant and its post-setup registry (for the no-per-frame
/// pipeline creation check).
pub struct Device {
    pub name: &'static str,
    pub gpu: Gpu,
    setup: Value,
}
impl Device {
    pub fn new(name: &'static str, gpu: Gpu) -> Self {
        let setup = gpu.registry();
        Self { name, gpu, setup }
    }
}

pub struct Run {
    pub case: &'static str,
    pub frames: Frames,
    pub verdict: Verdict,
    pub store: Store,
}
impl Run {
    pub fn new(case: &'static str, declared: u32, out: &Path) -> Result<Self> {
        Ok(Self {
            case,
            frames: Frames::new(case, declared),
            verdict: Verdict::default(),
            store: Store::create(out)?,
        })
    }
    pub fn check(&mut self, ok: bool, what: &str, detail: Value) {
        self.verdict.check(ok, what, detail);
    }
    /// Ordinary path: prepare, encode, submit immediately, read back, retain.
    pub fn draw(
        &mut self,
        device: &Device,
        label: &str,
        scene: &SceneInput,
        shot: Shot,
        meta: Value,
    ) -> Result<Rgba> {
        self.frames.charge(label)?;
        let renderer = &device.gpu.renderer;
        let prepared = renderer.prepare(scene)?.with_camera("camera")?;
        let texture = target(renderer, shot.size);
        let command = renderer.draw_scene(
            &prepared,
            &texture.create_view(&Default::default()),
            FORMAT,
            shot.size[0],
            shot.size[1],
            shot.viewport,
        )?;
        renderer.queue.submit([command]);
        let frame = read(renderer, &texture)?;
        self.store.keep(
            label,
            &frame,
            json!({"renderer": device.name, "route": "draw_scene", "size": shot.size,
                "viewport": viewport_json(shot.viewport), "scene": meta}),
        )?;
        Ok(frame)
    }
    /// Retained native path; records the actual resolution receipt.
    pub fn submit(
        &mut self,
        device: &Device,
        label: &str,
        retained: &mut RetainedScene,
        scene: &SceneInput,
        shot: Shot,
        meta: Value,
    ) -> Result<(Rgba, Value)> {
        self.frames.charge(label)?;
        let renderer = &device.gpu.renderer;
        let texture = target(renderer, shot.size);
        let view = texture.create_view(&Default::default());
        retained.submit(
            renderer,
            scene,
            RetainedTarget {
                view: &view,
                format: FORMAT,
                width: shot.size[0],
                height: shot.size[1],
                viewport: shot.viewport,
            },
        )?;
        let frame = read(renderer, &texture)?;
        let receipt = retained.resolution_frame().map_or(Value::Null, |f| {
            json!({"sequence": f.sequence(), "scale16": (f.scale().fraction() * 16.).round() as u32,
                "output": f.output_size(), "internal": f.internal_size(),
                "pixel_perfect": f.pixel_perfect()})
        });
        self.store.keep(
            label,
            &frame,
            json!({"renderer": device.name, "route": "retained", "size": shot.size,
                "viewport": viewport_json(shot.viewport), "scene": meta, "resolution": receipt}),
        )?;
        Ok((frame, receipt))
    }
    /// `draw` with the diagnostic R8 stage capture armed for exactly this one
    /// charged frame (the three copies are readbacks, not frames). The planes
    /// must come back at the frame's own size; anything else refuses the case.
    pub fn draw_with_stages(
        &mut self,
        device: &Device,
        label: &str,
        scene: &SceneInput,
        shot: Shot,
        meta: Value,
    ) -> Result<Rgba> {
        device.gpu.renderer.arm_ambient_occlusion_stage_capture()?;
        let frame = self.draw(device, label, scene, shot, meta)?;
        self.keep_stages(device, label, shot.size)?;
        Ok(frame)
    }
    /// `submit` with the stage capture armed (retained route).
    pub fn submit_with_stages(
        &mut self,
        device: &Device,
        label: &str,
        retained: &mut RetainedScene,
        scene: &SceneInput,
        shot: Shot,
        meta: Value,
    ) -> Result<(Rgba, Value)> {
        device.gpu.renderer.arm_ambient_occlusion_stage_capture()?;
        let result = self.submit(device, label, retained, scene, shot, meta)?;
        self.keep_stages(device, label, shot.size)?;
        Ok(result)
    }
    fn keep_stages(&mut self, device: &Device, label: &str, size: [u32; 2]) -> Result<()> {
        let stages = device.gpu.renderer.take_ambient_occlusion_stages()?;
        if stages.size != size {
            return Err(format!("{label}: stage planes are {:?}, the frame is {size:?}", stages.size).into());
        }
        self.store.keep_stages(
            label,
            stages.size,
            [&stages.raw[..], &stages.horizontal[..], &stages.vertical[..]],
        )
    }
    /// Encode without submitting (for out-of-order queue checks).
    pub fn encode(
        &mut self,
        device: &Device,
        label: &str,
        prepared: &incant_render::RenderScene,
        shot: Shot,
    ) -> Result<(wgpu::Texture, wgpu::CommandBuffer)> {
        self.frames.charge(label)?;
        let renderer = &device.gpu.renderer;
        let texture = target(renderer, shot.size);
        let command = renderer.draw_scene(
            prepared,
            &texture.create_view(&Default::default()),
            FORMAT,
            shot.size[0],
            shot.size[1],
            shot.viewport,
        )?;
        Ok((texture, command))
    }
}

/// Close a case: validation scopes, registry stability, receipts, index.
fn finish(
    run: Run,
    devices: &[&Device],
    scope_errors: Vec<Option<String>>,
    body: Result<()>,
) -> Result<bool> {
    let Run {
        case,
        frames,
        mut verdict,
        store,
    } = run;
    verdict.check(
        scope_errors.iter().all(Option::is_none),
        "no wgpu validation error in any renderer's case scope",
        json!(scope_errors),
    );
    let mut summary = Vec::new();
    for device in devices {
        let now = device.gpu.registry();
        let stable = now["render_pipelines"] == device.setup["render_pipelines"]
            && now["compute_pipelines"] == device.setup["compute_pipelines"];
        verdict.check(
            stable,
            &format!("{}: no pipeline created after setup", device.name),
            json!({"setup": device.setup, "end": now}),
        );
        summary.push(
            json!({"renderer": device.name, "adapter": device.gpu.adapter(),
            "setup_registry": device.setup, "end_registry": now}),
        );
    }
    if let Err(error) = &body {
        verdict.check(
            false,
            "case body ran to completion",
            json!(error.to_string()),
        );
    }
    frames.receipt();
    emit(case, "devices", json!(summary));
    store.finish(json!({"case": case, "frames": frames.used(), "devices": summary}))?;
    Ok(verdict.finish(case))
}

/// Push one validation scope per renderer, run the body, then always pop.
macro_rules! scoped {
    ([$($device:expr),+], $body:expr) => {{
        let scopes = [$($device.gpu.renderer.device.push_error_scope(wgpu::ErrorFilter::Validation)),+];
        let body: Result<()> = $body;
        let errors: Vec<Option<String>> = scopes
            .into_iter()
            .map(|scope| pollster::block_on(scope.pop()).map(|e| e.to_string()))
            .collect();
        (body, errors)
    }};
}

pub fn run_case(name: &str, out: &Path) -> Result<bool> {
    match name {
        "ssao_core" => ssao_core(out),
        "ssao_materials" => ssao_materials(out),
        "ssao_geometry" => ssao_geometry(out),
        "lifecycle_drs" => lifecycle_drs(out),
        "ssao_quality" => ssao_quality(out),
        // Kept from build004 for an unchanged diff; the filter plan does not run it.
        "bloom_numeric" => bloom_numeric(out),
        _ => Err(format!("unknown case {name}; the filter plan names five").into()),
    }
}

// ------------------------------------------------------------------ ssao_quality

/// Isolated sphere: gltf::sphere(24, 48) (radius 0.5) scaled 1.2 at view
/// (0, 0, -3), gray 0.5 so it stays distinguishable from any background
/// (a white dielectric under the uniform furnace matches the environment).
pub const SPHERE_CENTRE: [f64; 3] = [0., 0., -3.];
pub const SPHERE_SCALE: f64 = 1.2;
/// N8 stripe i: x in [-0.5 + 0.3 i, -0.4 + 0.3 i), y in [-0.5, 1.5), z = -3.
pub fn stripe(i: u32) -> Place {
    ([-0.45 + 0.3 * f64::from(i), 0.5, -3.], [0.1, 2., 1.])
}
fn gray() -> Value {
    gltf::pbr([0.5, 0.5, 0.5, 1.], 0., 0.5)
}
/// Analytic silhouette area (texels) of the radius-0.6 sphere in this viewport.
fn sphere_area(orthographic: bool) -> f64 {
    let radius = 0.5 * SPHERE_SCALE;
    let half_height = f64::from(RECT[3]) / 2.;
    let pixels = if orthographic {
        radius * half_height / 4.
    } else {
        let distance = SPHERE_CENTRE.iter().map(|v| v * v).sum::<f64>().sqrt();
        (radius / distance).asin().tan() * half_height / 0.5f64.tan()
    };
    std::f64::consts::PI * pixels * pixels
}
/// Texels wholly inside the viewport whose colour differs from the background
/// texel (16, 20). The fractional viewport's partly covered rim (columns 13
/// and 293, row 17) blends with the canvas and is never sphere coverage.
fn sphere_texels(frame: &Rgba) -> usize {
    let background = frame.px(16, 20);
    let [x0, y0, w, h] = RECT.map(f64::from);
    (0..frame.size[1])
        .flat_map(|y| (0..frame.size[0]).map(move |x| (x, y)))
        .filter(|&(x, y)| {
            let (fx, fy) = (f64::from(x), f64::from(y));
            fx >= x0
                && fy >= y0
                && fx + 1. <= x0 + w
                && fy + 1. <= y0 + h
                && frame.px(x, y) != background
        })
        .count()
}
/// The isolated sphere's coverage precondition, unchanged: within 10% of its
/// analytic silhouette area.
const SPHERE_AREA_TOLERANCE: f64 = 0.1;
fn sphere_covers(covered: usize, expected: f64) -> bool {
    (covered as f64 - expected).abs() <= SPHERE_AREA_TOLERANCE * expected
}

fn ssao_quality(out: &Path) -> Result<bool> {
    let off = Device::new("off", Gpu::new()?);
    let ao = Device::new("ao", Gpu::new()?.with_ao(AO)?);
    let mut run = Run::new("ssao_quality", 10, out)?;
    let (body, errors) = scoped!([off, ao], quality_body(&mut run, &off, &ao));
    finish(run, &[&off, &ao], errors, body)
}

fn quality_body(run: &mut Run, off: &Device, ao: &Device) -> Result<()> {
    let assets = Assets::new()?;
    let env = assets.environment()?;
    let quad = gltf::unit_quad(1., 1., false);
    let white_q = assets.model("quad_white", &quad, white(), None, None)?;
    let black_q = assets.model("quad_black", &quad, black(), None, None)?;
    let sphere = assets.model("sphere_24x48", &gltf::sphere(24, 48), gray(), None, None)?;
    let s = shot();
    let scale = [SPHERE_SCALE; 3];
    for orthographic in [false, true] {
        let (tag, proj) = if orthographic {
            ("o", "orthographic")
        } else {
            ("p", "perspective")
        };
        let world = DMat4::from_translation(DVec3::from_array(SPHERE_CENTRE))
            * DMat4::from_scale(DVec3::splat(SPHERE_SCALE));
        let lone = input(
            vec![camera(orthographic), key_light(0.), mesh("sphere", &sphere, world)],
            &env,
            1.,
        );
        let parts = json!([{"id": "sphere", "mesh": "gltf::sphere(24, 48)", "material": "gray",
            "center": SPHERE_CENTRE, "scale": scale}]);
        let off_sphere = run.draw(
            off,
            &format!("{tag}_off_sphere"),
            &lone,
            s,
            ibl_meta("isolated_sphere", proj, parts.clone(), 1.),
        )?;
        let ao_sphere = run.draw(
            ao,
            &format!("{tag}_ao_sphere"),
            &lone,
            s,
            ibl_meta("isolated_sphere", proj, parts, 1.),
        )?;
        check_pair(
            run,
            &format!("{proj}: isolated 24x48 sphere unchanged by AO"),
            &ao_sphere,
            &off_sphere,
        );
        let (covered, expected) = (sphere_texels(&off_sphere), sphere_area(orthographic));
        run.check(
            sphere_covers(covered, expected),
            &format!("{proj}: the sphere covers its analytic silhouette area within 10%"),
            json!({"texels": covered, "expected": expected}),
        );
        let stripes = |q: &Arc<RuntimeAsset>, material: &str| {
            let mut nodes = vec![
                camera(orthographic),
                key_light(0.),
                mesh("receiver", q, place(RECEIVER)),
            ];
            let mut described = vec![part("receiver", material, RECEIVER)];
            for i in 0..4 {
                nodes.push(mesh(&format!("stripe{i}"), q, place(stripe(i))));
                described.push(part(&format!("stripe{i}"), material, stripe(i)));
            }
            (input(nodes, &env, 1.), json!(described))
        };
        let (white_scene, wp) = stripes(&white_q, "white");
        let (black_scene, bp) = stripes(&black_q, "black");
        let off_white = run.draw(
            off,
            &format!("{tag}_off_stripes"),
            &white_scene,
            s,
            ibl_meta("stripes", proj, wp.clone(), 1.),
        )?;
        let ao_white = run.draw_with_stages(
            ao,
            &format!("{tag}_ao_stripes"),
            &white_scene,
            s,
            ibl_meta("stripes", proj, wp, 1.),
        )?;
        run.draw(
            off,
            &format!("{tag}_off_stripes_black"),
            &black_scene,
            s,
            ibl_meta("stripes_black", proj, bp, 1.),
        )?;
        check_occludes(
            run,
            &format!("{proj}: stripes darken the receiver only"),
            &ao_white,
            &off_white,
            200,
        );
    }
    Ok(())
}

fn meta(scene: &str, projection: &str, parts: Value, environment: f64) -> Value {
    json!({"name": scene, "projection": projection, "parts": parts,
        "environment": {"linear_byte": ENVIRONMENT_BYTE, "intensity": environment}})
}
// No authored light means the renderer deliberately supplies its preview key.
// The numeric IBL fixtures need an explicit zero light, not an empty light set.
fn ibl_meta(scene: &str, projection: &str, parts: Value, environment: f64) -> Value {
    let mut value = meta(scene, projection, parts, environment);
    value["key_light"] = json!(0.);
    value
}
fn part(id: &str, material: &str, at: Place) -> Value {
    json!({"id": id, "material": material, "center": at.0, "scale": at.1})
}
fn white() -> Value {
    gltf::pbr([1., 1., 1., 1.], 0., 0.5)
}
fn black() -> Value {
    gltf::pbr([0., 0., 0., 1.], 0., 0.5)
}
fn blend(base: [f32; 4]) -> Value {
    let mut material = gltf::pbr(base, 0., 0.5);
    material["alphaMode"] = json!("BLEND");
    material
}
fn mask(alpha: f32) -> Value {
    let mut material = gltf::pbr([1., 1., 1., alpha], 0., 0.5);
    material["alphaMode"] = json!("MASK");
    material["alphaCutoff"] = json!(0.5);
    material
}
fn check_pair(run: &mut Run, what: &str, a: &Rgba, b: &Rgba) {
    run.check(identical(a, b), what, same(a, b));
}
fn check_occludes(run: &mut Run, what: &str, on: &Rgba, off: &Rgba, minimum: usize) {
    let (darker, brighter) = darker_brighter(on, off);
    run.check(
        darker >= minimum && brighter == 0,
        what,
        json!({"darker_pixels": darker, "brighter_pixels": brighter, "minimum": minimum}),
    );
}

// ------------------------------------------------------------------ ssao_core

fn ssao_core(out: &Path) -> Result<bool> {
    let off = Device::new("off", Gpu::new()?);
    let ao = Device::new("ao", Gpu::new()?.with_ao(AO)?);
    let radius = Device::new(
        "ao_radius_0_5",
        Gpu::new()?.with_ao(AmbientOcclusion { radius: 0.5, ..AO })?,
    );
    let strength = Device::new(
        "ao_strength_0_5",
        Gpu::new()?.with_ao(AmbientOcclusion {
            strength: 0.5,
            ..AO
        })?,
    );
    let mut run = Run::new("ssao_core", 20, out)?;
    let (body, errors) = scoped!(
        [off, ao, radius, strength],
        core_body(&mut run, &off, &ao, &radius, &strength)
    );
    finish(run, &[&off, &ao, &radius, &strength], errors, body)
}

fn core_body(
    run: &mut Run,
    off: &Device,
    ao: &Device,
    radius: &Device,
    strength: &Device,
) -> Result<()> {
    let assets = Assets::new()?;
    let env = assets.environment()?;
    let quad = gltf::unit_quad(1., 1., false);
    let white_q = assets.model("quad_white", &quad, white(), None, None)?;
    let black_q = assets.model("quad_black", &quad, black(), None, None)?;
    // Front face toward the eye, authored normal away from it: sign control.
    let reversed_q = assets.model(
        "quad_reversed_normal",
        &gltf::unit_quad(1., -1., false),
        white(),
        None,
        None,
    )?;
    let scene = |orthographic: bool, parts: &[(&str, &Arc<RuntimeAsset>, &str, Place)]| {
        let mut nodes = vec![camera(orthographic), key_light(0.)];
        let mut described = Vec::new();
        for (id, source, material, at) in parts {
            nodes.push(mesh(id, source, place(*at)));
            described.push(part(id, material, *at));
        }
        let projection = if orthographic {
            "orthographic"
        } else {
            "perspective"
        };
        (input(nodes, &env, 1.), json!(described), projection)
    };
    let s = shot();
    let mut pairs = Vec::new();
    for orthographic in [false, true] {
        let tag = if orthographic { "o" } else { "p" };
        let (white_slab, wp, proj) = scene(
            orthographic,
            &[
                ("receiver", &white_q, "white", RECEIVER),
                ("slab", &white_q, "white", SLAB),
            ],
        );
        let (black_slab, bp, _) = scene(
            orthographic,
            &[
                ("receiver", &black_q, "black", RECEIVER),
                ("slab", &black_q, "black", SLAB),
            ],
        );
        let (flat, fp, _) = scene(orthographic, &[("receiver", &white_q, "white", RECEIVER)]);
        let off_white = run.draw(
            off,
            &format!("{tag}_off_white"),
            &white_slab,
            s,
            ibl_meta("white_slab", proj, wp.clone(), 1.),
        )?;
        let ao_white = run.draw_with_stages(
            ao,
            &format!("{tag}_ao_white"),
            &white_slab,
            s,
            ibl_meta("white_slab", proj, wp.clone(), 1.),
        )?;
        let off_black = run.draw(
            off,
            &format!("{tag}_off_black"),
            &black_slab,
            s,
            ibl_meta("black_slab", proj, bp.clone(), 1.),
        )?;
        let ao_black = run.draw(
            ao,
            &format!("{tag}_ao_black"),
            &black_slab,
            s,
            ibl_meta("black_slab", proj, bp, 1.),
        )?;
        let off_flat = run.draw(
            off,
            &format!("{tag}_off_flat"),
            &flat,
            s,
            ibl_meta("flat", proj, fp.clone(), 1.),
        )?;
        let ao_flat = run.draw(
            ao,
            &format!("{tag}_ao_flat"),
            &flat,
            s,
            ibl_meta("flat", proj, fp, 1.),
        )?;
        check_occludes(
            run,
            &format!("{proj}: real diffuse occlusion darkens only, near the slab"),
            &ao_white,
            &off_white,
            200,
        );
        check_pair(
            run,
            &format!("{proj}: specular-only (diffuse factor 0) unchanged by AO"),
            &ao_black,
            &off_black,
        );
        check_pair(
            run,
            &format!("{proj}: isolated flat control has accessibility 1"),
            &ao_flat,
            &off_flat,
        );
        let chrome = chrome_identical(&ao_white, &off_white, RECT, [0.; 4]);
        run.check(
            chrome["differing"] == 0,
            &format!("{proj}: chrome/canvas outside the viewport unchanged"),
            chrome,
        );
        pairs.push((off_white, ao_white, wp));
    }
    let (off_white, ao_white, wp) = pairs.swap_remove(0);
    let (reversed, rp, proj) = scene(
        false,
        &[
            ("receiver", &reversed_q, "white_reversed_normal", RECEIVER),
            ("slab", &white_q, "white", SLAB),
        ],
    );
    let off_rev = run.draw(
        off,
        "p_off_reversed",
        &reversed,
        s,
        ibl_meta("reversed_normal", proj, rp.clone(), 1.),
    )?;
    let ao_rev = run.draw(
        ao,
        "p_ao_reversed",
        &reversed,
        s,
        ibl_meta("reversed_normal", proj, rp, 1.),
    )?;
    check_pair(
        run,
        "normal sign: a receiver facing away from the slab is not occluded",
        &ao_rev,
        &off_rev,
    );
    let (far, farp, _) = scene(
        false,
        &[
            ("receiver", &white_q, "white", RECEIVER),
            ("slab", &white_q, "white", FAR_SLAB),
        ],
    );
    let off_far = run.draw(
        off,
        "p_off_far",
        &far,
        s,
        ibl_meta("far_slab", proj, farp.clone(), 1.),
    )?;
    let ao_far = run.draw(
        ao,
        "p_ao_far",
        &far,
        s,
        ibl_meta("far_slab", proj, farp, 1.),
    )?;
    check_pair(
        run,
        "far control: a slab beyond the radius does not occlude",
        &ao_far,
        &off_far,
    );
    let (white_slab, _, _) = scene(
        false,
        &[
            ("receiver", &white_q, "white", RECEIVER),
            ("slab", &white_q, "white", SLAB),
        ],
    );
    let small = run.draw(
        radius,
        "p_ao_radius_0_5",
        &white_slab,
        s,
        ibl_meta("white_slab", proj, wp.clone(), 1.),
    )?;
    check_pair(
        run,
        "radius control: radius 0.5 < slab distance 1 leaves accessibility 1",
        &small,
        &off_white,
    );
    let half = run.draw_with_stages(
        strength,
        "p_ao_strength_0_5",
        &white_slab,
        s,
        ibl_meta("white_slab", proj, wp, 1.),
    )?;
    let between = half
        .bytes
        .iter()
        .zip(&ao_white.bytes)
        .zip(&off_white.bytes)
        .all(|((h, a), o)| a <= h && h <= o);
    let (darker, _) = darker_brighter(&half, &off_white);
    run.check(
        between && darker > 0,
        "strength 0.5 lies between strength 1 and off, and still occludes",
        json!({"between": between, "darker_pixels": darker}),
    );
    let (edge, ep, _) = scene(
        false,
        &[
            ("receiver", &white_q, "white", RECEIVER),
            ("slab", &white_q, "white", EDGE_SLAB),
        ],
    );
    let off_edge = run.draw(
        off,
        "p_off_edge",
        &edge,
        s,
        ibl_meta("edge_slab", proj, ep.clone(), 1.),
    )?;
    let ao_edge = run.draw_with_stages(
        ao,
        "p_ao_edge",
        &edge,
        s,
        ibl_meta("edge_slab", proj, ep, 1.),
    )?;
    check_occludes(
        run,
        "slab crossing the frustum edge still occludes on-screen receivers",
        &ao_edge,
        &off_edge,
        50,
    );
    Ok(())
}

// ------------------------------------------------------------------ ssao_materials

fn ssao_materials(out: &Path) -> Result<bool> {
    let off = Device::new("off", Gpu::new()?);
    let ao = Device::new("ao", Gpu::new()?.with_ao(AO)?);
    let mut run = Run::new("ssao_materials", 10, out)?;
    let (body, errors) = scoped!([off, ao], materials_body(&mut run, &off, &ao));
    finish(run, &[&off, &ao], errors, body)
}

fn materials_body(run: &mut Run, off: &Device, ao: &Device) -> Result<()> {
    let assets = Assets::new()?;
    let env = assets.environment()?;
    let quad = gltf::unit_quad(1., 1., false);
    let white_q = assets.model("quad_white", &quad, white(), None, None)?;
    let emissive_q = assets.model(
        "quad_emissive",
        &quad,
        gltf::emissive([0.3, 0.2, 0.1], 1., "OPAQUE"),
        None,
        None,
    )?;
    let blend_q = assets.model("quad_blend", &quad, blend([1., 1., 1., 0.6]), None, None)?;
    let pane_q = assets.model("quad_pane", &quad, blend([0.2, 0.2, 0.2, 0.5]), None, None)?;
    let s = shot();
    let build = |light: bool, intensity: f64, parts: &[(&str, &Arc<RuntimeAsset>, &str, Place)]| {
        let mut nodes = vec![camera(false)];
        nodes.push(key_light(if light { 2. } else { 0. }));
        let mut described = Vec::new();
        for (id, source, material, at) in parts {
            nodes.push(mesh(id, source, place(*at)));
            described.push(part(id, material, *at));
        }
        (input(nodes, &env, intensity), json!(described))
    };
    let cases = [
        (
            "direct_only",
            true,
            0.,
            vec![
                ("receiver", &white_q, "white", RECEIVER),
                ("slab", &white_q, "white", SLAB),
            ],
            "direct-only (environment intensity 0) is not an AO receiver",
        ),
        (
            "emissive_only",
            false,
            0.,
            vec![
                ("receiver", &emissive_q, "emissive_0.3_0.2_0.1", RECEIVER),
                ("slab", &emissive_q, "emissive_0.3_0.2_0.1", SLAB),
            ],
            "emissive-only is not multiplied by accessibility",
        ),
        (
            "blend_only",
            false,
            1.,
            vec![
                ("receiver", &blend_q, "blend_white_0.6", RECEIVER),
                ("slab", &blend_q, "blend_white_0.6", SLAB),
            ],
            "BLEND-only geometry is neither AO receiver nor occluder",
        ),
        (
            "blend_occluder",
            false,
            1.,
            vec![
                ("receiver", &white_q, "white", RECEIVER),
                ("slab", &blend_q, "blend_white_0.6", SLAB),
            ],
            "a BLEND slab does not occlude an opaque receiver",
        ),
    ];
    for (name, light, intensity, parts, what) in cases {
        let (scene, described) = build(light, intensity, &parts);
        let mut m = meta(name, "perspective", described, intensity);
        m["key_light"] = json!(if light { 2. } else { 0. });
        let a = run.draw(off, &format!("m_off_{name}"), &scene, s, m.clone())?;
        let b = run.draw(ao, &format!("m_ao_{name}"), &scene, s, m)?;
        check_pair(run, what, &b, &a);
    }
    let (mixed, described) = build(
        false,
        1.,
        &[
            ("receiver", &white_q, "white", RECEIVER),
            ("slab", &white_q, "white", SLAB),
            ("pane", &pane_q, "blend_gray_0.2_alpha_0.5", PANE),
        ],
    );
    let m = ibl_meta("mixed_opaque_then_blend", "perspective", described, 1.);
    let a = run.draw(off, "m_off_mixed", &mixed, s, m.clone())?;
    let b = run.draw(ao, "m_ao_mixed", &mixed, s, m)?;
    check_occludes(
        run,
        "opaque/MASK then BLEND in one pass: AO still darkens only (pipeline/bind-group switch under the validation scope)",
        &b,
        &a,
        200,
    );
    Ok(())
}

// ------------------------------------------------------------------ ssao_geometry

fn ssao_geometry(out: &Path) -> Result<bool> {
    let off = Device::new("off", Gpu::new()?);
    let ao = Device::new("ao", Gpu::new()?.with_ao(AO)?);
    let mut run = Run::new("ssao_geometry", 17, out)?;
    let (body, errors) = scoped!([off, ao], geometry_body(&mut run, &off, &ao));
    finish(run, &[&off, &ao], errors, body)
}

/// Local plane x + z = 0 (normal (1,0,1)/sqrt 2); after scale (-3,2,1) and a
/// yaw of atan(1/3) it lies flat at z = -4 facing the eye. A wrong normal
/// space (world matrix instead of inverse transpose) tilts it by ~53 degrees.
fn tilted() -> (Mesh, DMat4) {
    let corners = [
        [-0.5, -0.5, 0.5],
        [0.5, -0.5, -0.5],
        [0.5, 0.5, -0.5],
        [-0.5, 0.5, 0.5],
    ];
    let n = std::f32::consts::FRAC_1_SQRT_2;
    let mesh = gltf::quad(corners, [1., 0., 1.], [n, 0., n], false);
    let world = DMat4::from_translation(DVec3::new(0., 0., -4.))
        * DMat4::from_quat(DQuat::from_rotation_y((1f64 / 3.).atan()))
        * DMat4::from_scale(DVec3::new(-3., 2., 1.));
    (mesh, world)
}

fn geometry_body(run: &mut Run, off: &Device, ao: &Device) -> Result<()> {
    let assets = Assets::new()?;
    let env = assets.environment()?;
    let quad = gltf::unit_quad(1., 1., false);
    let white_q = assets.model("quad_white", &quad, white(), None, None)?;
    let black_q = assets.model("quad_black", &quad, black(), None, None)?;
    let equal_q = assets.model("quad_mask_equal", &quad, mask(0.5), None, None)?;
    let below_q = assets.model("quad_mask_below", &quad, mask(0.49), None, None)?;
    // Left texel transparent, right opaque; nearest sampling, u grows with x.
    let cutout: &[u8] = &[255, 255, 255, 0, 255, 255, 255, 255];
    let cutout_q = assets.model(
        "quad_mask_cutout",
        &gltf::unit_quad(1., 1., true),
        mask(1.),
        Some((cutout, [2, 1])),
        None,
    )?;
    let mut double = white();
    double["doubleSided"] = json!(true);
    let back_q = assets.model(
        "quad_back_double_sided",
        &gltf::unit_quad(-1., -1., false),
        double,
        None,
        None,
    )?;
    let (tilted_mesh, tilted_world) = tilted();
    let tilted_q = assets.model("plane_tilted", &tilted_mesh, white(), None, None)?;
    let slab_corners = ([0.4f32, 3.], [-1.5f32, 1.5]);
    let skin_q = assets.model(
        "slab_skinned",
        &gltf::rect_z(slab_corners.0, slab_corners.1, -3.),
        white(),
        None,
        Some(&Skin {
            from: [0., 0., 2.],
            to: [0., 0., 0.],
        }),
    )?;
    let near_q = assets.model(
        "slab_baked_near",
        &gltf::rect_z(slab_corners.0, slab_corners.1, -3.),
        white(),
        None,
        None,
    )?;
    let far_q = assets.model(
        "slab_baked_far",
        &gltf::rect_z(slab_corners.0, slab_corners.1, -1.),
        white(),
        None,
        None,
    )?;
    let s = shot();
    let build = |parts: Vec<(&str, &Arc<RuntimeAsset>, &str, DMat4, Value)>| {
        let mut nodes = vec![camera(false)];
        let mut described = Vec::new();
        for (id, source, material, world, geometry) in parts {
            nodes.push(mesh(id, source, world));
            described.push(json!({"id": id, "material": material, "geometry": geometry}));
        }
        // Preserve the existing camera/mesh order used by retained mutations.
        nodes.push(key_light(0.));
        (input(nodes, &env, 1.), json!(described))
    };
    let at = |p: Place| json!({"center": p.0, "scale": p.1});
    let receiver = || ("receiver", &white_q, "white", place(RECEIVER), at(RECEIVER));
    let slab_with = |source, material| ("slab", source, material, place(SLAB), at(SLAB));

    let (base, d) = build(vec![receiver(), slab_with(&white_q, "white")]);
    let m = ibl_meta("white_slab", "perspective", d, 1.);
    let off_white = run.draw(off, "g_off_white", &base, s, m.clone())?;
    let ao_white = run.draw_with_stages(ao, "g_ao_white", &base, s, m)?;
    let (black_scene, d) = build(vec![
        ("receiver", &black_q, "black", place(RECEIVER), at(RECEIVER)),
        slab_with(&black_q, "black"),
    ]);
    run.draw(
        off,
        "g_off_black",
        &black_scene,
        s,
        ibl_meta("black_slab", "perspective", d, 1.),
    )?;
    check_occludes(run, "base occlusion reproduced", &ao_white, &off_white, 200);

    let (equal, d) = build(vec![
        receiver(),
        slab_with(&equal_q, "mask_alpha_0.5_cutoff_0.5"),
    ]);
    let ao_equal = run.draw_with_stages(
        ao,
        "g_ao_mask_equal",
        &equal,
        s,
        ibl_meta("mask_equal", "perspective", d, 1.),
    )?;
    check_pair(
        run,
        "MASK alpha equal to the cutoff is kept: same occlusion and colour as opaque",
        &ao_equal,
        &ao_white,
    );
    let (below, d) = build(vec![
        receiver(),
        slab_with(&below_q, "mask_alpha_0.49_cutoff_0.5"),
    ]);
    let m = ibl_meta("mask_below", "perspective", d, 1.);
    let off_below = run.draw(off, "g_off_mask_below", &below, s, m.clone())?;
    let ao_below = run.draw(ao, "g_ao_mask_below", &below, s, m)?;
    check_pair(
        run,
        "MASK alpha below the cutoff is discarded in both passes",
        &ao_below,
        &off_below,
    );
    let (cutout, d) = build(vec![
        receiver(),
        slab_with(&cutout_q, "mask_cutout_left_half"),
    ]);
    let ao_cutout = run.draw_with_stages(
        ao,
        "g_ao_mask_cutout",
        &cutout,
        s,
        ibl_meta("mask_cutout", "perspective", d, 1.),
    )?;
    // Exact cut-edge accessibility is checked offline against the reference.
    let (cut_dark, cut_bright) = darker_brighter(&ao_cutout, &off_white);
    let (full_dark, _) = darker_brighter(&ao_white, &off_white);
    run.check(
        cut_dark > 0 && cut_bright == 0 && !identical(&ao_cutout, &ao_white),
        "MASK cutout: the kept half still occludes and the frame differs from the full slab",
        json!({"cutout_darker": cut_dark, "cutout_brighter": cut_bright, "full_darker": full_dark}),
    );

    let (back, d) = build(vec![
        (
            "receiver",
            &back_q,
            "white_double_sided_back_face",
            place(RECEIVER),
            at(RECEIVER),
        ),
        slab_with(&white_q, "white"),
    ]);
    let ao_back = run.draw_with_stages(
        ao,
        "g_ao_backface",
        &back,
        s,
        ibl_meta("double_sided_back_receiver", "perspective", d, 1.),
    )?;
    check_occludes(
        run,
        "double-sided back face receives occlusion with the forward sign rule",
        &ao_back,
        &off_white,
        200,
    );
    let mirrored_at = ([0., 0., -4.], [-16., 12., 1.]);
    let (mirrored, d) = build(vec![
        (
            "receiver",
            &white_q,
            "white",
            place(mirrored_at),
            at(mirrored_at),
        ),
        slab_with(&white_q, "white"),
    ]);
    let ao_mirrored = run.draw_with_stages(
        ao,
        "g_ao_mirrored",
        &mirrored,
        s,
        ibl_meta("mirrored_receiver", "perspective", d, 1.),
    )?;
    check_occludes(
        run,
        "mirrored receiver instance keeps its front face and occlusion",
        &ao_mirrored,
        &off_white,
        200,
    );

    let tilted_geometry = json!({"local_plane": "x + z = 0", "scale": [-3., 2., 1.],
        "yaw_radians": (1f64 / 3.).atan(), "translation": [0., 0., -4.]});
    let (plane, d) = build(vec![(
        "plane",
        &tilted_q,
        "white",
        tilted_world,
        tilted_geometry,
    )]);
    let m = ibl_meta("tilted_nonuniform_mirrored_flat", "perspective", d, 1.);
    let off_plane = run.draw(off, "g_off_tilted", &plane, s, m.clone())?;
    let ao_plane = run.draw(ao, "g_ao_tilted", &plane, s, m)?;
    check_pair(
        run,
        "nonuniform mirrored flat plane: no self-occlusion from its normal space",
        &ao_plane,
        &off_plane,
    );

    // Retained skinned deformation: the far pose first, then the near pose.
    let (mut skinned, d) = build(vec![
        receiver(),
        (
            "slab",
            &skin_q,
            "white",
            DMat4::IDENTITY,
            json!({"skinned": "z -1 at t=0, z -3 at t=1"}),
        ),
    ]);
    let skin_meta = ibl_meta("skinned_slab", "perspective", d, 1.);
    let sample = |seconds: f64| {
        Some(AnimationSample {
            clip: Some(0),
            seconds,
        })
    };
    if let Some(binding) = skinned.nodes[2].mesh.as_mut() {
        binding.animation = sample(0.);
    }
    let mut retained = ao.gpu.renderer.prepare_retained(&skinned, "camera")?;
    let (skin_far, _) = run.submit_with_stages(
        ao,
        "g_ao_skin_t0",
        &mut retained,
        &skinned,
        s,
        skin_meta.clone(),
    )?;
    if let Some(binding) = skinned.nodes[2].mesh.as_mut() {
        binding.animation = sample(1.);
    }
    let (skin_near, _) = run.submit_with_stages(ao, "g_ao_skin_t1", &mut retained, &skinned, s, skin_meta)?;
    let baked = |source, z: f64| {
        (
            "slab",
            source,
            "white",
            DMat4::IDENTITY,
            json!({"baked_rect": {"x": [0.4, 3.0], "y": [-1.5, 1.5], "z": z}}),
        )
    };
    let (near_oracle, d) = build(vec![receiver(), baked(&near_q, -3.)]);
    let oracle_near = run.draw(
        ao,
        "g_ao_skin_oracle_near",
        &near_oracle,
        s,
        ibl_meta("baked_slab_near", "perspective", d, 1.),
    )?;
    let (far_oracle, d) = build(vec![receiver(), baked(&far_q, -1.)]);
    let oracle_far = run.draw(
        ao,
        "g_ao_skin_oracle_far",
        &far_oracle,
        s,
        ibl_meta("baked_slab_far", "perspective", d, 1.),
    )?;
    check_pair(
        run,
        "retained skin at t=0 equals the baked far oracle (no stale or rest-pose prepass)",
        &skin_far,
        &oracle_far,
    );
    check_pair(
        run,
        "retained skin at t=1 equals the baked near oracle (deformation reached the prepass)",
        &skin_near,
        &oracle_near,
    );

    // Retained instance motion: far transform first, then the base transform.
    let (mut moving, _) = build(vec![receiver(), slab_with(&white_q, "white")]);
    let motion_meta = |at: Place| {
        ibl_meta(
            "instance_motion",
            "perspective",
            json!([
                part("receiver", "white", RECEIVER),
                part("slab", "white", at)
            ]),
            1.,
        )
    };
    moving.nodes[2].world = place(FAR_SLAB);
    let mut motion = ao.gpu.renderer.prepare_retained(&moving, "camera")?;
    run.submit_with_stages(
        ao,
        "g_ao_motion_far",
        &mut motion,
        &moving,
        s,
        motion_meta(FAR_SLAB),
    )?;
    moving.nodes[2].world = place(SLAB);
    let (moved, _) = run.submit_with_stages(
        ao,
        "g_ao_motion_near",
        &mut motion,
        &moving,
        s,
        motion_meta(SLAB),
    )?;
    check_pair(
        run,
        "retained instance motion: the near frame equals the fresh base frame",
        &moved,
        &ao_white,
    );
    Ok(())
}

// ------------------------------------------------------------------ lifecycle_drs

fn lifecycle_drs(out: &Path) -> Result<bool> {
    let off = Device::new("off", Gpu::new()?);
    let ao = Device::new("ao", Gpu::new()?.with_ao(AO)?);
    let bloom = Device::new("bloom", Gpu::new()?.with_bloom(NUMERIC_BLOOM)?);
    let mut both_gpu = Gpu::new()?.with_ao(AO)?.with_bloom(NUMERIC_BLOOM)?;
    // Invalid replacements are refused at setup; every later valid frame must
    // equal its in-order reference, proving the prior configuration survived.
    let ao_refused = pollster::block_on(both_gpu.renderer.enable_ambient_occlusion(
        AmbientOcclusion {
            radius: f32::NAN,
            ..AO
        },
    ))
    .err()
    .map(|e| e.to_string());
    let bloom_refused = pollster::block_on(both_gpu.renderer.enable_bloom(Bloom {
        intensity: f32::NAN,
        ..NUMERIC_BLOOM
    }))
    .err()
    .map(|e| e.to_string());
    let both = Device::new("ao_bloom", both_gpu);
    let mut run = Run::new("lifecycle_drs", 18, out)?;
    run.check(
        ao_refused.is_some() && bloom_refused.is_some(),
        "invalid AO and bloom replacements refuse at setup",
        json!({"ao": ao_refused, "bloom": bloom_refused}),
    );
    let (body, errors) = scoped!(
        [off, ao, bloom, both],
        lifecycle_body(&mut run, &off, &ao, &bloom, &both)
    );
    finish(run, &[&off, &ao, &bloom, &both], errors, body)
}

fn lifecycle_body(
    run: &mut Run,
    off: &Device,
    ao: &Device,
    bloom: &Device,
    both: &Device,
) -> Result<()> {
    // Queued frames across alternating odd sizes, out-of-order submission,
    // dropped scene/assets/source files, and an invalid viewport in between.
    let a = shot();
    let b = Shot {
        size: [255, 147],
        viewport: Some(Viewport {
            rect: [9.5, 11.25, 220.25, 120.5],
            corner_radii: [0.; 4],
            canvas_srgb: CANVAS,
        }),
    };
    let (ref_a, ref_b, encoded) = {
        let assets = Assets::new()?;
        let env = assets.environment()?;
        let quad = gltf::unit_quad(1., 1., false);
        let white_q = assets.model("quad_white", &quad, white(), None, None)?;
        let emitter_q = assets.model(
            "quad_emitter",
            &quad,
            gltf::emissive([1., 1., 1.], 1., "OPAQUE"),
            None,
            None,
        )?;
        let emitter_at: Place = ([-1.5, 1., -3.5], [0.6, 0.6, 1.]);
        let scene = input(
            vec![
                camera(false),
                key_light(0.),
                mesh("receiver", &white_q, place(RECEIVER)),
                mesh("slab", &white_q, place(SLAB)),
                mesh("emitter", &emitter_q, place(emitter_at)),
            ],
            &env,
            1.,
        );
        let m = ibl_meta(
            "lifecycle",
            "perspective",
            json!([
                part("receiver", "white", RECEIVER),
                part("slab", "white", SLAB),
                part("emitter", "emissive_1", emitter_at)
            ]),
            1.,
        );
        let ref_a = run.draw(both, "l_ref_a", &scene, a, m.clone())?;
        let ref_b = run.draw(both, "l_ref_b", &scene, b, m)?;
        let prepared = both.gpu.renderer.prepare(&scene)?.with_camera("camera")?;
        let first = run.encode(both, "l_encoded_a", &prepared, a)?;
        let second = run.encode(both, "l_encoded_b", &prepared, b)?;
        run.frames.charge("l_invalid_viewport")?;
        let bad = target(&both.gpu.renderer, a.size);
        let refused = both
            .gpu
            .renderer
            .draw_scene(
                &prepared,
                &bad.create_view(&Default::default()),
                FORMAT,
                a.size[0],
                a.size[1],
                Some(Viewport {
                    rect: [0., 0., a.size[0] as f32 + 1., a.size[1] as f32],
                    corner_radii: [0.; 4],
                    canvas_srgb: CANVAS,
                }),
            )
            .err()
            .map(|e| e.to_string());
        run.check(
            refused.is_some(),
            "a viewport outside the target refuses before encoding",
            json!(refused),
        );
        let third = run.encode(both, "l_encoded_a2", &prepared, a)?;
        // Scene, loaded versions and every source/cache file are dropped here.
        (ref_a, ref_b, [first, second, third])
    };
    let [(ta, ca), (tb, cb), (ta2, ca2)] = encoded;
    let renderer = &both.gpu.renderer;
    renderer.queue.submit([ca2]);
    renderer.queue.submit([cb]);
    renderer.queue.submit([ca]);
    for (label, texture, reference, size, what) in [
        (
            "l_encoded_a2",
            &ta2,
            &ref_a,
            a.size,
            "late frame after refusals and drops equals its reference",
        ),
        (
            "l_encoded_b",
            &tb,
            &ref_b,
            b.size,
            "resized frame submitted out of order equals its reference",
        ),
        (
            "l_encoded_a",
            &ta,
            &ref_a,
            a.size,
            "first encoded frame submitted last equals its reference",
        ),
    ] {
        let frame = read(renderer, texture)?;
        run.store.keep(
            label,
            &frame,
            json!({"renderer": both.name, "route": "draw_scene queued", "size": size,
                "submitted_out_of_order": true}),
        )?;
        check_pair(run, what, &frame, reference);
    }

    // SSAO under fixed dynamic resolution: 100% equals disabled, reduced scales
    // equal the nearest expansion of internal-size references (offline check).
    let assets = Assets::new()?;
    let env = assets.environment()?;
    let quad = gltf::unit_quad(1., 1., false);
    let white_q = assets.model("quad_white", &quad, white(), None, None)?;
    let emitter_q = assets.model(
        "quad_emitter",
        &quad,
        gltf::emissive([1., 1., 1.], 1., "OPAQUE"),
        None,
        None,
    )?;
    let scene = input(
        vec![
            camera(false),
            key_light(0.),
            mesh("receiver", &white_q, place(RECEIVER)),
            mesh("slab", &white_q, place(SLAB)),
        ],
        &env,
        1.,
    );
    let m = ibl_meta(
        "white_slab",
        "perspective",
        json!([
            part("receiver", "white", RECEIVER),
            part("slab", "white", SLAB)
        ]),
        1.,
    );
    let mut retained = ao.gpu.renderer.prepare_retained(&scene, "camera")?;
    let mut drs = Vec::new();
    for (label, scale) in [
        ("d_ao_disabled", None),
        ("d_ao_16", Some(16u8)),
        ("d_ao_12", Some(12)),
        ("d_ao_8", Some(8)),
    ] {
        retained.set_dynamic_resolution(scale.map(fixed).transpose()?)?;
        let (frame, receipt) = run.submit(ao, label, &mut retained, &scene, a, m.clone())?;
        if let Some(n) = scale {
            let expected = internal(a.size, u32::from(n));
            let ok = receipt["scale16"] == json!(n)
                && receipt["internal"] == json!(expected)
                && receipt["output"] == json!(a.size);
            run.check(
                ok,
                &format!("{label}: actual receipt scale and internal size"),
                receipt,
            );
        }
        drs.push(frame);
    }
    check_pair(
        run,
        "fixed 100% equals disabled dynamic resolution",
        &drs[1],
        &drs[0],
    );
    for n in [12u32, 8] {
        let reference = Shot {
            size: internal(a.size, n),
            viewport: Some(scaled_viewport(n)),
        };
        run.draw(
            ao,
            &format!("d_ao_reference_{n}"),
            &scene,
            reference,
            m.clone(),
        )?;
    }

    // Bloom under fixed dynamic resolution: the halo stays on the physical
    // emitter (centroid checked offline); output size never changes.
    let emitter_at: Place = ([0.9, 0.4, -4.], [0.7, 0.7, 1.]);
    let emitter = input(
        vec![
            camera(false),
            mesh("emitter", &emitter_q, place(emitter_at)),
        ],
        &env,
        0.,
    );
    let em = meta(
        "single_emitter",
        "perspective",
        json!([part("emitter", "emissive_1", emitter_at)]),
        0.,
    );
    let mut lit = bloom.gpu.renderer.prepare_retained(&emitter, "camera")?;
    let mut dark = off.gpu.renderer.prepare_retained(&emitter, "camera")?;
    for n in [16u8, 12, 8] {
        lit.set_dynamic_resolution(Some(fixed(n)?))?;
        dark.set_dynamic_resolution(Some(fixed(n)?))?;
        let (with, _) = run.submit(
            bloom,
            &format!("e_bloom_{n}"),
            &mut lit,
            &emitter,
            a,
            em.clone(),
        )?;
        let (without, _) = run.submit(
            off,
            &format!("e_off_{n}"),
            &mut dark,
            &emitter,
            a,
            em.clone(),
        )?;
        let (_, brighter) = darker_brighter(&with, &without);
        let chrome = chrome_identical(&with, &without, RECT, [0.; 4]);
        run.check(
            brighter > 0 && chrome["differing"] == 0,
            &format!("bloom at {n}/16 adds a halo and leaves chrome unchanged"),
            json!({"brighter_pixels": brighter, "chrome": chrome}),
        );
    }
    let centre = project(emitter_at.0, RECT);
    emit(
        run.case,
        "expectation",
        json!({"emitter_physical_centre": centre}),
    );
    Ok(())
}

// ------------------------------------------------------------------ bloom_numeric

fn bloom_numeric(out: &Path) -> Result<bool> {
    let off = Device::new("off", Gpu::new()?);
    let b3 = Device::new(
        "bloom_t0.25_l3",
        Gpu::new()?.with_bloom(Bloom {
            levels: 3,
            ..NUMERIC_BLOOM
        })?,
    );
    let field = |levels| Bloom {
        threshold: 0.5,
        intensity: 0.5,
        levels,
    };
    let l1 = Device::new("bloom_t0.5_l1", Gpu::new()?.with_bloom(field(1))?);
    let l4 = Device::new("bloom_t0.5_l4", Gpu::new()?.with_bloom(field(4))?);
    let l6 = Device::new("bloom_t0.5_l6", Gpu::new()?.with_bloom(field(6))?);
    let mut run = Run::new("bloom_numeric", 8, out)?;
    let (body, errors) = scoped!(
        [off, b3, l1, l4, l6],
        bloom_body(&mut run, &off, &b3, [&l1, &l4, &l6])
    );
    finish(run, &[&off, &b3, &l1, &l4, &l6], errors, body)
}

/// Max channel difference inside a pixel box.
fn box_difference(a: &Rgba, b: &Rgba, centre: [f64; 2], half: f64) -> u8 {
    let mut worst = 0;
    let x0 = (centre[0] - half).max(0.) as u32;
    let y0 = (centre[1] - half).max(0.) as u32;
    let x1 = ((centre[0] + half) as u32).min(a.size[0] - 1);
    let y1 = ((centre[1] + half) as u32).min(a.size[1] - 1);
    for y in y0..=y1 {
        for x in x0..=x1 {
            let (p, q) = (a.px(x, y), b.px(x, y));
            for (u, v) in p.iter().zip(q.iter()).take(3) {
                worst = worst.max(u.abs_diff(*v));
            }
        }
    }
    worst
}

fn bloom_body(run: &mut Run, off: &Device, b3: &Device, field_devices: [&Device; 3]) -> Result<()> {
    let assets = Assets::new()?;
    let env = assets.environment()?;
    let quad = gltf::unit_quad(1., 1., false);
    let s = shot();
    let xs = [-2.4, 0., 2.4];
    let place_at = |x: f64| -> Place { ([x, 0., -4.], [0.6, 0.6, 1.]) };
    // Coverage: BLEND emitters with alpha 0, 0.5 and 1 (premultiplied H = a R).
    let mut nodes = vec![camera(false)];
    let mut described = Vec::new();
    for (x, alpha) in xs.into_iter().zip([0f32, 0.5, 1.]) {
        let source = assets.model(
            &format!("emitter_alpha_{}", (alpha * 100.) as u32),
            &quad,
            gltf::emissive([1., 1., 1.], alpha, "BLEND"),
            None,
            None,
        )?;
        nodes.push(mesh(&format!("emitter_{x}"), &source, place(place_at(x))));
        described.push(part(
            &format!("emitter_{x}"),
            &format!("blend_emissive_1_alpha_{alpha}"),
            place_at(x),
        ));
    }
    let coverage = input(nodes, &env, 0.);
    let m = meta("coverage", "perspective", json!(described), 0.);
    let c_off = run.draw(off, "c_off", &coverage, s, m.clone())?;
    let c_on = run.draw(b3, "c_bloom", &coverage, s, m)?;
    let centres: Vec<[f64; 2]> = xs.iter().map(|x| project([*x, 0., -4.], RECT)).collect();
    let near = [
        box_difference(&c_on, &c_off, centres[0], 40.),
        box_difference(&c_on, &c_off, centres[1], 40.),
        box_difference(&c_on, &c_off, centres[2], 40.),
    ];
    run.check(
        near[0] == 0 && near[1] > 0 && near[2] > 0,
        "zero-alpha emitter contributes exactly nothing; fractional and full coverage bloom",
        json!({"max_channel_change_alpha_0_0.5_1": near, "centres": centres}),
    );
    // Threshold 0.25: emission 0.125 below, 0.25 at, 0.5 above.
    let mut nodes = vec![camera(false)];
    let mut described = Vec::new();
    for (x, value) in xs.into_iter().zip([0.125f32, 0.25, 0.5]) {
        let source = assets.model(
            &format!("emitter_level_{}", (value * 1000.) as u32),
            &quad,
            gltf::emissive([value; 3], 1., "OPAQUE"),
            None,
            None,
        )?;
        nodes.push(mesh(&format!("emitter_{x}"), &source, place(place_at(x))));
        described.push(part(
            &format!("emitter_{x}"),
            &format!("emissive_{value}"),
            place_at(x),
        ));
    }
    let threshold = input(nodes, &env, 0.);
    let m = meta("threshold", "perspective", json!(described), 0.);
    let t_off = run.draw(off, "t_off", &threshold, s, m.clone())?;
    let t_on = run.draw(b3, "t_bloom", &threshold, s, m)?;
    let near = [
        box_difference(&t_on, &t_off, centres[0], 40.),
        box_difference(&t_on, &t_off, centres[1], 40.),
        box_difference(&t_on, &t_off, centres[2], 40.),
    ];
    run.check(
        near[0] == 0 && near[1] <= 1 && near[2] > 0,
        "luminance below/at threshold extracts nothing visible; above blooms",
        json!({"max_channel_change_below_at_above": near}),
    );
    // Constant emissive field 1.0 with threshold 0.5 at 1, 4 and 6 levels.
    let field_q = assets.model(
        "field",
        &quad,
        gltf::emissive([1., 1., 1.], 1., "OPAQUE"),
        None,
        None,
    )?;
    let field = input(
        vec![camera(false), mesh("field", &field_q, place(RECEIVER))],
        &env,
        0.,
    );
    let m = meta(
        "constant_field",
        "perspective",
        json!([part("field", "emissive_1", RECEIVER)]),
        0.,
    );
    let f_off = run.draw(off, "f_off", &field, s, m.clone())?;
    let mut fields = Vec::new();
    for (device, levels) in field_devices.into_iter().zip([1, 4, 6]) {
        fields.push(run.draw(device, &format!("f_bloom_l{levels}"), &field, s, m.clone())?);
    }
    let centre = [
        f64::from(RECT[0]) + f64::from(RECT[2]) / 2.,
        f64::from(RECT[1]) + f64::from(RECT[3]) / 2.,
    ];
    let spread = [
        box_difference(&fields[0], &fields[1], centre, 40.),
        box_difference(&fields[0], &fields[2], centre, 40.),
    ];
    let lifted = box_difference(&fields[0], &f_off, centre, 40.);
    run.check(
        spread[0] <= 1 && spread[1] <= 1 && lifted > 0,
        "normalized constant-field gain does not grow with 1, 4 or 6 levels",
        json!({"l1_vs_l4": spread[0], "l1_vs_l6": spread[1], "l1_vs_off": lifted}),
    );
    for (name, a, b) in [
        ("coverage", &c_on, &c_off),
        ("threshold", &t_on, &t_off),
        ("field", &fields[2], &f_off),
    ] {
        let chrome = chrome_identical(a, b, RECT, [0.; 4]);
        run.check(
            chrome["differing"] == 0,
            &format!("{name}: chrome/canvas outside the viewport unchanged by bloom"),
            chrome,
        );
    }
    Ok(())
}

/// Synthetic regression for the sphere-coverage precondition (CPU only; run
/// by the 0070-ssao-filter-002 build before any GPU child). The frame mirrors
/// the retained 0070-ssao-filter-001 sphere frames: canvas outside the
/// viewport, the measured canvas blends on its partly covered rim, the scene
/// backdrop inside and an optional flat disc about the viewport centre.
#[cfg(test)]
mod sphere_guard_tests {
    use super::*;

    const COLUMN_RIM: [u8; 4] = [22, 32, 45, 255];
    const ROW_RIM: [u8; 4] = [26, 47, 70, 255];
    const DISC: [u8; 4] = [128, 128, 128, 255];

    fn frame(radius: Option<f64>) -> Rgba {
        let [x0, y0, w, h] = RECT.map(f64::from);
        let (cx, cy) = (x0 + w / 2., y0 + h / 2.);
        let mut bytes = Vec::new();
        for y in 0..SIZE[1] {
            for x in 0..SIZE[0] {
                let (fx, fy) = (f64::from(x), f64::from(y));
                let overlaps = fx + 1. > x0 && fy + 1. > y0 && fx < x0 + w && fy < y0 + h;
                let whole = fx >= x0 && fy >= y0 && fx + 1. <= x0 + w && fy + 1. <= y0 + h;
                let disc = radius
                    .is_some_and(|r| (fx + 0.5 - cx).powi(2) + (fy + 0.5 - cy).powi(2) < r * r);
                let pixel = match (overlaps, whole) {
                    (false, _) => [CANVAS[0], CANVAS[1], CANVAS[2], 255],
                    (true, false) if fy < y0 => ROW_RIM,
                    (true, false) => COLUMN_RIM,
                    (true, true) if disc => DISC,
                    (true, true) => [BACKDROP_SRGB[0], BACKDROP_SRGB[1], BACKDROP_SRGB[2], 255],
                };
                bytes.extend_from_slice(&pixel);
            }
        }
        Rgba { size: SIZE, bytes }
    }

    fn count(frame: &Rgba, pixel: [u8; 4]) -> usize {
        (0..SIZE[1])
            .flat_map(|y| (0..SIZE[0]).map(move |x| (x, y)))
            .filter(|&(x, y)| frame.px(x, y) == pixel)
            .count()
    }

    #[test]
    fn sphere_guard_fractional_rim_alone_is_not_sphere() {
        let rim = frame(None);
        assert_eq!(count(&rim, COLUMN_RIM) + count(&rim, ROW_RIM), 601);
        assert_eq!(sphere_texels(&rim), 0);
    }

    #[test]
    fn sphere_guard_interior_disc_counts_within_tolerance() {
        for orthographic in [false, true] {
            let expected = sphere_area(orthographic);
            let disc = frame(Some((expected / std::f64::consts::PI).sqrt()));
            let painted = count(&disc, DISC);
            assert_eq!(sphere_texels(&disc), painted, "orthographic {orthographic}");
            assert!(
                sphere_covers(painted, expected),
                "{painted} texels against {expected}"
            );
        }
    }

    #[test]
    fn sphere_guard_tolerance_stays_ten_percent() {
        let expected = sphere_area(false);
        assert!(sphere_covers(2543, expected) && sphere_covers(3107, expected));
        assert!(!sphere_covers(2541, expected) && !sphere_covers(3109, expected));
        // The 001 frames: 3419 with the rim counted fails, 2818 without it passes.
        assert!(!sphere_covers(3419, expected) && sphere_covers(2818, expected));
    }
}

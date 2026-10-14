//! Opaque geometry/chart subset of immutable 0073 look-dev Stage.
//! Source selection and deliberate exclusions are recorded in the fixture note.
use glam::Vec3;
use serde_json::{Value, json};
const WIRE_HEIGHTS: [f32; 3] = [4.3, 3.9, 3.5];
const WIRE_HALVES: [f32; 3] = [0.016, 0.008, 0.004];
/// Shadow look-dev builders from the compiled 0068 review helper, plus UVs.
#[derive(Default)]
pub(super) struct Mesh {
    positions: Vec<f32>,
    normals: Vec<f32>,
    uvs: Vec<f32>,
}
impl Mesh {
    fn triangle(&mut self, v: [Vec3; 3], n: [Vec3; 3]) {
        let face = (v[1] - v[0]).cross(v[2] - v[0]);
        let order = if face.dot(n[0] + n[1] + n[2]) >= 0. {
            [0, 1, 2]
        } else {
            [0, 2, 1]
        };
        for i in order {
            self.positions.extend(v[i].to_array());
            self.normals.extend(n[i].normalize().to_array());
        }
    }
    fn quad(&mut self, v: [Vec3; 4], n: Vec3) {
        self.triangle([v[0], v[1], v[2]], [n; 3]);
        self.triangle([v[0], v[2], v[3]], [n; 3]);
    }
    fn tilted_quad(&mut self, center: Vec3, u: Vec3, v: Vec3) {
        let n = u.cross(v).normalize();
        self.quad(
            [
                center - u - v,
                center + u - v,
                center + u + v,
                center - u + v,
            ],
            n,
        );
    }
    fn cuboid(&mut self, center: Vec3, half: Vec3) {
        for axis in [Vec3::X, Vec3::Y, Vec3::Z] {
            for sign in [-1., 1.] {
                let n = axis * sign;
                let (a, b) = (
                    n.any_orthonormal_vector(),
                    n.cross(n.any_orthonormal_vector()),
                );
                let (u, v) = (a * half.dot(a.abs()), b * half.dot(b.abs()));
                self.tilted_quad(center + n * half.dot(axis), u, v);
            }
        }
    }
    fn sphere(&mut self, center: Vec3, radius: f32, rings: u32, segments: u32) {
        let point = |i: u32, j: u32| {
            let theta = std::f32::consts::PI * i as f32 / rings as f32;
            let phi = std::f32::consts::TAU * j as f32 / segments as f32;
            Vec3::new(
                theta.sin() * phi.cos(),
                theta.cos(),
                theta.sin() * phi.sin(),
            )
        };
        for i in 0..rings {
            for j in 0..segments {
                let n = [
                    point(i, j),
                    point(i + 1, j),
                    point(i + 1, j + 1),
                    point(i, j + 1),
                ];
                let v = n.map(|n| center + n * radius);
                if i + 1 != rings {
                    self.triangle([v[0], v[1], v[2]], [n[0], n[1], n[2]]);
                }
                if i != 0 {
                    self.triangle([v[0], v[2], v[3]], [n[0], n[2], n[3]]);
                }
            }
        }
    }
}
/// Ground to the horizon, a cube, a sphere, 16 thin posts receding to 56 m,
/// and an antenna whose three wires and diagonal sit against the backdrop.
pub(super) fn stage_mesh() -> Mesh {
    let mut m = Mesh::default();
    m.tilted_quad(
        Vec3::new(0., 0., -50.),
        Vec3::new(40., 0., 0.),
        Vec3::new(0., 0., -60.),
    );
    m.cuboid(Vec3::new(-2.6, 0.5, 2.6), Vec3::splat(0.5));
    m.sphere(Vec3::new(1.3, 0.85, 1.8), 0.85, 24, 48);
    for k in 0..16 {
        m.cuboid(
            Vec3::new(-1.4, 0.6, -1.5 - 3. * k as f32),
            Vec3::new(0.06, 0.6, 0.06),
        );
    }
    m.cuboid(Vec3::new(-4.4, 2.3, -3.), Vec3::new(0.05, 2.3, 0.05));
    for (height, half) in WIRE_HEIGHTS.into_iter().zip(WIRE_HALVES) {
        m.cuboid(Vec3::new(-4.4, height, -3.), Vec3::new(2.5, half, half));
    }
    // A camera-facing 0.012 m strip from (-6.9, 2.7) to (-1.9, 4.5).
    m.tilted_quad(
        Vec3::new(-4.4, 3.6, -3.),
        Vec3::new(2.5, 0.9, 0.),
        Vec3::new(-1.8, 5., 0.).normalize() * 0.006,
    );
    m
}
/// A 2.2 m camera-facing square with UVs; 128 texels span about 134 pixels.
pub(super) fn sign_mesh() -> Mesh {
    let center = Vec3::new(3.4, 1.25, 0.);
    let h = 1.1_f32;
    let corners = [
        center + Vec3::new(-h, -h, 0.),
        center + Vec3::new(h, -h, 0.),
        center + Vec3::new(h, h, 0.),
        center + Vec3::new(-h, h, 0.),
    ];
    let uv: [[f32; 2]; 4] = [[0., 1.], [1., 1.], [1., 0.], [0., 0.]];
    let mut m = Mesh::default();
    for i in [0, 1, 2, 0, 2, 3] {
        m.positions.extend(corners[i].to_array());
        m.normals.extend(Vec3::Z.to_array());
        m.uvs.extend(uv[i]);
    }
    m
}
pub(super) fn mover_mesh() -> Mesh {
    let mut m = Mesh::default();
    m.cuboid(Vec3::ZERO, Vec3::splat(0.32));
    m
}
/// 128x128 look-dev chart: 1-texel lines (top left), a 2-texel checker (top
/// right), rings whose spacing falls below a texel (bottom left) and a 4-texel
/// checker crossed by 1-texel diagonals (bottom right).
pub(super) fn sign_texture() -> Vec<u8> {
    let mut pixels: Vec<u8> = Vec::with_capacity(128 * 128 * 4);
    for y in 0..128_i32 {
        for x in 0..128_i32 {
            let on = match (x / 64, y / 64) {
                (0, 0) => x % 2 == 0,
                (1, 0) => (x / 2 + y / 2) % 2 == 0,
                (0, _) => ((x - 32).pow(2) + (y - 96).pow(2)) / 24 % 2 == 0,
                _ => (x / 4 + y / 4) % 2 == 0 || (x - y).rem_euclid(16) == 0,
            };
            let v: u8 = if on { 232 } else { 28 };
            pixels.extend([v, v, v, 255]);
        }
    }
    pixels
}
fn bounds(positions: &[f32]) -> ([f32; 3], [f32; 3]) {
    positions
        .chunks(3)
        .fold(([f32::MAX; 3], [f32::MIN; 3]), |(mut lo, mut hi), p| {
            for i in 0..3 {
                lo[i] = lo[i].min(p[i]);
                hi[i] = hi[i].max(p[i]);
            }
            (lo, hi)
        })
}
/// One glTF primitive: positions, normals and optional UVs in one buffer.
pub(super) fn gltf(mesh: &Mesh, bin: &str, material: Value) -> (Vec<u8>, Value) {
    let count = mesh.positions.len() / 3;
    let (min, max) = bounds(&mesh.positions);
    let mut bytes: Vec<u8> = mesh
        .positions
        .iter()
        .chain(&mesh.normals)
        .flat_map(|v| v.to_le_bytes())
        .collect();
    let half = count * 12;
    let mut views = vec![
        json!({"buffer":0,"byteLength":half}),
        json!({"buffer":0,"byteOffset":half,"byteLength":half}),
    ];
    let mut accessors = vec![
        json!({"bufferView":0,"componentType":5126,"count":count,"type":"VEC3","min":min,"max":max}),
        json!({"bufferView":1,"componentType":5126,"count":count,"type":"VEC3"}),
    ];
    let mut attributes = json!({"POSITION":0,"NORMAL":1});
    if !mesh.uvs.is_empty() {
        bytes.extend(mesh.uvs.iter().flat_map(|v| v.to_le_bytes()));
        views.push(json!({"buffer":0,"byteOffset":2 * half,"byteLength":count * 8}));
        accessors.push(json!({"bufferView":2,"componentType":5126,"count":count,"type":"VEC2"}));
        attributes["TEXCOORD_0"] = json!(2);
    }
    let document = json!({
        "asset":{"version":"2.0"}, "buffers":[{"uri":bin,"byteLength":bytes.len()}],
        "bufferViews":views, "accessors":accessors,
        "meshes":[{"primitives":[{"attributes":attributes,"material":0}]}],
        "materials":[material], "nodes":[{"mesh":0}], "scenes":[{"nodes":[0]}], "scene":0
    });
    (bytes, document)
}

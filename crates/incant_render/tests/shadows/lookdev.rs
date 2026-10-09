//! Look-dev captures for directional shadow appearance on contact, slopes and
//! curved surfaces. Real cooked glTF geometry rendered by the engine; this is a
//! review fixture, not game art. Assertions are deliberately coarse sanity
//! checks (contact is dark, open ground is uniformly lit); appearance approval
//! comes from the captured PNGs written to INCANT_SHADOW_EVIDENCE.
use super::super::materials::Fixture;
use super::{capture, set};
use glam::{DQuat, Vec3};
use incant_doc::{Entity, Transform};
use incant_render::Renderer;
use serde_json::json;

#[derive(Default)]
struct Mesh {
    positions: Vec<f32>,
    normals: Vec<f32>,
}
impl Mesh {
    /// Adds a triangle wound counter-clockwise when seen from its outward normal.
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
                // Row 0 starts at the north pole (v0 == v3); the last row ends at
                // the south pole (v1 == v2). Skip only the degenerate half.
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

fn scene_mesh() -> Mesh {
    let mut m = Mesh::default();
    // Ground 120 m deep so the receding post row crosses all four cascades and the fade.
    m.tilted_quad(
        Vec3::new(0., 0., -50.),
        Vec3::new(14., 0., 0.),
        Vec3::new(0., 0., -60.),
    );
    // Resting cube: contact and peter-panning check along its base.
    m.cuboid(Vec3::new(-2.2, 0.5, 0.5), Vec3::splat(0.5));
    // Sphere touching the ground: smooth terminator and self-shadowing.
    m.sphere(Vec3::new(0.6, 0.8, 1.0), 0.8, 24, 48);
    // 25-degree ramp with a thin post standing on it: sloped receiver and acne.
    let slope = 25_f32.to_radians();
    let up_slope = Vec3::new(0., slope.sin(), -slope.cos());
    let ramp_center = Vec3::new(3.4, 1.6 * slope.sin(), -1.2 - 1.6 * slope.cos());
    m.tilted_quad(ramp_center, Vec3::new(1.3, 0., 0.), up_slope * 1.6);
    m.cuboid(
        ramp_center + Vec3::new(0., 0.6, 0.),
        Vec3::new(0.06, 0.75, 0.06),
    );
    // 2 cm thin wall: light leaking through thin casters.
    m.cuboid(Vec3::new(-0.6, 0.6, -2.2), Vec3::new(1.4, 0.6, 0.01));
    // Posts every 4 m receding to 76 m: cascade transitions and distance fade.
    for k in 0..19 {
        m.cuboid(
            Vec3::new(-4.5, 0.75, -4. - 4. * k as f32),
            Vec3::new(0.12, 0.75, 0.12),
        );
    }
    m
}

fn write_scene(f: &mut Fixture) {
    let mesh = scene_mesh();
    let count = mesh.positions.len() / 3;
    let (min, max) =
        mesh.positions
            .chunks(3)
            .fold(([f32::MAX; 3], [f32::MIN; 3]), |(mut lo, mut hi), p| {
                for i in 0..3 {
                    lo[i] = lo[i].min(p[i]);
                    hi[i] = hi[i].max(p[i]);
                }
                (lo, hi)
            });
    let bytes: Vec<u8> = mesh
        .positions
        .iter()
        .chain(&mesh.normals)
        .flat_map(|v| v.to_le_bytes())
        .collect();
    let half = count * 12;
    std::fs::write(f.root.path().join("quad.bin"), &bytes).unwrap();
    let document = json!({
        "asset":{"version":"2.0"}, "buffers":[{"uri":"quad.bin","byteLength":bytes.len()}],
        "bufferViews":[{"buffer":0,"byteLength":half},{"buffer":0,"byteOffset":half,"byteLength":half}],
        "accessors":[{"bufferView":0,"componentType":5126,"count":count,"type":"VEC3","min":min,"max":max},
            {"bufferView":1,"componentType":5126,"count":count,"type":"VEC3"}],
        "meshes":[{"primitives":[{"attributes":{"POSITION":0,"NORMAL":1},"material":0}]}],
        "materials":[{"pbrMetallicRoughness":{"baseColorFactor":[0.62,0.62,0.6,1],"metallicFactor":0,"roughnessFactor":0.85}}],
        "nodes":[{"mesh":0}], "scenes":[{"nodes":[0]}],"scene":0
    });
    std::fs::write(
        f.root.path().join("quad.gltf"),
        serde_json::to_vec(&document).unwrap(),
    )
    .unwrap();
    f.cook();
}

fn sun(elevation_degrees: f64, azimuth_degrees: f64) -> [f64; 4] {
    // Light direction is the rotated -Z axis: pitch down by the elevation, then
    // yaw so shadows fall toward the camera's right.
    (DQuat::from_rotation_y(azimuth_degrees.to_radians())
        * DQuat::from_rotation_x(-elevation_degrees.to_radians()))
    .to_array()
}

#[test]
#[ignore = "requires a native GPU; look-dev captures for Claude review"]
fn directional_shadow_lookdev_contact_slopes_and_curves() {
    let r = Renderer::headless().unwrap();
    let mut f = Fixture::new(json!({}));
    write_scene(&mut f);
    // Dim, slightly cool uniform fill so shadowed regions keep their shape.
    f.environment(
        &[34, 36, 44, 255].repeat(2),
        2,
        1,
        incant_doc::TextureUsage::Linear,
    );
    let entities = &mut f.project.scenes.values_mut().next().unwrap().entities;
    let mut camera = Entity::new("Look-dev camera");
    camera.components.insert(
        "Camera".into(),
        json!({"fov_degrees":50,"near":0.1,"far":120}),
    );
    camera.components.insert(
        "Transform".into(),
        json!(Transform {
            translation: [0.4, 4.2, 8.5],
            rotation: DQuat::from_rotation_x(-0.42).to_array(),
            ..Default::default()
        }),
    );
    let camera_id = camera.id.clone();
    entities.insert(camera_id.clone(), camera);
    let mut light = Entity::new("Look-dev sun");
    light.components.insert(
        "DirectionalLight".into(),
        json!({"color":[1,0.96,0.9],"intensity":3.2,"shadows":{"distance":60}}),
    );
    light.components.insert(
        "Transform".into(),
        json!(Transform {
            rotation: sun(38., 240.),
            ..Default::default()
        }),
    );
    let light_id = light.id.clone();
    entities.insert(light_id.clone(), light);

    let mid = capture(&f, &r, &camera_id, "lookdev-sun-38");
    for (name, elevation, azimuth) in [("lookdev-sun-12", 12., 240.), ("lookdev-sun-70", 70., 240.)]
    {
        set(
            &mut f,
            &light_id,
            "Transform",
            json!(Transform {
                rotation: sun(elevation, azimuth),
                ..Default::default()
            }),
        );
        capture(&f, &r, &camera_id, name);
    }
    set(
        &mut f,
        &light_id,
        "Transform",
        json!(Transform {
            rotation: sun(38., 240.),
            ..Default::default()
        }),
    );
    set(
        &mut f,
        &light_id,
        "DirectionalLight",
        json!({"color":[1,0.96,0.9],"intensity":3.2}),
    );
    let unshadowed = capture(&f, &r, &camera_id, "lookdev-unshadowed");
    assert_ne!(mid, unshadowed, "the look-dev scene must show shadows");
    // Acne guard: the near ground below row 360 receives no shadow at 38 degrees,
    // so it must match the unshadowed render exactly. Lower depth/slope bias
    // produced tens of thousands of moire pixels here during look-dev.
    let (lit, reference) = (rgba(&mid), rgba(&unshadowed));
    let start = 360 * 640 * 4;
    let acne = lit[start..]
        .chunks(4)
        .zip(reference[start..].chunks(4))
        .filter(|(a, b)| a != b)
        .count();
    assert_eq!(acne, 0, "self-shadowing changed {acne} open-ground pixels");
}

fn rgba(png: &[u8]) -> Vec<u8> {
    let mut reader = png::Decoder::new(std::io::Cursor::new(png))
        .read_info()
        .unwrap();
    let mut bytes = vec![0; reader.output_buffer_size().unwrap()];
    reader.next_frame(&mut bytes).unwrap();
    bytes
}

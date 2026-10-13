//! Source-generated numerical glTF documents (never game art), cooked through
//! the existing typed glTF route. Factors stay inside the importer's glTF
//! ranges (for example emissive <= 1); radiance above one comes from lighting.
use serde_json::{Value, json};

#[derive(Clone)]
pub struct Mesh {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Option<Vec<[f32; 2]>>,
    pub indices: Vec<u16>,
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// Two triangles, counter-clockwise when seen from `front`. The authored normal
/// is independent of the winding so normal-sign controls are explicit.
pub fn quad(corners: [[f32; 3]; 4], front: [f32; 3], normal: [f32; 3], uv: bool) -> Mesh {
    let ccw = dot(
        cross(sub(corners[1], corners[0]), sub(corners[2], corners[0])),
        front,
    ) > 0.;
    Mesh {
        positions: corners.to_vec(),
        normals: vec![normal; 4],
        uvs: uv.then(|| vec![[0., 1.], [1., 1.], [1., 0.], [0., 0.]]),
        indices: if ccw {
            vec![0, 1, 2, 0, 2, 3]
        } else {
            vec![0, 2, 1, 0, 3, 2]
        },
    }
}

/// Unit square in the XY plane, corners (+-0.5, +-0.5, 0); u grows with x.
pub fn unit_quad(front_z: f32, normal_z: f32, uv: bool) -> Mesh {
    quad(
        [
            [-0.5, -0.5, 0.],
            [0.5, -0.5, 0.],
            [0.5, 0.5, 0.],
            [-0.5, 0.5, 0.],
        ],
        [0., 0., front_z],
        [0., 0., normal_z],
        uv,
    )
}

/// The same square with explicit corners (used for the baked skin oracle).
pub fn rect_z(x: [f32; 2], y: [f32; 2], z: f32) -> Mesh {
    quad(
        [
            [x[0], y[0], z],
            [x[1], y[0], z],
            [x[1], y[1], z],
            [x[0], y[1], z],
        ],
        [0., 0., 1.],
        [0., 0., 1.],
        false,
    )
}

/// Axis-aligned box with flat per-face normals (24 vertices, 36 indices).
pub fn cuboid() -> Mesh {
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
            let face: [u16; 6] = if sign > 0. {
                [0, 1, 2, 0, 2, 3]
            } else {
                [0, 2, 1, 0, 3, 2]
            };
            indices.extend(face.map(|i| base + i));
        }
    }
    Mesh {
        positions,
        normals,
        uvs: None,
        indices,
    }
}

/// UV sphere of radius 0.5 with smooth analytic normals (normal precision).
pub fn sphere(rings: u16, sectors: u16) -> Mesh {
    let (mut positions, mut normals, mut indices) = (Vec::new(), Vec::new(), Vec::new());
    for y in 0..=rings {
        let theta = std::f32::consts::PI * f32::from(y) / f32::from(rings);
        for x in 0..=sectors {
            let phi = std::f32::consts::TAU * f32::from(x) / f32::from(sectors);
            let n = [
                theta.sin() * phi.cos(),
                theta.cos(),
                theta.sin() * phi.sin(),
            ];
            positions.push(n.map(|v| v * 0.5));
            normals.push(n);
        }
    }
    let row = sectors + 1;
    for y in 0..rings {
        for x in 0..sectors {
            let a = y * row + x;
            let b = a + row;
            // Outward winding: counter-clockwise seen from outside.
            for tri in [[a, a + 1, b], [a + 1, b + 1, b]] {
                let p = tri.map(|i| positions[usize::from(i)]);
                let normal = cross(sub(p[1], p[0]), sub(p[2], p[0]));
                // Pole rows collapse to points; skip their zero-area triangles.
                if dot(normal, normal) < 1e-12 {
                    continue;
                }
                let outward = dot(normal, p[0]) >= 0.;
                indices.extend(if outward {
                    tri
                } else {
                    [tri[0], tri[2], tri[1]]
                });
            }
        }
    }
    Mesh {
        positions,
        normals,
        uvs: None,
        indices,
    }
}

/// glTF material helpers within the importer's validated ranges.
pub fn pbr(base: [f32; 4], metallic: f32, roughness: f32) -> Value {
    json!({"pbrMetallicRoughness": {"baseColorFactor": base, "metallicFactor": metallic,
        "roughnessFactor": roughness}})
}
/// Pure emission: black base, metallic one (no diffuse), coverage `alpha`.
pub fn emissive(emissive: [f32; 3], alpha: f32, mode: &str) -> Value {
    json!({"pbrMetallicRoughness": {"baseColorFactor": [0., 0., 0., alpha], "metallicFactor": 1.},
        "emissiveFactor": emissive, "alphaMode": mode})
}

/// Rigid one-joint skin whose joint translation animates linearly from
/// `from` (t = 0 s) to `to` (t = 1 s); identity inverse bind matrix.
pub struct Skin {
    pub from: [f32; 3],
    pub to: [f32; 3],
}

struct Builder {
    bin: Vec<u8>,
    views: Vec<Value>,
    accessors: Vec<Value>,
}
impl Builder {
    fn add(&mut self, data: &[u8], component: u32, kind: &str, count: usize) -> usize {
        while !self.bin.len().is_multiple_of(4) {
            self.bin.push(0);
        }
        let view = self.views.len();
        self.views
            .push(json!({"buffer": 0, "byteOffset": self.bin.len(), "byteLength": data.len()}));
        self.bin.extend_from_slice(data);
        self.accessors.push(
            json!({"bufferView": view, "componentType": component, "type": kind, "count": count}),
        );
        self.accessors.len() - 1
    }
}
fn floats(values: impl IntoIterator<Item = f32>) -> Vec<u8> {
    values.into_iter().flat_map(f32::to_le_bytes).collect()
}

/// A complete one-mesh, one-material document and its binary buffer. `image`
/// names a PNG next to the document used as the base colour texture.
pub fn document(
    name: &str,
    mesh: &Mesh,
    material: Value,
    image: Option<&str>,
    skin: Option<&Skin>,
) -> (Value, Vec<u8>) {
    let mut b = Builder {
        bin: Vec::new(),
        views: Vec::new(),
        accessors: Vec::new(),
    };
    let count = mesh.positions.len();
    let positions = b.add(
        &floats(mesh.positions.iter().flatten().copied()),
        5126,
        "VEC3",
        count,
    );
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for p in &mesh.positions {
        for ((lo, hi), v) in min.iter_mut().zip(max.iter_mut()).zip(p) {
            *lo = lo.min(*v);
            *hi = hi.max(*v);
        }
    }
    b.accessors[positions]["min"] = json!(min);
    b.accessors[positions]["max"] = json!(max);
    let normals = b.add(
        &floats(mesh.normals.iter().flatten().copied()),
        5126,
        "VEC3",
        count,
    );
    let mut attributes = json!({"POSITION": positions, "NORMAL": normals});
    if let Some(uvs) = &mesh.uvs {
        let accessor = b.add(&floats(uvs.iter().flatten().copied()), 5126, "VEC2", count);
        attributes["TEXCOORD_0"] = json!(accessor);
    }
    let indices = b.add(
        &mesh
            .indices
            .iter()
            .flat_map(|i| i.to_le_bytes())
            .collect::<Vec<_>>(),
        5123,
        "SCALAR",
        mesh.indices.len(),
    );
    let mut material = material;
    let mut doc = json!({"asset": {"version": "2.0"}, "scene": 0});
    if let Some(image) = image {
        material["pbrMetallicRoughness"]["baseColorTexture"] = json!({"index": 0});
        doc["images"] = json!([{"uri": image}]);
        doc["textures"] = json!([{"source": 0, "sampler": 0}]);
        doc["samplers"] =
            json!([{"magFilter": 9728, "minFilter": 9728, "wrapS": 33071, "wrapT": 33071}]);
    }
    if let Some(skin) = skin {
        let joints = b.add(&vec![0u8; count * 8], 5123, "VEC4", count);
        let weights = b.add(
            &floats(std::iter::repeat_n([1f32, 0., 0., 0.], count).flatten()),
            5126,
            "VEC4",
            count,
        );
        attributes["JOINTS_0"] = json!(joints);
        attributes["WEIGHTS_0"] = json!(weights);
        let identity = [
            1f32, 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
        ];
        let bind = b.add(&floats(identity), 5126, "MAT4", 1);
        let times = b.add(&floats([0f32, 1.]), 5126, "SCALAR", 2);
        b.accessors[times]["min"] = json!([0.]);
        b.accessors[times]["max"] = json!([1.]);
        let moves = b.add(
            &floats(skin.from.into_iter().chain(skin.to)),
            5126,
            "VEC3",
            2,
        );
        doc["nodes"] = json!([{"translation": skin.to}, {"mesh": 0, "skin": 0}]);
        doc["skins"] = json!([{"joints": [0], "skeleton": 0, "inverseBindMatrices": bind}]);
        doc["animations"] = json!([{"name": "move",
            "samplers": [{"input": times, "output": moves, "interpolation": "LINEAR"}],
            "channels": [{"sampler": 0, "target": {"node": 0, "path": "translation"}}]}]);
        doc["scenes"] = json!([{"nodes": [0, 1]}]);
    } else {
        doc["nodes"] = json!([{"mesh": 0}]);
        doc["scenes"] = json!([{"nodes": [0]}]);
    }
    doc["materials"] = json!([material]);
    doc["meshes"] = json!([{"primitives": [{"attributes": attributes, "indices": indices,
        "material": 0}]}]);
    doc["buffers"] = json!([{"uri": format!("{name}.bin"), "byteLength": b.bin.len()}]);
    doc["bufferViews"] = json!(b.views);
    doc["accessors"] = json!(b.accessors);
    (doc, b.bin)
}

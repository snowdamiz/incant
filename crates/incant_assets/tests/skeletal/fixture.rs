use serde_json::{Value, json};
use std::{fs, path::Path};
pub fn fixture(root: &Path) -> Value {
    let mut bytes = Vec::new();
    let mut views = Vec::new();
    let mut accessors = Vec::new();
    let mut add = |data: Vec<u8>, component: u32, kind: &str, count: usize| {
        while !bytes.len().is_multiple_of(4) {
            bytes.push(0);
        }
        let i = accessors.len();
        views.push(json!({"buffer":0,"byteOffset":bytes.len(),"byteLength":data.len()}));
        bytes.extend(data);
        accessors.push(json!({"bufferView":i,"componentType":component,"type":kind,"count":count}));
        i
    };
    let floats = |v: &[f32]| v.iter().flat_map(|f| f.to_le_bytes()).collect::<Vec<_>>();
    let positions = add(
        floats(&[1., 0., 0., 2., 0., 0., 1., 1., 0.]),
        5126,
        "VEC3",
        3,
    );
    let indices = add(
        [2u16, 0, 1]
            .into_iter()
            .flat_map(u16::to_le_bytes)
            .collect(),
        5123,
        "SCALAR",
        3,
    );
    let joints = add(
        [0u16, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0]
            .into_iter()
            .flat_map(u16::to_le_bytes)
            .collect(),
        5123,
        "VEC4",
        3,
    );
    let weights = add(
        floats(&[1., 0., 0., 0., 1., 0., 0., 0., 0.25, 0.75, 0., 0.]),
        5126,
        "VEC4",
        3,
    );
    let bind = add(
        floats(&[
            1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., -1., 0., 0., 1., 1., 0., 0., 0., 0.,
            1., 0., 0., 0., 0., 1., 0., -1., -1., 0., 1.,
        ]),
        5126,
        "MAT4",
        2,
    );
    let times = add(floats(&[0., 2.]), 5126, "SCALAR", 2);
    let translations = add(floats(&[1., 0., 0., 3., 0., 0.]), 5126, "VEC3", 2);
    let rotations = add(floats(&[0., 0., 0., 1., 0., 0., 1., 0.]), 5126, "VEC4", 2);
    let scales = add(floats(&[1., 1., 1., -1., 2., 1.]), 5126, "VEC3", 2);
    accessors[positions]["min"] = json!([1, 0, 0]);
    accessors[positions]["max"] = json!([2, 1, 0]);
    accessors[times]["min"] = json!([0]);
    accessors[times]["max"] = json!([2]);
    let animation = |name: &str, output: usize, node: usize, path: &str, interpolation: &str| json!({"name":name,"samplers":[{"input":times,"output":output,"interpolation":interpolation}],"channels":[{"sampler":0,"target":{"node":node,"path":path}}]});
    let doc = json!({"asset":{"version":"2.0"},"buffers":[{"uri":"model.bin","byteLength":bytes.len()}],"bufferViews":views,"accessors":accessors,
        "meshes":[{"primitives":[{"attributes":{"POSITION":positions,"JOINTS_0":joints,"WEIGHTS_0":weights},"indices":indices}]}],
        "nodes":[{"translation":[10,0,0],"children":[1]},{"translation":[1,0,0],"children":[2]},{"translation":[0,1,0]},{"mesh":0,"skin":0,"translation":[100,0,0]},{"mesh":0,"skin":0,"scale":[-2,1,1]}],
        "skins":[{"name":"two joints","joints":[1,2],"skeleton":0,"inverseBindMatrices":bind}],"animations":[animation("move",translations,1,"translation","LINEAR"),animation("rotate",rotations,2,"rotation","LINEAR"),animation("scale",scales,1,"scale","STEP")],"scenes":[{"nodes":[0,3,4]}],"scene":0});
    fs::write(root.join("model.bin"), bytes).unwrap();
    write(root, &doc);
    doc
}
pub fn write(root: &Path, doc: &Value) {
    fs::write(root.join("model.gltf"), doc.to_string()).unwrap();
}

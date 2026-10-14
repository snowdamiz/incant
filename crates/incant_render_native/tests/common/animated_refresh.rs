//! Real bounded glTF/skin/PNG inputs shared by native and Play controls.
#![allow(dead_code)] // Each executable uses a different subset of this fixture.
#[path = "model_refresh.rs"]
mod images;
#[path = "../../../incant_assets/tests/skeletal/fixture.rs"]
mod skeletal;
pub use images::{DARK, RED, id, key, reference, texture};
use incant_runtime::{VisualAssetKind, VisualAssetRef};
use serde_json::{Value, json};
use std::{fs, path::Path};

#[derive(Clone, Copy)]
pub enum Change {
    Original,
    Keys,
    KeysOnly,
    NonRootRotation,
    Duration,
    ClipOrder,
    ClipName,
    ChannelTarget,
    Interpolation,
    InverseBind,
    JointOrder,
    SkinBinding,
    Influences,
    Rest,
    Geometry,
}

fn floats(root: &Path, document: &Value, accessor: usize, values: &[f32]) {
    let view = document["accessors"][accessor]["bufferView"]
        .as_u64()
        .unwrap() as usize;
    let offset = document["bufferViews"][view]["byteOffset"]
        .as_u64()
        .unwrap() as usize;
    let mut bytes = fs::read(root.join("model.bin")).unwrap();
    for (i, value) in values.iter().enumerate() {
        bytes[offset + i * 4..offset + i * 4 + 4].copy_from_slice(&value.to_le_bytes());
    }
    fs::write(root.join("model.bin"), bytes).unwrap();
}
fn append(root: &Path, document: &mut Value, kind: &str, count: usize, values: &[f32]) -> usize {
    let mut bytes = fs::read(root.join("model.bin")).unwrap();
    while !bytes.len().is_multiple_of(4) {
        bytes.push(0);
    }
    let offset = bytes.len();
    bytes.extend(values.iter().flat_map(|v| v.to_le_bytes()));
    let view = document["bufferViews"].as_array().unwrap().len();
    document["bufferViews"]
        .as_array_mut()
        .unwrap()
        .push(json!({"buffer":0,"byteOffset":offset,"byteLength":values.len()*4}));
    let accessor = document["accessors"].as_array().unwrap().len();
    document["accessors"]
        .as_array_mut()
        .unwrap()
        .push(json!({"bufferView":view,"componentType":5126,"count":count,"type":kind}));
    document["buffers"][0]["byteLength"] = json!(bytes.len());
    fs::write(root.join("model.bin"), bytes).unwrap();
    accessor
}

pub fn model(root: &Path, change: Change) -> VisualAssetRef {
    let mut document = skeletal::fixture(root);
    // Root1 remains rigid so a valid root-only simulation catalog can also
    // exercise non-root refresh and an explicit changed-root refusal.
    document["animations"][2]["channels"][0]["target"]["node"] = json!(2);
    let uv = append(root, &mut document, "VEC2", 3, &[0., 0., 1., 0., 0., 1.]);
    document["meshes"][0]["primitives"][0]["attributes"]["TEXCOORD_0"] = json!(uv);
    document["meshes"][0]["primitives"][0]["material"] = json!(0);
    document["materials"] = json!([{"pbrMetallicRoughness":{"baseColorFactor":[1,1,1,1],"baseColorTexture":{"index":0},"metallicFactor":0.2,"roughnessFactor":0.8}}]);
    document["textures"] = json!([{"source":0}]);
    document["images"] = json!([{"uri":"rig.png"}]);
    fs::write(root.join("rig.png"), RED).unwrap();
    match change {
        Change::Original => {}
        Change::Keys | Change::KeysOnly => {
            let times = append(root, &mut document, "SCALAR", 3, &[0., 0.5, 2.]);
            document["accessors"][times]["min"] = json!([0]);
            document["accessors"][times]["max"] = json!([2]);
            let values = append(
                root,
                &mut document,
                "VEC3",
                3,
                &[1., 0., 0., 2., 0., 0., 5., 0., 0.],
            );
            document["animations"][0]["samplers"][0]["input"] = json!(times);
            document["animations"][0]["samplers"][0]["output"] = json!(values);
            if matches!(change, Change::Keys) {
                document["materials"][0]["pbrMetallicRoughness"]["baseColorFactor"] =
                    json!([0.2, 0.4, 0.6, 1]);
                fs::write(root.join("rig.png"), DARK).unwrap();
            }
        }
        Change::NonRootRotation => {
            let output = document["animations"][1]["samplers"][0]["output"]
                .as_u64()
                .unwrap() as usize;
            let q = std::f32::consts::FRAC_1_SQRT_2;
            floats(root, &document, output, &[0., 0., 0., 1., 0., 0., q, q]);
        }
        Change::Duration => {
            // An unselected clip still belongs to the complete duration table.
            let input = append(root, &mut document, "SCALAR", 2, &[0., 3.]);
            document["accessors"][input]["min"] = json!([0]);
            document["accessors"][input]["max"] = json!([3]);
            document["animations"][1]["samplers"][0]["input"] = json!(input);
        }
        Change::ClipOrder => document["animations"].as_array_mut().unwrap().swap(0, 1),
        Change::ClipName => document["animations"][1]["name"] = json!("renamed"),
        Change::ChannelTarget => {
            document["animations"][0]["channels"][0]["target"]["node"] = json!(2)
        }
        Change::Interpolation => {
            document["animations"][0]["samplers"][0]["interpolation"] = json!("STEP")
        }
        Change::InverseBind => {
            let accessor = document["skins"][0]["inverseBindMatrices"]
                .as_u64()
                .unwrap() as usize;
            floats(
                root,
                &document,
                accessor,
                &[
                    1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., -1.25, 0., 0., 1.,
                ],
            );
        }
        Change::JointOrder => document["skins"][0]["joints"] = json!([2, 1]),
        Change::SkinBinding => {
            document["nodes"][3].as_object_mut().unwrap().remove("skin");
        }
        Change::Influences => {
            let accessor = document["meshes"][0]["primitives"][0]["attributes"]["WEIGHTS_0"]
                .as_u64()
                .unwrap() as usize;
            floats(root, &document, accessor, &[0.5, 0.5, 0., 0.]);
        }
        Change::Rest => document["nodes"][1]["translation"] = json!([1.25, 0, 0]),
        Change::Geometry => {
            let accessor = document["meshes"][0]["primitives"][0]["attributes"]["POSITION"]
                .as_u64()
                .unwrap() as usize;
            floats(root, &document, accessor, &[1.25, 0., 0.]);
        }
    }
    skeletal::write(root, &document);
    let model = incant_assets::cook_gltf(
        root,
        Path::new("model.gltf"),
        &root.join(".incant/cache/models"),
    )
    .unwrap();
    reference(90, VisualAssetKind::Model, &model.metadata.fingerprint)
}

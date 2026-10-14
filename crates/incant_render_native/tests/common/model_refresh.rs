#![allow(dead_code)]
//! Real glTF/PNG writer and bounded cooker inputs shared by native/play controls.
use incant_runtime::{StableId, VisualAssetKind, VisualAssetRef};
use serde_json::json;
use std::{fs, path::Path};

pub const RED: &[u8] = &[
    137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0,
    0, 0, 31, 21, 196, 137, 0, 0, 0, 13, 73, 68, 65, 84, 120, 156, 99, 248, 207, 192, 240, 31, 0,
    5, 0, 1, 255, 137, 153, 61, 29, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
];
pub const DARK: &[u8] = &[
    137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0,
    0, 0, 31, 21, 196, 137, 0, 0, 0, 13, 73, 68, 65, 84, 120, 156, 99, 80, 100, 96, 248, 15, 0, 1,
    136, 1, 33, 206, 188, 226, 197, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
];
pub fn id(n: u128) -> StableId {
    StableId(n.to_be_bytes())
}
pub fn key(reference: VisualAssetRef) -> String {
    reference
        .fingerprint
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
pub fn reference(n: u128, kind: VisualAssetKind, key: &str) -> VisualAssetRef {
    VisualAssetRef {
        id: id(n),
        kind,
        fingerprint: std::array::from_fn(|i| {
            u8::from_str_radix(&key[2 * i..2 * i + 2], 16).unwrap()
        }),
    }
}
#[derive(Clone, Copy)]
pub enum Variant {
    Static,
    LargerGeometry,
    TransformedNode,
    DetachedPrimitive,
    ExtraPrimitive,
    MaterialBinding,
    Skinned,
}
pub fn model(
    root: &Path,
    scale: f32,
    image: &[u8],
    factor: [f32; 4],
    variant: Variant,
) -> VisualAssetRef {
    let positions = [-scale, 0., 0., scale, 0., 0., 0., scale, 0.];
    let uv = [0f32, 0., 1., 0., 0.5, 1.];
    let mut bytes: Vec<_> = positions
        .into_iter()
        .chain(uv)
        .flat_map(f32::to_le_bytes)
        .collect();
    let mut source = json!({"asset":{"version":"2.0"},"scene":0,"scenes":[{"nodes":[0]}],
        "nodes":[{"name":"Static","mesh":0}],"meshes":[{"primitives":[{"attributes":{"POSITION":0,"TEXCOORD_0":1},"material":0}]}],
        "materials":[{"pbrMetallicRoughness":{"baseColorFactor":factor,"baseColorTexture":{"index":0},"metallicFactor":0.2,"roughnessFactor":0.8}}],
        "textures":[{"source":0}],"images":[{"uri":"model.png"}],
        "buffers":[{"uri":"tri.bin","byteLength":60}],
        "bufferViews":[{"buffer":0,"byteOffset":0,"byteLength":36},{"buffer":0,"byteOffset":36,"byteLength":24}],
        "accessors":[{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3","min":[-scale,0,0],"max":[scale,scale,0]},
                     {"bufferView":1,"componentType":5126,"count":3,"type":"VEC2"}]});
    match variant {
        Variant::Static => {}
        Variant::LargerGeometry => {
            let positions = [
                -scale, 0., 0., scale, 0., 0., 0., scale, 0., -scale, 0., 1., scale, 0., 1., 0.,
                scale, 1.,
            ];
            let uv = [0f32, 0., 1., 0., 0.5, 1., 0., 0., 1., 0., 0.5, 1.];
            bytes = positions
                .into_iter()
                .chain(uv)
                .flat_map(f32::to_le_bytes)
                .collect();
            source["buffers"][0]["byteLength"] = json!(120);
            source["bufferViews"][0]["byteLength"] = json!(72);
            source["bufferViews"][1]["byteOffset"] = json!(72);
            source["bufferViews"][1]["byteLength"] = json!(48);
            source["accessors"][0]["count"] = json!(6);
            source["accessors"][0]["max"] = json!([scale, scale, 1]);
            source["accessors"][1]["count"] = json!(6);
        }
        Variant::TransformedNode => source["nodes"][0]["translation"] = json!([1, 0, 0]),
        Variant::DetachedPrimitive => {
            source["nodes"][0].as_object_mut().unwrap().remove("mesh");
        }
        Variant::ExtraPrimitive => {
            let next = source["meshes"][0]["primitives"][0].clone();
            source["meshes"][0]["primitives"]
                .as_array_mut()
                .unwrap()
                .push(next);
        }
        Variant::MaterialBinding => {
            let next = source["materials"][0].clone();
            source["materials"].as_array_mut().unwrap().push(next);
            source["meshes"][0]["primitives"][0]["material"] = json!(1);
        }
        Variant::Skinned => {
            bytes.extend([0u8; 12]);
            bytes.extend(
                [1f32, 0., 0., 0.]
                    .into_iter()
                    .cycle()
                    .take(12)
                    .flat_map(f32::to_le_bytes),
            );
            source["buffers"][0]["byteLength"] = json!(120);
            source["bufferViews"].as_array_mut().unwrap().extend([
                json!({"buffer":0,"byteOffset":60,"byteLength":12}),
                json!({"buffer":0,"byteOffset":72,"byteLength":48}),
            ]);
            source["accessors"].as_array_mut().unwrap().extend([
                json!({"bufferView":2,"componentType":5121,"count":3,"type":"VEC4"}),
                json!({"bufferView":3,"componentType":5126,"count":3,"type":"VEC4"}),
            ]);
            source["meshes"][0]["primitives"][0]["attributes"]["JOINTS_0"] = json!(2);
            source["meshes"][0]["primitives"][0]["attributes"]["WEIGHTS_0"] = json!(3);
            source["nodes"][0]["skin"] = json!(0);
            source["nodes"][0]["children"] = json!([1]);
            source["nodes"]
                .as_array_mut()
                .unwrap()
                .push(json!({"name":"Joint"}));
            source["skins"] = json!([{"joints":[1]}]);
        }
    }
    fs::write(root.join("tri.bin"), bytes).unwrap();
    fs::write(root.join("model.png"), image).unwrap();
    fs::write(root.join("tri.gltf"), serde_json::to_vec(&source).unwrap()).unwrap();
    let model = incant_assets::cook_gltf(
        root,
        Path::new("tri.gltf"),
        &root.join(".incant/cache/models"),
    )
    .unwrap();
    reference(90, VisualAssetKind::Model, &model.metadata.fingerprint)
}
pub fn texture(root: &Path, image: &[u8]) -> VisualAssetRef {
    fs::write(root.join("pixel.png"), image).unwrap();
    let texture = incant_assets::cook_texture(
        root,
        Path::new("pixel.png"),
        &root.join(".incant/cache/textures"),
        incant_assets::TextureUsage::Color,
    )
    .unwrap();
    reference(91, VisualAssetKind::Texture, &texture.metadata.fingerprint)
}

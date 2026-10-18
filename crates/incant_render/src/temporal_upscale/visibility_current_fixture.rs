//! Public cooker fixture: real textures/PBR, removed source/cache before rendering.
use super::*;
use incant_assets_runtime::{RuntimeAsset, RuntimeAssetData};
use std::{path::Path, sync::Arc};

pub(super) fn scene(layers: &[bool]) -> SceneInput {
    scene_material(layers, false)
}
pub(super) fn scene_material(layers: &[bool], alternate: bool) -> SceneInput {
    let mut input = super::super::fixture::scene();
    let dir = tempfile::tempdir().unwrap();
    let mut positions = Vec::<f32>::new();
    let mut uv = Vec::<f32>::new();
    for &reverse_uv in layers {
        for polygon in [geometry::EDGE, geometry::STRIP] {
            for corner in [0, 2, 1, 0, 3, 2] {
                let p = polygon[corner];
                positions.extend([(p[0] - 32.) as f32 / 8., (32. - p[1]) as f32 / 8., 0.]);
                let u = [0., 1., 1., 0.][corner];
                let v = [0., 0., 1., 1.][corner];
                uv.extend([if reverse_uv { 1. - u } else { u }, v]);
            }
        }
    }
    let offset = positions.len() * 4;
    let n = positions.len() / 3;
    let bytes: Vec<_> = positions
        .iter()
        .chain(&uv)
        .flat_map(|v| v.to_le_bytes())
        .collect();
    std::fs::write(dir.path().join("mesh.bin"), &bytes).unwrap();
    let maps = [
        [
            230, 40, 30, 255, 30, 190, 60, 255, 35, 60, 220, 255, 200, 190, 40, 255,
        ],
        [
            255, 60, 10, 255, 255, 200, 220, 255, 255, 120, 90, 255, 255, 230, 180, 255,
        ],
        [
            128, 128, 255, 255, 170, 128, 235, 255, 128, 160, 240, 255, 100, 128, 250, 255,
        ],
        [
            255, 255, 255, 255, 160, 160, 160, 255, 220, 220, 220, 255, 190, 190, 190, 255,
        ],
        [25, 10, 0, 255, 0, 15, 10, 255, 0, 0, 15, 255, 10, 5, 0, 255],
    ];
    for (i, map) in maps.iter().enumerate() {
        let file = std::fs::File::create(dir.path().join(format!("map{i}.png"))).unwrap();
        let mut e = png::Encoder::new(file, 2, 2);
        e.set_color(png::ColorType::Rgba);
        e.set_depth(png::BitDepth::Eight);
        e.write_header().unwrap().write_image_data(map).unwrap();
    }
    let doc = json!({"asset":{"version":"2.0"},"buffers":[{"uri":"mesh.bin","byteLength":bytes.len()}],
        "bufferViews":[{"buffer":0,"byteLength":offset},{"buffer":0,"byteOffset":offset,"byteLength":uv.len()*4}],
        "accessors":[{"bufferView":0,"componentType":5126,"count":n,"type":"VEC3","min":[-4,-4,0],"max":[4,4,0]},{"bufferView":1,"componentType":5126,"count":n,"type":"VEC2"}],
        "images":(0..5).map(|i|json!({"uri":format!("map{i}.png")})).collect::<Vec<_>>(),
        "textures":(0..5).map(|i|json!({"source":i,"sampler":0})).collect::<Vec<_>>(),
        "samplers":[{"magFilter":9729,"minFilter":9987,"wrapS":33071,"wrapT":33071}],
        "materials":[{"pbrMetallicRoughness":{"baseColorFactor":if alternate {[0.2,0.7,1.0,1.0]}else{[1.0;4]},"metallicFactor":if alternate {0.15}else{0.8},"roughnessFactor":0.7,"baseColorTexture":{"index":0},"metallicRoughnessTexture":{"index":1}},"normalTexture":{"index":2},"occlusionTexture":{"index":3},"emissiveTexture":{"index":4},"emissiveFactor":[0.1,0.1,0.1]}],
        "meshes":[{"primitives":[{"attributes":{"POSITION":0,"TEXCOORD_0":1},"material":0}]}],"nodes":[{"mesh":0}],"scenes":[{"nodes":[0]}],"scene":0});
    std::fs::write(
        dir.path().join("mesh.gltf"),
        serde_json::to_vec(&doc).unwrap(),
    )
    .unwrap();
    let cache = dir.path().join("cache");
    let cooked = incant_assets::cook_gltf(dir.path(), Path::new("mesh.gltf"), &cache).unwrap();
    let source = Arc::new(
        RuntimeAsset::load_model("fallback-pbr", 1, &cache, &cooked.metadata.fingerprint).unwrap(),
    );
    input.nodes[0].mesh.as_mut().unwrap().source = source;
    input.environment.as_mut().unwrap().intensity = 0.2;
    if let Some(incant_render_data::SceneLight::Directional(light)) = &mut input.nodes[3].light {
        light.intensity = 2.;
    } else {
        panic!("typed key")
    }
    let mut point = input.nodes[3].clone();
    point.id = "fallback-point".into();
    point.world = glam::DMat4::from_translation(glam::DVec3::new(0., 0., 3.));
    point.light = Some(incant_render_data::SceneLight::Point(
        incant_types::PointLight {
            color: [0.8, 0.9, 1.],
            intensity: 12.,
            range: 10.,
        },
    ));
    input.nodes.push(point);
    dir.close().unwrap();
    input
}
#[test]
fn cooked_current_pbr_fixture_preserves_textures_and_light_variation() {
    let input = scene(&[false, true]);
    let RuntimeAssetData::Model(model) = input.nodes[0].mesh.as_ref().unwrap().source.data() else {
        panic!("model")
    };
    assert_eq!(model.meshes.len(), 1);
    assert_eq!(model.meshes[0].vertices.len(), 24);
    assert!(model.meshes[0].vertices.iter().any(|v| v[6] != 0.));
    assert_ne!(model.materials[0].base_color, [0., 0., 0., 1.]);
    assert!(model.materials[0].maps.iter().all(Option::is_some));
    assert_eq!(model.textures.len(), 5);
    assert_eq!(
        model.materials[0].alpha_mode,
        incant_assets_runtime::AlphaMode::Opaque
    );
    let resolved = crate::scene::resolve(&input).unwrap();
    let (lights, directional, _) = resolved.lights.finish();
    assert_eq!(directional, 1);
    assert_eq!(lights.len(), 2);
    assert!(lights.iter().all(|l| l.radiance_inner[0] > 0.));
    assert!(resolved.environment.unwrap().intensity > 0.);
}

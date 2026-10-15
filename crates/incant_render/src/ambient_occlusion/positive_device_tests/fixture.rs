use glam::{DMat4, DVec3};
use incant_assets_runtime::{RuntimeAsset, RuntimeAssetData};
use incant_render_data::{MeshInput, SceneInput, SceneLight, SceneNode};
use serde_json::json;
use std::{fs, path::Path, sync::Arc};

pub(super) fn scene() -> (SceneInput, String) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let mut bytes = Vec::new();
    let mut views = Vec::new();
    let mut accessors = Vec::new();
    let mut primitives = Vec::new();
    for (extent, z, material) in [(2., 0., 0), (0.75, 0.5, 1)] {
        let mut add = |payload: Vec<u8>,
                       component,
                       count,
                       shape,
                       bounds: Option<([f32; 3], [f32; 3])>| {
            let index = views.len();
            views.push(json!({"buffer":0,"byteOffset":bytes.len(),"byteLength":payload.len()}));
            bytes.extend(payload);
            let mut accessor =
                json!({"bufferView":index,"componentType":component,"count":count,"type":shape});
            if let Some((min, max)) = bounds {
                accessor["min"] = json!(min);
                accessor["max"] = json!(max);
            }
            accessors.push(accessor);
            index
        };
        let positions: [[f32; 3]; 4] = [
            [-extent, -extent, z],
            [extent, -extent, z],
            [extent, extent, z],
            [-extent, extent, z],
        ];
        let position = add(
            positions
                .into_iter()
                .flatten()
                .flat_map(f32::to_le_bytes)
                .collect(),
            5126,
            4,
            "VEC3",
            Some(([-extent, -extent, z], [extent, extent, z])),
        );
        let normal = add(
            [[0_f32, 0., 1.]; 4]
                .into_iter()
                .flatten()
                .flat_map(f32::to_le_bytes)
                .collect(),
            5126,
            4,
            "VEC3",
            None,
        );
        let uv = add(
            [[0_f32, 0.], [1., 0.], [1., 1.], [0., 1.]]
                .into_iter()
                .flatten()
                .flat_map(f32::to_le_bytes)
                .collect(),
            5126,
            4,
            "VEC2",
            None,
        );
        let indices = add(
            [0_u16, 1, 2, 0, 2, 3]
                .into_iter()
                .flat_map(u16::to_le_bytes)
                .collect(),
            5123,
            6,
            "SCALAR",
            None,
        );
        primitives.push(json!({"attributes":{"POSITION":position,"NORMAL":normal,"TEXCOORD_0":uv},"indices":indices,"material":material}));
    }
    fs::write(root.join("geometry.bin"), &bytes).unwrap();
    let file = fs::File::create(root.join("mask.png")).unwrap();
    let mut encoder = png::Encoder::new(file, 2, 2);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&[
            240, 100, 30, 255, 20, 220, 110, 0, 30, 100, 230, 255, 200, 200, 40, 255,
        ])
        .unwrap();
    fs::write(root.join("scene.gltf"), serde_json::to_vec(&json!({
        "asset":{"version":"2.0"}, "buffers":[{"uri":"geometry.bin","byteLength":bytes.len()}],
        "bufferViews":views,"accessors":accessors,"images":[{"uri":"mask.png"}],
        "samplers":[{"magFilter":9728,"minFilter":9728,"wrapS":33071,"wrapT":33071}],
        "textures":[{"source":0,"sampler":0}],
        "materials":[
            {"pbrMetallicRoughness":{"baseColorFactor":[0.7,0.7,0.7,1.],"metallicFactor":0.,"roughnessFactor":1.}},
            {"alphaMode":"MASK","alphaCutoff":0.5,"pbrMetallicRoughness":{"baseColorFactor":[0.9,0.9,0.9,1.],"baseColorTexture":{"index":0},"metallicFactor":0.,"roughnessFactor":1.}}
        ],
        "meshes":[{"primitives":primitives}],"nodes":[{"mesh":0}],"scenes":[{"nodes":[0]}],"scene":0
    })).unwrap()).unwrap();
    let cache = root.join("cache");
    let cooked = incant_assets::cook_gltf(root, Path::new("scene.gltf"), &cache).unwrap();
    let source = Arc::new(
        RuntimeAsset::load_model("positive-ssao", 1, &cache, &cooked.metadata.fingerprint).unwrap(),
    );
    let RuntimeAssetData::Model(model) = source.data() else {
        panic!("model fixture");
    };
    assert_eq!(model.meshes.len(), 2);
    assert_eq!(
        model.materials[0].alpha_mode,
        incant_assets_runtime::AlphaMode::Opaque
    );
    assert_eq!(
        model.materials[1].alpha_mode,
        incant_assets_runtime::AlphaMode::Mask
    );
    assert!(model.materials[1].maps[0].is_some());
    assert_eq!(model.images.len(), 1);
    assert_eq!(model.materials[0].base_color, [0.7, 0.7, 0.7, 1.]);
    assert_eq!(model.materials[0].metallic, 0.);
    assert_eq!(model.materials[0].roughness, 1.);
    let image = &model.images[model.materials[1].maps[0].unwrap().image];
    assert_eq!([image.width, image.height], [2, 2]);
    assert_eq!(
        image.format,
        incant_assets_runtime::TextureFormat::Rgba8Srgb
    );
    let texels: Vec<_> = image.levels[0].as_chunks::<4>().0.iter().collect();
    assert_eq!(texels.len(), 4);
    assert_eq!(texels.iter().filter(|p| p[3] == 0).count(), 1);
    assert_eq!(texels.iter().filter(|p| p[3] == 255).count(), 3);
    assert_eq!(
        texels
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        4
    );
    let mut bindings = model.metadata.mesh_materials.clone();
    bindings.sort();
    assert_eq!(bindings, [Some(0), Some(1)]);
    for (mesh, material) in model.meshes.iter().zip(&model.metadata.mesh_materials) {
        let z = if *material == Some(0) { 0. } else { 0.5 };
        assert_eq!(mesh.indices.len(), 6);
        assert!(mesh.vertices.iter().all(|v| v[2] == z && v[5] == 1.));
    }
    temp.close().unwrap(); // Both GPU routes use only the same admitted cooked data.
    let node = |id: &str, world| SceneNode {
        id: id.into(),
        world,
        camera: None,
        light: None,
        mesh: None,
        sprite: None,
        tilemap: None,
        diagnostic: false,
    };
    let mut geometry = node("receiver-and-masked-occluder", DMat4::IDENTITY);
    geometry.mesh = Some(MeshInput {
        source,
        cast_shadows: false,
        animation: None,
        animation_blend: None,
        animation_pose: None,
        animation_ik: None,
        animation_root: None,
    });
    let mut camera = node("camera", DMat4::from_translation(DVec3::new(0., 0., 4.)));
    camera.camera = Some(incant_types::Camera {
        fov_degrees: 60.,
        near: 0.1,
        far: 10.,
        projection: incant_types::CameraProjection::Orthographic { vertical_size: 4. },
    });
    let mut no_key = node("explicit-zero-key", DMat4::IDENTITY);
    no_key.light = Some(SceneLight::Directional(incant_types::DirectionalLight {
        color: [1.; 3],
        intensity: 0.,
        shadows: None,
    }));
    (
        SceneInput {
            nodes: vec![geometry, camera, no_key],
            ..Default::default()
        },
        cooked.metadata.fingerprint,
    )
}

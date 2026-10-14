//! Engine-owned numerical fixture, never game art or appearance approval.
use crate::{SceneInput, hdr_output};
use glam::{DMat4, DQuat, DVec3};
use incant_assets::{AssetStore, cook_gltf, cook_texture};
use incant_doc::{
    Asset, AssetImportSettings, Entity, MeshRenderer, Project, Scene, TextureUsage, Transform,
    new_id,
};
use incant_render_data::SceneNode;
use serde_json::json;
use std::path::Path;

pub(super) fn node(id: &str, world: DMat4, diagnostic: bool) -> SceneNode {
    SceneNode {
        id: id.into(),
        world,
        camera: None,
        light: None,
        mesh: None,
        sprite: None,
        tilemap: None,
        diagnostic,
    }
}
pub(super) fn camera(pixel_grid: bool) -> SceneNode {
    let mut camera = node(
        "camera",
        DMat4::from_translation(DVec3::new(0., 0., 8.)),
        false,
    );
    camera.camera = Some(incant_types::Camera {
        fov_degrees: 60.,
        near: 0.1,
        far: 30.,
        projection: if pixel_grid {
            incant_types::CameraProjection::PixelPerfect {
                pixels_per_unit: 16.,
                reference_resolution: [160, 96],
            }
        } else {
            incant_types::CameraProjection::Orthographic { vertical_size: 6. }
        },
    });
    camera
}
/// Isolated ordinary-editor diagnostic cube, large enough to inspect its
/// silhouette separately from small mixed-pass correctness fixtures.
pub(super) fn cube() -> SceneInput {
    let mut camera = camera(false);
    camera.world = DMat4::from_translation(DVec3::new(0., 0., 4.));
    camera.camera.as_mut().unwrap().projection = incant_types::CameraProjection::Perspective {};
    camera.camera.as_mut().unwrap().fov_degrees = 50.;
    SceneInput {
        nodes: vec![
            camera,
            node(
                "cube",
                DMat4::from_scale_rotation_translation(
                    DVec3::splat(1.5),
                    DQuat::from_euler(glam::EulerRot::YXZ, 0.45, -0.25, 0.12),
                    DVec3::ZERO,
                ),
                true,
            ),
        ],
        text: None,
        environment: None,
    }
}
/// Three separate primitive/material draws: opaque, MASK and BLEND.
/// All non-visual source files disappear before the returned input is rendered.
pub(super) fn mixed(mask_alpha: f32) -> SceneInput {
    let root = tempfile::tempdir().unwrap();
    let mut positions = Vec::new();
    for x in [-2., 0., 2.] {
        positions.extend([x - 0.8, -2., 0., x + 0.8, -2., 0., x, -0.2, 0.]);
    }
    let bytes: Vec<_> = positions.into_iter().flat_map(f32::to_le_bytes).collect();
    std::fs::write(root.path().join("mesh.bin"), bytes).unwrap();
    let gltf = json!({"asset":{"version":"2.0"},"buffers":[{"uri":"mesh.bin","byteLength":108}],
        "bufferViews":[{"buffer":0,"byteLength":108}],
        "accessors": (0..3).map(|i| json!({"bufferView":0,"byteOffset":i*36,"componentType":5126,"count":3,"type":"VEC3",
            "min":[i as f32*2.-2.8,-2.,0.],"max":[i as f32*2.-1.2,-0.2,0.]})).collect::<Vec<_>>(),
        "materials":[
            {"pbrMetallicRoughness":{"baseColorFactor":[0,0,0,1],"metallicFactor":1},"emissiveFactor":[1,0,0]},
            {"pbrMetallicRoughness":{"baseColorFactor":[0,0,0,mask_alpha],"metallicFactor":1},"emissiveFactor":[0,1,0],"alphaMode":"MASK","alphaCutoff":0.5},
            {"pbrMetallicRoughness":{"baseColorFactor":[0,0,0,0.5],"metallicFactor":1},"emissiveFactor":[0,0,1],"alphaMode":"BLEND"}],
        "meshes":[{"primitives":(0..3).map(|i| json!({"attributes":{"POSITION":i},"material":i})).collect::<Vec<_>>() }],
        "nodes":[{"mesh":0}],"scenes":[{"nodes":[0]}],"scene":0});
    std::fs::write(
        root.path().join("mesh.gltf"),
        serde_json::to_vec(&gltf).unwrap(),
    )
    .unwrap();
    let cooked = cook_gltf(
        root.path(),
        Path::new("mesh.gltf"),
        &root.path().join(".incant/cache/models"),
    )
    .unwrap();
    let mut project = Project::empty("MSAA pass fixture");
    let model_id = new_id();
    project.assets.insert(
        model_id.clone(),
        Asset {
            id: model_id.clone(),
            name: "Numerical triangles".into(),
            path: "mesh.gltf".into(),
            kind: "model".into(),
            sha256: cooked.metadata.fingerprint,
            import_settings: None,
        },
    );
    let texture_id = new_id();
    let path = root.path().join("sprite.png");
    let mut png = png::Encoder::new(std::fs::File::create(&path).unwrap(), 1, 1);
    png.set_color(png::ColorType::Rgba);
    png.set_depth(png::BitDepth::Eight);
    png.write_header()
        .unwrap()
        .write_image_data(&[255, 255, 0, 160])
        .unwrap();
    let cooked = cook_texture(
        root.path(),
        Path::new("sprite.png"),
        &root.path().join(".incant/cache/textures"),
        TextureUsage::Color,
    )
    .unwrap();
    project.assets.insert(
        texture_id.clone(),
        Asset {
            id: texture_id.clone(),
            name: "Numerical sprite".into(),
            path: "sprite.png".into(),
            kind: "texture".into(),
            sha256: cooked.metadata.fingerprint,
            import_settings: Some(AssetImportSettings::Texture {
                usage: TextureUsage::Color,
            }),
        },
    );
    let mut scene = Scene::new("Scene");
    let mut mesh = Entity::new("Model");
    mesh.components
        .insert("Transform".into(), json!(Transform::default()));
    mesh.components.insert(
        "MeshRenderer".into(),
        json!(MeshRenderer {
            mesh: model_id,
            materials: vec![],
            cast_shadows: false
        }),
    );
    scene.entities.insert(mesh.id.clone(), mesh);
    let mut sprite = Entity::new("Sprite");
    sprite.components.insert(
        "Transform".into(),
        json!(Transform {
            translation: [-2., 1., 0.5],
            rotation: DQuat::from_rotation_z(0.3).to_array(),
            ..Default::default()
        }),
    );
    sprite.components.insert(
        "SpriteRenderer".into(),
        json!(incant_doc::SpriteRenderer {
            texture: texture_id,
            region: [0, 0, 1, 1].into(),
            size: [1.2, 1.2],
            pivot: [0.5, 0.5],
            tint: [1.; 4],
            flip_x: false,
            flip_y: false,
            layer: 0,
            order: 0,
            sampling: incant_doc::SpriteSampling::Linear,
            mode: incant_doc::SpriteMode::Simple
        }),
    );
    scene.entities.insert(sprite.id.clone(), sprite);
    project.scenes.insert(scene.id.clone(), scene);
    let mut assets = AssetStore::default();
    assets.sync_project(&project, root.path()).unwrap();
    let mut input = incant_render_authoring::project_input(&project, Some(&assets)).unwrap();
    input.nodes.push(camera(false));
    input.nodes.push(node(
        "cube",
        DMat4::from_scale_rotation_translation(
            DVec3::splat(0.8),
            DQuat::from_rotation_y(0.4),
            DVec3::new(0., 1., 0.),
        ),
        true,
    ));
    input.text = Some(crate::text::tests::input(&[(
        1,
        "MSAA",
        28.,
        DMat4::from_scale_rotation_translation(
            DVec3::splat(3.),
            DQuat::from_rotation_z(0.12),
            DVec3::new(1.2, 1.3, 0.5),
        ),
    )]));
    drop(assets);
    drop(project);
    root.close().unwrap();
    input
}
pub(super) fn draw(
    renderer: &crate::Renderer,
    input: &SceneInput,
    size: [u32; 2],
    viewport: Option<crate::Viewport>,
) -> Vec<u8> {
    let prepared = renderer
        .prepare(input)
        .unwrap()
        .with_camera("camera")
        .unwrap();
    let output = hdr_output::texture(
        renderer,
        wgpu::TextureFormat::Rgba8UnormSrgb,
        size[0],
        size[1],
    );
    renderer.queue.submit([renderer
        .draw_scene(
            &prepared,
            &output.create_view(&Default::default()),
            output.format(),
            size[0],
            size[1],
            viewport,
        )
        .unwrap()]);
    hdr_output::read(renderer, &output)
}

/// Optional fresh immutable raw evidence, separate from appearance acceptance.
pub(super) fn record(case: &str, renderer: &crate::Renderer, pixels: &[u8], size: [u32; 2]) {
    use sha2::{Digest, Sha256};
    use std::io::Write as _;
    let Some(directory) = std::env::var_os("INCANT_SCENE_MSAA_EVIDENCE") else {
        return;
    };
    let directory = std::path::Path::new(&directory);
    std::fs::create_dir_all(directory).unwrap();
    assert_eq!(pixels.len(), (size[0] * size[1] * 4) as usize);
    let info = renderer.adapter.get_info();
    let changed: Vec<_> = pixels
        .as_chunks::<4>()
        .0
        .iter()
        .enumerate()
        .filter_map(|(i, p)| {
            (*p != [20, 21, 25, 255]).then_some([i as u32 % size[0], i as u32 / size[0]])
        })
        .collect();
    let bounds = (!changed.is_empty()).then(|| {
        [
            changed.iter().map(|p| p[0]).min().unwrap(),
            changed.iter().map(|p| p[1]).min().unwrap(),
            changed.iter().map(|p| p[0]).max().unwrap() + 1,
            changed.iter().map(|p| p[1]).max().unwrap() + 1,
        ]
    });
    let receipt = serde_json::json!({"case":case,"size":size,"rgba_bytes":pixels.len(),
        "rgba_sha256":format!("{:x}",Sha256::digest(pixels)),"raw_file":format!("{case}.rgba"),
        "configured_scene_samples":renderer.scene_sample_count(),"backend":format!("{:?}",info.backend),"adapter":info.name,
        "cooked_geometry":cfg!(feature="cooked-geometry"),"cooked_materials":cfg!(feature="cooked-materials"),
        "cooked_sprites":cfg!(feature="cooked-sprites"),"cooked_text":cfg!(feature="cooked-text"),
        "normalization":"row-major RGBA8 sRGB display output, row padding removed",
        "changed_from_clear_pixels":changed.len(),"changed_from_clear_bounds_exclusive":bounds,
        "scope":"numerical fixture pixels; no appearance/device/performance acceptance"});
    for (suffix, bytes) in [
        ("rgba", pixels.to_vec()),
        ("json", serde_json::to_vec_pretty(&receipt).unwrap()),
    ] {
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(directory.join(format!("{case}.{suffix}")))
            .unwrap()
            .write_all(&bytes)
            .unwrap();
    }
}

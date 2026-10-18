//! Known emission through ordinary PBR lighting, without the preview defaults.
use crate::SceneInput;
use incant_assets_runtime::{RuntimeAsset, RuntimeAssetData};
use incant_render_data::{EnvironmentInput, SceneLight, SceneNode};
use std::{path::Path, sync::Arc};

pub(super) fn scene() -> SceneInput {
    use crate::temporal_upscale::{raster_geometry as geometry, visibility_oracle::OCCLUDER};
    let mut input = geometry::fixture();
    let mut node = geometry::fixture_with(&[OCCLUDER], [1., 0., 0.], 2)
        .nodes
        .remove(0);
    node.id = "occluder".into();
    input.nodes.push(node);
    // No lights means a nonzero studio key. An explicit zero light disables it;
    // append after the original mesh/camera/occluder so their indices stay fixed.
    input.nodes.push(SceneNode {
        id: "zero-key".into(),
        world: glam::DMat4::IDENTITY,
        camera: None,
        light: Some(SceneLight::Directional(incant_types::DirectionalLight {
            color: [1.; 3],
            intensity: 0.,
            shadows: None,
        })),
        mesh: None,
        sprite: None,
        tilemap: None,
        diagnostic: false,
    });
    // None also means nonzero studio IBL. Use the real typed/cooked environment
    // route at zero intensity; black dielectric alone retains specular light.
    let root = tempfile::tempdir().unwrap();
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, 2, 1);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&[255; 8])
            .unwrap();
    }
    std::fs::write(root.path().join("environment.png"), bytes).unwrap();
    let cache = root.path().join("cache");
    let cooked = incant_assets::cook_texture(
        root.path(),
        Path::new("environment.png"),
        &cache,
        incant_assets::TextureUsage::Color,
    )
    .unwrap();
    let source = Arc::new(
        RuntimeAsset::load_texture(
            "emission-only-environment",
            1,
            &cache,
            &cooked.metadata.fingerprint,
            Some(incant_assets::TextureUsage::Color),
        )
        .unwrap(),
    );
    root.close().unwrap();
    input.environment = Some(EnvironmentInput {
        source,
        intensity: 0.,
        rotation_degrees: 0.,
    });
    input
}

#[test]
fn emission_fixture_resolves_dark_lighting_after_source_deletion() {
    let input = scene();
    assert_eq!(input.nodes[0].id, "mesh");
    assert_eq!(input.nodes[1].id, "camera");
    assert_eq!(input.nodes[2].id, "occluder");
    for (index, emission) in [(0, [1.; 3]), (2, [1., 0., 0.])] {
        let RuntimeAssetData::Model(model) =
            input.nodes[index].mesh.as_ref().unwrap().source.data()
        else {
            panic!("cooked model")
        };
        let material = &model.materials[0];
        assert_eq!(material.emissive, emission);
        assert_eq!(material.base_color, [0., 0., 0., 1.]);
        assert_eq!(material.metallic, 0.);
        assert_eq!(
            material.alpha_mode,
            incant_assets_runtime::AlphaMode::Opaque
        );
    }
    let resolved = crate::scene::resolve(&input).unwrap();
    let (lights, directional, shadows) = resolved.lights.finish();
    assert_eq!(directional, 1);
    assert_eq!(lights.len(), 1);
    assert_eq!(lights[0].radiance_inner[..3], [0.; 3]);
    assert!(shadows.is_empty());
    let environment = resolved.environment.unwrap();
    assert_eq!(environment.intensity, 0.);
    let RuntimeAssetData::Texture(texture) = environment.source.data() else {
        panic!("cooked environment")
    };
    let radiance = texture.radiance_texture().unwrap();
    radiance.validate().unwrap();
    assert_eq!((radiance.width, radiance.height), (2, 1));
    assert_eq!(&*radiance.levels[0], &[255; 8]);
}

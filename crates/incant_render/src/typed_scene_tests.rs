use super::*;
use incant_render_data::{EnvironmentInput, SceneLight, SceneNode};

fn node(id: &str) -> SceneNode {
    SceneNode {
        id: id.into(),
        world: glam::DMat4::IDENTITY,
        camera: None,
        light: None,
        mesh: None,
        sprite: None,
        tilemap: None,
        diagnostic: false,
    }
}

#[test]
fn retained_typed_model_input_outlives_authoring_and_source_files() {
    let temp = tempfile::tempdir().unwrap();
    let (project, assets, _) = crate::test_support::fixture(temp.path());
    let input = incant_render_authoring::project_input(&project, Some(&assets)).unwrap();
    drop(project);
    drop(assets);
    temp.close().unwrap();
    let scene = resolve(&input).unwrap();
    assert_eq!(scene.stats.primitive_instances, 2);
    assert_eq!(scene.stats.model_draw_calls, 1);
    let mut origins: Vec<_> = scene.models.values().next().unwrap().primitives[&(0, true)]
        .iter()
        .map(|i| i.world[3])
        .collect();
    origins.sort_by(|a, b| a[0].total_cmp(&b[0]));
    assert_eq!(origins, [[-2., 2., 0., 1.], [1., 2., 0., 1.]]);
}

#[test]
fn typed_input_checks_identity_projection_light_and_affine_transform_without_documents() {
    let mut input = SceneInput {
        nodes: vec![node("one"), node("one")],
        environment: None,
    };
    assert!(matches!(resolve(&input), Err(SceneError::Input(_))));
    input.nodes.pop();
    input.nodes[0].camera = Some(incant_types::Camera {
        fov_degrees: -60.,
        near: 0.1,
        far: 100.,
        projection: Default::default(),
    });
    assert!(matches!(resolve(&input), Err(SceneError::CameraProjection)));
    input.nodes[0].camera = None;
    input.nodes[0].light = Some(SceneLight::Point(incant_types::PointLight {
        color: [1.; 3],
        intensity: 1.,
        range: 0.,
    }));
    assert!(matches!(resolve(&input), Err(SceneError::Input(_))));
    input.nodes[0].light = None;
    input.nodes[0].world = glam::DMat4::perspective_rh(1., 1., 0.1, 100.);
    assert!(matches!(resolve(&input), Err(SceneError::TransformRange)));
    input.nodes[0].world = glam::DMat4::IDENTITY;
    assert!(resolve(&input).is_ok());
}

#[test]
fn typed_environment_rejects_normal_map_and_out_of_range_rotation_at_startup() {
    use incant_assets::{TextureUsage, cook_texture};
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("texture.png");
    let mut encoder = png::Encoder::new(std::fs::File::create(&path).unwrap(), 1, 1);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&[128, 128, 255, 255])
        .unwrap();
    let cache = temp.path().join("cache");
    let cooked = cook_texture(
        temp.path(),
        std::path::Path::new("texture.png"),
        &cache,
        TextureUsage::Normal,
    )
    .unwrap();
    let source = Arc::new(
        RuntimeAsset::load_texture(
            "normal",
            1,
            &cache,
            &cooked.metadata.fingerprint,
            Some(TextureUsage::Normal),
        )
        .unwrap(),
    );
    let mut input = SceneInput {
        nodes: vec![],
        environment: Some(EnvironmentInput {
            source,
            intensity: 1.,
            rotation_degrees: 0.,
        }),
    };
    assert!(matches!(
        resolve(&input),
        Err(SceneError::MissingEnvironment(_))
    ));
    let source = Arc::clone(&input.environment.as_ref().unwrap().source);
    let mut sprite_input = SceneInput {
        nodes: vec![node("sprite")],
        environment: None,
    };
    sprite_input.nodes[0].sprite = Some(incant_render_data::SpriteInput {
        source,
        sprite: serde_json::from_value(
            serde_json::json!({"texture":"normal","size":[1.,1.],"region":[0,0,1,1],"pivot":[0.5,0.5],"tint":[1.,1.,1.,1.]}),
        )
        .unwrap(),
    });
    assert!(matches!(
        resolve(&sprite_input),
        Err(SceneError::MissingSprite(_))
    ));
    input.environment.as_mut().unwrap().rotation_degrees = f64::MAX;
    assert!(matches!(resolve(&input), Err(SceneError::Input(_))));
}

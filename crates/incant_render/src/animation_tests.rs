use super::*;
#[path = "../../incant_assets/tests/skeletal/fixture.rs"]
mod fixture;
use incant_render_data::{SceneInput, SceneNode};

fn source(dir: &std::path::Path) -> Arc<RuntimeAsset> {
    fixture::fixture(dir);
    let cache = dir.join("cache");
    let cooked = incant_assets::cook_gltf(dir, std::path::Path::new("model.gltf"), &cache).unwrap();
    Arc::new(RuntimeAsset::load_model("skin", 1, &cache, &cooked.metadata.fingerprint).unwrap())
}
fn node(source: Arc<RuntimeAsset>, id: &str, seconds: f64) -> SceneNode {
    SceneNode {
        id: id.into(),
        world: DMat4::IDENTITY,
        camera: None,
        light: None,
        mesh: Some(MeshInput {
            source,
            cast_shadows: true,
            animation_blend: None,
            animation_pose: None,
            animation_ik: None,
            animation_root: None,
            animation: Some(incant_types::AnimationSample {
                clip: Some(0),
                seconds,
            }),
        }),
        sprite: None,
        tilemap: None,
        diagnostic: false,
    }
}
#[test]
fn source_deleted_independent_instances_deform_full_checked_vertices() {
    let dir = tempfile::tempdir().unwrap();
    let source = source(dir.path());
    let input = SceneInput {
        text: None,
        nodes: vec![
            node(Arc::clone(&source), "early", 0.),
            node(Arc::clone(&source), "late", 1.),
        ],
        environment: None,
    };
    std::fs::remove_file(dir.path().join("model.gltf")).unwrap();
    std::fs::remove_file(dir.path().join("model.bin")).unwrap();
    std::fs::remove_dir_all(dir.path().join("cache")).unwrap();
    let resolved = crate::scene::resolve(&input).unwrap();
    assert_eq!(resolved.stats.model_entities, 2);
    assert_eq!(resolved.stats.primitive_instances, 4);
    assert_eq!(resolved.animations.len(), 2);
    let model = model(&source);
    let mut first = vec![[0.; 12]; model.meshes[0].vertices.len()];
    let mut second = first.clone();
    let early = &resolved.animations[0].player;
    let late = &resolved.animations[1].player;
    Geometry::fill(early, 3, 0, &model.meshes[0].vertices, &mut first).unwrap();
    Geometry::fill(late, 3, 0, &model.meshes[0].vertices, &mut second).unwrap();
    assert_ne!(first, second);
    for (i, source) in model.meshes[0].vertices.iter().enumerate() {
        let expected = late
            .skin_position(3, 0, i, [source[0], source[1], source[2]])
            .unwrap();
        for (a, b) in second[i][..3].iter().zip(expected) {
            assert!((f64::from(*a) - b).abs() < 1e-6);
        }
        assert_eq!(&second[i][6..8], &source[6..8]);
    }
    // The second mesh node has a negative scale but references the same skin;
    // model-space deformation ignores that mesh node, as does the reference.
    Geometry::fill(late, 4, 0, &model.meshes[0].vertices, &mut first).unwrap();
    assert_eq!(first, second);
    let allocations = crate::retained_alloc::measured(|| {
        for _ in 0..64 {
            Geometry::fill(late, 3, 0, &model.meshes[0].vertices, &mut first).unwrap();
        }
    });
    assert_eq!(allocations, 0);
}
#[test]
fn aggregate_charge_refuses_before_pose_copy_or_geometry_expansion() {
    let dir = tempfile::tempdir().unwrap();
    let source = source(dir.path());
    let node = node(source, "mesh", 0.);
    let mut stats = SceneStats::default();
    let mut bytes = MAX_RETAINED_ANIMATION_BYTES;
    let allocations = crate::retained_alloc::measured(|| {
        assert!(matches!(
            resolve(
                0,
                DMat4::IDENTITY,
                node.mesh.as_ref().unwrap(),
                &mut stats,
                &mut bytes
            ),
            Err(SceneError::Input("retained animation byte limit"))
        ));
    });
    assert_eq!(allocations, 0);
    assert_eq!(stats.primitive_instances, 0);
}
#[test]
fn invalid_samples_refuse_before_publication() {
    let dir = tempfile::tempdir().unwrap();
    let source = source(dir.path());
    let mut input = SceneInput {
        text: None,
        nodes: vec![node(source, "mesh", f64::NAN)],
        environment: None,
    };
    assert!(crate::scene::resolve(&input).is_err());
    input.nodes[0].mesh.as_mut().unwrap().animation = Some(incant_types::AnimationSample {
        clip: Some(u16::MAX),
        seconds: 0.,
    });
    assert!(crate::scene::resolve(&input).is_err());
}

#[test]
fn clip_only_model_retains_rigid_node_hierarchy_and_static_models_reject_samples() {
    let dir = tempfile::tempdir().unwrap();
    let mut doc = fixture::fixture(dir.path());
    doc.as_object_mut().unwrap().remove("skins");
    for n in [3, 4] {
        doc["nodes"][n].as_object_mut().unwrap().remove("skin");
        doc["nodes"][n].as_object_mut().unwrap().remove("mesh");
    }
    doc["nodes"][2]["mesh"] = serde_json::json!(0);
    for attribute in ["JOINTS_0", "WEIGHTS_0"] {
        doc["meshes"][0]["primitives"][0]["attributes"]
            .as_object_mut()
            .unwrap()
            .remove(attribute);
    }
    fixture::write(dir.path(), &doc);
    let cache = dir.path().join("cache");
    let cooked =
        incant_assets::cook_gltf(dir.path(), std::path::Path::new("model.gltf"), &cache).unwrap();
    let asset = Arc::new(
        RuntimeAsset::load_model("rigid", 0, &cache, &cooked.metadata.fingerprint).unwrap(),
    );
    let mut n = node(asset, "mesh", 2.);
    n.mesh.as_mut().unwrap().animation.as_mut().unwrap().clip = Some(2);
    let resolved = crate::scene::resolve(&SceneInput {
        text: None,
        nodes: vec![n],
        environment: None,
    })
    .unwrap();
    let plan = &resolved.animations[0];
    assert_eq!(plan.placements, vec![(2, 0)]);
    assert_eq!(
        plan.player.globals()[2]
            .transform_point3(glam::DVec3::X)
            .to_array(),
        [10., 2., 0.]
    );
    assert!(plan.player.globals()[2].determinant() < 0.);

    let registered = crate::test_support::model(dir.path(), 0.);
    let source = Arc::new(
        RuntimeAsset::load_model(
            "static",
            0,
            &dir.path().join(".incant/cache/models"),
            &registered.sha256,
        )
        .unwrap(),
    );
    let input = SceneInput {
        text: None,
        nodes: vec![node(source, "static", 0.)],
        environment: None,
    };
    assert!(matches!(
        crate::scene::resolve(&input),
        Err(SceneError::Input(
            "animation sample requires animated model"
        ))
    ));
}

fn camera() -> SceneNode {
    SceneNode {
        id: "camera".into(),
        world: DMat4::look_at_rh(
            glam::DVec3::new(12., 1., 8.),
            glam::DVec3::new(12., 1., 0.),
            glam::DVec3::Y,
        )
        .inverse(),
        camera: Some(incant_types::Camera {
            fov_degrees: 45.,
            near: 0.1,
            far: 50.,
            projection: Default::default(),
        }),
        light: None,
        mesh: None,
        sprite: None,
        tilemap: None,
        diagnostic: false,
    }
}
fn pixels(renderer: &Renderer, input: &SceneInput) -> Vec<u8> {
    let scene = renderer
        .prepare(input)
        .unwrap()
        .with_camera("camera")
        .unwrap();
    let output = crate::hdr_output::texture(renderer, wgpu::TextureFormat::Rgba8UnormSrgb, 160, 96);
    renderer.queue.submit([renderer
        .draw_scene(
            &scene,
            &output.create_view(&Default::default()),
            wgpu::TextureFormat::Rgba8UnormSrgb,
            160,
            96,
            None,
        )
        .unwrap()]);
    crate::hdr_output::read(renderer, &output)
}
// Independent mathematical static oracle for fixture's Z-rotation clip. This
// writes only source test geometry, through the existing static glTF cooker.
fn oracle(dir: &std::path::Path, seconds: f64) -> Arc<RuntimeAsset> {
    let angle = std::f64::consts::FRAC_PI_2 * seconds;
    let (s, c) = angle.sin_cos();
    let positions = [
        11f32,
        0.,
        0.,
        (11. + c + s) as f32,
        (1. + s - c) as f32,
        0.,
        11.,
        1.,
        0.,
    ];
    std::fs::write(
        dir.join("oracle.bin"),
        positions
            .into_iter()
            .flat_map(f32::to_le_bytes)
            .collect::<Vec<_>>(),
    )
    .unwrap();
    let doc = serde_json::json!({"asset":{"version":"2.0"},"buffers":[{"uri":"oracle.bin","byteLength":36}],"bufferViews":[{"buffer":0,"byteLength":36}],"accessors":[{"bufferView":0,"componentType":5126,"type":"VEC3","count":3,"min":[11,0,0],"max":[13,3,0]}],"meshes":[{"primitives":[{"attributes":{"POSITION":0}}]}],"nodes":[{"mesh":0},{"mesh":0}],"scenes":[{"nodes":[0,1]}],"scene":0});
    std::fs::write(dir.join("oracle.gltf"), doc.to_string()).unwrap();
    let cache = dir.join("oracle-cache");
    let model = incant_assets::cook_gltf(dir, std::path::Path::new("oracle.gltf"), &cache).unwrap();
    Arc::new(RuntimeAsset::load_model("oracle", 0, &cache, &model.metadata.fingerprint).unwrap())
}

#[test]
#[ignore = "requires a native GPU; coordinate numerical GPU ownership"]
fn retained_skin_matches_independent_static_oracle_and_preserves_rejected_frame() {
    let dir = tempfile::tempdir().unwrap();
    let source = source(dir.path());
    let mut input = SceneInput {
        text: None,
        nodes: vec![node(source, "mesh", 0.), camera()],
        environment: None,
    };
    input.nodes[0]
        .mesh
        .as_mut()
        .unwrap()
        .animation
        .as_mut()
        .unwrap()
        .clip = Some(1);
    let renderer = Renderer::headless().unwrap();
    let mut retained = renderer.prepare_retained(&input, "camera").unwrap();
    let output =
        crate::hdr_output::texture(&renderer, wgpu::TextureFormat::Rgba8UnormSrgb, 160, 96);
    let view = output.create_view(&Default::default());
    let target = |format| super::super::RetainedTarget {
        view: &view,
        format,
        width: 160,
        height: 96,
        viewport: None,
    };
    std::fs::remove_file(dir.path().join("model.gltf")).unwrap();
    std::fs::remove_file(dir.path().join("model.bin")).unwrap();
    std::fs::remove_dir_all(dir.path().join("cache")).unwrap();
    let mut original = None;
    let mut changed = false;
    for seconds in [0., 0.2, 0.5, 0.8, 1.] {
        input.nodes[0].world = if seconds > 0.5 {
            DMat4::from_translation(glam::DVec3::new(24., 0., 0.))
                * DMat4::from_scale(glam::DVec3::new(-1., 1., 1.))
        } else {
            DMat4::IDENTITY
        };
        input.nodes[0]
            .mesh
            .as_mut()
            .unwrap()
            .animation
            .as_mut()
            .unwrap()
            .seconds = seconds;
        retained
            .submit(
                &renderer,
                &input,
                target(wgpu::TextureFormat::Rgba8UnormSrgb),
            )
            .unwrap();
        let actual = crate::hdr_output::read(&renderer, &output);
        let mut expected = node(oracle(dir.path(), seconds), "oracle", 0.);
        expected.world = input.nodes[0].world;
        expected.mesh.as_mut().unwrap().animation = None;
        let expected = pixels(
            &renderer,
            &SceneInput {
                text: None,
                nodes: vec![expected, camera()],
                environment: None,
            },
        );
        assert_eq!(actual, expected, "sample {seconds}");
        if let Some(first) = &original {
            changed |= *first != actual;
        } else {
            original = Some(actual.clone());
        }
    }
    assert!(changed, "nonvacuous animation");
    let before = crate::hdr_output::read(&renderer, &output);
    input.nodes[0]
        .mesh
        .as_mut()
        .unwrap()
        .animation
        .as_mut()
        .unwrap()
        .seconds = 0.3;
    assert!(
        retained
            .submit(&renderer, &input, target(wgpu::TextureFormat::Rgba32Float))
            .is_err()
    );
    assert_eq!(crate::hdr_output::read(&renderer, &output), before);
    input.nodes[0]
        .mesh
        .as_mut()
        .unwrap()
        .animation
        .as_mut()
        .unwrap()
        .clip = Some(u16::MAX);
    assert!(
        retained
            .submit(
                &renderer,
                &input,
                target(wgpu::TextureFormat::Rgba8UnormSrgb)
            )
            .is_err()
    );
    assert_eq!(crate::hdr_output::read(&renderer, &output), before);
    input.nodes[0].mesh.as_mut().unwrap().animation = Some(incant_types::AnimationSample {
        clip: Some(1),
        seconds: 0.3,
    });
    let allocations = crate::retained_alloc::measured(|| {
        for _ in 0..64 {
            retained.stage(&input).unwrap();
        }
    });
    assert_eq!(allocations, 0);
    retained
        .submit(
            &renderer,
            &input,
            target(wgpu::TextureFormat::Rgba8UnormSrgb),
        )
        .unwrap();
    assert_eq!(
        crate::hdr_output::read(&renderer, &output),
        pixels(&renderer, &input)
    );
}

#[test]
fn uniform_skin_reflection_is_retained_and_mixed_vertex_orientation_refuses() {
    let dir = tempfile::tempdir().unwrap();
    let asset = source(dir.path());
    let model = model(&asset);
    let mut player = AnimationPlayer::new(model).unwrap();
    let mut out = vec![[0.; 12]; model.meshes[0].vertices.len()];
    player.sample(Some(2), 2.).unwrap();
    let (_, _, mirrored) =
        Geometry::fill(&player, 3, 0, &model.meshes[0].vertices, &mut out).unwrap();
    assert!(mirrored);
    player.sample(Some(0), 0.).unwrap();
    assert!(
        !Geometry::fill(&player, 3, 0, &model.meshes[0].vertices, &mut out)
            .unwrap()
            .2
    );

    let mut doc = fixture::fixture(dir.path());
    // Only the child joint reflects. Expanded vertices now have different signs.
    doc["animations"][2]["channels"][0]["target"]["node"] = serde_json::json!(2);
    fixture::write(dir.path(), &doc);
    let model = incant_assets::cook_gltf(
        dir.path(),
        std::path::Path::new("model.gltf"),
        &dir.path().join("mixed"),
    )
    .unwrap();
    let mut player = AnimationPlayer::new(&model).unwrap();
    player.sample(Some(2), 2.).unwrap();
    assert!(
        Geometry::fill(&player, 3, 0, &model.meshes[0].vertices, &mut out)
            .unwrap_err()
            .to_string()
            .contains("mixed skin orientation")
    );
}

fn static_mesh(
    dir: &std::path::Path,
    name: &str,
    positions: &[[f32; 3]],
    count: usize,
) -> Arc<RuntimeAsset> {
    let bytes: Vec<_> = positions
        .iter()
        .flatten()
        .flat_map(|v| v.to_le_bytes())
        .collect();
    std::fs::write(dir.join(format!("{name}.bin")), &bytes).unwrap();
    let doc = serde_json::json!({"asset":{"version":"2.0"},"buffers":[{"uri":format!("{name}.bin"),"byteLength":bytes.len()}],"bufferViews":[{"buffer":0,"byteLength":bytes.len()}],"accessors":[{"bufferView":0,"componentType":5126,"type":"VEC3","count":positions.len(),"min":[-100,-100,-100],"max":[100,100,100]}],"meshes":[{"primitives":[{"attributes":{"POSITION":0}}]}],"nodes":(0..count).map(|_|serde_json::json!({"mesh":0})).collect::<Vec<_>>(),"scenes":[{"nodes":(0..count).collect::<Vec<_>>()}],"scene":0});
    let path = format!("{name}.gltf");
    std::fs::write(dir.join(&path), doc.to_string()).unwrap();
    let cache = dir.join(format!("{name}-cache"));
    let model = incant_assets::cook_gltf(dir, std::path::Path::new(&path), &cache).unwrap();
    Arc::new(RuntimeAsset::load_model(name, 0, &cache, &model.metadata.fingerprint).unwrap())
}
fn shadow_light() -> SceneNode {
    let mut node = camera();
    node.id = "key".into();
    node.camera = None;
    node.world = DMat4::from_rotation_y(0.6);
    node.light = Some(incant_render_data::SceneLight::Directional(
        incant_types::DirectionalLight {
            color: [1.; 3],
            intensity: 2.,
            shadows: Some(incant_types::DirectionalShadows { distance: 30. }),
        },
    ));
    node
}
#[test]
#[ignore = "requires a native GPU; coordinate numerical GPU ownership"]
fn single_sided_reflected_skin_and_shadow_match_static_transform_oracle() {
    let dir = tempfile::tempdir().unwrap();
    let asset = source(dir.path());
    let raw = static_mesh(
        dir.path(),
        "raw",
        &[[1., 1., 0.], [1., 0., 0.], [2., 0., 0.]],
        2,
    );
    let receiver = static_mesh(
        dir.path(),
        "receiver",
        &[
            [8., -2., -0.5],
            [15., -2., -0.5],
            [15., 4., -0.5],
            [8., -2., -0.5],
            [15., 4., -0.5],
            [8., 4., -0.5],
        ],
        1,
    );
    let mut floor = node(receiver, "receiver", 0.);
    floor.mesh.as_mut().unwrap().animation = None;
    floor.mesh.as_mut().unwrap().cast_shadows = false;
    let mut input = SceneInput {
        text: None,
        nodes: vec![node(asset, "skin", 2.), camera(), shadow_light(), floor],
        environment: None,
    };
    input.nodes[0]
        .mesh
        .as_mut()
        .unwrap()
        .animation
        .as_mut()
        .unwrap()
        .clip = Some(2);
    dir.close().unwrap();
    let renderer = Renderer::headless().unwrap();
    let mut retained = renderer.prepare_retained(&input, "camera").unwrap();
    let output =
        crate::hdr_output::texture(&renderer, wgpu::TextureFormat::Rgba8UnormSrgb, 160, 96);
    let view = output.create_view(&Default::default());
    for outer_reflected in [false, true, false] {
        input.nodes[0].world = if outer_reflected {
            DMat4::from_translation(glam::DVec3::new(22., 0., 0.))
                * DMat4::from_scale(glam::DVec3::new(-1., 1., 1.))
        } else {
            DMat4::IDENTITY
        };
        retained
            .submit(
                &renderer,
                &input,
                super::super::RetainedTarget {
                    view: &view,
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    width: 160,
                    height: 96,
                    viewport: None,
                },
            )
            .unwrap();
        let actual = crate::hdr_output::read(&renderer, &output);
        let mut expected = node(Arc::clone(&raw), "oracle", 0.);
        expected.mesh.as_mut().unwrap().animation = None;
        expected.world = input.nodes[0].world
            * DMat4::from_translation(glam::DVec3::new(12., 0., 0.))
            * DMat4::from_scale(glam::DVec3::new(-1., 2., 1.));
        let mut floor = node(
            Arc::clone(&input.nodes[3].mesh.as_ref().unwrap().source),
            "receiver",
            0.,
        );
        floor.mesh.as_mut().unwrap().animation = None;
        floor.mesh.as_mut().unwrap().cast_shadows = false;
        let mut oracle = SceneInput {
            text: None,
            nodes: vec![expected, camera(), shadow_light(), floor],
            environment: None,
        };
        assert_eq!(
            actual,
            pixels(&renderer, &oracle),
            "outer reflected {outer_reflected}"
        );
        oracle.nodes[0].mesh.as_mut().unwrap().cast_shadows = false;
        assert_ne!(
            actual,
            pixels(&renderer, &oracle),
            "shadow must affect receiver"
        );
        oracle.nodes.remove(0);
        assert_ne!(
            actual,
            pixels(&renderer, &oracle),
            "single-sided skin must be visible"
        );
    }
}

#[path = "animation_crossfade_tests.rs"]
mod crossfade;

#[test]
fn weighted_tree_consumer_uses_full_local_pose_and_refuses_inconsistent_bindings() {
    use incant_types::{AnimationPoseSamples, AnimationSample, WeightedAnimationSample};
    let dir = tempfile::tempdir().unwrap();
    let source = source(dir.path());
    let entries = [
        WeightedAnimationSample {
            sample: AnimationSample {
                clip: Some(0),
                seconds: 1.0,
            },
            weight: 0.25,
        },
        WeightedAnimationSample {
            sample: AnimationSample::REST,
            weight: 0.75,
        },
    ];
    let pose = AnimationPoseSamples::new(&entries).unwrap();
    let mut input = SceneInput {
        text: None,
        nodes: vec![node(source.clone(), "weighted", 1.0)],
        environment: None,
    };
    input.nodes[0].mesh.as_mut().unwrap().animation_pose = Some(pose);
    let resolved = crate::scene::resolve(&input).unwrap();
    let player = &resolved.animations[0].player;
    // Clip zero translates joint1 by +1 X at t=1. Rest contributes 75%.
    let v = model(&source).meshes[0].vertices[0];
    let actual = player.skin_position(3, 0, 0, [v[0], v[1], v[2]]).unwrap();
    assert!((actual[0] - 11.25).abs() < 1e-6);
    let mesh = input.nodes[0].mesh.as_mut().unwrap();
    let mut reusable = AnimationPlayer::new(model(&source)).unwrap();
    sample(&mut reusable, mesh).unwrap();
    let allocations = crate::retained_alloc::measured(|| {
        for _ in 0..64 {
            sample(&mut reusable, mesh).unwrap();
        }
    });
    assert_eq!(allocations, 0);
    let before = reusable.globals().to_vec();
    mesh.animation = Some(AnimationSample::REST);
    assert!(sample(&mut reusable, mesh).is_err());
    assert_eq!(reusable.globals(), before);
    mesh.animation = Some(entries[0].sample);
    mesh.animation_blend = Some(incant_types::AnimationBlendSample {
        sample: AnimationSample::REST,
        weight: 0.5,
    });
    assert!(sample(&mut reusable, mesh).is_err());
    assert_eq!(reusable.globals(), before);
}

#[path = "animation_ik_tests.rs"]
mod ik;

#[path = "animation_grouped_tests.rs"]
mod grouped;

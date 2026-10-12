use super::*;
fn constraints() -> Arc<[incant_types::AnimationIkSample]> {
    Arc::from([incant_types::AnimationIkSample {
        id: incant_types::StableId([3; 16]),
        chain: [0, 1, 2],
        target: [11., 1., 0.],
        pole: [10., 1., 0.],
        weight: 1.,
    }])
}
#[test]
fn canonical_resolve_applies_ik_after_pose_and_deforms_both_source_deleted_meshes() {
    let dir = tempfile::tempdir().unwrap();
    let source = source(dir.path());
    let mut mesh = node(source.clone(), "rig", 0.);
    let binding = mesh.mesh.as_mut().unwrap();
    binding.animation = Some(incant_types::AnimationSample::REST);
    binding.animation_ik = Some(constraints());
    let input = SceneInput {
        text: None,
        nodes: vec![mesh],
        environment: None,
    };
    dir.close().unwrap();
    let result = crate::scene::resolve(&input).unwrap();
    let p = &result.animations[0].player;
    assert!(
        p.globals()[2]
            .w_axis
            .truncate()
            .abs_diff_eq(glam::DVec3::new(11., 1., 0.), 1e-9)
    );
    let source_model = model(&source);
    let mut a = vec![[0.; 12]; source_model.meshes[0].vertices.len()];
    let mut b = a.clone();
    Geometry::fill(p, 3, 0, &source_model.meshes[0].vertices, &mut a).unwrap();
    Geometry::fill(p, 4, 0, &source_model.meshes[0].vertices, &mut b).unwrap();
    assert_eq!(a, b);
    assert!((a[2][0] - 10.).abs() < 1e-6);
    assert!(a[2][1].abs() < 1e-6);
    let mut scratch = AnimationPlayer::new(source_model).unwrap();
    let allocations = crate::retained_alloc::measured(|| {
        for _ in 0..256 {
            sample(&mut scratch, input.nodes[0].mesh.as_ref().unwrap()).unwrap();
            Geometry::fill(&scratch, 3, 0, &source_model.meshes[0].vertices, &mut a).unwrap();
        }
    });
    assert_eq!(allocations, 0);
}
#[test]
fn pose_dependent_reflection_or_degenerate_chain_refuses_canonical_candidate() {
    for change in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let mut doc = fixture::fixture(dir.path());
        if change {
            doc["nodes"][2]["translation"] = serde_json::json!([0, 0, 0]);
            fixture::write(dir.path(), &doc);
        }
        let cache = dir.path().join("cache");
        let cooked =
            incant_assets::cook_gltf(dir.path(), std::path::Path::new("model.gltf"), &cache)
                .unwrap();
        let source = Arc::new(
            RuntimeAsset::load_model("rig", 0, &cache, &cooked.metadata.fingerprint).unwrap(),
        );
        let mut mesh = node(source, "rig", 2.);
        let binding = mesh.mesh.as_mut().unwrap();
        binding.animation.as_mut().unwrap().clip = Some(2);
        binding.animation_ik = Some(constraints());
        let input = SceneInput {
            text: None,
            nodes: vec![mesh],
            environment: None,
        };
        assert!(matches!(
            crate::scene::resolve(&input),
            Err(SceneError::Input(
                "unsupported or degenerate sampled IK pose"
            ))
        ));
    }
}
#[test]
fn global_ik_budget_refuses_before_any_private_pose_allocation() {
    let dir = tempfile::tempdir().unwrap();
    let source = source(dir.path());
    let template = constraints()[0];
    let payload: Arc<[_]> = (0u16..32)
        .map(|n| incant_types::AnimationIkSample {
            id: incant_types::StableId((n as u128).to_be_bytes()),
            ..template
        })
        .collect();
    let input = SceneInput {
        text: None,
        nodes: (0..129)
            .map(|i| {
                let mut n = node(source.clone(), &format!("rig{i}"), 0.);
                n.mesh.as_mut().unwrap().animation_ik = Some(payload.clone());
                n
            })
            .collect(),
        environment: None,
    };
    let allocations = crate::retained_alloc::measured(|| {
        assert!(matches!(
            crate::scene::resolve(&input),
            Err(SceneError::Input("IK aggregate capacity"))
        ))
    });
    assert_eq!(allocations, 0);
}

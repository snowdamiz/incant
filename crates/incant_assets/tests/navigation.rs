use incant_assets::{CookedModel, Mesh, ModelMetadata, ModelNode, model_navigation_geometry};
#[test]
fn default_scene_hierarchy_mirrors_and_primitive_instances_match_geometry() {
    let floor = Mesh {
        vertices: [[-5., 0., -3.], [5., 0., -3.], [5., 0., 3.], [-5., 0., 3.]]
            .map(|p| {
                let mut v = [0.; 12];
                v[..3].copy_from_slice(&p);
                v[4] = 1.;
                v
            })
            .to_vec(),
        indices: vec![0, 2, 1, 0, 3, 2],
    };
    let mut model = CookedModel {
        metadata: ModelMetadata {
            fingerprint: "a".repeat(64),
            dependencies: vec![],
            nodes: vec![
                ModelNode {
                    name: None,
                    children: vec![1],
                    meshes: vec![],
                    transform: glam::Mat4::from_translation(glam::vec3(2., 3., 0.))
                        .to_cols_array_2d(),
                },
                ModelNode {
                    name: None,
                    children: vec![],
                    meshes: vec![0],
                    transform: glam::Mat4::from_scale(glam::vec3(-2., 1., 2.)).to_cols_array_2d(),
                },
                ModelNode {
                    name: None,
                    children: vec![],
                    meshes: vec![99],
                    transform: glam::Mat4::IDENTITY.to_cols_array_2d(),
                },
            ],
            scenes: vec![vec![2], vec![0]],
            default_scene: Some(1),
            materials: vec![],
            mesh_materials: vec![None],
            textures: vec![],
        },
        materials: vec![],
        meshes: vec![floor],
        images: vec![],
        cache_hit: true,
    };
    let geometry = model_navigation_geometry(&model).unwrap();
    assert_eq!(geometry.vertices.len(), 4);
    assert_eq!(geometry.vertices[0], [12., 3., -6.]);
    for face in &geometry.triangles {
        let [a, b, c] = face.map(|i| glam::Vec3::from_array(geometry.vertices[i as usize]));
        assert!(
            (b - a).cross(c - a).y > 0.,
            "mirrored instance lost walkable winding"
        );
    }
    model.metadata.nodes[0].meshes.push(0);
    assert_eq!(
        model_navigation_geometry(&model).unwrap().triangles.len(),
        4
    );
    model.metadata.nodes[1].children.push(0);
    assert!(model_navigation_geometry(&model).is_err());
    model.metadata.nodes[1].children.clear();
    model.metadata.default_scene = Some(0);
    assert!(model_navigation_geometry(&model).is_err());
}

//! Same cooked Stage and semantic node identity for all three image arms.
use super::{geometry::*, materials};
use crate::SceneInput;
use glam::{DMat4, DQuat, DVec3};
use incant_assets::cook_gltf;
use incant_doc::{Asset, Entity, MeshRenderer, TextureUsage, Transform};
use incant_render_authoring::project_input;
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::Path};
const CAMERA_AT: [f64; 3] = [0., 2.4, 9.5];
const CAMERA_PITCH: f64 = -0.16;
const MOVER_AT: [f64; 3] = [2.02, 1.05, 1.2];
/// Replaces the fixture's own model with the static stage and recooks it.
fn write_stage(f: &mut materials::Fixture) {
    let material = json!({"pbrMetallicRoughness":{"baseColorFactor":[0.62,0.62,0.6,1],"metallicFactor":0,"roughnessFactor":0.85}});
    let (bytes, document) = gltf(&stage_mesh(), "quad.bin", material);
    std::fs::write(f.root.path().join("quad.bin"), &bytes).unwrap();
    std::fs::write(
        f.root.path().join("quad.gltf"),
        serde_json::to_vec(&document).unwrap(),
    )
    .unwrap();
    f.cook();
}
/// Cooks one more model beside the fixture's own, optionally with a base
/// colour texture sampled trilinearly from its full mip chain.
fn add_model(
    f: &mut materials::Fixture,
    stem: &str,
    mesh: &Mesh,
    material: Value,
    texture: Option<(&[u8], u32, u32)>,
) -> String {
    let root = f.root.path().to_path_buf();
    let (bytes, mut document) = gltf(mesh, &format!("{stem}.bin"), material);
    std::fs::write(root.join(format!("{stem}.bin")), &bytes).unwrap();
    if let Some((pixels, width, height)) = texture {
        materials::write_png(&root.join(format!("{stem}.png")), pixels, width, height);
        document["images"] = json!([{"uri": format!("{stem}.png")}]);
        document["textures"] = json!([{"source":0,"sampler":0}]);
        document["samplers"] =
            json!([{"magFilter":9729,"minFilter":9987,"wrapS":33071,"wrapT":33071}]);
    }
    let file = format!("{stem}.gltf");
    std::fs::write(root.join(&file), serde_json::to_vec(&document).unwrap()).unwrap();
    let cooked = cook_gltf(&root, Path::new(&file), &root.join(".incant/cache/models")).unwrap();
    let id = incant_doc::new_id();
    f.project.assets.insert(
        id.clone(),
        Asset {
            id: id.clone(),
            name: stem.into(),
            path: file,
            kind: "model".into(),
            sha256: cooked.metadata.fingerprint,
            import_settings: None,
        },
    );
    id
}
fn insert(f: &mut materials::Fixture, entity: Entity) -> String {
    let id = entity.id.clone();
    f.project
        .scenes
        .values_mut()
        .next()
        .unwrap()
        .entities
        .insert(id.clone(), entity);
    id
}
fn mesh_entity(name: &str, asset: &str, cast_shadows: bool, transform: Transform) -> Entity {
    let mut entity = Entity::new(name);
    entity
        .components
        .insert("Transform".into(), json!(transform));
    entity.components.insert(
        "MeshRenderer".into(),
        json!(MeshRenderer {
            mesh: asset.into(),
            materials: vec![],
            cast_shadows
        }),
    );
    entity
}
fn sun_rotation(elevation_degrees: f64, azimuth_degrees: f64) -> [f64; 4] {
    (DQuat::from_rotation_y(azimuth_degrees.to_radians())
        * DQuat::from_rotation_x(-elevation_degrees.to_radians()))
    .to_array()
}

pub(super) struct Stage {
    pub fixture: materials::Fixture,
    pub camera: String,
    mover: String,
    roles: BTreeMap<String, String>,
}
impl Stage {
    pub fn new() -> Self {
        let mut f = materials::Fixture::new(json!({}));
        write_stage(&mut f);
        f.environment(&[34, 36, 44, 255].repeat(2), 2, 1, TextureUsage::Linear);
        let chart = sign_texture();
        let sign = add_model(
            &mut f,
            "sign",
            &sign_mesh(),
            json!({"pbrMetallicRoughness":{"baseColorTexture":{"index":0},"metallicFactor":0,"roughnessFactor":0.9}}),
            Some((&chart, 128, 128)),
        );
        let mover_asset = add_model(
            &mut f,
            "mover",
            &mover_mesh(),
            json!({"pbrMetallicRoughness":{"baseColorFactor":[0.75,0.12,0.08,1],"metallicFactor":0,"roughnessFactor":0.6}}),
            None,
        );
        let mut roles = BTreeMap::new();
        for e in f.project.scenes.values().next().unwrap().entities.values() {
            let role = if e.components.contains_key("MeshRenderer") {
                "static-stage"
            } else {
                "environment"
            };
            assert!(roles.insert(role.into(), e.id.clone()).is_none());
        }
        let mut camera = Entity::new("Review camera");
        camera.components.insert(
            "Camera".into(),
            json!({"fov_degrees":50.,"near":0.1,"far":120.}),
        );
        camera.components.insert(
            "Transform".into(),
            json!(Transform {
                translation: CAMERA_AT,
                rotation: DQuat::from_rotation_x(CAMERA_PITCH).to_array(),
                ..Default::default()
            }),
        );
        let camera = insert(&mut f, camera);
        roles.insert("camera".into(), camera.clone());
        let mut sun = Entity::new("Review sun");
        sun.components.insert(
            "DirectionalLight".into(),
            json!({"color":[1,0.96,0.9],"intensity":3.2,"shadows":{"distance":60}}),
        );
        sun.components.insert(
            "Transform".into(),
            json!(Transform {
                rotation: sun_rotation(38., 30.),
                ..Default::default()
            }),
        );
        roles.insert("sun".into(), insert(&mut f, sun));
        roles.insert(
            "detail-sign".into(),
            insert(
                &mut f,
                mesh_entity("Detail sign", &sign, false, Transform::default()),
            ),
        );
        let mover = insert(
            &mut f,
            mesh_entity(
                "Mover",
                &mover_asset,
                true,
                Transform {
                    translation: MOVER_AT,
                    ..Default::default()
                },
            ),
        );
        roles.insert("mover".into(), mover.clone());
        f.assets.sync_project(&f.project, f.root.path()).unwrap();
        Self {
            fixture: f,
            camera,
            mover,
            roles,
        }
    }
    pub fn input(&self) -> SceneInput {
        project_input(&self.fixture.project, Some(&self.fixture.assets)).unwrap()
    }
    pub fn pose(&self, input: &mut SceneInput, frame: u32) {
        let (camera, mover) = poses(frame);
        for (id, world) in [(&self.camera, camera), (&self.mover, mover)] {
            input.nodes.iter_mut().find(|n| &n.id == id).unwrap().world = world;
        }
    }
    pub fn bindings(&self, input: &SceneInput) -> Value {
        let nodes: Vec<_> = self.roles.iter().map(|(role, id)| {
            let node = input.nodes.iter().find(|n| &n.id == id).unwrap();
            json!({"role":role, "id":id, "world":node.world.to_cols_array(),
                "camera":node.camera, "diagnostic":node.diagnostic,
                "light":node.light.as_ref().map(|l| match l {
                    incant_render_data::SceneLight::Directional(v) => json!({"directional":v}),
                    incant_render_data::SceneLight::Point(v) => json!({"point":v}),
                    incant_render_data::SceneLight::Spot(v) => json!({"spot":v}),
                }),
                "mesh":node.mesh.as_ref().map(|m| json!({"asset":m.source.info(),
                    "version":format!("{:?}",m.source.version_id()), "cast_shadows":m.cast_shadows}))})
        }).collect();
        assert_eq!(nodes.len(), input.nodes.len());
        json!({"nodes":nodes, "environment":input.environment.as_ref().map(|e| json!({
            "asset":e.source.info(), "version":format!("{:?}", e.source.version_id()),
            "intensity":e.intensity, "rotation_degrees":e.rotation_degrees}))})
    }
}
/// The original Stage's pan/mover functions, with this experiment's explicit
/// stationary interval and one cut. No renderer-specific pose adjustment.
pub(super) fn poses(frame: u32) -> (DMat4, DMat4) {
    assert!((1..=32).contains(&frame));
    let camera_step = if frame <= 16 {
        1
    } else if frame <= 24 {
        frame - 2
    } else {
        frame + 18
    };
    let mover_step = if frame <= 16 { 1 } else { frame - 8 };
    let pan = f64::from(camera_step.saturating_sub(14)) * 0.05;
    let moved = f64::from(mover_step.saturating_sub(8)) * 0.12;
    (
        DMat4::from_rotation_translation(
            DQuat::from_rotation_x(CAMERA_PITCH),
            DVec3::new(CAMERA_AT[0] + pan, CAMERA_AT[1], CAMERA_AT[2]),
        ),
        DMat4::from_translation(DVec3::new(MOVER_AT[0] + moved, MOVER_AT[1], MOVER_AT[2])),
    )
}

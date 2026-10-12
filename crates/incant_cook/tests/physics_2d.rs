#![cfg(feature = "scene")]
#[path = "support/scene_format.rs"]
mod format;
use format::scene_format;
use incant_cook::{cook_scene, stable_id};
use incant_doc::{AngularVelocity2D, BodyMotion, Collider2D, RigidBody, Velocity};
use incant_doc::{Entity, Project, Scene, Transform, new_id};
use std::collections::BTreeMap;
fn fixture() -> (Project, String, String) {
    let mut p = Project::empty("Planar cook");
    p.settings.tick_rate = 64;
    let scene = new_id();
    let entity = new_id();
    p.scenes.insert(
        scene.clone(),
        Scene {
            program: None,
            id: scene.clone(),
            name: "2D".into(),
            entities: BTreeMap::from([(
                entity.clone(),
                Entity {
                    id: entity.clone(),
                    name: "Body".into(),
                    parent: None,
                    provenance: None,
                    components: BTreeMap::from([
                        (
                            "Transform".into(),
                            serde_json::to_value(Transform::default()).unwrap(),
                        ),
                        (
                            "RigidBody2D".into(),
                            serde_json::to_value(RigidBody {
                                motion: BodyMotion::Kinematic,
                                ..Default::default()
                            })
                            .unwrap(),
                        ),
                        (
                            "Collider2D".into(),
                            serde_json::to_value(Collider2D::default()).unwrap(),
                        ),
                        (
                            "AngularVelocity2D".into(),
                            serde_json::to_value(AngularVelocity2D { angular: 2. }).unwrap(),
                        ),
                        (
                            "Velocity".into(),
                            serde_json::to_value(Velocity {
                                linear: [4., 0., 0.],
                            })
                            .unwrap(),
                        ),
                    ]),
                },
            )]),
        },
    );
    (p, scene, entity)
}
#[test]
fn shared_authoring_cooks_v7_without_mutation_and_runtime_needs_no_project() {
    let (project, scene, entity) = fixture();
    let before = serde_json::to_vec(&project).unwrap();
    let bytes = cook_scene(&project, &scene).unwrap();
    assert_eq!(bytes, cook_scene(&project, &scene).unwrap());
    assert_eq!(serde_json::to_vec(&project).unwrap(), before);
    assert_eq!(&bytes[8..12], &scene_format(7).to_le_bytes());
    drop(project);
    let mut world = incant_runtime::NativeWorld::from_bytes(&bytes).unwrap();
    world.step().unwrap();
    let state = world.physics_2d_state(stable_id(&entity).unwrap()).unwrap();
    assert_eq!(state.translation, [0.0625, 0.]);
    assert!((state.rotation - 0.03125).abs() < 1e-6);
}
#[test]
fn authoring_and_cook_refuse_planar_domain_and_component_contract_violations() {
    let (project, scene, entity) = fixture();
    for change in 0..5 {
        let mut p = project.clone();
        let e = p
            .scenes
            .get_mut(&scene)
            .unwrap()
            .entities
            .get_mut(&entity)
            .unwrap();
        match change {
            0 => {
                e.components.remove("Collider2D");
            }
            1 => {
                e.components.get_mut("Velocity").unwrap()["linear"][2] = 1.into();
            }
            2 => {
                e.components.get_mut("Transform").unwrap()["scale"][0] = 2.into();
            }
            3 => {
                e.components.insert(
                    "Collider".into(),
                    serde_json::to_value(incant_doc::Collider::default()).unwrap(),
                );
            }
            _ => {
                e.components.get_mut("Collider2D").unwrap()["shape"]["half_extents"][0] = 0.into();
            }
        }
        assert!(p.validated().is_err(), "case{change}");
        assert!(cook_scene(&p, &scene).is_err());
    }
}

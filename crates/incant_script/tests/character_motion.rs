use incant_doc::{
    BodyMotion, Collider, ColliderShape, Entity, Project, RigidBody, Scene, Transform,
};
use incant_script::PlaySession;
use serde_json::json;

fn course(wall: bool) -> (Project, String, String) {
    let mut project = Project::empty("Continuous character travel");
    let mut scene = Scene::new("Course");
    for (id, name, position, shape) in [
        (
            "01JA2CHAR00000000000000000",
            "Floor",
            [0.4, -0.1, 0.],
            ColliderShape::Box {
                half_extents: [5.5, 0.1, 4.6],
            },
        ),
        (
            "01JA2CHAR00000000000000008",
            "Wall",
            [1., 0.5, -4.35],
            ColliderShape::Box {
                half_extents: [4., 0.5, 0.1],
            },
        ),
        (
            "01JA2CHAR0000000000000000H",
            "Player",
            [-3.5, 0.83, if wall { -3.25 } else { 0. }],
            ColliderShape::Capsule {
                half_height: 0.5,
                radius: 0.3,
            },
        ),
    ] {
        if name == "Wall" && !wall {
            continue;
        }
        let mut entity = Entity::new(name);
        entity.id = id.into();
        entity.components.insert(
            "Transform".into(),
            json!(Transform {
                translation: position,
                ..Default::default()
            }),
        );
        entity.components.insert(
            "Collider".into(),
            json!(Collider {
                shape,
                ..Default::default()
            }),
        );
        if name == "Player" {
            entity.components.insert(
                "RigidBody".into(),
                json!(RigidBody {
                    motion: BodyMotion::Kinematic,
                    gravity_scale: 0.,
                    can_sleep: false,
                    ..Default::default()
                }),
            );
            entity
                .components
                .insert("Velocity".into(), json!({"linear": [0., 0., 0.]}));
        }
        scene.entities.insert(entity.id.clone(), entity);
    }
    let scene_id = scene.id.clone();
    project.scenes.insert(scene.id.clone(), scene);
    (project, scene_id, "01JA2CHAR0000000000000000H".into())
}

#[test]
fn walking_and_wall_sliding_keep_tangential_speed_on_every_tick() {
    for compound in [false, true] {
        for (shape, height) in [
            (
                ColliderShape::Capsule {
                    half_height: 0.5,
                    radius: 0.3,
                },
                0.8,
            ),
            (ColliderShape::Sphere { radius: 0.3 }, 0.3),
            (
                ColliderShape::Box {
                    half_extents: [0.3, 0.8, 0.3],
                },
                0.8,
            ),
        ] {
            for wall in [false, true] {
                for autostep in [
                    "null",
                    "{max_height:0.25,min_width:0.15,include_dynamic_bodies:false}",
                ] {
                    let (mut project, scene, player) = course(wall);
                    if compound {
                        let entities = &mut project.scenes.get_mut(&scene).unwrap().entities;
                        let obstacles: Vec<_> = entities
                            .values()
                            .filter(|e| e.id != player)
                            .cloned()
                            .collect();
                        let parts: Vec<_> = obstacles
                            .iter()
                            .map(|e| incant_doc::ColliderPart {
                                id: e.id.clone(),
                                translation: serde_json::from_value(
                                    e.components["Transform"]["translation"].clone(),
                                )
                                .unwrap(),
                                rotation: [0., 0., 0., 1.],
                                shape: serde_json::from_value(
                                    e.components["Collider"]["shape"].clone(),
                                )
                                .unwrap(),
                            })
                            .collect();
                        for e in &obstacles {
                            entities.remove(&e.id);
                        }
                        let mut combined = Entity::new("Compound floor and wall");
                        combined
                            .components
                            .insert("Transform".into(), json!(Transform::default()));
                        combined.components.insert(
                            "Collider".into(),
                            json!(Collider {
                                shape: ColliderShape::Compound { parts },
                                ..Default::default()
                            }),
                        );
                        entities.insert(combined.id.clone(), combined);
                    }
                    let e = project
                        .scenes
                        .get_mut(&scene)
                        .unwrap()
                        .entities
                        .get_mut(&player)
                        .unwrap();
                    e.components.get_mut("Collider").unwrap()["shape"] = json!(shape);
                    e.components.get_mut("Transform").unwrap()["translation"][1] =
                        json!(height + 0.03);
                    let dx = if wall { 1.6 / 1.36_f64.sqrt() } else { 1.6 };
                    let dz = if wall { -0.6 * dx } else { 0. };
                    let source = format!(
                        r#"exports.default={{initialState:{{vy:0,delta:[0,0,0],grounded:false,sliding:false}},update(api,dt,state){{
                state.vy-=9.81*dt;
                const m=api.computeCharacterMotion({{scene_id:{scene:?},entity_id:{player:?},translation:[{dx}*dt,state.vy*dt,{dz}*dt],options:{{offset:0.02,slide:true,max_slope_climb_angle:Math.PI/4,min_slope_slide_angle:Math.PI/4,snap_to_ground:0.3,autostep:{autostep}}}}});
                if(m.grounded && state.vy<0)state.vy=0;
                state.delta=m.translation;state.grounded=m.grounded;state.sliding=m.sliding_down_slope;
                api.command({{op:'set_component',scene_id:{scene:?},entity_id:{player:?},component:'Velocity',value:{{linear:m.translation.map(v=>v/dt)}}}});
            }}}};"#
                    );
                    let mut play = PlaySession::new(&project, &source).unwrap();
                    for tick in 1..=300 {
                        play.tick().unwrap();
                        let state = play.host.state();
                        let delta = state["delta"][0].as_f64().unwrap();
                        assert!(
                            delta > dx / 60. * 0.95,
                            "compound={compound}, shape={shape:?}, wall={wall}, autostep={autostep}, tick={tick}: {state}"
                        );
                        assert_eq!(state["sliding"], false, "tick={tick}: {state}");
                        let position = play.snapshot().entities[&player].translation;
                        assert!(
                            position[1] > height + 0.01 && position[1] < height + 0.04,
                            "tick={tick}: {position:?}"
                        );
                        assert!(position[2] > -3.931, "wall penetration: {position:?}");
                    }
                }
            }
        }
    }
}

use crate::{Result, Template, TemplateError};
use incant_cmd::{Command, SourceVersion};
use incant_doc::{Asset, Entity, Id, Scene, ScriptSource, new_id};
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::BTreeMap;

const CHARACTER: &str = include_str!("../../../templates/builtin/scripts/character.ts");
const PLANAR: &str = include_str!("../../../templates/builtin/scripts/planar.ts");
pub const SPRITE_FRAMES: [&str; 3] = [
    "01JA42FR000000000000000001",
    "01JA42FR000000000000000002",
    "01JA42FR000000000000000003",
];
pub const TILE_FRAME: &str = "01JA42FR00000000000000000B";

/// Once prepared, identities and commands are explicit replayable data. Nothing
/// generates IDs while applying a command or redoing the transaction.
#[derive(Debug, Serialize)]
pub struct TemplatePlan {
    pub template: Template,
    pub roles: BTreeMap<String, Id>,
    pub commands: Vec<Command>,
}
fn transform(at: [f64; 3], scale: [f64; 3]) -> Value {
    json!({"translation":at,"rotation":[0,0,0,1],"scale":scale})
}
fn insert(scene: &mut Scene, ids: &mut BTreeMap<String, Id>, role: &str, components: Value) {
    let mut entity = Entity::new(role);
    entity.components = serde_json::from_value(components).expect("static component object");
    ids.insert(role.into(), entity.id.clone());
    scene.entities.insert(entity.id.clone(), entity);
}
fn mesh_child(
    scene: &mut Scene,
    ids: &mut BTreeMap<String, Id>,
    parent: &str,
    size: [f64; 3],
    asset: &Asset,
) {
    let role = format!("{parent}_visual");
    insert(
        scene,
        ids,
        &role,
        json!({
            "Transform":transform([0.;3],size),
            "MeshRenderer":{"mesh":asset.id,"materials":[],"cast_shadows":true}
        }),
    );
    scene
        .entities
        .get_mut(&ids[&role])
        .expect("inserted")
        .parent = Some(ids[parent].clone());
}
fn collider(shape: Value) -> Value {
    json!({"shape":shape,"density":1000,"friction":0,"restitution":0,
           "sensor":false,"memberships":u32::MAX,"filter":u32::MAX})
}
fn body(motion: &str, gravity: f64) -> Value {
    json!({"motion":motion,"gravity_scale":gravity,"linear_damping":0,
           "angular_damping":0,"can_sleep":false,"ccd":true})
}
fn actions(planar: bool) -> Value {
    let forward = if planar { 1 } else { -1 };
    json!({
        "move":{"kind":"axis2","dead_zone":0.15,"bindings":[
            {"type":"key","code":"KeyD","scale":[1,0]},
            {"type":"key","code":"KeyA","scale":[-1,0]},
            {"type":"key","code":"KeyW","scale":[0,forward]},
            {"type":"key","code":"KeyS","scale":[0,-forward]},
            {"type":"gamepad_stick","id":0,"stick":"left","invert_y":planar}]},
        "look":{"kind":"axis2","dead_zone":0.15,"bindings":[
            {"type":"key","code":"ArrowRight","scale":[1,0]},
            {"type":"key","code":"ArrowLeft","scale":[-1,0]},
            {"type":"key","code":"ArrowUp","scale":[0,1]},
            {"type":"key","code":"ArrowDown","scale":[0,-1]},
            {"type":"gamepad_stick","id":0,"stick":"right","invert_y":true}]},
        "jump":{"kind":"button","bindings":[{"type":"key","code":"Space"},
            {"type":"gamepad_button","id":0,"button":"south"}]}
    })
}

/// Prepare a new scene using a previously imported bundled model/atlas. This
/// does not mutate a document or write a source. The caller applies the returned
/// commands with its existing source-aware CommandBus and current revision.
pub fn plan(template: Template, asset: Option<&Asset>) -> Result<TemplatePlan> {
    match (template, asset) {
        (Template::Empty, None) => {}
        (Template::Empty, Some(_)) | (_, None) => return Err(TemplateError::Asset),
        (_, Some(asset)) => {
            let (kind, path) = if template.planar() {
                ("texture", "template-sprites.atlas.json")
            } else {
                ("model", "template-cube.gltf")
            };
            if asset.kind != kind || asset.path != path {
                return Err(TemplateError::Asset);
            }
        }
    }
    let mut scene = Scene::new("Main");
    let mut roles = BTreeMap::from([("scene".into(), scene.id.clone())]);
    let planar = template.planar();
    let at = if planar {
        [2., 3.5, 10.]
    } else {
        [0., 2.9, 6.]
    };
    let projection = if planar {
        json!({"kind":"orthographic","vertical_size":10})
    } else {
        json!({"kind":"perspective"})
    };
    insert(
        &mut scene,
        &mut roles,
        "camera",
        json!({"Transform":transform(at,[1.;3]),
        "Camera":{"fov_degrees":60,"near":0.1,"far":100,"projection":projection}}),
    );
    let mut commands = Vec::new();
    if template != Template::Empty {
        let asset = asset.expect("checked");
        if planar {
            planar_scene(template, &mut scene, &mut roles, asset);
        } else {
            spatial_scene(template, &mut scene, &mut roles, asset);
        }
        let script_id = new_id();
        roles.insert("script".into(), script_id.clone());
        let props = if planar {
            json!({"player":roles["player"],"camera":roles["camera"],
            "platformer":template==Template::Platformer2d})
        } else {
            json!({"player":roles["player"],"camera":roles["camera"],
            "firstPerson":template==Template::FirstPerson})
        };
        insert(
            &mut scene,
            &mut roles,
            "controller",
            json!({"Script":{"source":script_id,"props":props}}),
        );
        commands.extend([
            Command::UpsertScript {
                script: ScriptSource {
                    id: script_id.clone(),
                    name: "Starter controller".into(),
                    path: "controller.ts".into(),
                },
            },
            Command::WriteScript {
                script_id,
                expected_source: SourceVersion::Missing,
                source: if planar { PLANAR } else { CHARACTER }.into(),
            },
            Command::SetInputActions {
                actions: serde_json::from_value(actions(planar))?,
            },
        ]);
    }
    commands.push(Command::CreateScene { scene });
    Ok(TemplatePlan {
        template,
        roles,
        commands,
    })
}

fn spatial_scene(
    template: Template,
    scene: &mut Scene,
    ids: &mut BTreeMap<String, Id>,
    asset: &Asset,
) {
    insert(
        scene,
        ids,
        "player",
        json!({"Transform":transform([0.,1.2,0.],[1.;3]),
        "Velocity":{"linear":[0,0,0]},"RigidBody":body("kinematic",0.),
        "Collider":collider(json!({"type":"capsule","half_height":0.45,"radius":0.35}))}),
    );
    if template == Template::ThirdPerson {
        mesh_child(scene, ids, "player", [0.7, 1.6, 0.7], asset);
    }
    for (role, at, size) in [
        ("ground", [0., -0.5, 0.], [24., 1., 24.]),
        ("wall", [6., 1.5, 0.], [1., 3., 8.]),
        ("platform", [0., 0.5, -5.], [4., 1., 4.]),
    ] {
        insert(
            scene,
            ids,
            role,
            json!({"Transform":transform(at,[1.;3]),
            "Collider":collider(json!({"type":"box","half_extents":size.map(|x|x/2.)}))}),
        );
        mesh_child(scene, ids, role, size, asset);
    }
    insert(
        scene,
        ids,
        "sun",
        json!({"Transform":{
        "translation":[0,8,0],"rotation":[-0.3826834323650898,0,0,0.9238795325112867],"scale":[1,1,1]},
        "DirectionalLight":{"color":[1,1,1],"intensity":2,"shadows":null}}),
    );
}

fn planar_scene(
    template: Template,
    scene: &mut Scene,
    ids: &mut BTreeMap<String, Id>,
    asset: &Asset,
) {
    let platformer = template == Template::Platformer2d;
    let mut cells = vec![None; 24 * 16];
    for y in 0..16 {
        for x in 0..24 {
            let solid = if platformer {
                y == 0 || (y == 3 && (6..10).contains(&x)) || (x == 18 && y < 6)
            } else {
                x == 0 || x == 23 || y == 0 || y == 15 || (x == 10 && (1..11).contains(&y))
            };
            if solid {
                cells[y * 24 + x] = Some(TILE_FRAME);
            }
        }
    }
    let layer = new_id();
    ids.insert("tile_layer".into(), layer.clone());
    insert(
        scene,
        ids,
        "terrain",
        json!({"Transform":transform([0.;3],[1.;3]),
        "Tilemap":{"tileset":asset.id,"size":[24,16],"cell_size":[1,1],"layers":[
            {"id":layer,"cells":cells,"solid":true,"layer":0,"order":0}]}}),
    );
    insert(
        scene,
        ids,
        "player",
        json!({"Transform":transform([2.,2.,0.],[1.;3]),
        "Velocity":{"linear":[0,0,0]},"RigidBody2D":body("dynamic",if platformer {1.} else {0.}),
        "Collider2D":collider(json!({"kind":"box","half_extents":[0.4,0.45]})),
        "SpriteRenderer":{"texture":asset.id,"region":{"frame":SPRITE_FRAMES[0]},
            "size":[0.8,0.9],"pivot":[0.5,0.5],"tint":[1,1,1,1],"layer":1,"order":0,"sampling":"nearest"},
        "SpriteFlipbook":{"frames":SPRITE_FRAMES.map(|frame|json!({"frame":frame})),
            "rate":{"numerator":8,"denominator":1},"looping":true,"playing":false}}),
    );
}

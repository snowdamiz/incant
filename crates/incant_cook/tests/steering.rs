#![cfg(feature = "typescript")]
use incant_cook::{compile_project_sources, prepare_script_scene, stable_id};
use incant_doc::{Entity, Project, Scene, ScriptSource, new_id};
use serde_json::json;

#[test]
fn strict_authored_steering_cooks_and_moves_after_all_source_is_deleted() {
    let root = tempfile::tempdir().unwrap();
    let mut project = Project::empty("Native avoidance");
    project.settings.physics_mode = incant_runtime::PHYSICS_VARIANT.mode();
    let mut scene = Scene::new("Main");
    let mut entity = Entity::new("Mover");
    let source = new_id();
    entity
        .components
        .insert("Script".into(), json!({"source":source,"props":{}}));
    let entity_id = entity.id.clone();
    scene.entities.insert(entity_id.clone(), entity);
    let scene_id = scene.id.clone();
    project.scenes.insert(scene_id.clone(), scene);
    project.scripts.insert(
        source.clone(),
        ScriptSource {
            id: source,
            name: "Avoidance".into(),
            path: "main.ts".into(),
        },
    );
    let helper = include_str!("../../../sdk/templates/native-navigation/steering.ts")
        .replace("../../ts/src/native", "@incant/runtime/native");
    std::fs::write(root.path().join("avoid.ts"), helper).unwrap();
    std::fs::write(
        root.path().join("main.ts"),
        r#"
import type { NativeBehavior, NativeSteeringAgent } from '@incant/runtime/native';
import { avoid } from './avoid';
export default { update({instances,chunks,commands,navigation}) {
  for (const chunk of chunks) for (let i=0;i<chunk.ids.length;i++) {
    const id=chunk.ids[i];
    if (id !== instances[0].entity) continue;
    const agent: NativeSteeringAgent = {
      id, position: new Float64Array([chunk.transforms[i*10],0,0]),
      velocity: [0,0], preferred_velocity: new Float64Array([2,0]),
      radius:.25, height:1.8, max_speed:2, responsibility:1,
    };
    const out=avoid(navigation,[agent],[{
      id:'00000000000000000000000000', closed:false,
      vertices:[new Float64Array([0,20]), [10,20]], min_y:0, max_y:2,
    }]);
    commands.setVelocity(id,[out[0].velocity[0],0,out[0].velocity[1]]);
  }
} } satisfies NativeBehavior;
"#,
    )
    .unwrap();
    let before = project.canonical_text().unwrap();
    let prepared = prepare_script_scene(&project, &scene_id).unwrap();
    let compiled = compile_project_sources(prepared, root.path()).unwrap();
    assert_eq!(project.canonical_text().unwrap(), before);
    let (scene, script) = compiled.program.into_parts();
    drop(project);
    root.close().unwrap();
    let mut session = incant_script::NativeSession::new(scene, script, Default::default()).unwrap();
    // A session simulates first, then commits script commands for the next step.
    session.tick(&[]).unwrap();
    assert_eq!(
        session
            .world()
            .unwrap()
            .inspect(stable_id(&entity_id).unwrap())
            .unwrap()
            .transform
            .translation,
        [0.; 3]
    );
    for _ in 0..60 {
        session.tick(&[]).unwrap();
    }
    let position = session
        .world()
        .unwrap()
        .inspect(stable_id(&entity_id).unwrap())
        .unwrap()
        .transform
        .translation;
    assert!((position[0] - 2.).abs() < 1e-12);
    assert_eq!(position[1..], [0., 0.]);
}

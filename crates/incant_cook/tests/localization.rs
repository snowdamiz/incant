#![cfg(feature = "typescript")]
use incant_cook::{compile_project_sources, prepare_script_scene, stable_id};
use incant_doc::{Entity, Project, Scene, ScriptSource, new_id};
use serde_json::json;
const TABLE: &str = "00000000000000000000000002";
const SOURCE: &str = r#"
import type {NativeBehavior} from '@incant/runtime/native';
export default {update({chunks,instances,localization,commands,clock}) {
  const id=instances[0].entity;
  for(let i=0;i<(instances[0].props.exhaust?33:1);i++) localization.locale();
  const result=localization.localize({table_id:'00000000000000000000000002',key:'hello',arguments:{name:'Ada'}});
  const expected=clock.tick===1?'Hello Ada':'Bonjour Ada';
  if(result.text!==expected) throw new Error('uncommitted locale leaked across source groups');
  for(const chunk of chunks) {const i=chunk.ids.indexOf(id);if(i>=0)chunk.transforms[i*10]+=result.text.length;}
  if(clock.tick===1) commands.setLocale({locale:'fr',fallbacks:['en']});
  return undefined;
}} satisfies NativeBehavior;
"#;
fn project(exhaust: bool) -> (Project, String, Vec<String>) {
    let mut p = Project::empty("Native localization");
    let mut s = Scene::new("Main");
    let mut ids = vec![];
    p.string_tables.insert(TABLE.into(),serde_json::from_value(json!({"id":TABLE,"name":"Game","source_locale":"en","messages":{"hello":{"en":"Hello {name}","fr":"Bonjour {name}"}}})).unwrap());
    for index in 0..2 {
        let source = new_id();
        let mut e = Entity::new("Localized actor");
        e.components.insert(
            "Script".into(),
            json!({"source":source,"props":{"exhaust":exhaust}}),
        );
        ids.push(e.id.clone());
        s.entities.insert(e.id.clone(), e);
        p.scripts.insert(
            source.clone(),
            ScriptSource {
                id: source,
                name: format!("Behavior {index}"),
                path: format!("{index}.ts"),
            },
        );
    }
    let scene = s.id.clone();
    p.scenes.insert(scene.clone(), s);
    (p, scene, ids)
}
#[test]
fn actual_authored_modules_share_committed_locale_and_survive_source_deletion() {
    let root = tempfile::tempdir().unwrap();
    for n in 0..2 {
        std::fs::write(root.path().join(format!("{n}.ts")), SOURCE).unwrap();
    }
    let (project, scene, ids) = project(false);
    let before = project.canonical_text().unwrap();
    let program =
        compile_project_sources(prepare_script_scene(&project, &scene).unwrap(), root.path())
            .unwrap()
            .program;
    for n in 0..2 {
        std::fs::remove_file(root.path().join(format!("{n}.ts"))).unwrap();
    }
    let (scene, script) = program.into_parts();
    let mut session = incant_script::NativeSession::new(scene, script, Default::default()).unwrap();
    session.tick(&[]).unwrap();
    session.tick(&[]).unwrap();
    let world = session.world().unwrap();
    assert_eq!(world.localization().settings().locale, "fr");
    for id in ids {
        assert_eq!(
            world
                .inspect(stable_id(&id).unwrap())
                .unwrap()
                .transform
                .translation[0],
            20.
        );
    }
    assert_eq!(project.canonical_text().unwrap(), before);
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}
#[test]
fn localization_budget_and_rollback_span_every_authored_source_group() {
    let root = tempfile::tempdir().unwrap();
    for n in 0..2 {
        std::fs::write(root.path().join(format!("{n}.ts")), SOURCE).unwrap();
    }
    let (project, scene, _) = project(true);
    let authored = project.canonical_text().unwrap();
    let (scene, script) =
        compile_project_sources(prepare_script_scene(&project, &scene).unwrap(), root.path())
            .unwrap()
            .program
            .into_parts();
    let mut world = incant_runtime::NativeWorld::new(scene);
    let before = world.save_bytes().unwrap();
    let mut host = incant_script::ScriptHost::for_world(script, &world).unwrap();
    assert!(host.tick(&mut world, 1. / 60.).is_err());
    assert_eq!(world.save_bytes().unwrap(), before);
    assert_eq!(project.canonical_text().unwrap(), authored);
}

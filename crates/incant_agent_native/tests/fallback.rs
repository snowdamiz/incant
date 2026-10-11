//! Observable controls for trusted host fallback selection and candidate publication.
use incant_agent::{AgentError, EngineHost, NoViewport, play::RunRequest};
use incant_agent_native::Host;
use incant_doc::{Entity, Project, Scene, ScriptSource, Transform, new_id};
use incant_script::{CompiledScript, ScriptError};
use serde_json::json;
use std::{cell::Cell, sync::atomic::AtomicBool};

thread_local! {
    static FALLBACK: Cell<(u32, u8)> = const { Cell::new((0, 0)) };
}
fn fallback() -> Result<CompiledScript, ScriptError> {
    let mode = FALLBACK.with(|state| {
        let (calls, mode) = state.get();
        state.set((calls + 1, mode));
        mode
    });
    match mode {
        1 => Err(ScriptError::Execution),
        2 => panic!("trusted fallback failure"),
        3 => CompiledScript::compile("exports.default={update(){throw Error('must not tick')}};"),
        _ => CompiledScript::compile(
            "exports.default={update({chunks}){for(const c of chunks)c.transforms[0]+=1}};",
        ),
    }
}
fn fixture() -> (Project, String, String) {
    let mut project = Project::empty("Host fallback");
    project.settings.tick_rate = 10;
    let mut scene = Scene::new("Main");
    let mut entity = Entity::new("Mover");
    entity
        .components
        .insert("Transform".into(), json!(Transform::default()));
    let id = entity.id.clone();
    scene.entities.insert(id.clone(), entity);
    let sid = scene.id.clone();
    project.scenes.insert(sid.clone(), scene);
    (project, sid, id)
}
fn request() -> RunRequest {
    serde_json::from_value(json!({"seconds":0.5})).unwrap()
}
fn position(host: &mut impl EngineHost, id: &str) -> serde_json::Value {
    host.play_state_query(&format!("/entities/{id}/transform/translation"))
        .unwrap()
}

#[test]
fn engine_fallback_is_called_once_and_its_failures_preserve_previous_completed_state() {
    FALLBACK.set((0, 0));
    let root = tempfile::tempdir().unwrap();
    let (project, _, id) = fixture();
    let before = project.canonical_text().unwrap();
    let mut host = Host::new(root.path().to_owned(), NoViewport, fallback);
    let result = host
        .play_run(&project, 7, request(), &AtomicBool::new(false))
        .unwrap();
    assert_eq!(FALLBACK.get(), (1, 0));
    assert_eq!(result["state"]["script_source_parsing"], false);
    let completed = position(&mut host, &id);
    assert_eq!(completed["untrusted_runtime_data"], json!([5., 0., 0.]));
    assert_eq!(completed["started_revision"], 7);
    FALLBACK.set((1, 1));
    assert!(
        host.play_run(&project, 8, request(), &AtomicBool::new(false))
            .is_err()
    );
    assert_eq!(position(&mut host, &id), completed);
    FALLBACK.set((2, 2));
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            host.play_run(&project, 9, request(), &AtomicBool::new(false))
        }))
        .is_err()
    );
    assert_eq!(position(&mut host, &id), completed);
    assert_eq!(project.canonical_text().unwrap(), before);
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}

#[test]
fn checked_preparation_refuses_before_first_guest_tick_and_preserves_prior_candidate() {
    FALLBACK.set((0, 0));
    let root = tempfile::tempdir().unwrap();
    let (project, _, id) = fixture();
    let mut host = Host::new(root.path().to_owned(), NoViewport, fallback);
    host.play_run(&project, 1, request(), &AtomicBool::new(false))
        .unwrap();
    let completed = position(&mut host, &id);
    FALLBACK.set((1, 3));
    let mut called = false;
    let error = host
        .play_run_checked(&project, 2, request(), &AtomicBool::new(false), &mut || {
            assert_eq!(
                FALLBACK.get(),
                (2, 3),
                "preparation must finish before validation"
            );
            called = true;
            Err(AgentError::ProjectChanged)
        })
        .unwrap_err();
    assert!(called);
    assert!(matches!(error, AgentError::ProjectChanged), "{error}");
    assert_eq!(position(&mut host, &id), completed);
    // Accepting the exact same prepared behavior reaches the intentional guest
    // error, proving the rejected case didn't merely run a harmless no-op.
    let error = host
        .play_run_checked(&project, 2, request(), &AtomicBool::new(false), &mut || {
            Ok(())
        })
        .unwrap_err();
    assert!(matches!(error, AgentError::Tool(_)));
    assert_eq!(position(&mut host, &id), completed);
}

#[test]
fn authored_attachments_never_invoke_or_silently_fall_back_to_the_host_function() {
    FALLBACK.set((0, 1));
    let root = tempfile::tempdir().unwrap();
    let (mut project, scene, id) = fixture();
    let source = new_id();
    project.scripts.insert(
        source.clone(),
        ScriptSource {
            id: source.clone(),
            name: "Authored".into(),
            path: "motion.ts".into(),
        },
    );
    project
        .scenes
        .get_mut(&scene)
        .unwrap()
        .entities
        .get_mut(&id)
        .unwrap()
        .components
        .insert("Script".into(), json!({"source":source,"props":{}}));
    std::fs::write(root.path().join("motion.ts"), r#"
import type {NativeBehavior} from '@incant/runtime/native';
export default {update({chunks}){for(const c of chunks)c.transforms[0]+=3;return undefined}} satisfies NativeBehavior;
"#).unwrap();
    let before = project.canonical_text().unwrap();
    let mut host = Host::new(root.path().to_owned(), NoViewport, fallback);
    let result = host
        .play_run(&project, 4, request(), &AtomicBool::new(false))
        .unwrap();
    assert_eq!(result["state"]["script_source_parsing"], true);
    assert_eq!(
        position(&mut host, &id)["untrusted_runtime_data"],
        json!([15., 0., 0.])
    );
    assert_eq!(FALLBACK.get(), (0, 1));
    std::fs::remove_file(root.path().join("motion.ts")).unwrap();
    let completed = position(&mut host, &id);
    assert!(
        host.play_run(&project, 5, request(), &AtomicBool::new(false))
            .is_err()
    );
    assert_eq!(position(&mut host, &id), completed);
    assert_eq!(FALLBACK.get(), (0, 1));
    assert_eq!(project.canonical_text().unwrap(), before);
}

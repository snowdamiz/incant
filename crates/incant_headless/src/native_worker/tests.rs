use super::*;
use incant_doc::{Entity, Project, Scene};
use incant_script::{CompiledScript, ScriptError};
use std::io::Cursor;

fn fixture() -> (Controller, String) {
    let root = tempfile::tempdir().unwrap();
    let mut project = Project::empty("Worker test");
    project.settings.physics_mode = PHYSICS_VARIANT.mode();
    let mut scene = Scene::new("Main");
    let entity = Entity::new("Mover");
    let id = entity.id.clone();
    scene.entities.insert(id.clone(), entity);
    project.scenes.insert(scene.id.clone(), scene);
    let bus = incant_cmd::CommandBus::new(project).unwrap();
    fn behavior() -> std::result::Result<CompiledScript, ScriptError> {
        CompiledScript::compile(
            "exports.default={update({chunks,dt}){for(const c of chunks)for(let i=0;i<c.ids.length;i++)c.transforms[i*c.transformStride]+=6*dt;}}",
        )
    }
    let mut controller = Controller::new(behavior);
    controller.start(&bus, root.path(), json!({})).unwrap();
    drop(bus);
    root.close().unwrap();
    (controller, id)
}

fn requests(values: &[Value]) -> Cursor<Vec<u8>> {
    Cursor::new(
        values
            .iter()
            .map(|v| format!("{v}\n"))
            .collect::<String>()
            .into_bytes(),
    )
}
fn request(id: u64, op: &str, params: Value) -> Value {
    json!({"version":VERSION,"id":id,"request":{"op":op,"params":params}})
}
fn replies(bytes: &[u8]) -> Vec<Value> {
    bytes
        .split(|b| *b == b'\n')
        .filter(|b| !b.is_empty())
        .map(|b| serde_json::from_slice(b).unwrap())
        .collect()
}

#[test]
fn accepted_ticks_are_never_replayed_and_observation_is_explicit() {
    let (mut game, id) = fixture();
    let mut input = requests(&[
        request(1, "step", json!({})),
        request(1, "step", json!({})),
        request(2, "observe", json!({"entities":[id]})),
        request(3, "step", json!({})),
        request(4, "observe", json!({"entities":[id]})),
    ]);
    let mut output = Vec::new();
    serve(&mut game, &mut input, &mut output).unwrap();
    let rows = replies(&output);
    assert_eq!(rows[0]["result"]["tick"], 1);
    assert_eq!(rows[0]["result"]["entities"], json!({}));
    assert_eq!(rows[1]["error"]["code"], "sequence");
    for (row, tick, x) in [(2, 1, 0.1), (4, 2, 0.2)] {
        assert_eq!(rows[row]["result"]["tick"], tick);
        assert_eq!(
            rows[row]["result"]["entities"][&id]["transform"]["translation"][0],
            x
        );
        assert!(rows[row]["result"].get("project").is_none());
    }
}

#[test]
fn malformed_version_and_unknown_mutation_do_not_consume_ids_or_advance() {
    let (mut game, _) = fixture();
    let mut wrong_version = request(1, "step", json!({}));
    wrong_version["version"] = json!(2);
    let mut input = requests(&[
        wrong_version,
        request(1, "command.execute", json!({})),
        json!({"version":1,"id":1,"request":{"op":"stop","unexpected":true}}),
        request(1, "step", json!({"observe":{"entities":["not-an-id"]}})),
        request(2, "observe", json!({})),
    ]);
    let mut output = Vec::new();
    serve(&mut game, &mut input, &mut output).unwrap();
    let rows = replies(&output);
    for (row, code) in [
        (0, "version"),
        (1, "invalid_request"),
        (2, "invalid_request"),
        (3, "native"),
    ] {
        assert_eq!(rows[row]["error"]["code"], code);
    }
    assert_eq!(rows[4]["result"]["tick"], 0);
}

#[test]
fn stop_destroys_the_native_session_and_does_not_read_following_requests() {
    let (mut game, _) = fixture();
    let mut input = requests(&[
        request(1, "step", json!({})),
        json!({"version":1,"id":2,"request":{"op":"stop"}}),
        request(3, "step", json!({})),
    ]);
    let mut output = Vec::new();
    serve(&mut game, &mut input, &mut output).unwrap();
    assert_eq!(replies(&output).len(), 2);
    assert!(game.state(json!({})).is_err());
    assert!(input.position() < input.get_ref().len() as u64);
}

#[test]
fn oversized_and_truncated_frames_are_fatal_before_dispatch() {
    let (mut game, _) = fixture();
    for bytes in [vec![b' '; MAX_REQUEST + 1], b"{\"version\":1".to_vec()] {
        let mut output = Vec::new();
        assert!(serve(&mut game, &mut Cursor::new(bytes), &mut output).is_err());
        assert!(output.is_empty());
        assert_eq!(game.state(json!({})).unwrap()["tick"], 0);
    }
    assert!(frame(&mut Cursor::new(Vec::<u8>::new())).unwrap().is_none());
    let mut exact = vec![b' '; MAX_REQUEST];
    *exact.last_mut().unwrap() = b'\n';
    assert_eq!(
        frame(&mut Cursor::new(exact)).unwrap().unwrap().len(),
        MAX_REQUEST - 1
    );
}

#[test]
fn broken_output_reports_failure_instead_of_acknowledging_a_tick() {
    let (mut game, _) = fixture();
    struct Closed;
    impl Write for Closed {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::ErrorKind::BrokenPipe.into())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let mut input = requests(&[request(1, "step", json!({})), request(2, "step", json!({}))]);
    assert!(matches!(
        serve(&mut game, &mut input, &mut Closed),
        Err(Error::Io(_))
    ));
    // The first tick was accepted, but no second tick can be consumed after the
    // lost acknowledgement. The outer owner drops this whole session on error.
    assert_eq!(game.state(json!({})).unwrap()["tick"], 1);
}

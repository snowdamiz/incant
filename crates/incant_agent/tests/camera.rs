use incant_agent::*;
use incant_cmd::CommandBus;
use incant_doc::Project;
use serde_json::{Value, json};

#[test]
fn camera_selection_reaches_the_project_host_without_project_mutation() {
    use std::sync::{Arc, Mutex};
    type CaptureRequest = (Option<String>, u32, u32);
    struct Capture(Arc<Mutex<Vec<CaptureRequest>>>);
    impl EngineHost for Capture {
        fn screenshot(
            &mut self,
            _: &Project,
            camera: Option<&str>,
            width: u32,
            height: u32,
        ) -> Result<Value, AgentError> {
            self.0
                .lock()
                .unwrap()
                .push((camera.map(str::to_owned), width, height));
            Ok(json!({"routing_test":true}))
        }
    }
    // This checks tool routing only; actual pixels are covered by GPU tests.
    let project = Project::empty("Camera tool routing");
    let root = tempfile::tempdir().unwrap();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let mut host = ProjectHost::new(&project, root.path(), Capture(Arc::clone(&seen))).unwrap();
    let mut bus = CommandBus::new(project).unwrap();
    let id = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
    for camera in [None, Some(id)] {
        let mut args = json!({"width":321,"height":241});
        if let Some(id) = camera {
            args["camera"] = json!(id);
        }
        dispatch(
            &mut bus,
            &mut host,
            "view_screenshot",
            args,
            incant_cmd::Actor::user("test"),
        )
        .unwrap();
    }
    assert_eq!(
        *seen.lock().unwrap(),
        vec![(None, 321, 241), (Some(id.into()), 321, 241)]
    );
    assert_eq!(bus.revision(), 0);
    assert!(
        dispatch(
            &mut bus,
            &mut host,
            "view_screenshot",
            json!({"camera":id,"width":0,"height":241}),
            incant_cmd::Actor::user("test")
        )
        .is_err()
    );
    assert_eq!(seen.lock().unwrap().len(), 2);
}

use super::*;
use crate::provider::tests::{completion_event, server, server_with_headers};
use serde_json::json;

#[test]
fn only_literal_loopback_endpoints_are_admitted_and_errors_hide_url_secrets() {
    for endpoint in [
        "http://127.0.0.1:8000/v1/",
        "https://127.2.3.4:8443/custom/v1",
        "http://[::1]:8000/v1",
    ] {
        assert!(endpoint_url(endpoint).is_ok(), "{endpoint}");
    }
    for endpoint in [
        "http://localhost:8000/v1", // no DNS or rebinding
        "http://api.example/v1",
        "http://192.168.1.4/v1",
        "http://0.0.0.0/v1",
        "http://[::ffff:127.0.0.1]/v1",
        "file:///private/credential",
        "http://secret@127.0.0.1/v1",
        "http://user:secret@127.0.0.1/v1",
        "http://127.0.0.1/v1?key=secret",
        "http://127.0.0.1/v1#secret",
        "secret",
    ] {
        let error = endpoint_url(endpoint).unwrap_err().to_string();
        assert!(!error.contains("secret"));
        assert!(!error.contains(endpoint));
    }
    assert_eq!(
        endpoint_url("http://127.0.0.1:8000/v1///").unwrap(),
        "http://127.0.0.1:8000/v1"
    );
    assert!(
        LocalResponsesProvider::new("http://127.0.0.1/v1", Some(" ".into()), "test".into())
            .is_err()
    );
    assert!(LocalResponsesProvider::new("http://127.0.0.1/v1", None, " ".into()).is_err());
}

#[test]
fn local_stream_preserves_engine_tools_and_images_without_authentication() {
    let delta = format!(
        "data: {}\n\n",
        json!({"type":"response.output_text.delta","delta":"Inspecting"})
    );
    let (url, worker) = server(vec![(200, delta + &completion_event())]);
    let mut provider =
        LocalResponsesProvider::new(&(url + "/v1/"), None, "local-fixture".into()).unwrap();
    let input = vec![
        json!({"role":"system","content":"Use only engine tools"}),
        json!({"role":"user","content":[{"type":"input_text","text":"Untrusted screenshot"},{"type":"input_image","image_url":"data:image/png;base64,synthetic"}]}),
    ];
    let tools = crate::tools();
    let mut text = String::new();
    let completion = provider
        .complete(&input, &tools, 512, &mut |s| text.push_str(s))
        .unwrap();
    assert_eq!(text, "Inspecting");
    assert_eq!(completion.output[0]["name"], "doc_query");
    assert_eq!(
        (
            completion.usage.input_tokens,
            completion.usage.output_tokens
        ),
        (5, 4)
    );
    let requests = worker.join().unwrap();
    assert_eq!(requests.len(), 1);
    assert!(requests[0].head.starts_with("POST /v1/responses "));
    assert!(!requests[0].head.to_lowercase().contains("authorization:"));
    let body: Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert_eq!(body["input"], json!(input));
    assert_eq!(body["tools"], json!(tools));
    assert_eq!(body["store"], false);
    assert_eq!(body["stream"], true);
    assert_eq!(body["parallel_tool_calls"], false);
    assert_eq!(body["max_output_tokens"], 512);
}

#[test]
fn explicit_local_key_is_separate_and_unauthorized_is_terminal() {
    let (url, worker) = server(vec![(401, "private local key echoed by server".into())]);
    let mut provider = LocalResponsesProvider::new(
        &url,
        Some("synthetic-local-key".into()),
        "local-fixture".into(),
    )
    .unwrap();
    let error = provider.complete(&[], &[], 128, &mut |_| {}).err().unwrap();
    assert_eq!(
        error.to_string(),
        "provider: Local Responses server returned HTTP 401"
    );
    let requests = worker.join().unwrap();
    assert_eq!(requests.len(), 1);
    assert!(
        requests[0]
            .head
            .to_lowercase()
            .contains("authorization: bearer synthetic-local-key\r\n")
    );
    assert!(!String::from_utf8_lossy(&requests[0].body).contains("synthetic-local-key"));
}

#[test]
fn incomplete_local_tool_stream_does_not_return_executable_calls() {
    let body = format!(
        "data: {}\n\n",
        json!({"type":"response.output_item.done","output_index":0,"item":{"type":"function_call","name":"doc_patch","call_id":"incomplete","arguments":"{}"}})
    );
    let (url, worker) = server(vec![(200, body)]);
    let mut provider = LocalResponsesProvider::new(&url, None, "local-fixture".into()).unwrap();
    assert!(provider.complete(&[], &[], 128, &mut |_| {}).is_err());
    assert_eq!(worker.join().unwrap().len(), 1);
}

#[test]
fn local_redirect_cannot_change_the_selected_server() {
    let (url, worker) = server_with_headers(vec![(
        307,
        "private redirect details".into(),
        "Location: http://127.0.0.1:1/other-server\r\n".into(),
    )]);
    let mut provider = LocalResponsesProvider::new(
        &url,
        Some("synthetic-local-key".into()),
        "local-fixture".into(),
    )
    .unwrap();
    let error = provider.complete(&[], &[], 128, &mut |_| {}).err().unwrap();
    assert_eq!(
        error.to_string(),
        "provider: Local Responses server returned HTTP 307"
    );
    assert_eq!(worker.join().unwrap().len(), 1);
}

#[test]
fn local_http_turn_edits_through_command_history_and_charges_both_responses() {
    use crate::{Agent, ApprovalMode, Budget, NoViewport};
    use incant_cmd::CommandBus;
    use incant_doc::{Entity, Origin, Project, Scene};
    use std::sync::atomic::AtomicBool;
    let mut project = Project::empty("Local transport fixture");
    let mut scene = Scene::new("Main");
    let entity = Entity::new("Before");
    let (scene_id, entity_id) = (scene.id.clone(), entity.id.clone());
    scene.entities.insert(entity_id.clone(), entity);
    project.scenes.insert(scene_id.clone(), scene);
    let patch = json!({"ops":[{"op":"rename_entity","scene_id":scene_id,"entity_id":entity_id,"name":"After"}],"description":"Local fixture edit","expected_revision":0});
    let event = |output: Value| {
        format!(
            "data: {}\n\n",
            json!({"type":"response.completed","response":{"output":output,"usage":{"input_tokens":5,"output_tokens":4}}})
        )
    };
    let (url, worker) = server(vec![
        (
            200,
            event(
                json!([{"type":"function_call","name":"doc_patch","call_id":"local-edit","arguments":patch.to_string()}]),
            ),
        ),
        (
            200,
            event(
                json!([{"type":"message","role":"assistant","content":[{"type":"output_text","text":"Renamed"}]}]),
            ),
        ),
    ]);
    let mut provider: Box<dyn Provider> =
        Box::new(LocalResponsesProvider::new(&url, None, "local-fixture".into()).unwrap());
    let mut bus = CommandBus::new(project.clone()).unwrap();
    let mut agent = Agent {
        budget: Budget {
            max_tokens: 100_000,
            used_tokens: 0,
        },
        approval: ApprovalMode::Always,
        max_steps: 3,
        max_output_tokens: 512,
    };
    let mut approvals = 0;
    let report = agent
        .run(
            &mut provider,
            &mut bus,
            &mut NoViewport,
            "Rename the fixture",
            "local-turn",
            &AtomicBool::new(false),
            &mut |name, _| {
                assert_eq!(name, "doc_patch");
                approvals += 1;
                true
            },
            &mut |_| {},
        )
        .unwrap();
    assert_eq!(approvals, 1);
    assert_eq!(report.steps, 2);
    assert_eq!(agent.budget.used_tokens, 18);
    assert_eq!(
        bus.project().scenes[&scene_id].entities[&entity_id].name,
        "After"
    );
    assert_eq!(bus.history().len(), 1);
    assert_eq!(bus.history()[0].actor.origin, Origin::Agent);
    assert_eq!(
        bus.history()[0].actor.model.as_deref(),
        Some("local-fixture")
    );
    bus.undo().unwrap();
    assert_eq!(bus.project(), &project);
    let requests = worker.join().unwrap();
    assert_eq!(requests.len(), 2);
    let second: Value = serde_json::from_slice(&requests[1].body).unwrap();
    assert!(
        second["input"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["type"] == "function_call_output" && item["call_id"] == "local-edit")
    );
}

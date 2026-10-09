use super::*;
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::{Arc, Mutex},
    thread,
};

struct Request {
    head: String,
    body: Vec<u8>,
}
fn server(replies: Vec<(u16, String)>) -> (String, thread::JoinHandle<Vec<Request>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let worker = thread::spawn(move || {
        let mut requests = vec![];
        for (status, body) in replies {
            let deadline = std::time::Instant::now() + Duration::from_secs(5);
            let (mut socket, _) = loop {
                match listener.accept() {
                    Ok(connection) => break connection,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(
                            std::time::Instant::now() < deadline,
                            "request never arrived"
                        );
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(error) => panic!("test server failed: {error}"),
                }
            };
            socket.set_nonblocking(false).unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut bytes = vec![];
            let boundary = loop {
                let mut chunk = [0; 1024];
                let size = socket.read(&mut chunk).unwrap();
                assert_ne!(size, 0);
                bytes.extend_from_slice(&chunk[..size]);
                if let Some(end) = bytes.windows(4).position(|x| x == b"\r\n\r\n") {
                    break end + 4;
                }
                assert!(bytes.len() < 64 * 1024);
            };
            let head = String::from_utf8(bytes[..boundary].to_vec()).unwrap();
            let length = head
                .lines()
                .find_map(|line| {
                    line.to_lowercase()
                        .strip_prefix("content-length: ")
                        .and_then(|n| n.parse::<usize>().ok())
                })
                .unwrap_or(0);
            assert!(length < 1024 * 1024);
            while bytes.len() < boundary + length {
                let mut chunk = [0; 1024];
                let size = socket.read(&mut chunk).unwrap();
                assert_ne!(size, 0);
                bytes.extend_from_slice(&chunk[..size]);
            }
            requests.push(Request {
                head,
                body: bytes[boundary..boundary + length].to_vec(),
            });
            write!(
                socket,
                "HTTP/1.1 {status} Test\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
        }
        requests
    });
    (url, worker)
}
fn completion_event() -> String {
    format!(
        "data: {}\n\n",
        json!({"type":"response.completed","response":{"output":[{"type":"function_call","name":"doc_query","call_id":"test-call","arguments":"{\"path\":\"\"}"}],"usage":{"input_tokens":5,"output_tokens":4}}})
    )
}

#[test]
fn api_key_catalog_and_inference_use_the_same_explicit_credential() {
    let (url, server) = server(vec![
        (200, r#"{"data":[{"id":"test-model"}]}"#.into()),
        (200, completion_event()),
    ]);
    let mut provider =
        OpenAiProvider::new("synthetic-api-key".into(), "test-model".into()).unwrap();
    provider.base_url = url;
    assert_eq!(provider.models().unwrap()["data"][0]["id"], "test-model");
    let result = provider
        .complete(
            &[json!({"role":"user","content":"test"})],
            &[],
            512,
            &mut |_| {},
        )
        .unwrap();
    assert_eq!(result.output[0]["name"], "doc_query");
    let requests = server.join().unwrap();
    assert!(requests[0].head.starts_with("GET /models "));
    assert!(requests[1].head.starts_with("POST /responses "));
    for request in &requests {
        assert!(
            request
                .head
                .to_lowercase()
                .contains("authorization: bearer synthetic-api-key\r\n")
        );
    }
    let body: Value = serde_json::from_slice(&requests[1].body).unwrap();
    assert_eq!(body["model"], "test-model");
    assert_eq!(body["max_output_tokens"], 512);
    assert_eq!(body["store"], false);
    assert_eq!(body["stream"], true);
    assert_eq!(body["parallel_tool_calls"], false);
    assert!(!String::from_utf8_lossy(&requests[1].body).contains("synthetic-api-key"));
}

#[test]
fn renewable_sessions_reload_tokens_and_retry_unauthorized_only_once() {
    let (url, server) = server(vec![
        (401, "private error body".into()),
        (200, completion_event()),
        (401, String::new()),
        (401, String::new()),
    ]);
    let calls = Arc::new(Mutex::new(vec![]));
    let observed = calls.clone();
    let mut provider = OpenAiProvider::with_token_source(
        Box::new(move |force| {
            let mut calls = observed.lock().unwrap();
            calls.push(force);
            Ok(Zeroizing::new(format!("synthetic-{}", calls.len())))
        }),
        true,
        "test-model".into(),
    )
    .unwrap();
    provider.base_url = url;
    provider.complete(&[], &[], 128, &mut |_| {}).unwrap();
    let error = provider.complete(&[], &[], 128, &mut |_| {}).err().unwrap();
    assert_eq!(error.to_string(), "provider: OpenAI returned HTTP 401");
    assert_eq!(*calls.lock().unwrap(), [false, true, false, true]);
    for (index, request) in server.join().unwrap().iter().enumerate() {
        assert!(request.head.to_lowercase().contains(&format!(
            "authorization: bearer synthetic-{}\r\n",
            index + 1
        )));
    }
}

#[test]
fn api_key_unauthorized_is_terminal_and_server_text_is_redacted() {
    let (url, server) = server(vec![(401, "synthetic-api-key echoed by server".into())]);
    let mut provider =
        OpenAiProvider::new("synthetic-api-key".into(), "test-model".into()).unwrap();
    provider.base_url = url;
    let error = provider.complete(&[], &[], 128, &mut |_| {}).err().unwrap();
    assert_eq!(error.to_string(), "provider: OpenAI returned HTTP 401");
    assert_eq!(server.join().unwrap().len(), 1);
}

#[test]
fn lost_account_stops_before_sending_another_request() {
    let mut provider = OpenAiProvider::with_token_source(
        Box::new(|_| Err(AgentError::Provider("signed out".into()))),
        true,
        "test-model".into(),
    )
    .unwrap();
    // A closed local port would produce a transport error if contacted.
    provider.base_url = "http://127.0.0.1:1".into();
    assert_eq!(
        provider
            .complete(&[], &[], 128, &mut |_| {})
            .err()
            .unwrap()
            .to_string(),
        "provider: signed out"
    );
}

use super::*;
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::{Arc, Mutex},
    thread,
};

pub(super) struct Request {
    pub(super) head: String,
    pub(super) body: Vec<u8>,
}
pub(super) fn server(replies: Vec<(u16, String)>) -> (String, thread::JoinHandle<Vec<Request>>) {
    server_with_headers(
        replies
            .into_iter()
            .map(|(status, body)| (status, body, String::new()))
            .collect(),
    )
}
pub(super) fn server_with_headers(
    replies: Vec<(u16, String, String)>,
) -> (String, thread::JoinHandle<Vec<Request>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let worker = thread::spawn(move || {
        let mut requests = vec![];
        for (status, body, headers) in replies {
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
                "HTTP/1.1 {status} Test\r\nContent-Length: {}\r\nConnection: close\r\n{headers}\r\n{body}",
                body.len()
            )
            .unwrap();
        }
        requests
    });
    (url, worker)
}
pub(super) fn completion_event() -> String {
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

#[test]
fn active_metadata_checks_never_resolve_or_refresh_credentials() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let mut provider = OpenAiProvider::with_token_source(
        Box::new(|_| panic!("metadata checks must not read or refresh credentials")),
        true,
        "fixture-model".into(),
    )
    .unwrap();
    assert!(provider.check_active().is_ok());
    let checks = Arc::new(AtomicUsize::new(0));
    let observed = checks.clone();
    provider.active = Some(Box::new(move || {
        observed.fetch_add(1, Ordering::Relaxed);
        Err(AgentError::Provider("selection changed".into()))
    }));
    let provider: Box<dyn Provider> = Box::new(provider);
    assert_eq!(provider.model(), "fixture-model");
    for _ in 0..3 {
        assert!(matches!(
            provider.check_active(),
            Err(AgentError::Provider(_))
        ));
    }
    assert_eq!(checks.load(Ordering::Relaxed), 3);
}

#[test]
fn oversized_unterminated_line_stops_reading_at_the_stream_budget() {
    let mut reader = std::io::Cursor::new(vec![b'x'; 4096]);
    let mut emitted = String::new();
    let error = read_stream_limited(&mut reader, &mut |text| emitted.push_str(text), 64)
        .err()
        .unwrap();
    assert_eq!(
        error.to_string(),
        "provider: response stream limit exceeded"
    );
    assert_eq!(reader.position(), 65);
    assert!(emitted.is_empty());
}

#[test]
fn empty_lines_and_comment_delimiters_consume_the_same_response_budget() {
    for bytes in [b"\n".repeat(4096), b": ping\r\n".repeat(512)] {
        let mut reader = std::io::Cursor::new(bytes);
        let error = read_stream_limited(&mut reader, &mut |_| panic!("no text event"), 64)
            .err()
            .unwrap();
        assert_eq!(
            error.to_string(),
            "provider: response stream limit exceeded"
        );
        assert_eq!(reader.position(), 65);
    }
}

#[test]
fn exact_budget_preserves_crlf_multiline_data_and_utf8_terminal_output() {
    let response = json!({"output":[{"type":"message","role":"assistant",
        "content":[{"type":"output_text","text":"café 日本語"}]}],
        "usage":{"input_tokens":2,"output_tokens":3}});
    let event = format!(
        "data: {{\"type\":\"response.completed\",\r\ndata: \"response\":{response}}}\r\n\r\n"
    );
    let complete = read_stream_limited(event.as_bytes(), &mut |_| {}, event.len()).unwrap();
    assert_eq!(
        complete.output,
        response["output"].as_array().unwrap().clone()
    );
    assert_eq!(complete.usage.input_tokens, 2);
    assert_eq!(complete.usage.output_tokens, 3);
    assert!(
        read_stream_limited(event.as_bytes(), &mut |_| {}, event.len() - 1)
            .err()
            .unwrap()
            .to_string()
            .contains("stream limit exceeded")
    );
}

#[test]
fn bounded_stream_refuses_invalid_utf8_without_echoing_response_bytes() {
    let bytes = b"data: secret-provider-response-\xff\n\n";
    let error = read_stream_limited(bytes.as_slice(), &mut |_| panic!("invalid text"), 128)
        .err()
        .unwrap();
    assert_eq!(error.to_string(), "provider: invalid UTF-8 response stream");
}

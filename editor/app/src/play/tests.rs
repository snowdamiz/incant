use super::*;
#[test]
fn stale_failure_cannot_replace_a_new_session_and_counters_fail_before_mutation() {
    let (service, _) = Service::new();
    service
        .edit(|s| {
            s.generation = 2;
            s.phase = "running";
            Ok(())
        })
        .unwrap();
    service.failed(1, "obsolete startup".into());
    let state = service.snapshot().unwrap();
    assert_eq!(state.generation, 2);
    assert_eq!(state.phase, "running");
    assert!(state.error.is_none());
    service.status.lock().unwrap().sequence = u64::MAX;
    assert!(
        service
            .edit(|s| {
                s.phase = "stopped";
                Ok(())
            })
            .is_err()
    );
    assert_eq!(service.snapshot().unwrap().phase, "running");
}
#[test]
fn queue_pressure_and_worker_shutdown_refuse_without_hanging() {
    let (service, rx) = Service::new();
    for _ in 0..16 {
        let (reply, _) = mpsc::channel();
        service
            .tx
            .try_send(Message {
                request: Request::Action(Action::Stop { generation: 0 }),
                reply,
            })
            .unwrap();
    }
    assert!(
        service
            .call(Request::Action(Action::Stop { generation: 0 }))
            .unwrap_err()
            .contains("full")
    );
    drop(rx);
    assert!(
        service
            .call(Request::Action(Action::Stop { generation: 0 }))
            .unwrap_err()
            .contains("stopped")
    );
}
#[test]
fn wire_refuses_unknown_commands_and_fields() {
    for invalid in [
        serde_json::json!({"op":"run_shell", "generation":0}),
        serde_json::json!({"op":"stop", "generation":0,"script":"bad"}),
    ] {
        assert!(serde_json::from_value::<Action>(invalid).is_err());
    }
}

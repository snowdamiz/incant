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
    service.status.lock().unwrap().status.sequence = u64::MAX;
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

fn running() -> Arc<Service> {
    let (service, _) = Service::new();
    service
        .edit(|s| {
            s.generation = 1;
            s.phase = "running";
            Ok(())
        })
        .unwrap();
    service
}
#[test]
fn copied_prefix_preserves_duplicates_levels_ids_and_reads_only_bounded_tails() {
    use incant_runtime::logging::{LogLevel, NativeLogs};
    let service = running();
    let mut logs = NativeLogs::default();
    logs.commit(
        1,
        vec![
            (LogLevel::Info, "same".into()),
            (LogLevel::Warn, "same".into()),
        ],
    );
    service.publish_logs(1, &logs, |_| {}).unwrap();
    let first = service.read_logs(1, 0).unwrap();
    assert_eq!(first.next, 2);
    assert_eq!(first.entries[0].message, first.entries[1].message);
    assert_ne!(first.entries[0].id, first.entries[1].id);
    assert_eq!(first.entries[0].level, LogLevel::Info);
    assert_eq!(first.entries[1].level, LogLevel::Warn);
    for _ in 0..3 {
        service.publish_logs(1, &logs, |_| {}).unwrap();
    }
    assert_eq!(
        service.read_logs(1, 0).unwrap().entries[0].id,
        first.entries[0].id
    );
    assert!(service.read_logs(1, 2).unwrap().entries.is_empty());
    assert!(service.read_logs(1, 3).is_err());
    logs.commit(
        2,
        (0..64)
            .map(|_| (LogLevel::Debug, "x".repeat(1024)))
            .collect(),
    );
    service.publish_logs(1, &logs, |_| {}).unwrap();
    let tail = service.read_logs(1, 2).unwrap();
    assert_eq!(tail.next, 18);
    assert_eq!(
        tail.entries.iter().map(|r| r.message.len()).sum::<usize>(),
        logs::MAX_BATCH_BYTES
    );
    let next = service.read_logs(1, tail.next).unwrap();
    assert_eq!(next.from, 18);
    assert_eq!(next.entries.len(), 16);
}
#[test]
fn terminal_failure_and_restart_refuse_unread_old_rows_and_reset_the_prefix() {
    use incant_runtime::logging::{LogLevel, NativeLogs};
    let service = running();
    let mut logs = NativeLogs::default();
    logs.commit(1, vec![(LogLevel::Info, "accepted".into())]);
    service.publish_logs(1, &logs, |_| {}).unwrap();
    let first = service.read_logs(1, 0).unwrap();
    service.failed(1, "fixture failure".into());
    assert!(service.read_logs(1, 0).is_err());
    assert!(service.publish_logs(1, &logs, |_| {}).is_err());
    service
        .edit(|s| {
            s.generation = 2;
            s.phase = "stopped";
            Ok(())
        })
        .unwrap();
    assert_eq!(service.snapshot().unwrap().logs.count, 0);
    assert!(service.read_logs(1, 0).is_err());
    assert!(service.read_logs(2, 0).is_err());
    service
        .edit(|s| {
            s.generation = 3;
            s.phase = "running";
            Ok(())
        })
        .unwrap();
    service.publish_logs(3, &logs, |_| {}).unwrap();
    let restarted = service.read_logs(3, 0).unwrap();
    assert_eq!(restarted.next, 1);
    assert_ne!(restarted.entries[0].id, first.entries[0].id);
    assert_eq!(restarted.warning_id, first.warning_id);
}
#[test]
fn saturation_exposes_drops_and_counter_failure_keeps_unpublished_rows_private() {
    use incant_runtime::logging::{LogLevel, MAX_RETAINED_ENTRIES, NativeLogs};
    let service = running();
    let mut logs = NativeLogs::default();
    for tick in 1..=17 {
        logs.commit(
            tick,
            (0..64).map(|_| (LogLevel::Info, "x".into())).collect(),
        );
        service.publish_logs(1, &logs, |_| {}).unwrap();
    }
    let status = service.snapshot().unwrap();
    assert_eq!(status.logs.count, MAX_RETAINED_ENTRIES);
    assert_eq!(status.logs.dropped, 64);
    let batch = service.read_logs(1, MAX_RETAINED_ENTRIES).unwrap();
    assert_eq!(batch.dropped, 64);
    assert!(batch.entries.is_empty());
    let other = running();
    other.status.lock().unwrap().status.sequence = u64::MAX;
    assert!(other.publish_logs(1, &logs, |_| {}).is_err());
    assert_eq!(other.snapshot().unwrap().logs.count, 0);
    assert!(other.read_logs(1, 0).unwrap().entries.is_empty());
}
#[test]
fn actual_cpu_play_commits_logs_once_and_failed_callback_never_enters_transport() {
    use incant_audio_runtime::{AudioManagerSettings, OfflineBackend};
    use incant_runtime::{CookedEntity, CookedScene, StableId};
    use incant_script::CompiledScript;
    let scene = StableId([9; 16]);
    let prepared = PreparedPlay {
        scene: ulid::Ulid::from_bytes(scene.0).to_string(),
        script_source_sha256: "fixture".into(),
        authored_scripts: false,
        cooked: CookedScene::new(
            scene,
            60,
            vec![CookedEntity {
                id: StableId([1; 16]),
                parent: None,
                transform: Default::default(),
                velocity: None,
            }],
        )
        .unwrap(),
        code: CompiledScript::compile(
            "exports.default={update({log,chunks}) {\n\
            if(chunks[0].transforms[0]>0) {log('rejected','error'); throw Error('fixture');}\n\
            log('same','debug');log('same','info');log('same','warn');log('same','error');\n\
            chunks[0].transforms[0]+=1;\n\
        }};",
        )
        .unwrap()
        .into(),
        actions: Default::default(),
        clips: vec![],
        sample_rate: 48000,
        render: None,
    };
    let mut game = incant_play::PlaySession::<OfflineBackend>::new(
        prepared,
        AudioManagerSettings {
            backend_settings: 48000,
            ..Default::default()
        },
        (),
    )
    .unwrap();
    let service = running();
    game.step(&[]).unwrap();
    service
        .publish_logs(1, game.logs().unwrap(), |_| {})
        .unwrap();
    let accepted = service.read_logs(1, 0).unwrap();
    assert_eq!(accepted.entries.len(), 4);
    assert!(
        accepted
            .entries
            .iter()
            .all(|row| row.message == "same" && row.tick == 1)
    );
    for _ in 0..3 {
        game.refresh().unwrap();
        service
            .publish_logs(1, game.logs().unwrap(), |_| {})
            .unwrap();
    }
    assert_eq!(service.snapshot().unwrap().logs.count, 4);
    assert!(service.read_logs(1, 4).unwrap().entries.is_empty());
    assert!(game.step(&[]).is_err());
    assert!(game.logs().is_err());
    service.failed(1, "Native Play operation failed".into());
    assert!(service.read_logs(1, 0).is_err());
    // A delivered packet remains accepted historical data; the rejected row is absent.
    assert_eq!(accepted.entries.len(), 4);
    assert!(accepted.entries.iter().all(|row| row.message != "rejected"));
}

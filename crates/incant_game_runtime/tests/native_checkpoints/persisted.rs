use super::*;
use sha2::{Digest, Sha256};

fn save(game: &mut OfflineGameSession) -> Vec<u8> {
    capture(game)
        .persisted_bytes(&mut payload(), &mut planar(), &mut spatial())
        .unwrap()
}
fn load(bytes: &[u8], initial: &CookedScene) -> incant_game_runtime::Result<PendingGameCheckpoint> {
    PendingGameCheckpoint::from_persisted_bytes(
        bytes,
        initial,
        code(),
        Default::default(),
        &mut payload(),
        &mut planar(),
        &mut spatial(),
        &mut CheckpointAllocationBudget::new(64 * 1024 * 1024, 100_000),
    )
}
fn rehash_game(bytes: &mut [u8]) {
    let mut hash = Sha256::new();
    hash.update(&bytes[..32]);
    hash.update(&bytes[64..]);
    bytes[32..64].copy_from_slice(&hash.finalize());
}

#[test]
fn persisted_physics_script_and_silent_or_real_pcm_continue_from_file_bytes() {
    for audio in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let bindings = if audio { clips(temp.path()) } else { vec![] };
        let initial = scene(audio);
        let mut live = OfflineGameSession::new(
            initial.clone(),
            code(),
            Default::default(),
            bindings.clone(),
            44100,
        )
        .unwrap();
        for _ in 0..67 {
            tick(&mut live);
        }
        let bytes = save(&mut live);
        let path = temp.path().join("game.save");
        std::fs::write(&path, &bytes).unwrap();
        let pending = load(&std::fs::read(&path).unwrap(), &initial).unwrap();
        assert_eq!(pending.sample_rate(), Some(44100));
        let mut restored = OfflineGameSession::from_checkpoint(pending, bindings).unwrap();
        assert_eq!(
            restored.rendered_frames().unwrap(),
            live.rendered_frames().unwrap()
        );
        assert_ne!(
            restored.world().unwrap().incarnation(),
            live.world().unwrap().incarnation()
        );
        let immediate = save(&mut restored);
        assert!(load(&immediate, &initial).is_ok());
        for _ in 0..37 {
            assert_eq!(tick(&mut live), tick(&mut restored));
            assert_eq!(
                live.world().unwrap().snapshot(),
                restored.world().unwrap().snapshot()
            );
            assert_eq!(
                live.world().unwrap().physics_events(),
                restored.world().unwrap().physics_events()
            );
            assert_eq!(
                live.world().unwrap().physics_2d_events(),
                restored.world().unwrap().physics_2d_events()
            );
        }
        // Imported readiness must permit another validated save, not silently
        // clear warm caches or require an extra hidden simulation tick.
        let again = save(&mut restored);
        assert!(load(&again, &initial).is_ok());
        assert!(live.checkpoint_bytes().is_err()); // old format remains closed
    }
}

#[test]
fn persisted_suspension_and_audio_clock_cannot_be_rewritten_by_outer_metadata() {
    let initial = scene(false);
    let mut live =
        OfflineGameSession::new(initial.clone(), code(), Default::default(), vec![], 44100)
            .unwrap();
    for _ in 0..13 {
        tick(&mut live);
    }
    live.set_suspended(true).unwrap();
    let bytes = save(&mut live);
    let mut restored =
        OfflineGameSession::from_checkpoint(load(&bytes, &initial).unwrap(), vec![]).unwrap();
    assert_eq!(
        restored.rendered_frames().unwrap(),
        live.rendered_frames().unwrap()
    );
    let mut pcm = vec![7.; restored.next_output_frames().unwrap() * 2];
    assert!(matches!(
        restored.tick(&[], &mut pcm),
        Err(incant_game_runtime::GameError::Suspended)
    ));
    assert!(pcm.iter().all(|v| *v == 7.));
    for (offset, value) in [(12, 2u32), (16, 48000u32)] {
        let mut bad = bytes.clone();
        bad[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        rehash_game(&mut bad);
        assert!(matches!(
            load(&bad, &initial),
            Err(incant_game_runtime::GameError::Checkpoint(
                incant_script::CheckpointError::Invalid("game/audio clock pairing")
            ))
        ));
    }
    live.set_suspended(false).unwrap();
    restored.set_suspended(false).unwrap();
    for _ in 0..17 {
        assert_eq!(tick(&mut live), tick(&mut restored));
    }
    assert_eq!(
        live.world().unwrap().snapshot(),
        restored.world().unwrap().snapshot()
    );
}

#[test]
fn persisted_refusal_preserves_live_game_and_rejects_rehashed_framing_and_budget_failures() {
    let initial = scene(false);
    let mut live =
        OfflineGameSession::new(initial.clone(), code(), Default::default(), vec![], 44100)
            .unwrap();
    for _ in 0..13 {
        tick(&mut live);
    }
    let bytes = save(&mut live);
    let before = live.world().unwrap().snapshot();
    for offset in [8, 12, 20, 24] {
        let mut bad = bytes.clone();
        bad[offset] = 255;
        rehash_game(&mut bad);
        assert!(load(&bad, &initial).is_err());
    }
    let mut bad = bytes.clone();
    bad[64 + 8..64 + 12].copy_from_slice(&3u32.to_le_bytes());
    // Rewrite both checksums: unsupported format remains a semantic refusal.
    let mut hash = Sha256::new();
    hash.update(&bad[64..64 + 104]);
    hash.update(&bad[64 + 136..]);
    bad[64 + 104..64 + 136].copy_from_slice(&hash.finalize());
    rehash_game(&mut bad);
    assert!(load(&bad, &initial).is_err());
    assert!(
        PendingGameCheckpoint::from_persisted_bytes(
            &bytes,
            &initial,
            code(),
            Default::default(),
            &mut payload(),
            &mut PlanarCheckpointWorkBudget::new(0),
            &mut spatial(),
            &mut CheckpointAllocationBudget::new(64 * 1024 * 1024, 100_000),
        )
        .is_err()
    );
    assert_eq!(live.world().unwrap().snapshot(), before);
    tick(&mut live);
}

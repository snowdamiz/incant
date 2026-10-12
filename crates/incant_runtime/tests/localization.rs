#[path = "support/localization.rs"]
mod fixture;
#[path = "support/scene_format.rs"]
mod scene_format;
use fixture::*;
use incant_localization::{LocalizationReportRequest, MessageArgument, MissingKind};
use incant_runtime::{
    CookedScene, FrameCommands, NativeLocalization, NativeWorld, SaveError, StableId,
    StructuralCommand,
};
use sha2::{Digest, Sha256};
fn rehash(bytes: &mut [u8], prefix: usize, header: usize) {
    let mut h = Sha256::new();
    h.update(&bytes[..prefix]);
    h.update(&bytes[header..]);
    bytes[prefix..header].copy_from_slice(&h.finalize());
}
fn locale(world: &mut NativeWorld, value: incant_localization::LocaleSettings) {
    let mut commands = FrameCommands::default();
    commands.push(StructuralCommand::SetLocale(value)).unwrap();
    world.apply(&mut commands).unwrap();
}
#[test]
fn cooked_localization_round_trips_and_old_plain_scenes_keep_defaults() {
    let bytes = scene().to_bytes();
    assert_eq!(
        &bytes[8..12],
        &crate::scene_format::scene_format(10).to_le_bytes()
    );
    let loaded = CookedScene::from_bytes(&bytes).unwrap();
    assert_eq!(loaded.to_bytes(), bytes);
    let mut world = NativeWorld::new(loaded);
    let mut r = request("items");
    r.arguments.insert("n".into(), MessageArgument::Number(22.));
    locale(&mut world, settings("ru"));
    assert_eq!(
        world.localization().localize(&r).unwrap().text,
        "22 предмета"
    );
    let plain = CookedScene::new(StableId([99; 16]), 60, vec![])
        .unwrap()
        .to_bytes();
    assert_eq!(
        &plain[8..12],
        &crate::scene_format::scene_format(1).to_le_bytes()
    );
    assert_eq!(
        NativeWorld::from_bytes(&plain)
            .unwrap()
            .localization()
            .settings(),
        &settings("en")
    );
}
#[test]
fn locale_coverage_is_prepared_and_pages_are_bounded_with_fallback_and_missing_keys() {
    let mut world = NativeWorld::new(scene());
    let mut s = settings("de");
    s.fallbacks.push("fr".into());
    locale(&mut world, s);
    let mut r = request("hello");
    r.arguments
        .insert("name".into(), MessageArgument::Text("Ada".into()));
    let result = world.localization().localize(&r).unwrap();
    assert_eq!(result.text, "Bonjour Ada");
    assert_eq!(result.missing.unwrap().kind, MissingKind::Translation);
    let first = world
        .localization()
        .report(LocalizationReportRequest {
            offset: 0,
            limit: 2,
        })
        .unwrap();
    let second = world
        .localization()
        .report(LocalizationReportRequest {
            offset: 2,
            limit: 2,
        })
        .unwrap();
    assert_eq!(first.total, 4);
    assert_eq!(first.entries.len() + second.entries.len(), 4);
    assert!(first.entries[1].key < second.entries[0].key);
    assert!(
        world
            .localization()
            .report(LocalizationReportRequest {
                offset: 0,
                limit: 129
            })
            .is_err()
    );
    assert_eq!(
        world
            .localization()
            .localize(&request("absent"))
            .unwrap()
            .missing
            .unwrap()
            .kind,
        MissingKind::Key
    );
}
#[test]
fn save_v3_preserves_current_locale_catalog_origin_and_atomic_restore() {
    let mut world = NativeWorld::new(scene());
    let origin = world.source_scene_fingerprint();
    let mut s = settings("fr-CA");
    s.pseudo = true;
    s.fallbacks.push("en".into());
    locale(&mut world, s.clone());
    world.step().unwrap();
    let saved = world.save_bytes().unwrap();
    assert_eq!(&saved[8..12], &3u32.to_le_bytes());
    let restored = NativeWorld::from_save_bytes(&saved).unwrap();
    assert_eq!(restored.localization().settings(), &s);
    assert_eq!(restored.source_scene_fingerprint(), origin);
    assert_eq!(restored.save_bytes().unwrap(), saved);
    let mut target = NativeWorld::new(scene());
    target.restore_bytes(&saved).unwrap();
    assert_eq!(target.localization().settings(), &s);
    for v in [1u32, 2] {
        let mut old = saved.clone();
        old[8..12].copy_from_slice(&v.to_le_bytes());
        rehash(&mut old, 32, 64);
        assert!(target.restore_bytes(&old).is_err());
        assert_eq!(target.save_bytes().unwrap(), saved);
    }
    let mut other = tables();
    other
        .get_mut(TABLE)
        .unwrap()
        .messages
        .get_mut("hello")
        .unwrap()
        .insert("en".into(), "Changed {name}".into());
    let foreign = scene()
        .with_localization(NativeLocalization::new(other, s).unwrap())
        .unwrap();
    let mut foreign = NativeWorld::new(foreign).save_bytes().unwrap();
    foreign[64..96].copy_from_slice(&saved[64..96]);
    rehash(&mut foreign, 32, 64);
    assert_eq!(
        target.restore_bytes(&foreign),
        Err(SaveError::WorldMismatch)
    );
    assert_eq!(target.save_bytes().unwrap(), saved);
}
#[test]
fn invalid_locale_or_mixed_scalar_commands_leave_all_state_and_queue_unchanged() {
    let mut world = NativeWorld::new(scene());
    let before = world.save_bytes().unwrap();
    for commands in [
        vec![StructuralCommand::SetLocale(settings("en-us"))],
        vec![
            StructuralCommand::SetLocale(settings("fr")),
            StructuralCommand::SetVelocity {
                entity: StableId([1; 16]),
                velocity: Some(incant_types::Velocity {
                    linear: [f64::NAN, 0., 0.],
                }),
            },
        ],
        vec![
            StructuralCommand::SetLocale(settings("fr")),
            StructuralCommand::ControlAudio {
                entity: StableId([1; 16]),
                control: incant_runtime::AudioControl::SourceGain(0.),
            },
        ],
    ] {
        let mut q = FrameCommands::default();
        for c in commands {
            q.push(c).unwrap();
        }
        let count = q.len();
        assert!(world.apply(&mut q).is_err());
        assert_eq!(q.len(), count);
        assert_eq!(world.save_bytes().unwrap(), before);
    }
}
#[test]
fn malformed_rehashed_localization_suffixes_refuse_without_changing_live_state() {
    let bytes = scene().to_bytes();
    let mut world = NativeWorld::new(scene());
    let before = world.save_bytes().unwrap();
    // One transform-only record plus seven empty section counts precede v10.
    let suffix = 72 + 104 + 28;
    for (offset, value) in [
        (suffix, u32::MAX),
        (suffix + 4, 129),
        (suffix + 14, 17),
        (suffix + 19, 65),
    ] {
        let mut bad = bytes.clone();
        bad[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        rehash(&mut bad, 40, 72);
        assert!(CookedScene::from_bytes(&bad).is_err());
    }
    let mut bad = bytes;
    let pos = bad.windows(12).position(|b| b == b"Hello {name}").unwrap();
    bad[pos + 11] = b'?';
    rehash(&mut bad, 40, 72);
    assert!(CookedScene::from_bytes(&bad).is_err());
    let mut bad_save = before.clone();
    bad_save[104..].copy_from_slice(&bad);
    rehash(&mut bad_save, 32, 64);
    assert!(world.restore_bytes(&bad_save).is_err());
    assert_eq!(world.save_bytes().unwrap(), before);
}
#[test]
fn committed_query_snapshot_expires_on_return_and_after_restore() {
    let mut world = NativeWorld::new(scene());
    let old = world
        .with_numeric_physics(|_, q| {
            assert_eq!(q.locale().unwrap().locale, "en");
            q
        })
        .unwrap();
    assert!(old.locale().is_err());
    locale(&mut world, settings("fr"));
    let saved = world.save_bytes().unwrap();
    world.restore_bytes(&saved).unwrap();
    assert!(old.locale().is_err());
    world
        .with_numeric_physics(|_, q| {
            assert_eq!(q.locale().unwrap().locale, "fr");
            assert!(old.locale().is_err());
        })
        .unwrap();
}

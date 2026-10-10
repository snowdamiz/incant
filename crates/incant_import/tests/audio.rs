#[path = "../../incant_assets/tests/support/audio.rs"]
mod support;
use incant_assets::{AssetStore, RuntimeAssetData};
use incant_cmd::{Actor, CommandBus};
use incant_doc::Project;
use incant_import::{ImportRequest, ImportSnapshot, SourceWatcher};
use std::{
    fs,
    time::{Duration, Instant},
};
fn request(source: &str) -> ImportRequest {
    ImportRequest {
        source: source.into(),
        texture_usage: None,
    }
}
fn sample(store: &AssetStore, id: &str) -> f32 {
    let asset = store.get(id).unwrap();
    let RuntimeAssetData::Audio(audio) = asset.data() else {
        panic!("audio missing")
    };
    audio.read_chunk(0).unwrap()[0][0]
}
#[test]
fn audio_import_reimport_watch_undo_and_retained_versions_share_command_history() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fs::write(
        root.join("tone.wav"),
        support::wav(1, 48000, &[[1000, 1000]; 1000]),
    )
    .unwrap();
    let mut bus = CommandBus::new(Project::empty("Audio")).unwrap();
    let imported = ImportSnapshot::capture(&bus)
        .prepare(root, None, &[request("tone.wav")])
        .unwrap()
        .commit(&mut bus, Actor::import("test"), "Import audio")
        .unwrap();
    let id = imported.imports[0].asset.id.clone();
    assert_eq!(imported.imports[0].asset.kind, "audio");
    assert_eq!(bus.revision(), 1);
    let mut assets = AssetStore::default();
    assets.sync_project(bus.project(), root).unwrap();
    let retained = assets.get(&id).unwrap();
    let start = Instant::now();
    let mut watcher = SourceWatcher::new(root, Duration::from_millis(10));
    assert!(watcher.poll(&mut bus, start).unwrap().imports.is_empty());
    fs::write(
        root.join("tone.wav"),
        support::wav(1, 48000, &[[2000, 2000]; 1000]),
    )
    .unwrap();
    assert_eq!(
        watcher
            .poll(&mut bus, start + Duration::from_millis(20))
            .unwrap()
            .pending,
        1
    );
    let changed = watcher
        .poll(&mut bus, start + Duration::from_millis(40))
        .unwrap();
    assert_eq!(changed.imports.len(), 1);
    assert_eq!(changed.imports[0].asset.id, id);
    assert_eq!(bus.revision(), 2);
    assets.sync_project(bus.project(), root).unwrap();
    assert_eq!(sample(&assets, &id), 2000. / 32768.);
    let RuntimeAssetData::Audio(old) = retained.data() else {
        panic!()
    };
    assert_eq!(old.read_chunk(0).unwrap()[0], [1000. / 32768.; 2]);
    bus.undo().unwrap();
    assets.sync_project(bus.project(), root).unwrap();
    assert_eq!(sample(&assets, &id), 1000. / 32768.);
    assert!(
        watcher
            .poll(&mut bus, start + Duration::from_millis(100))
            .unwrap()
            .imports
            .is_empty()
    );
    fs::remove_file(root.join("tone.wav")).unwrap();
    let mut reopened = AssetStore::default();
    reopened.sync_project(bus.project(), root).unwrap();
    assert_eq!(sample(&reopened, &id), 1000. / 32768.);
}
#[test]
fn invalid_audio_batch_or_settings_cannot_partially_change_the_project() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fs::write(
        root.join("good.wav"),
        support::wav(1, 48000, &[[1000, 1000]; 100]),
    )
    .unwrap();
    fs::write(root.join("bad.ogg"), b"bad").unwrap();
    let bus = CommandBus::new(Project::empty("Audio")).unwrap();
    let before = bus.project().canonical_text().unwrap();
    assert!(
        ImportSnapshot::capture(&bus)
            .prepare(root, None, &[request("good.wav"), request("bad.ogg")])
            .is_err()
    );
    assert_eq!(bus.project().canonical_text().unwrap(), before);
    assert_eq!(bus.revision(), 0);
    let mut wrong = request("good.wav");
    wrong.texture_usage = Some(incant_assets::TextureUsage::Color);
    assert!(
        ImportSnapshot::capture(&bus)
            .prepare(root, None, &[wrong])
            .is_err()
    );
}

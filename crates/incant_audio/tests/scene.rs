#[path = "../../incant_assets/tests/support/audio.rs"]
mod support;
use incant_assets::{AssetStore, cook_audio};
use incant_audio::{AudioError, SceneAudio};
use incant_core::Engine;
use incant_doc::{Asset, AudioSource, AudioSpatial, Entity, Project, Scene, Transform, new_id};
use serde_json::json;
use std::{fs, path::Path};

fn fixture(root: &Path) -> (Project, AssetStore, String, String, String) {
    fs::write(
        root.join("tone.wav"),
        support::wav(1, 48000, &vec![[8000; 2]; 1000]),
    )
    .unwrap();
    let clip = cook_audio(root, Path::new("tone.wav"), &root.join("cache/audio")).unwrap();
    let mut project = Project::empty("Spatial scene");
    let mut scene = Scene::new("Main");
    let asset = new_id();
    project.assets.insert(
        asset.clone(),
        Asset {
            id: asset.clone(),
            name: "Tone".into(),
            path: "tone.wav".into(),
            sha256: clip.metadata.fingerprint,
            kind: "audio".into(),
            import_settings: None,
        },
    );
    let mut parent = Entity::new("Source parent");
    parent.components.insert(
        "Transform".into(),
        json!(Transform {
            translation: [-5., 0., 0.],
            ..Default::default()
        }),
    );
    let mut listener = Entity::new("Listener");
    listener
        .components
        .insert("Transform".into(), json!(Transform::default()));
    listener
        .components
        .insert("AudioListener".into(), json!({}));
    let mut source = Entity::new("Source");
    source.parent = Some(parent.id.clone());
    source.components.insert(
        "Transform".into(),
        json!(Transform {
            translation: [10., 0., 0.],
            ..Default::default()
        }),
    );
    source.components.insert(
        "AudioSource".into(),
        json!(AudioSource {
            clip: asset,
            bus: None,
            gain_db: 0.,
            pan: 0.,
            rate: 1.,
            start_seconds: 0.,
            playing: true,
            looping: true,
            streaming: true,
            spatial: Some(AudioSpatial {
                min_distance: 1.,
                max_distance: 10.
            })
        }),
    );
    let source_id = source.id.clone();
    let listener_id = listener.id.clone();
    let scene_id = scene.id.clone();
    for e in [source, parent, listener] {
        scene.entities.insert(e.id.clone(), e);
    }
    project.scenes.insert(scene.id.clone(), scene);
    let mut assets = AssetStore::default();
    assets.sync(&project, &root.join("cache")).unwrap();
    (project, assets, scene_id, source_id, listener_id)
}
fn levels(samples: &[f32]) -> [f32; 2] {
    let mut result = [0.; 2];
    for f in samples.as_chunks::<2>().0.iter() {
        result[0] += f[0].abs();
        result[1] += f[1].abs();
    }
    result.map(|v| v / (samples.len() / 2) as f32)
}
#[test]
fn spatial_sources_use_composed_world_positions_and_listener_rotation() {
    let temp = tempfile::tempdir().unwrap();
    let (mut project, assets, sid, _, listener) = fixture(temp.path());
    let mut engine = Engine::new(&project).unwrap();
    let mut audio = SceneAudio::offline(48000, &project).unwrap();
    audio.sync(&project, &engine.snapshot(), &assets).unwrap();
    let mut samples = vec![0.; 4096];
    audio.render(&mut samples).unwrap();
    let right = levels(&samples[256..]);
    // Local X=10 would be out of range; world X=5 must be audible on the right.
    // Kira linearly maps the distance fraction to -60..0 dB.
    let expected = (8000. / 32768.) * 10_f32.powf((-60. * 4. / 9.) / 20.);
    let near_ear = expected * (1. + std::f32::consts::FRAC_PI_8.cos()) / 2.;
    let far_ear = expected - near_ear;
    assert!(
        (right[1] - near_ear).abs() < 1e-5 && (right[0] - far_ear).abs() < 1e-5,
        "right={right:?}, expected={expected}"
    );
    let mut rotated_parent = Entity::new("Listener parent");
    rotated_parent.components.insert(
        "Transform".into(),
        json!(Transform {
            rotation: [0., 1., 0., 0.],
            scale: [2., 3., 4.],
            ..Default::default()
        }),
    );
    let scene = project.scenes.get_mut(&sid).unwrap();
    scene.entities.get_mut(&listener).unwrap().parent = Some(rotated_parent.id.clone());
    scene
        .entities
        .insert(rotated_parent.id.clone(), rotated_parent);
    engine.sync(&project).unwrap();
    audio.sync(&project, &engine.snapshot(), &assets).unwrap();
    audio.render(&mut samples).unwrap();
    let left = levels(&samples[256..]);
    assert!(
        (left[0] - near_ear).abs() < 1e-5 && (left[1] - far_ear).abs() < 1e-5,
        "left={left:?}, expected={expected}"
    );
}
#[test]
fn finished_sources_stay_finished_until_a_play_edge_and_bad_sync_is_terminal() {
    let temp = tempfile::tempdir().unwrap();
    let (mut project, assets, sid, source, _) = fixture(temp.path());
    project
        .scenes
        .get_mut(&sid)
        .unwrap()
        .entities
        .get_mut(&source)
        .unwrap()
        .components
        .get_mut("AudioSource")
        .unwrap()["looping"] = json!(false);
    let mut engine = Engine::new(&project).unwrap();
    let mut audio = SceneAudio::offline(48000, &project).unwrap();
    audio.sync(&project, &engine.snapshot(), &assets).unwrap();
    let mut samples = vec![0.; 4000];
    audio.render(&mut samples).unwrap();
    assert!(samples.iter().any(|v| *v > 0.));
    audio.sync(&project, &engine.snapshot(), &assets).unwrap();
    audio.render(&mut samples).unwrap();
    assert!(samples.iter().all(|v| *v == 0.));
    for playing in [false, true] {
        project
            .scenes
            .get_mut(&sid)
            .unwrap()
            .entities
            .get_mut(&source)
            .unwrap()
            .components
            .get_mut("AudioSource")
            .unwrap()["playing"] = json!(playing);
        audio.sync(&project, &engine.snapshot(), &assets).unwrap();
    }
    audio.render(&mut samples).unwrap();
    assert!(samples.iter().any(|v| *v > 0.));
    // Invalid retained asset resolution cannot be silently resumed after repair.
    assert!(matches!(
        audio.sync(&project, &engine.snapshot(), &AssetStore::default()),
        Err(AudioError::MissingAsset(_))
    ));
    assert!(matches!(
        audio.render(&mut samples),
        Err(AudioError::FailedSession)
    ));
    assert!(matches!(
        audio.sync(&project, &engine.snapshot(), &assets),
        Err(AudioError::FailedSession)
    ));
}

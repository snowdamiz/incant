use super::{Fixture, actions, code, id, key};
use incant_game_runtime::HostedOfflineGameSession;
use incant_render_data::{
    material_keys::MaterialSamples,
    prewarm::{MaterialProfile, MaterialProgram, MaterialRecorder, RecordingContext},
};
use incant_render_game::{MaterialRecordingConsumer, NativeRenderHost, NativeRenderSettings};
#[path = "animation.rs"]
mod animation;

fn context() -> RecordingContext {
    RecordingContext {
        scene: id(99).0,
        cooked_scene_sha256: [1; 32],
        profile: MaterialProfile {
            samples: MaterialSamples::One,
            program: MaterialProgram::Wgsl,
        },
        script_sha256: [3; 32],
        input_sha256: [4; 32],
        tick_rate: 64,
    }
}
#[test]
fn native_committed_ticks_record_only_actual_mirror_keys_after_cache_deletion() {
    let fixture = Fixture::new(false);
    let settings = fixture.settings();
    let settings = NativeRenderSettings {
        consumer: context(),
        model_cache: settings.model_cache,
        texture_cache: settings.texture_cache,
        camera: settings.camera,
        text: settings.text,
        ui: settings.ui,
    };
    let mut game = HostedOfflineGameSession::<NativeRenderHost<MaterialRecordingConsumer>>::new(
        fixture.scene,
        code("for(const c of chunks) for(let i=0;i<c.ids.length;i++) if(c.ids[i]==='00000000000000000000000001' && input.actions.jump.pressed) c.transforms[i*10+7]=-1;"),
        actions(), vec![fixture.clip], 48000, settings,
    ).unwrap();
    let manifest = game.host().unwrap().consumer().manifest().unwrap();
    assert_eq!(manifest.observed.len(), 1);
    assert!(!manifest.observed[0].key.mirrored());
    assert_eq!(manifest.observed[0].ticks, 1);
    assert_eq!(manifest.identity.initial_models.len(), 1);
    fixture.temp.close().unwrap();
    for events in [vec![key(true)], vec![], vec![]] {
        let mut output = vec![0.; game.next_output_frames().unwrap() * 2];
        game.tick(&events, &mut output).unwrap();
    }
    let manifest = game.host().unwrap().consumer().manifest().unwrap();
    assert_eq!(manifest.recording.last_tick, 3);
    assert_eq!(manifest.observed.len(), 2);
    assert_eq!(manifest.observed[0].ticks, 1);
    assert_eq!(manifest.observed[0].last_tick, 0);
    assert!(manifest.observed[1].key.mirrored());
    assert_eq!(manifest.observed[1].first_tick, 1);
    assert_eq!(manifest.observed[1].last_tick, 3);
    assert_eq!(manifest.observed[1].ticks, 3);
    assert_eq!(
        manifest.identity.initial_models,
        manifest.recording.observed_models
    );
    assert_eq!(
        manifest.admit(&manifest.identity).unwrap().keys().count(),
        2
    );
}
#[test]
fn refreshes_do_not_invent_ticks_and_failed_observations_cannot_publish() {
    let (game, _temp) = Fixture::new(false).game(code(""));
    let native = game.host().unwrap().render_input();
    let input = native.input();
    let camera = native.camera_label();
    let mut recorder = MaterialRecorder::new(input, camera, 20, context()).unwrap();
    recorder.accept(input, camera, 20).unwrap();
    recorder.accept(input, camera, 20).unwrap();
    let manifest = recorder.manifest().unwrap();
    assert_eq!(manifest.recording.first_tick, 20);
    assert_eq!(manifest.recording.last_tick, 20);
    assert_eq!(manifest.observed[0].ticks, 1);
    recorder.accept(input, camera, 22).unwrap();
    assert_eq!(recorder.manifest().unwrap().observed[0].ticks, 2);
    assert!(recorder.accept(input, camera, 21).is_err());
    assert!(recorder.manifest().is_err());
    assert!(recorder.accept(input, camera, 23).is_err());
}

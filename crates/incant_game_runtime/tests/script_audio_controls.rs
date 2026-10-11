//! Exercise actual Kira PCM through the composed native VM/audio lifecycle.
#[path = "../../incant_runtime/tests/support/audio_controls.rs"]
mod fixture;
#[path = "../../incant_assets_runtime/tests/support/mod.rs"]
mod support;
use fixture::*;
use incant_audio_runtime::{Bus, ClipBinding, Mixer, PlayOptions, SourceState};
use incant_game_runtime::{GameError, OfflineGameSession};
use incant_input::InputActions;
use incant_script::CompiledScript;
use std::sync::Arc;
fn code(body: &str) -> CompiledScript {
    CompiledScript::compile(&format!("let n=0;exports.default={{update({{chunks,commands}}){{const bus='00000000000000000000000001',source='00000000000000000000000002';{body};n++;}}}};")).unwrap()
}
fn tick(game: &mut OfflineGameSession) -> Vec<f32> {
    let mut pcm = vec![0.; game.next_output_frames().unwrap() * 2];
    game.tick(&[], &mut pcm).unwrap();
    pcm
}
fn energy(pcm: &[f32]) -> f32 {
    pcm.iter().map(|v| v * v).sum::<f32>() / pcm.len() as f32
}
#[test]
fn actual_pcm_pause_resume_preserves_cursor_and_source_bus_gains_for_static_and_streamed_clips() {
    let temp = tempfile::tempdir().unwrap();
    let samples = (0..20000)
        .map(|i| {
            let v = (i as f32 * 0.035).sin() * 0.25;
            [v; 2]
        })
        .collect::<Vec<_>>();
    let key = support::cooked(temp.path(), &samples);
    for streaming in [false, true] {
        let mut game=OfflineGameSession::new(scene(streaming,true,true),code("if(n===1)commands.setAudioPlaying(source,false);if(n===3)commands.setAudioPlaying(source,true);if(n===4){commands.setAudioGain(source,-6);commands.setAudioBusGain(bus,-6);}"),InputActions::new(),vec![ClipBinding::load(id(90),temp.path(),&key).unwrap()],48000).unwrap();
        let mut reference = Mixer::offline(
            48000,
            &[Bus {
                id: "music".into(),
                parent: None,
                gain_db: 0.,
            }],
        )
        .unwrap();
        let voice = reference
            .play(
                Arc::new(incant_assets_runtime::load_audio(temp.path(), &key).unwrap()),
                PlayOptions {
                    bus: Some("music".into()),
                    looping: true,
                    streaming,
                    ..Default::default()
                },
            )
            .unwrap();
        for n in 0..6 {
            if n == 1 {
                reference.set_paused(voice, true).unwrap();
            }
            if n == 3 {
                reference.set_paused(voice, false).unwrap();
            }
            if n == 4 {
                reference.configure_voice(voice, -6., 0., 1.).unwrap();
                reference.set_bus_gain(Some("music"), -6.).unwrap();
            }
            let actual = tick(&mut game);
            let mut expected = vec![0.; actual.len()];
            reference.render(&mut expected).unwrap();
            assert_eq!(
                actual, expected,
                "cursor/PCM diverged: streaming={streaming}, tick={n}"
            );
            if n == 2 {
                assert!(energy(&actual[64..]) < 1e-10);
                assert_eq!(game.source_state(id(2)).unwrap(), SourceState::Paused);
            }
            if n == 3 {
                assert!(energy(&actual[256..]) > 0.001);
                assert_eq!(game.source_state(id(2)).unwrap(), SourceState::Playing);
            }
            if n == 5 {
                let nodes = audio(game.world().unwrap());
                assert_eq!(nodes[0].bus.as_ref().unwrap().gain_db, -6.);
                assert_eq!(nodes[1].source.as_ref().unwrap().gain_db, -6.);
                assert!(energy(&actual) < 0.004);
            }
        }
    }
}
#[test]
fn start_and_ended_voice_edges_match_documented_desired_state_semantics() {
    let temp = tempfile::tempdir().unwrap();
    let key = support::cooked(temp.path(), &[[0.2; 2]; 128]);
    for streaming in [false, true] {
        let mut game=OfflineGameSession::new(scene(streaming,false,false),code("if(n===1||n===2||n===5)commands.setAudioPlaying(source,true);if(n===3){commands.setAudioPlaying(source,false);commands.setAudioPlaying(source,true);}if(n===4)commands.setAudioPlaying(source,false);"),InputActions::new(),vec![ClipBinding::load(id(90),temp.path(),&key).unwrap()],48000).unwrap();
        assert_eq!(tick(&mut game), vec![0.; 1600]);
        assert!(energy(&tick(&mut game)) > 0.001);
        // Unchanged true and false/true in one transaction cannot create a mixer edge.
        assert_eq!(tick(&mut game), vec![0.; 1600]);
        assert_eq!(tick(&mut game), vec![0.; 1600]);
        assert_eq!(tick(&mut game), vec![0.; 1600]);
        assert!(energy(&tick(&mut game)) > 0.001);
    }
}
#[test]
fn script_failure_silences_unpublished_output_and_terminalizes_composed_audio_session() {
    let temp = tempfile::tempdir().unwrap();
    let key = support::cooked(temp.path(), &[[0.2; 2]; 24000]);
    for invalid in [
        "try{commands.setAudioGain(source,NaN)}catch{}",
        "commands.setAudioBusGain(source,-12)",
        "commands.despawn(source)",
    ] {
        let mut game=OfflineGameSession::new(scene(false,true,true),code(&format!("if(n===1){{chunks[0].transforms[0]=40;commands.setAudioPlaying(source,false);commands.setAudioBusGain(bus,-24);{invalid}}}")),InputActions::new(),vec![ClipBinding::load(id(90),temp.path(),&key).unwrap()],48000).unwrap();
        assert!(energy(&tick(&mut game)) > 0.001);
        let mut output = vec![0.7; game.next_output_frames().unwrap() * 2];
        assert!(game.tick(&[], &mut output).is_err());
        assert!(output.iter().all(|v| *v == 0.));
        assert!(game.is_failed());
        assert!(matches!(game.world(), Err(GameError::Failed)));
        assert!(game.source_state(id(2)).is_err());
    }
}

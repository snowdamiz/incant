//! Real native VM/state/timer + Kira reset. Platform/device acceptance is separate.
#[path = "../../incant_assets_runtime/tests/support/mod.rs"]
mod audio_support;
use incant_audio_runtime::ClipBinding;
use incant_game_runtime::{GameSeed, OfflineGameSession};
use incant_input::{InputEvent, KeyCode};
use incant_runtime::{CookedAudio, CookedEntity, CookedScene, NativeAudioSource, StableId};
use incant_script::{CompiledScript, SessionRequest};
fn id(n: u128) -> StableId {
    StableId(n.to_be_bytes())
}
fn seed(root: &std::path::Path) -> GameSeed {
    let mut clips = vec![];
    for (clip, rate) in [(90, 22050), (91, 32000)] {
        let path = root.join(clip.to_string());
        let pcm = (0..997)
            .map(|i| {
                [
                    (i as f32 * 0.071).sin() * 0.2,
                    (i as f32 * 0.043).cos() * 0.2,
                ]
            })
            .collect::<Vec<_>>();
        let key = audio_support::cooked(&path, &pcm);
        let metadata = path.join(format!("{key}.json"));
        let mut data: incant_assets_runtime::AudioMetadata =
            serde_json::from_slice(&std::fs::read(&metadata).unwrap()).unwrap();
        data.sample_rate = rate;
        std::fs::write(&metadata, serde_json::to_vec(&data).unwrap()).unwrap();
        clips.push(ClipBinding::load(id(clip), &path, &key).unwrap());
    }
    let scene = CookedScene::new(
        id(99),
        144,
        (1..=3)
            .map(|n| CookedEntity {
                id: id(n),
                parent: None,
                transform: Default::default(),
                velocity: None,
            })
            .collect(),
    )
    .unwrap()
    .with_audio(
        (1..=2)
            .map(|n| CookedAudio {
                entity: id(n),
                bus: None,
                listener: false,
                source: Some(NativeAudioSource {
                    transport_epoch: 0,
                    clip: id(89 + n),
                    bus: None,
                    gain_db: -5.,
                    pan: 0.,
                    rate: if n == 1 { 1.13 } else { 0.87 },
                    start_seconds: 0.00037,
                    playing: true,
                    looping: true,
                    streaming: n == 2,
                    spatial: None,
                }),
            })
            .collect(),
    )
    .unwrap();
    let script=CompiledScript::compile(r#"
let calls=0;
exports.default={persistence:{version:1},update(t){
  calls++; const state=Number(t.state.get()||0)+1+t.timers.events.length;
  if(calls!==t.clock.tick)throw Error('VM not fresh');
  if(t.clock.tick===1){if(state!==1)throw Error('state not fresh');t.timers.set('pulse',3,3);}
  for(const c of t.chunks)for(let i=0;i<c.ids.length;i++)c.transforms[i*c.transformStride]=state;
  if(t.clock.tick===3)t.commands.despawn('00000000000000000000000003');
  if(t.clock.tick===5)t.commands.setAudioGain('00000000000000000000000001',-10);
  if(t.clock.tick===9)t.commands.requestRestart();
  t.state.set(state);t.log(`${calls}:${state}:${t.timers.events.length}:${t.input.keyboard.held.length}`);
}}"#).unwrap();
    GameSeed::new(scene, script, Default::default(), clips).unwrap()
}
fn run(game: &mut OfflineGameSession) -> Vec<f32> {
    let mut result = vec![];
    for tick in 1..=9 {
        let mut output = vec![0.; 2 * game.next_output_frames().unwrap()];
        let events = if tick == 4 {
            vec![InputEvent::Key {
                code: KeyCode::KeyD,
                down: true,
            }]
        } else {
            vec![]
        };
        game.tick(&events, &mut output).unwrap();
        result.extend(output);
    }
    result
}
#[test]
fn reset_recreates_vm_world_input_timers_and_static_streamed_mixer_clocks() {
    let temp = tempfile::tempdir().unwrap();
    let seed = seed(temp.path());
    let mut game = OfflineGameSession::from_seed(&seed, 44117).unwrap();
    let first_id = game
        .world()
        .unwrap()
        .spatial_view()
        .unwrap()
        .identity()
        .clone();
    let first = run(&mut game);
    assert!(first.iter().any(|v| v.abs() > 0.01));
    let logs = serde_json::to_value(game.logs().unwrap()).unwrap();
    assert_eq!(game.world().unwrap().entity_count(), 2);
    assert!(game.input().unwrap().keyboard.held.contains(&KeyCode::KeyD));
    assert_eq!(
        game.take_session_request().unwrap(),
        Some(SessionRequest::Restart)
    );
    game.restart_from(&seed).unwrap();
    assert_ne!(
        &first_id,
        game.world().unwrap().spatial_view().unwrap().identity()
    );
    assert_eq!(game.world().unwrap().tick(), 0);
    assert_eq!(game.world().unwrap().entity_count(), 3);
    assert_eq!(game.rendered_frames().unwrap(), 0);
    assert!(game.logs().unwrap().entries().is_empty());
    assert!(game.input().unwrap().keyboard.held.is_empty());
    assert_eq!(game.take_session_request().unwrap(), None);
    assert_eq!(run(&mut game), first);
    assert_eq!(serde_json::to_value(game.logs().unwrap()).unwrap(), logs);
    let mut fresh = OfflineGameSession::from_seed(&seed, 44117).unwrap();
    assert_eq!(run(&mut fresh), first);
    assert_eq!(serde_json::to_value(fresh.logs().unwrap()).unwrap(), logs);
}

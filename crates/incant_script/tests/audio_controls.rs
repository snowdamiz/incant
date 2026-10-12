#![cfg(feature = "compiler")]
#[path = "../../incant_runtime/tests/support/audio_controls.rs"]
mod fixture;
use fixture::*;
use incant_runtime::*;
use incant_script::{CompiledScript, ScriptError, ScriptHost};
use std::time::Duration;
fn host(body: &str) -> ScriptHost {
    ScriptHost::with_budget(CompiledScript::compile(&format!("exports.default={{update({{chunks,commands}}){{const bus='00000000000000000000000001',source='00000000000000000000000002',plain='00000000000000000000000003';{body}}}}};")).unwrap(),Duration::from_secs(2)).unwrap()
}
#[test]
fn script_controls_preserve_other_audio_fields_and_expired_functions_never_reactivate() {
    let mut world = NativeWorld::new(scene(false, true, true));
    let mut host = host(
        "if(globalThis.old){for(const f of old){let refused=false;try{f(source,0)}catch{refused=true}if(!refused)throw Error('old cap')}}globalThis.old=[commands.setAudioPlaying,commands.setAudioGain,commands.setAudioBusGain];commands.setAudioPlaying(source,false);commands.setAudioPlaying(source,true);commands.setAudioGain(source,-9);commands.setAudioBusGain(bus,-6);",
    );
    let mut expected = audio(&world);
    expected[0].bus.as_mut().unwrap().gain_db = -6.;
    expected[1].source.as_mut().unwrap().gain_db = -9.;
    for _ in 0..3 {
        host.tick(&mut world, 1. / 60.).unwrap();
        assert_eq!(audio(&world), expected);
    }
}
#[test]
fn caught_bad_arguments_and_bad_final_roles_roll_back_all_script_commands_and_numeric_writes() {
    for bad in [
        "commands.stopAudio(source,0)",
        "commands.restartAudio()",
        "commands.restartAudio(bus)",
        "commands.stopAudio(plain)",
        "commands.seekAudio(source,'0')",
        "commands.seekAudio(source,NaN)",
        "commands.seekAudio(source,Infinity)",
        "commands.seekAudio(source,-0.000001)",
        "commands.seekAudio(source,3600.0000001)",
        "commands.setAudioPlaying(source,0)",
        "commands.setAudioPlaying(source,true,0)",
        "commands.setAudioGain(source,'0')",
        "commands.setAudioGain(source,NaN)",
        "commands.setAudioGain(source,Infinity)",
        "commands.setAudioGain(source,24.000000001)",
        "commands.setAudioBusGain(bus,-80.000000001)",
        "commands.setAudioGain('bad',0)",
        "commands.setAudioPlaying(bus,false)",
        "commands.setAudioBusGain(source,0)",
        "commands.setAudioGain(plain,0)",
    ] {
        let mut world = NativeWorld::new(scene(false, true, true));
        let before = audio(&world);
        let snapshot = world.snapshot();
        let mut host = host(&format!(
            "chunks[0].transforms[0]=40;commands.setAudioPlaying(source,false);commands.setAudioBusGain(bus,-20);commands.setVelocity(plain,[7,0,0]);try{{{bad}}}catch{{}}"
        ));
        assert!(host.tick(&mut world, 1. / 60.).is_err(), "{bad}");
        assert_eq!(audio(&world), before, "{bad}");
        assert_eq!(world.snapshot(), snapshot, "{bad}");
        assert!(matches!(
            host.tick(&mut world, 1. / 60.),
            Err(ScriptError::FailedSession)
        ));
    }
}

#[test]
fn script_transport_restarts_in_one_tick_and_expired_capabilities_never_reactivate() {
    let mut world = NativeWorld::new(scene(false, true, true));
    let mut host = host(
        "if(globalThis.old){for(let i=0;i<old.length;i++){let refused=false;try{if(i===2)old[i](source,0);else old[i](source)}catch{refused=true}if(!refused)throw Error('old transport capability')}}globalThis.old=[commands.stopAudio,commands.restartAudio,commands.seekAudio];commands.stopAudio(source);commands.restartAudio(source);commands.seekAudio(source,0.125);",
    );
    for tick in 1..=3 {
        host.tick(&mut world, 1. / 60.).unwrap();
        let nodes = audio(&world);
        let s = nodes[1].source.as_ref().unwrap();
        assert_eq!(
            (s.transport_epoch, s.start_seconds, s.playing),
            (tick * 3, 0.125, true)
        );
    }
}
#[test]
fn mixed_despawn_rejection_and_shared_queue_limit_cannot_publish_audio_prefix() {
    for tail in [
        "commands.despawn(source);",
        "for(let i=0;i<65536;i++){try{commands.setAudioPlaying(source,true)}catch{}}",
    ] {
        let mut world = NativeWorld::new(scene(false, true, true));
        let before = audio(&world);
        let snapshot = world.snapshot();
        let mut host = host(&format!(
            "chunks[0].transforms[0]=40;commands.setAudioPlaying(source,false);commands.setAudioGain(source,-12);{tail}"
        ));
        assert!(host.tick(&mut world, 1. / 60.).is_err());
        assert_eq!(audio(&world), before);
        assert_eq!(world.snapshot(), snapshot);
    }
}

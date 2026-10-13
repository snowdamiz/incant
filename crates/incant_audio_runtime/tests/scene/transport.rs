use super::*;
use incant_runtime::AudioControl;

fn control(world: &mut NativeWorld, items: &[AudioControl]) {
    let mut commands = FrameCommands::default();
    for &control in items {
        commands
            .push(StructuralCommand::ControlAudio {
                entity: id(2),
                control,
            })
            .unwrap();
    }
    world.apply(&mut commands).unwrap();
}

#[test]
fn explicit_restart_replays_finished_one_shots_without_an_intermediate_pause_tick() {
    let temp = tempfile::tempdir().unwrap();
    let key = support::cooked(temp.path(), &[[0.2; 2]; 128]);
    for streaming in [false, true] {
        let mut node = source(streaming);
        node.source.as_mut().unwrap().looping = false;
        let mut world = world(vec![bus(), node]);
        let mut host = SceneAudio::offline(&world, vec![binding(&temp, &key)], 48000).unwrap();
        assert!(energy(&render(&mut host, 256)) > 0.001);
        host.sync(&world).unwrap();
        assert_eq!(render(&mut host, 256), vec![0.; 512]);
        for _ in 0..3 {
            control(&mut world, &[AudioControl::Restart]);
            host.sync(&world).unwrap();
            assert!(
                energy(&render(&mut host, 256)) > 0.001,
                "streaming={streaming}"
            );
            host.sync(&world).unwrap();
            assert_eq!(render(&mut host, 256), vec![0.; 512]);
        }
    }
}

#[test]
fn stop_resets_and_paused_seek_preserves_silence_until_resume() {
    let temp = tempfile::tempdir().unwrap();
    let key = tone(&temp);
    for streaming in [false, true] {
        let mut world = world(vec![bus(), source(streaming)]);
        let mut host = SceneAudio::offline(&world, vec![binding(&temp, &key)], 48000).unwrap();
        render(&mut host, 1200);
        control(&mut world, &[AudioControl::Stop, AudioControl::Seek(0.1)]);
        host.sync(&world).unwrap();
        render(&mut host, 512); // Allow the acknowledged old voice to retire.
        host.sync(&world).unwrap();
        assert_eq!(render(&mut host, 512), vec![0.; 1024]);
        control(&mut world, &[AudioControl::SetPlaying(true)]);
        host.sync(&world).unwrap();
        assert!(energy(&render(&mut host, 512)) > 0.001);
        control(&mut world, &[AudioControl::Restart, AudioControl::Stop]);
        host.sync(&world).unwrap();
        render(&mut host, 512);
        host.sync(&world).unwrap();
        assert_eq!(render(&mut host, 512), vec![0.; 1024]);
    }
}

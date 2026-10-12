use super::*;
use incant_runtime::AudioControl;

fn control(world: &mut NativeWorld, entity: u128, control: AudioControl) {
    let mut commands = FrameCommands::default();
    commands
        .push(StructuralCommand::ControlAudio {
            entity: id(entity),
            control,
        })
        .unwrap();
    world.apply(&mut commands).unwrap();
}

#[test]
fn saved_cursors_bind_consumed_transport_epochs_before_restore() {
    let temp = tempfile::tempdir().unwrap();
    let key = fixture(temp.path());
    let mut world = NativeWorld::new(scene());
    let mut host = SceneAudio::offline(&world, clips(temp.path(), &key), 44100).unwrap();
    render(&mut host, 193);
    let before = host.checkpoint_for_world(&world).unwrap();
    assert_eq!(&before.bytes()[8..12], &1u32.to_le_bytes());
    // Seeking the current desired start leaves every old scalar unchanged.
    // Only the request epoch can distinguish this new playback request.
    control(&mut world, 2, AudioControl::Seek(0.00037));
    assert!(PendingAudioCheckpoint::new(&world, before.bytes()).is_err());
    assert!(host.checkpoint_for_world(&world).is_err());
    host.sync(&world).unwrap();
    // The mixer only captures accepted render boundaries, never a queued stop.
    render(&mut host, 71);
    let snapshot = host.checkpoint_for_world(&world).unwrap();
    assert_eq!(&snapshot.bytes()[8..12], &2u32.to_le_bytes());
    let fresh = NativeWorld::from_save_bytes(&world.save_bytes().unwrap()).unwrap();
    let pending = PendingAudioCheckpoint::new(&fresh, snapshot.bytes()).unwrap();
    let mut restored = pending
        .restore(&fresh, clips(temp.path(), &key), 44100)
        .unwrap();
    assert_eq!(
        restored.checkpoint_for_world(&fresh).unwrap().bytes(),
        snapshot.bytes()
    );
    for frames in [13, 171, 23, 500, 73] {
        host.sync(&world).unwrap();
        restored.sync(&fresh).unwrap();
        assert_eq!(render(&mut host, frames), render(&mut restored, frames));
        assert_eq!(
            host.checkpoint_for_world(&world).unwrap().bytes(),
            restored.checkpoint_for_world(&fresh).unwrap().bytes()
        );
    }
}

#[test]
fn stop_seek_restart_continuation_preserves_static_and_streamed_pcm() {
    let temp = tempfile::tempdir().unwrap();
    let key = fixture(temp.path());
    let mut world = NativeWorld::new(scene());
    let mut live = SceneAudio::offline(&world, clips(temp.path(), &key), 44100).unwrap();
    render(&mut live, 211);
    for n in [2, 3] {
        control(&mut world, n, AudioControl::Stop);
        control(&mut world, n, AudioControl::Seek(0.02));
    }
    live.sync(&world).unwrap();
    render(&mut live, 71);
    let snapshot = live.checkpoint_for_world(&world).unwrap();
    let mut fresh = NativeWorld::from_save_bytes(&world.save_bytes().unwrap()).unwrap();
    let mut restored = PendingAudioCheckpoint::new(&fresh, snapshot.bytes())
        .unwrap()
        .restore(&fresh, clips(temp.path(), &key), 44100)
        .unwrap();
    for (step, frames) in [171, 31, 700, 23, 511].into_iter().enumerate() {
        if step == 1 || step == 3 {
            for w in [&mut world, &mut fresh] {
                for n in [2, 3] {
                    control(
                        w,
                        n,
                        if step == 1 {
                            AudioControl::SetPlaying(true)
                        } else {
                            AudioControl::Restart
                        },
                    );
                }
            }
        }
        live.sync(&world).unwrap();
        restored.sync(&fresh).unwrap();
        assert_eq!(render(&mut live, frames), render(&mut restored, frames));
        assert_eq!(
            live.checkpoint_for_world(&world).unwrap().bytes(),
            restored.checkpoint_for_world(&fresh).unwrap().bytes()
        );
    }
}

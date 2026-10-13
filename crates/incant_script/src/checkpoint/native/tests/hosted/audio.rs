use super::*;
use incant_audio_runtime::{OfflineBackend, SceneAudio};
use incant_runtime::CookedAudio;
type Audio = SceneAudio<OfflineBackend>;
fn scene() -> CookedScene {
    physical_scene()
        .with_audio(vec![CookedAudio {
            entity: id(1),
            bus: None,
            source: None,
            listener: true,
        }])
        .unwrap()
}
fn tick(live: &mut NativeSession, audio: &mut Audio) -> Vec<f32> {
    let tick = live.world.tick();
    let rate = u128::from(44100u32);
    let hz = u128::from(live.world.tick_rate());
    let frames = ((u128::from(tick + 1) * rate / hz) - (u128::from(tick) * rate / hz)) as usize;
    live.tick(&[]).unwrap();
    audio.sync(&live.world).unwrap();
    let mut output = vec![7.; frames * 2];
    audio.render(&mut output).unwrap();
    output
}
fn capture(live: &mut NativeSession, audio: &Audio) -> Result<NativeSessionCheckpoint> {
    live.checkpoint_with_audio(
        audio,
        #[cfg(feature = "ui")]
        None,
        &mut payload(),
        &mut planar(),
        &mut spatial(),
    )
}
#[test]
fn offline_clock_owner_and_physics_restore_before_vm() {
    let mut live = live(scene());
    let mut audio = Audio::offline(&live.world, vec![], 44100).unwrap();
    for _ in 0..13 {
        assert!(tick(&mut live, &mut audio).iter().all(|&v| v == 0.));
    }
    let checkpoint = capture(&mut live, &audio).unwrap();
    assert!(prepare(&checkpoint).unwrap().start().is_err());
    assert!(
        prepare(&checkpoint)
            .unwrap()
            .start_with_audio(
                &audio,
                #[cfg(feature = "ui")]
                None
            )
            .is_err()
    );
    let pending = prepare(&checkpoint).unwrap();
    let saved = pending.audio().unwrap();
    assert_eq!(saved.rendered_frames(), 3981);
    assert!(saved.restore(pending.world(), vec![], 48000).is_err());
    let mut rebuilt = saved.restore(pending.world(), vec![], 44100).unwrap();
    let mut resumed = pending
        .start_with_audio(
            &rebuilt,
            #[cfg(feature = "ui")]
            None,
        )
        .unwrap();
    for _ in 0..32 {
        assert_eq!(
            tick(&mut live, &mut audio),
            tick(&mut resumed, &mut rebuilt)
        );
        assert_eq!(live.world.snapshot(), resumed.world.snapshot());
        assert_eq!(
            audio.checkpoint_for_world(&live.world).unwrap().bytes(),
            rebuilt
                .checkpoint_for_world(&resumed.world)
                .unwrap()
                .bytes()
        );
    }
    let before = live.world.snapshot();
    assert!(capture(&mut live, &rebuilt).is_err());
    audio.render(&mut [0.; 2]).unwrap();
    assert!(matches!(
        capture(&mut live, &audio),
        Err(CheckpointError::Audio(
            incant_audio_runtime::AudioError::Invalid("audio/world sample clock")
        ))
    ));
    assert_eq!(live.world.snapshot(), before);
}

#[cfg(feature = "ui")]
#[test]
fn both_required_owners_are_checked_against_one_pending_world() {
    use incant_ui::UiRuntime;
    let initial = super::ui::scene()
        .with_audio(vec![CookedAudio {
            entity: id(3),
            bus: None,
            source: None,
            listener: true,
        }])
        .unwrap();
    let mut live = live(initial);
    let mut audio = Audio::offline(&live.world, vec![], 44100).unwrap();
    tick(&mut live, &mut audio);
    let mut ui = UiRuntime::for_world(&live.world).unwrap();
    super::ui::sync(&mut ui, &live, &[]);
    assert!(capture(&mut live, &audio).is_err());
    assert!(
        live.checkpoint_with_ui(&ui, &mut payload(), &mut planar(), &mut spatial())
            .is_err()
    );
    let checkpoint = live
        .checkpoint_with_audio(
            &audio,
            Some(&ui),
            &mut payload(),
            &mut planar(),
            &mut spatial(),
        )
        .unwrap();
    let pending = prepare(&checkpoint).unwrap();
    let rebuilt = pending
        .audio()
        .unwrap()
        .restore(pending.world(), vec![], 44100)
        .unwrap();
    assert!(pending.start_with_audio(&rebuilt, None).is_err());
    let pending = prepare(&checkpoint).unwrap();
    let rebuilt = pending
        .audio()
        .unwrap()
        .restore(pending.world(), vec![], 44100)
        .unwrap();
    let rebuilt_ui = pending
        .ui()
        .unwrap()
        .restore(pending.world(), [400.; 2], &mut super::ui::Measure)
        .unwrap();
    assert!(
        pending
            .start_with_audio(&rebuilt, Some(&rebuilt_ui))
            .is_ok()
    );
}

#[test]
fn silent_owner_clock_survives_a_later_audio_declaration() {
    use incant_runtime::{FrameCommands, StructuralCommand};
    let mut live = live(physical_scene());
    let mut audio = Audio::offline(&live.world, vec![], 44100).unwrap();
    for _ in 0..13 {
        tick(&mut live, &mut audio);
    }
    assert!(!live.world.requires_audio_checkpoint());
    let checkpoint = capture(&mut live, &audio).unwrap();
    let pending = prepare(&checkpoint).unwrap();
    assert_eq!(pending.audio().unwrap().rendered_frames(), 3981);
    assert!(pending.start().is_err());
    #[cfg(feature = "ui")]
    {
        let pending = prepare(&checkpoint).unwrap();
        let ui = incant_ui::UiRuntime::for_world(pending.world()).unwrap();
        assert!(pending.start_with_ui(&ui).is_err());
    }
    let pending = prepare(&checkpoint).unwrap();
    let mut rebuilt = pending
        .audio()
        .unwrap()
        .restore(pending.world(), vec![], 44100)
        .unwrap();
    let mut resumed = pending
        .start_with_audio(
            &rebuilt,
            #[cfg(feature = "ui")]
            None,
        )
        .unwrap();
    for session in [&mut live, &mut resumed] {
        let mut commands = FrameCommands::default();
        commands
            .push(StructuralCommand::SetAudio {
                entity: id(1),
                audio: Some(CookedAudio {
                    entity: id(1),
                    bus: None,
                    source: None,
                    listener: true,
                }),
            })
            .unwrap();
        session.world.apply(&mut commands).unwrap();
        assert!(session.world.requires_audio_checkpoint());
    }
    for _ in 0..17 {
        assert_eq!(
            tick(&mut live, &mut audio),
            tick(&mut resumed, &mut rebuilt)
        );
        assert_eq!(live.world.snapshot(), resumed.world.snapshot());
        let a = audio.checkpoint_for_world(&live.world).unwrap();
        let b = rebuilt.checkpoint_for_world(&resumed.world).unwrap();
        a.validate_session(&live.world).unwrap();
        b.validate_session(&resumed.world).unwrap();
        assert_eq!(a.bytes(), b.bytes());
        assert_eq!(b.rendered_frames(), live.world.tick() * 44100 / 144);
    }
    assert!(capture(&mut live, &audio).is_ok());
    assert!(capture(&mut resumed, &rebuilt).is_ok());
}

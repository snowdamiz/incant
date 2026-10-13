use super::*;
use incant_runtime::CookedUi;
use incant_types::{UiKind, UiLength, UiNode, UiStyle, UiText};
use incant_ui::{MeasureSpace, TextMeasure, UiError, UiEvent, UiRuntime, Widget};
use std::sync::Arc;

pub(super) struct Measure;
impl TextMeasure for Measure {
    fn measure(
        &mut self,
        _: &Widget,
        text: &str,
        known: [Option<f32>; 2],
        _: [MeasureSpace; 2],
    ) -> std::result::Result<[f32; 2], UiError> {
        Ok([
            known[0].unwrap_or(text.chars().count() as f32 * 8.),
            known[1].unwrap_or(20.),
        ])
    }
}
pub(super) fn scene() -> CookedScene {
    physical_scene()
        .with_text(
            vec![incant_runtime::FontAssetRef {
                id: id(90),
                fingerprint: [7; 32],
            }],
            vec![],
        )
        .unwrap()
        .with_ui(vec![
            CookedUi {
                entity: id(1),
                node: Arc::new(UiNode {
                    kind: UiKind::Root,
                    order: 0,
                    disabled: false,
                    style: Default::default(),
                }),
            },
            CookedUi {
                entity: id(2),
                node: Arc::new(UiNode {
                    kind: UiKind::TextInput {
                        text: UiText {
                            content: "seed".into(),
                            fonts: vec![id(90)],
                            font_size: 16.,
                            line_height: 20.,
                            wrap: incant_types::TextWrap::Word,
                        },
                    },
                    order: 0,
                    disabled: false,
                    style: UiStyle {
                        size: [UiLength::Pixels(180.), UiLength::Pixels(30.)],
                        ..Default::default()
                    },
                }),
            },
        ])
        .unwrap()
}
pub(super) fn sync(ui: &mut UiRuntime, live: &NativeSession, events: &[UiEvent]) {
    let next = ui.prepare(events, [400.; 2], &mut Measure).unwrap();
    ui.commit_for_world(next, live.world().unwrap()).unwrap();
}
fn capture(live: &mut NativeSession, ui: &UiRuntime) -> NativeSessionCheckpoint {
    live.checkpoint_with_ui(ui, &mut payload(), &mut planar(), &mut spatial())
        .unwrap()
}
fn restored(checkpoint: &NativeSessionCheckpoint) -> (NativeSession, UiRuntime) {
    let pending = prepare(checkpoint).unwrap();
    let ui = pending
        .ui()
        .unwrap()
        .restore(pending.world(), [400.; 2], &mut Measure)
        .unwrap();
    (pending.start_with_ui(&ui).unwrap(), ui)
}

#[test]
fn exact_ui_owner_precedes_vm_and_preserves_widget_and_physics_continuation() {
    let mut live = live(scene());
    let mut ui = UiRuntime::for_world(live.world().unwrap()).unwrap();
    sync(&mut ui, &live, &[]);
    sync(&mut ui, &live, &[UiEvent::Focus(id(2))]);
    let target = ui.focus().unwrap();
    sync(
        &mut ui,
        &live,
        &[UiEvent::Commit {
            target,
            text: " saved".into(),
        }],
    );
    for _ in 0..13 {
        live.tick(&[]).unwrap();
        sync(&mut ui, &live, &[]);
    }
    live.enable_restart_requests().unwrap();
    let before = live.world().unwrap().snapshot();
    let checkpoint = capture(&mut live, &ui);
    assert!(reconstruct(&checkpoint).is_err());
    assert!(prepare(&checkpoint).unwrap().start_with_ui(&ui).is_err());
    let mut unrelated = live_from_same_scene();
    assert!(
        unrelated
            .checkpoint_with_ui(&ui, &mut payload(), &mut planar(), &mut spatial())
            .is_err()
    );
    let (mut resumed, mut rebuilt_ui) = restored(&checkpoint);
    assert!(!host(&resumed).unwrap().restart_requests);
    assert!(rebuilt_ui.text(id(2)).unwrap().text().contains(" saved"));
    assert_ne!(resumed.world.incarnation(), live.world.incarnation());
    let pending = prepare_for(&checkpoint, &live).unwrap();
    let rebuilt = pending
        .ui()
        .unwrap()
        .restore(pending.world(), [400.; 2], &mut Measure)
        .unwrap();
    assert!(
        host(&pending.start_with_ui(&rebuilt).unwrap())
            .unwrap()
            .restart_requests
    );
    assert_eq!(live.world.snapshot(), before);
    for _ in 0..32 {
        live.tick(&[]).unwrap();
        resumed.tick(&[]).unwrap();
        sync(&mut ui, &live, &[]);
        sync(&mut rebuilt_ui, &resumed, &[]);
        assert_eq!(live.world.snapshot(), resumed.world.snapshot());
        assert_eq!(live.world.physics_events(), resumed.world.physics_events());
        assert_eq!(
            ui.checkpoint_for_world(&live.world).unwrap().bytes(),
            rebuilt_ui
                .checkpoint_for_world(&resumed.world)
                .unwrap()
                .bytes()
        );
    }
    // Starting a session tick without publishing its UI owner makes capture stale.
    live.tick(&[]).unwrap();
    assert!(
        live.checkpoint_with_ui(&ui, &mut payload(), &mut planar(), &mut spatial())
            .is_err()
    );
}
fn live_from_same_scene() -> NativeSession {
    live(scene())
}

#[test]
fn host_pairing_failure_precedes_late_guest_initialization() {
    let mut live = live(scene());
    live.tick(&[]).unwrap();
    let mut ui = UiRuntime::for_world(live.world().unwrap()).unwrap();
    sync(&mut ui, &live, &[]);
    let mut checkpoint = capture(&mut live, &ui);
    // Private test-only mutation reaches a late trap without a production setter.
    checkpoint.code = CompiledScript::compile("throw new Error('late init');").unwrap();
    let pending = prepare(&checkpoint).unwrap();
    assert!(matches!(
        pending.start_with_ui(&ui),
        Err(CheckpointError::Ui(_))
    ));
    let pending = prepare(&checkpoint).unwrap();
    let rebuilt = pending
        .ui()
        .unwrap()
        .restore(pending.world(), [400.; 2], &mut Measure)
        .unwrap();
    assert!(matches!(
        pending.start_with_ui(&rebuilt),
        Err(CheckpointError::Script(_))
    ));
    live.tick(&[]).unwrap();
    assert_eq!(live.world.tick(), 2);
}

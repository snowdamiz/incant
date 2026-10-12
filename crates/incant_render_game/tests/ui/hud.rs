use super::*;
#[test]
fn accepted_hud_text_rebuilds_actual_shaped_input_and_preserves_pending_typing() {
    let (mut g, temp) = game(
        "if(input.ui.events.some(e=>e.kind==='value_changed'))commands.setUiText('00000000000000000000000002','Health 9');",
    );
    temp.close().unwrap();
    tick(&mut g, &[], &[UiEvent::Focus(id(3))]);
    let old = Arc::clone(g.host().unwrap().ui_input().unwrap());
    let target = g.host().unwrap().ui().unwrap().focus().unwrap();
    tick(
        &mut g,
        &[],
        &[UiEvent::Commit {
            target: target.clone(),
            text: "X".into(),
        }],
    );
    let input = g.host().unwrap().ui_input().unwrap();
    assert!(!Arc::ptr_eq(&old, input));
    assert_eq!(
        old.widgets
            .iter()
            .find(|w| w.id == id(2))
            .unwrap()
            .layout
            .source(),
        "Name"
    );
    assert_eq!(
        input
            .widgets
            .iter()
            .find(|w| w.id == id(2))
            .unwrap()
            .layout
            .source(),
        "Health 9"
    );
    assert_eq!(
        g.host().unwrap().ui().unwrap().text(id(3)).unwrap().text(),
        "NameX"
    );
    assert!(g.host().unwrap().ui().unwrap().focus().is_some());
    tick(
        &mut g,
        &[],
        &[UiEvent::Preedit {
            target,
            text: "stale".into(),
            cursor: None,
        }],
    );
    assert_eq!(g.input().unwrap().ui.discarded_events, 1);
    assert_eq!(
        g.host().unwrap().ui().unwrap().text(id(3)).unwrap().text(),
        "NameX"
    );
}
#[test]
fn disabled_subtree_retires_focus_and_changed_input_declaration_resets_buffer() {
    let (mut g, _temp) = game(
        "if(input.ui.events.some(e=>e.kind==='value_changed')){commands.setUiText('00000000000000000000000003','Reset');commands.setUiDisabled('00000000000000000000000001',true);}",
    );
    tick(&mut g, &[], &[UiEvent::Focus(id(3))]);
    let target = g.host().unwrap().ui().unwrap().focus().unwrap();
    tick(
        &mut g,
        &[],
        &[UiEvent::Commit {
            target,
            text: "X".into(),
        }],
    );
    assert_eq!(
        g.host().unwrap().ui().unwrap().text(id(3)).unwrap().text(),
        "Reset"
    );
    assert!(g.host().unwrap().ui().unwrap().focus().is_none());
    assert!(
        g.host()
            .unwrap()
            .ui_input()
            .unwrap()
            .widgets
            .iter()
            .all(|w| w.disabled)
    );
}
#[test]
fn invalid_hud_batch_does_not_publish_pending_input_or_pcm() {
    let (mut g, _temp) = game(
        "if(input.ui.events.length){commands.setUiText('00000000000000000000000002','prefix');commands.setUiText('00000000000000000000000001','invalid root text');}",
    );
    tick(&mut g, &[], &[UiEvent::Focus(id(3))]);
    let accepted = Arc::clone(g.host().unwrap().ui_input().unwrap());
    let target = g.host().unwrap().ui().unwrap().focus().unwrap();
    let mut pcm = vec![1.; g.next_output_frames().unwrap() * 2];
    assert!(
        g.tick_with_ui(
            &[],
            &[UiEvent::Commit {
                target,
                text: "X".into()
            }],
            &mut pcm
        )
        .is_err()
    );
    assert!(g.is_failed());
    assert!(g.host().is_err());
    assert!(pcm.iter().all(|x| *x == 0.));
    assert_eq!(
        accepted
            .widgets
            .iter()
            .find(|w| w.id == id(2))
            .unwrap()
            .layout
            .source(),
        "Name"
    );
    assert_eq!(
        accepted
            .widgets
            .iter()
            .find(|w| w.id == id(3))
            .unwrap()
            .layout
            .source(),
        "Name"
    );
}

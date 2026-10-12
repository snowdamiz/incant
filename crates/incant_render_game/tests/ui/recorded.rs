use super::*;
use incant_input::RecordedUiEvent as Event;
use incant_render_game::tick_recorded_ui;
fn label(n: u128) -> String {
    ulid::Ulid::from_bytes(id(n).0).to_string()
}
fn run(
    game: &mut Game,
    events: &[Event],
    viewport: Option<[f32; 2]>,
) -> Result<(), incant_render_game::RecordedUiError> {
    let mut pcm = vec![0.; game.next_output_frames().unwrap() * 2];
    tick_recorded_ui(game, &[], events, viewport, &mut pcm)
}
#[test]
fn replay_resolves_preedit_cancel_commit_and_pointer_after_resize() {
    let (mut g, temp) = game("");
    temp.close().unwrap();
    run(
        &mut g,
        &[Event::Focus {
            entity_id: label(3),
        }],
        None,
    )
    .unwrap();
    run(
        &mut g,
        &[Event::Preedit {
            entity_id: label(3),
            text: "é".into(),
            cursor: Some([0, 2]),
        }],
        Some([240., 180.]),
    )
    .unwrap();
    assert_eq!(
        g.host().unwrap().ui().unwrap().text(id(3)).unwrap().text(),
        "Name"
    );
    assert!(
        g.host()
            .unwrap()
            .ui()
            .unwrap()
            .text(id(3))
            .unwrap()
            .preedit()
            .is_some()
    );
    run(
        &mut g,
        &[Event::Cancel {
            entity_id: label(3),
        }],
        None,
    )
    .unwrap();
    run(
        &mut g,
        &[Event::Commit {
            entity_id: label(3),
            text: "ok".into(),
        }],
        Some([300., 160.]),
    )
    .unwrap();
    assert_eq!(
        g.host().unwrap().ui().unwrap().text(id(3)).unwrap().text(),
        "Nameok"
    );
    assert_eq!(g.input().unwrap().ui.events.len(), 1);
    let rect = g
        .host()
        .unwrap()
        .ui()
        .unwrap()
        .layout()
        .unwrap()
        .nodes
        .iter()
        .find(|n| n.id == id(2))
        .unwrap()
        .hit;
    let position = std::array::from_fn(|i| rect.origin[i] + rect.size[i] * 0.5);
    run(
        &mut g,
        &[Event::Pointer {
            position,
            pressed: true,
        }],
        Some([310., 160.]),
    )
    .unwrap();
    run(
        &mut g,
        &[Event::Pointer {
            position,
            pressed: false,
        }],
        None,
    )
    .unwrap();
    assert_eq!(g.input().unwrap().ui.events.len(), 1);
    run(&mut g, &[], None).unwrap();
    assert!(g.input().unwrap().ui.events.is_empty());
}
#[test]
fn invalid_recorded_packet_does_not_resize_or_change_accepted_owner() {
    let (mut g, _) = game("");
    run(
        &mut g,
        &[Event::Focus {
            entity_id: label(3),
        }],
        None,
    )
    .unwrap();
    let before = Arc::clone(g.host().unwrap().ui_input().unwrap());
    let cases = vec![
        vec![Event::Commit {
            entity_id: label(2),
            text: "wrong".into(),
        }],
        vec![Event::Commit {
            entity_id: label(3),
            text: "x".repeat(16385),
        }],
        vec![Event::Preedit {
            entity_id: label(3),
            text: "é".into(),
            cursor: Some([1, 2]),
        }],
        vec![Event::Focus {
            entity_id: "invalid".into(),
        }],
        vec![Event::Traverse { reverse: false }; 257],
    ];
    for records in cases {
        assert!(run(&mut g, &records, Some([120., 90.])).is_err());
        assert_eq!(g.world().unwrap().tick(), 1);
        assert!(Arc::ptr_eq(&before, g.host().unwrap().ui_input().unwrap()));
    }
    assert!(run(&mut g, &[], Some([f32::NAN, 90.])).is_err());
    assert!(Arc::ptr_eq(&before, g.host().unwrap().ui_input().unwrap()));
}
#[test]
fn recorded_guest_failure_has_no_prefix_and_never_exposes_old_world_as_live() {
    let (mut g, _) = game("if(input.ui.events.length)throw Error('refuse');");
    run(
        &mut g,
        &[Event::Focus {
            entity_id: label(3),
        }],
        None,
    )
    .unwrap();
    let before = Arc::clone(g.host().unwrap().ui_input().unwrap());
    assert!(
        run(
            &mut g,
            &[Event::Commit {
                entity_id: label(3),
                text: "no".into()
            }],
            None
        )
        .is_err()
    );
    assert!(g.world().is_err());
    assert!(g.host().is_err());
    assert_eq!(
        before
            .widgets
            .iter()
            .find(|w| w.id == id(3))
            .unwrap()
            .layout
            .source(),
        "Name"
    );
}

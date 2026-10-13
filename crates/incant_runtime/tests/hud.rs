#[path = "support/hud.rs"]
mod fixture;
use fixture::*;
use incant_runtime::*;
use std::sync::Arc;
fn ui(n: u128, control: UiControl) -> StructuralCommand {
    StructuralCommand::ControlUi {
        entity: id(n),
        control,
    }
}
fn text(value: &str) -> StructuralCommand {
    StructuralCommand::ControlText {
        entity: id(4),
        control: TextControl::Content(value.into()),
    }
}
#[test]
fn ordered_controls_preserve_fonts_layout_and_old_immutable_publication() {
    let mut w = NativeWorld::new(scene());
    let before = declarations(&w);
    let old = w.ui_view().unwrap().nodes().nth(1).unwrap().node.clone();
    let mut q = FrameCommands::default();
    for c in [
        ui(2, UiControl::Text("Health 9".into())),
        ui(2, UiControl::Disabled(true)),
        ui(2, UiControl::Text("Health 8".into())),
        text("العالم"),
        text("世界"),
    ] {
        q.push(c).unwrap();
    }
    w.apply(&mut q).unwrap();
    let after = declarations(&w);
    assert_eq!(old.as_ref(), &before.0[1].1);
    assert_eq!(after.0[1].1.kind.text().unwrap().content, "Health 8");
    assert_eq!(
        after.0[1].1.kind.text().unwrap().fonts,
        before.0[1].1.kind.text().unwrap().fonts
    );
    assert_eq!(after.0[1].1.style, before.0[1].1.style);
    assert!(after.0[1].1.disabled);
    assert_eq!(after.1[0].1.content.as_ref(), "世界");
    assert!(Arc::ptr_eq(&after.1[0].1.fonts, &before.1[0].1.fonts));
    let generation = w.ui_view().unwrap().generation();
    let tg = w.text_view().unwrap().generation();
    for c in [
        ui(2, UiControl::Text("Health 8".into())),
        ui(2, UiControl::Disabled(true)),
        text("世界"),
    ] {
        q.push(c).unwrap();
    }
    w.apply(&mut q).unwrap();
    assert_eq!(w.ui_view().unwrap().generation(), generation);
    assert_eq!(w.text_view().unwrap().generation(), tg);
}
#[test]
fn invalid_roles_and_lifetimes_refuse_entire_native_batch() {
    for tail in [
        vec![ui(1, UiControl::Text("bad".into()))],
        vec![ui(5, UiControl::Disabled(true))],
        vec![
            StructuralCommand::Despawn(id(2)),
            ui(2, UiControl::Disabled(false)),
        ],
        vec![
            StructuralCommand::SetText {
                entity: id(4),
                text: None,
            },
            text("bad"),
        ],
    ] {
        let mut w = NativeWorld::new(scene());
        let before = declarations(&w);
        let snapshot = w.snapshot();
        let mut q = FrameCommands::default();
        q.push(ui(2, UiControl::Text("prefix".into()))).unwrap();
        q.push(StructuralCommand::SetVelocity {
            entity: id(5),
            velocity: Some(incant_types::Velocity {
                linear: [5., 0., 0.],
            }),
        })
        .unwrap();
        for c in tail {
            q.push(c).unwrap();
        }
        let count = q.len();
        assert!(w.apply(&mut q).is_err());
        assert_eq!(q.len(), count);
        assert_eq!(declarations(&w), before);
        assert_eq!(w.snapshot(), snapshot);
    }
}
#[test]
fn native_queue_enforces_aggregate_text_and_call_caps_before_retention() {
    let mut q = FrameCommands::default();
    let value: Arc<str> = "x".repeat(MAX_TEXT_BYTES).into();
    for _ in 0..MAX_HUD_COMMAND_BYTES / MAX_TEXT_BYTES {
        q.push(ui(2, UiControl::Text(value.clone()))).unwrap();
    }
    let n = q.len();
    assert!(q.push(ui(2, UiControl::Text("x".into()))).is_err());
    assert_eq!(q.len(), n);
    q.clear();
    for _ in 0..MAX_HUD_COMMANDS {
        q.push(ui(2, UiControl::Disabled(false))).unwrap();
    }
    assert!(q.push(ui(2, UiControl::Disabled(false))).is_err());
    q.clear();
    assert!(
        q.push(ui(
            2,
            UiControl::Text("x".repeat(MAX_TEXT_BYTES + 1).into())
        ))
        .is_err()
    );
    assert!(q.is_empty());
    q.push(text("valid")).unwrap();
}

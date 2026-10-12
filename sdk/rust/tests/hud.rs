use incant_module::{HudControl, HudEventKind, HudRole, HudSnapshot};
use incant_wasm_abi as abi;
fn id(n: u8) -> [u8; 16] {
    [n; 16]
}
fn frame() -> Vec<u8> {
    let mut b = abi::HudHeader {
        magic: *abi::HUD_MAGIC,
        scene: id(9),
        tick: u64::MAX,
        widgets: 3,
        texts: 1,
        events: 2,
        bytes: 12,
        focused: id(3),
        flags: 3,
        discarded: 256,
        widget_stride: abi::HudWidget::SIZE as u32,
        text_stride: abi::HudText::SIZE as u32,
        event_stride: abi::HudEvent::SIZE as u32,
        reserved: 0,
    }
    .encode()
    .to_vec();
    for (entity, role, offset, len) in [(id(1), 0, 0, 0), (id(2), 3, 0, 1), (id(3), 4, 1, 1)] {
        b.extend(
            abi::HudWidget {
                entity,
                role,
                disabled: 0,
                offset,
                len,
            }
            .encode(),
        );
    }
    b.extend(
        abi::HudText {
            entity: id(3),
            offset: 2,
            len: 4,
        }
        .encode(),
    );
    b.extend(
        abi::HudEvent {
            entity: id(2),
            kind: 0,
            offset: 6,
            len: 0,
            reserved: 0,
        }
        .encode(),
    );
    b.extend(
        abi::HudEvent {
            entity: id(3),
            kind: 1,
            offset: 6,
            len: 6,
            reserved: 0,
        }
        .encode(),
    );
    b.extend("ABwide中文".as_bytes());
    b
}
fn word(b: &mut [u8], at: usize, n: u32) {
    b[at..at + 4].copy_from_slice(&n.to_le_bytes());
}
#[test]
fn validated_unaligned_frame_preserves_declarations_outcomes_and_exact_clock() {
    let mut b = vec![0xff];
    b.extend(frame());
    let s = HudSnapshot::parse(&b[1..]).unwrap();
    assert_eq!(s.tick(), u64::MAX);
    assert_eq!(s.scene().0, id(9));
    assert_eq!(s.focused().unwrap().0, id(3));
    assert!(s.text_input());
    assert_eq!(s.discarded_events(), 256);
    let w: Vec<_> = s.widgets().collect();
    assert_eq!(w[0].role, HudRole::Root);
    assert_eq!(w[0].text, None);
    assert_eq!(w[2].text, Some("B"));
    assert_eq!(s.texts().next().unwrap().text, "wide");
    let e: Vec<_> = s.events().collect();
    assert_eq!(e[0].kind, HudEventKind::Activated);
    assert_eq!(e[0].value, None);
    assert_eq!(e[1].value, Some("中文"));
}
#[test]
fn hostile_counts_order_associations_ranges_utf8_and_reserved_bytes_refuse_before_views() {
    let good = frame();
    for end in 0..good.len() {
        assert!(HudSnapshot::parse(&good[..end]).is_err(), "truncate {end}");
    }
    let h = abi::HudHeader::SIZE;
    let t = h + 3 * abi::HudWidget::SIZE;
    let e = t + abi::HudText::SIZE;
    for (at, value) in [
        (abi::HudHeader::WIDGETS_OFFSET, 1025),
        (abi::HudHeader::TEXTS_OFFSET, 1025),
        (abi::HudHeader::EVENTS_OFFSET, 257),
        (abi::HudHeader::BYTES_OFFSET, u32::MAX),
        (abi::HudHeader::DISCARDED_OFFSET, 257),
        (abi::HudHeader::FLAGS_OFFSET, 2),
        (abi::HudHeader::WIDGET_STRIDE_OFFSET, 33),
        (abi::HudHeader::RESERVED_OFFSET, 1),
        (h + abi::HudWidget::ROLE_OFFSET, 5),
        (h + abi::HudWidget::DISABLED_OFFSET, 2),
        (h + abi::HudWidget::LEN_OFFSET, 1),
        (h + abi::HudWidget::SIZE + abi::HudWidget::OFFSET_OFFSET, 1),
        (t + abi::HudText::OFFSET_OFFSET, u32::MAX),
        (e + abi::HudEvent::KIND_OFFSET, 1),
        (e + abi::HudEvent::RESERVED_OFFSET, 1),
    ] {
        let mut b = good.clone();
        word(&mut b, at, value);
        assert!(HudSnapshot::parse(&b).is_err(), "at {at} value {value}");
    }
    for at in [
        h + abi::HudWidget::SIZE,
        h + 2 * abi::HudWidget::SIZE,
        abi::HudHeader::FOCUSED_OFFSET,
        e + abi::HudEvent::SIZE,
    ] {
        let mut b = good.clone();
        b[at..at + 16].copy_from_slice(&id(1));
        assert!(HudSnapshot::parse(&b).is_err(), "identity {at}");
    }
    let mut b = good.clone();
    *b.last_mut().unwrap() = 0xff;
    assert!(HudSnapshot::parse(&b).is_err());
    let mut b = good;
    b.push(0);
    assert!(HudSnapshot::parse(&b).is_err());
}
#[test]
fn commands_round_trip_without_allocation_and_invalid_input_never_writes_scratch() {
    let mut b = vec![0xa5; abi::HudCommand::SIZE + abi::HUD_MAX_TEXT_BYTES];
    for control in [
        HudControl::UiText("مرحبا\n中文"),
        HudControl::UiDisabled(true),
        HudControl::UiDisabled(false),
        HudControl::Text(""),
    ] {
        let n = abi::hud::encode_command(id(3), control, &mut b).unwrap();
        assert_eq!(abi::hud::command(&b[..n]).unwrap(), (id(3), control));
    }
    let before = b.clone();
    assert!(
        abi::hud::encode_command(
            id(3),
            HudControl::Text(&"a".repeat(abi::HUD_MAX_TEXT_BYTES + 1)),
            &mut b
        )
        .is_err()
    );
    assert_eq!(b, before);
    let n = abi::hud::encode_command(id(3), HudControl::UiDisabled(true), &mut b).unwrap();
    for (offset, v) in [
        (abi::HudCommand::KIND_OFFSET, 3),
        (abi::HudCommand::VALUE_OFFSET, 2),
        (abi::HudCommand::LEN_OFFSET, 1),
        (abi::HudCommand::RESERVED_OFFSET, 1),
    ] {
        let mut bad = b[..n].to_vec();
        word(&mut bad, offset, v);
        assert!(abi::hud::command(&bad).is_err());
    }
}

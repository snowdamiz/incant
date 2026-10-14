use super::*;

fn state(captured: bool) -> State {
    State::new(
        Some([true, false, false, false, true, false, true, false]),
        [captured, captured, false],
        [u64::from(captured), 0, 0],
    )
}
fn observation(mark: Option<Mark>, before: State) -> Observation {
    Observation {
        mark,
        before,
        microseconds: 0,
    }
}
fn report(trace: &Trace) -> serde_json::Value {
    let mut bytes = Vec::new();
    trace.write(&mut bytes).unwrap();
    let json = bytes.strip_prefix(b"INCANT_POINTER_TRACE ").unwrap();
    serde_json::from_slice(json).unwrap()
}

#[test]
fn fixed_capacity_stops_growth_and_declares_observation_overflow() {
    let mut trace = Trace::new();
    let allocation = trace.rows.as_ptr();
    for _ in 0..MAX_ROWS + 19 {
        trace.end(
            observation(Some(Mark::new(Kind::DeviceButton, 1)), state(false)),
            state(false),
        );
    }
    trace.cleanup(state(false), state(false), true, Some(true));
    assert_eq!(trace.rows.len(), MAX_ROWS);
    assert_eq!(trace.rows.capacity(), MAX_ROWS);
    assert_eq!(trace.rows.as_ptr(), allocation);
    let value = report(&trace);
    assert_eq!(value["omitted"], 20);
    assert_eq!(value["inconclusive"], true);
    assert_eq!(value["completion_ok"], true);
    assert_eq!(value["cursor_cleanup_ok"], true);
    assert_eq!(value["rows"].as_array().unwrap().len(), MAX_ROWS);
}

#[test]
fn stable_callbacks_are_not_logged_but_state_changes_and_relevant_events_are() {
    let mut trace = Trace::new();
    for _ in 0..20_000 {
        trace.end(observation(None, state(false)), state(false));
    }
    assert_eq!(trace.rows.len(), 1); // initial state, no per-frame row/log
    trace.end(observation(None, state(false)), state(true));
    trace.end(
        observation(Some(Mark::new(Kind::WindowButton, 1)), state(true)),
        state(true),
    );
    trace.end(
        observation(Some(Mark::new(Kind::DeviceButton, 1)), state(true)),
        state(true),
    );
    assert_eq!(trace.rows.len(), 4);
    assert!(matches!(trace.rows[2].event.kind, Kind::WindowButton));
    assert!(matches!(trace.rows[3].event.kind, Kind::DeviceButton));
    assert_eq!(trace.rows[3].sequence, 20_003);
    assert_eq!(trace.omitted, 0);
}

#[test]
fn raw_values_are_exact_only_in_owned_eligible_capture_and_cleanup_error_survives() {
    let positive = Mark::motion((-0., f64::MIN_POSITIVE), state(true));
    assert_eq!(
        positive.value_bits,
        [(-0_f64).to_bits(), f64::MIN_POSITIVE.to_bits()]
    );
    assert!(!positive.redacted);
    for bit in [1, 2, 3, 5] {
        let mut inactive = state(true);
        inactive.flags |= 1 << bit;
        let mark = Mark::motion((7., -9.), inactive);
        assert!(mark.redacted);
        assert_eq!(mark.value_bits, [0, 0]);
    }
    assert!(Mark::motion((7., -9.), state(false)).redacted);
    let mut trace = Trace::new();
    trace.cleanup(state(true), state(false), false, Some(true));
    let value = report(&trace);
    assert_eq!(value["completion_ok"], false); // original guest error preserved
    assert_eq!(value["cursor_cleanup_ok"], true); // independent cursor cleanup
    assert_eq!(value["inconclusive"], false); // complete observation, not behavior pass
    assert_eq!(value["rows"][0]["event"]["kind"], "cleanup");
}

#[test]
fn output_writer_is_bounded_and_cannot_write_past_its_reserve() {
    let mut target = Vec::new();
    let mut writer = Bounded {
        writer: &mut target,
        remaining: 4,
    };
    io::Write::write_all(&mut writer, b"abc").unwrap();
    assert!(io::Write::write_all(&mut writer, b"de").is_err());
    io::Write::write_all(&mut writer, b"d").unwrap();
    assert_eq!(target, b"abcd");
}

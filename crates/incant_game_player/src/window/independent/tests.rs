use super::*;
use crate::window::clock::Clock;
use std::time::Duration;

#[test]
fn callback_before_deadline_does_not_advance_and_timer_alone_admits_due_step() {
    let start = Instant::now();
    let mut clock = Clock::new(60, start);
    let early = start + Duration::from_nanos(16_666_666);
    assert!(ready(early, clock.deadline(), true));
    assert_eq!(clock.due(early), 0);
    let due = clock.deadline();
    assert_eq!(due, start + Duration::from_nanos(16_666_667));
    assert!(ready(due, due, false));
    assert_eq!(clock.due(due), 1);
    assert!(!ready(due, clock.deadline(), false));
}

#[test]
fn alternating_callback_phase_does_not_force_zero_two_simulation_batches() {
    let start = Instant::now();
    let mut clock = Clock::new(60, start);
    let mut legacy = Clock::new(60, start);
    let mut legacy_ticks = 0;
    let mut ticks = 0;
    let mut displays = 0;
    // Timer deadlines are independently visited even when display delivery
    // alternates just before and just after the rational 60Hz boundary.
    for frame in 1u64..=240 {
        let rational = (u128::from(frame) * 1_000_000_000).div_ceil(60) as u64;
        let callback = start
            + Duration::from_nanos(if frame % 2 == 1 {
                rational - 1
            } else {
                rational + 1
            });
        let legacy_due = legacy.due(callback);
        assert_eq!(legacy_due, if frame % 2 == 1 { 0 } else { 2 });
        legacy_ticks += legacy_due;
        let deadline = clock.deadline();
        if deadline <= callback {
            assert!(ready(deadline, deadline, false));
            assert_eq!(clock.due(deadline), 1);
            ticks += 1;
        }
        assert!(ready(callback, clock.deadline(), true));
        assert_eq!(clock.due(callback), 0);
        displays += 1;
        let boundary = start + Duration::from_nanos(rational);
        if callback < boundary {
            assert_eq!(clock.due(boundary), 1);
            ticks += 1;
        }
    }
    assert_eq!((ticks, displays, clock.dropped), (240, 240, 0));
    assert_eq!((legacy_ticks, legacy.dropped), (240, 0));
}

#[test]
fn busy_loop_retains_bounded_catchup_and_explicit_excess_debt() {
    let start = Instant::now();
    let mut clock = Clock::new(60, start);
    let busy_end = start + Duration::from_millis(150);
    assert_eq!(clock.due(busy_end), 4);
    assert_eq!(clock.dropped, 5);
    assert_eq!(clock.due(busy_end), 0);
    assert!(clock.deadline() > busy_end);
    // Focus/resume cannot bring suspended debt into the next active session.
    let resume = start + Duration::from_secs(40);
    clock.reset(resume);
    assert_eq!(clock.due(resume), 0);
    assert_eq!(clock.due(clock.deadline()), 1);
    assert_eq!(clock.dropped, 5);
}

#[test]
fn wait_uses_earliest_deadline_without_polls_or_postponing_display_health() {
    let start = Instant::now();
    let simulation = start + Duration::from_millis(17);
    let health = start + Duration::from_millis(500);
    assert_eq!(wait_until(simulation, health, start), simulation);
    assert_eq!(
        wait_until(simulation, health, start + Duration::from_millis(50)),
        start + Duration::from_millis(50)
    );
    assert_eq!(
        wait_until(simulation, health, start + Duration::from_secs(1)),
        health
    );
    assert_eq!(wait_until(health, simulation, start), simulation);
}

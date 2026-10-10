//! CPU-only input mapping cost. This is not a complete-game/device frame gate.
use incant_input::{InputAction, InputActions, InputBinding, InputEvent, InputRuntime, KeyCode};
use serde_json::json;
use std::{hint::black_box, time::Instant};

fn measure(actions: usize, bindings: usize, events: usize) -> serde_json::Value {
    let map: InputActions = (0..actions)
        .map(|n| {
            (
                format!("action{n}"),
                InputAction::Button {
                    threshold: 0.5,
                    bindings: vec![
                        InputBinding::Key {
                            code: KeyCode::Space,
                            scale: [1., 0.]
                        };
                        bindings
                    ],
                },
            )
        })
        .collect();
    let events: Vec<_> = (0..events)
        .map(|n| InputEvent::Key {
            code: KeyCode::Space,
            down: n % 2 == 0,
        })
        .collect();
    let mut runtime = InputRuntime::default();
    let mut times = Vec::new();
    for tick in 0..220 {
        let start = Instant::now();
        let frame = runtime
            .advance_mapped(black_box(&events), 1. / 60., black_box(&map))
            .unwrap();
        black_box(frame);
        let elapsed = start.elapsed().as_secs_f64() * 1000.;
        if tick >= 20 {
            times.push(elapsed);
        }
    }
    if actions > 0 && !events.is_empty() {
        assert!(
            runtime
                .frame()
                .actions
                .values()
                .all(|a| a.pressed && a.released && !a.active)
        );
    }
    times.sort_by(f64::total_cmp);
    json!({"actions":actions,"bindings_per_action":bindings,"events_per_tick":events.len(),
        "samples":times.len(),"median_ms":times[times.len()/2],"p95_ms":times[times.len()*95/100],"max_ms":times.last()})
}
fn main() {
    println!(
        "{}",
        json!({"os":std::env::consts::OS,"arch":std::env::consts::ARCH,
        "scope":"input CPU only; no renderer or device adapter", "runs":[
            measure(0,0,0),measure(12,3,4),measure(64,16,1024)]})
    );
}

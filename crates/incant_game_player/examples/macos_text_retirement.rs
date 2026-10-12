//! Actual AppKit lifetime regression, with an invisible window and no renderer.
//! Run on macOS with `--features native-window --example macos_text_retirement`.
#[cfg(all(target_os = "macos", feature = "native-window"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use tao::{
        event::Event,
        event_loop::{ControlFlow, EventLoop},
        platform::{
            macos::{ActivationPolicy, EventLoopExtMacOS},
            run_return::EventLoopExtRunReturn,
        },
        text_input::TextInputError,
        window::WindowBuilder,
    };

    let mut events = EventLoop::new();
    events.set_activation_policy(ActivationPolicy::Prohibited);
    events.set_activate_ignoring_other_apps(false);
    let mut window = Some(
        WindowBuilder::new()
            .with_title("Incant text context lifetime probe")
            .with_visible(false)
            .with_focused(false)
            .build(&events)?,
    );
    let mut cycles = 0_u64;
    let mut result: Result<(), TextInputError> = Ok(());
    events.run_return(|event, _, flow| {
        *flow = ControlFlow::Poll;
        if let Event::MainEventsCleared = event
            && retire_cycle(&mut window, &mut cycles, &mut result, |owner, cycles| {
                let serial = cycles * 4 + 1;
                owner.set_text_input_session(serial, true)?;
                owner.set_text_input_session(serial, true)?; // Same owner is idempotent.
                owner.set_text_input_session(serial + 1, true)?; // Replace an active context.
                owner.set_text_input_session(serial + 2, false)?; // Blur/retire it.
                assert_eq!(
                    owner.set_text_input_session(serial, true),
                    Err(TextInputError::Session)
                );
                owner.set_text_input_session(serial + 3, true)?;
                Ok(())
            })
        {
            *flow = ControlFlow::Exit;
        }
    });
    result?;
    assert_eq!(cycles, 256);
    println!(
        "{}",
        serde_json::json!({
            "passed": true,
            "native_contexts": cycles * 3,
            "cycles": cycles,
            "stale_sessions_refused": cycles,
            "active_owner_destroyed": true,
            "visible_window": false,
            "gpu_submitted": false,
            "input_injected": false,
        })
    );
    Ok(())
}

#[cfg(not(all(target_os = "macos", feature = "native-window")))]
fn main() {
    eprintln!("This probe requires macOS and the native-window feature.");
    std::process::exit(2);
}

// Returns true once the event loop should exit. Kept separate so late callbacks
// can be tested deterministically without creating an AppKit window.
#[cfg(any(all(target_os = "macos", feature = "native-window"), test))]
fn retire_cycle<W, E>(
    window: &mut Option<W>,
    cycles: &mut u64,
    result: &mut Result<(), E>,
    attempt: impl FnOnce(&W, u64) -> Result<(), E>,
) -> bool {
    // Tao may deliver another queued callback after ControlFlow::Exit.
    // Neither successful retirement nor an error may restart native work.
    if result.is_err() || window.is_none() {
        return true;
    }
    let owner = window.as_ref().expect("live hidden window");
    if let Err(error) = attempt(owner, *cycles) {
        *result = Err(error);
        return true;
    }
    *cycles += 1;
    if *cycles == 256 {
        // Destroy the final active owner before requesting event-loop exit.
        drop(window.take());
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::retire_cycle;
    use std::{cell::Cell, rc::Rc};

    struct Owner(Rc<Cell<usize>>);
    impl Drop for Owner {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }

    #[test]
    fn completion_destroys_active_owner_once_and_ignores_late_callbacks() {
        let drops = Rc::new(Cell::new(0));
        let mut window = Some(Owner(drops.clone()));
        let mut cycles = 0;
        let mut result: Result<(), &'static str> = Ok(());
        let mut observed = Vec::new();
        for expected in 0..256 {
            let exit = retire_cycle(&mut window, &mut cycles, &mut result, |_, cycle| {
                assert_eq!(drops.get(), 0);
                observed.push(cycle);
                Ok(())
            });
            assert_eq!(exit, expected == 255);
        }
        assert_eq!(observed, (0..256).collect::<Vec<_>>());
        assert_eq!(drops.get(), 1);
        assert!(window.is_none());
        for _ in 0..8 {
            assert!(retire_cycle(
                &mut window,
                &mut cycles,
                &mut result,
                |_, _| { panic!("late callback attempted another native session") }
            ));
        }
        assert_eq!(cycles, 256);
        assert_eq!(result, Ok(()));
        assert_eq!(drops.get(), 1);
    }

    #[test]
    fn first_failure_is_preserved_and_late_callbacks_do_not_retry() {
        let drops = Rc::new(Cell::new(0));
        let mut window = Some(Owner(drops.clone()));
        let mut cycles = 0;
        let mut result = Ok(());
        assert!(retire_cycle(
            &mut window,
            &mut cycles,
            &mut result,
            |_, _| { Err("first error") }
        ));
        for _ in 0..8 {
            assert!(retire_cycle(
                &mut window,
                &mut cycles,
                &mut result,
                |_, _| { panic!("failed probe attempted another native session") }
            ));
        }
        assert_eq!(cycles, 0);
        assert_eq!(result, Err("first error"));
        assert_eq!(drops.get(), 0);
        drop(window.take());
        assert_eq!(drops.get(), 1);
    }
}

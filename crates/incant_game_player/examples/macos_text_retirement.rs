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
    let mut result = Ok(());
    events.run_return(|event, _, flow| {
        *flow = ControlFlow::Poll;
        if let Event::MainEventsCleared = event {
            let attempt = (|| -> Result<(), TextInputError> {
                let owner = window.as_ref().expect("live hidden window");
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
            })();
            if let Err(error) = attempt {
                result = Err(error);
                *flow = ControlFlow::Exit;
            } else {
                cycles += 1;
                if cycles == 256 {
                    // Closing with the last context active exercises the other
                    // destructor path, without a preceding explicit retirement.
                    drop(window.take());
                    *flow = ControlFlow::Exit;
                }
            }
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

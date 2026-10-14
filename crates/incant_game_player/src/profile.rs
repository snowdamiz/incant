//! Host-only profiling options. Never interpreted from project/gameplay data.
use incant_profile::{Capture, Error};
pub(crate) fn split(
    mut args: Vec<String>,
    standalone: bool,
) -> Result<(Vec<String>, Option<Capture>), Error> {
    let mut selected = None;
    for arg in &args {
        if matches!(arg.as_str(), "--profile" | "--profile-tracy") {
            if selected.is_some() {
                return Err(Error::Options);
            }
            selected = Some(arg == "--profile-tracy");
        }
    }
    if selected.is_none() {
        return Ok((args, None));
    }
    if selected == Some(true) && !standalone {
        return Err(Error::TracyProcessOnly);
    }
    // Cargo can unify a dependency's features through another package. Only
    // this player's explicit features admit options and the corresponding report.
    if !cfg!(feature = "profile") {
        return Err(Error::Disabled);
    }
    if selected == Some(true) && !cfg!(feature = "profile-tracy") {
        return Err(Error::TracyUnavailable);
    }
    if args.iter().any(|a| a == "--build-info")
        || (args.iter().any(|a| a == "--window")
            && !cfg!(all(
                feature = "native-window",
                any(
                    target_os = "macos",
                    target_os = "windows",
                    target_os = "linux"
                )
            )))
    {
        return Err(Error::UnsupportedRoute);
    }
    args.retain(|arg| !matches!(arg.as_str(), "--profile" | "--profile-tracy"));
    let capture = match selected {
        None => None,
        Some(false) => Some(Capture::start()?),
        Some(true) => Some(Capture::start_tracy_process()?),
    };
    Ok((args, capture))
}
/// One actual simulation attempt. Event-loop waits and redraw-only work do not
/// pass through this boundary. Inclusive nested native/script zones remain separate.
pub(crate) fn frame<T, E>(run: impl FnOnce() -> Result<T, E>) -> Result<T, E> {
    incant_profile::frame(run)
}
#[cfg(all(
    feature = "native-window",
    any(target_os = "macos", target_os = "windows", target_os = "linux")
))]
pub(crate) fn presentation<T, E>(run: impl FnOnce() -> Result<T, E>) -> Result<T, E> {
    let _scope = incant_profile::scope(incant_profile::Phase::Presentation);
    run()
}
pub(crate) fn append(
    report: &mut serde_json::Value,
    capture: &Option<Capture>,
) -> Result<(), serde_json::Error> {
    #[cfg(feature = "profile")]
    if let Some(capture) = capture {
        report["profile"] = serde_json::to_value(capture.report())?;
    }
    #[cfg(not(feature = "profile"))]
    let _ = (report, capture);
    Ok(())
}
#[cfg(test)]
mod tests {
    #[test]
    fn embedded_transport_refuses_before_initialization() {
        assert!(matches!(
            super::split(vec!["--profile-tracy".into()], false),
            Err(incant_profile::Error::TracyProcessOnly)
        ));
    }
    #[test]
    fn duplicate_flags_refuse_without_starting_capture() {
        assert!(matches!(
            super::split(vec!["--profile".into(), "--profile".into()], false),
            Err(incant_profile::Error::Options)
        ));
    }
    #[cfg(not(feature = "profile"))]
    #[test]
    fn default_build_rejects_profile_explicitly() {
        assert!(matches!(
            super::split(vec!["--profile".into()], false),
            Err(incant_profile::Error::Disabled)
        ));
    }
    #[cfg(all(feature = "profile", not(feature = "profile-tracy")))]
    #[test]
    fn summary_build_rejects_transport_explicitly() {
        assert!(matches!(
            super::split(vec!["--profile-tracy".into()], true),
            Err(incant_profile::Error::TracyUnavailable)
        ));
    }
}

#[cfg(all(test, feature = "profile"))]
#[test]
fn completed_attempts_refreshes_and_capture_retirement_are_distinct() {
    let (args, capture) = split(vec!["--profile".into()], false).unwrap();
    assert!(args.is_empty());
    assert!(incant_profile::enabled());
    assert_eq!(frame(|| Ok::<_, &str>(17)), Ok(17));
    assert_eq!(frame(|| Err::<(), _>("failed step")), Err("failed step"));
    #[cfg(all(
        feature = "native-window",
        any(target_os = "macos", target_os = "windows", target_os = "linux")
    ))]
    {
        assert_eq!(presentation(|| Ok::<_, &str>(23)), Ok(23));
        assert_eq!(
            presentation(|| Err::<(), _>("failed refresh")),
            Err("failed refresh")
        );
    }
    let mut report = serde_json::json!({"tick":17});
    append(&mut report, &capture).unwrap();
    assert_eq!(report["tick"], 17);
    assert_eq!(report["profile"]["completed_frames"], 1);
    assert_eq!(report["profile"]["failed_frames"], 1);
    assert_eq!(report["profile"]["version"], 2);
    assert_eq!(report["profile"]["frame_wall"]["attempted_frames"], 2);
    assert_eq!(report["profile"]["frame_wall"]["failed_frames"], 1);
    let count = |name: &str| {
        report["profile"]["zones"]
            .as_array()
            .unwrap()
            .iter()
            .find(|zone| zone["name"] == name)
            .map_or(0, |zone| zone["totals"]["calls"].as_u64().unwrap())
    };
    assert_eq!(count("player.frame"), 2);
    #[cfg(all(
        feature = "native-window",
        any(target_os = "macos", target_os = "windows", target_os = "linux")
    ))]
    assert_eq!(count("player.presentation"), 2);
    drop(capture);
    assert!(!incant_profile::enabled());
    let mut without = serde_json::json!({"tick":17});
    append(&mut without, &None).unwrap();
    assert_eq!(without, serde_json::json!({"tick":17}));
    assert!(matches!(
        split(vec!["--build-info".into(), "--profile".into()], false),
        Err(Error::UnsupportedRoute)
    ));
    assert!(!incant_profile::enabled());

    #[cfg(all(
        feature = "native-window",
        any(target_os = "macos", target_os = "windows", target_os = "linux")
    ))]
    {
        // Real public native route reaches its checked argument parser, then
        // retires capture on error before creating a window, GPU or audio device.
        let mut output = Vec::new();
        let error = crate::run(vec!["--window".into(), "--profile".into()], |line| {
            output.push(line.to_owned())
        })
        .unwrap_err();
        assert!(error.to_string().contains("expected --window"));
        assert!(output.is_empty());
        assert!(!incant_profile::enabled());
    }
    #[cfg(not(all(
        feature = "native-window",
        any(target_os = "macos", target_os = "windows", target_os = "linux")
    )))]
    assert!(matches!(
        split(vec!["--window".into(), "--profile".into()], false),
        Err(Error::UnsupportedRoute)
    ));
}

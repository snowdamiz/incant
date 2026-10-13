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
    if args
        .iter()
        .any(|a| matches!(a.as_str(), "--window" | "--build-info"))
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

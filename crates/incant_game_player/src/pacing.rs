//! Trusted process selection only; never read from a package or guest program.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Mode {
    #[default]
    Ordinary,
    MinimumDuration60,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Error {
    Unsupported,
    Selection,
    WindowBound,
    Conflict,
    SceneRate,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Unsupported => "minimum-duration60 requires native-minimum-duration on macOS",
            Self::Selection => "expected one --metal-present-mode minimum-duration60",
            Self::WindowBound => {
                "minimum-duration60 requires one --window and --stop-after 1..36000"
            }
            Self::Conflict => {
                "minimum-duration60 conflicts with offline, capture, checkpoint or material modes"
            }
            Self::SceneRate => "minimum-duration60 requires a cooked 60Hz scene",
        })
    }
}
impl std::error::Error for Error {}

impl Mode {
    pub(crate) fn split(args: Vec<String>) -> Result<(Vec<String>, Self), Error> {
        Self::split_enabled(
            args,
            cfg!(all(
                feature = "native-minimum-duration",
                target_os = "macos"
            )),
        )
    }
    fn split_enabled(args: Vec<String>, enabled: bool) -> Result<(Vec<String>, Self), Error> {
        // Ordinary argv and all existing parser ordering stay unchanged.
        if !args.iter().any(|a| a == "--metal-present-mode") {
            return Ok((args, Self::Ordinary));
        }
        if !enabled {
            return Err(Error::Unsupported);
        }
        let mut result = Vec::with_capacity(args.len());
        let mut args = args.into_iter();
        let mut selected = false;
        while let Some(arg) = args.next() {
            if arg == "--metal-present-mode" {
                if selected || args.next().as_deref() != Some("minimum-duration60") {
                    return Err(Error::Selection);
                }
                selected = true;
            } else {
                result.push(arg);
            }
        }
        if result.iter().any(|a| {
            matches!(
                a.as_str(),
                "--gpu-offscreen"
                    | "--capture-frames"
                    | "--capture-at"
                    | "--capture-size"
                    | "--load-checkpoint"
                    | "--save-checkpoint"
                    | "--record-materials"
                    | "--prewarm-materials"
                    | "--material-program"
                    | "--material-samples"
                    | "--lifecycle-probe"
            )
        }) {
            return Err(Error::Conflict);
        }
        let windows = result.iter().filter(|a| a.as_str() == "--window").count();
        let bounds: Vec<_> = result
            .iter()
            .enumerate()
            .filter(|(_, a)| a.as_str() == "--stop-after")
            .collect();
        if windows != 1
            || bounds.len() != 1
            || result
                .get(bounds[0].0 + 1)
                .and_then(|s| s.parse::<u64>().ok())
                .is_none_or(|n| !(1..=36_000).contains(&n))
        {
            return Err(Error::WindowBound);
        }
        Ok((result, Self::MinimumDuration60))
    }
    pub(crate) fn admit_scene(self, rate: u32) -> Result<(), Error> {
        if self == Self::MinimumDuration60 && rate != 60 {
            return Err(Error::SceneRate);
        }
        Ok(())
    }
    #[cfg(feature = "native-window")]
    pub(crate) fn selected(self) -> bool {
        self == Self::MinimumDuration60
    }
    #[cfg(feature = "native-window")]
    pub(crate) fn admit_platform(self) -> crate::session::Result<()> {
        if !self.selected() {
            return Ok(());
        }
        #[cfg(all(feature = "native-minimum-duration", target_os = "macos"))]
        {
            wgpu_hal::metal::minimum_duration::check_supported()?;
            Ok(())
        }
        #[cfg(not(all(feature = "native-minimum-duration", target_os = "macos")))]
        Err(Error::Unsupported.into())
    }
}

#[cfg(all(test, feature = "native-minimum-duration", target_os = "macos"))]
#[path = "../../../third_party/wgpu-hal/src/metal/minimum_duration_policy.rs"]
mod hal_policy;

#[cfg(test)]
mod tests {
    use super::*;
    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|v| (*v).into()).collect()
    }
    #[test]
    fn ordinary_arguments_are_preserved_exactly_without_capability() {
        let input = args(&["--package", "missing", "--window", "--stop-after", "2156"]);
        assert_eq!(
            Mode::split_enabled(input.clone(), false),
            Ok((input, Mode::Ordinary))
        );
    }
    #[test]
    fn explicit_supported_selection_preserves_all_other_arguments() {
        let input = args(&["--package", "data", "--window", "--stop-after", "2156"]);
        let mut selected = input.clone();
        selected.extend(args(&["--metal-present-mode", "minimum-duration60"]));
        assert_eq!(
            Mode::split_enabled(selected.clone(), false),
            Err(Error::Unsupported)
        );
        assert_eq!(
            Mode::split_enabled(selected, true),
            Ok((input, Mode::MinimumDuration60))
        );
    }
    #[test]
    fn malformed_duplicate_unbounded_and_conflicting_selection_is_refused() {
        for values in [
            vec!["--metal-present-mode"],
            vec!["--metal-present-mode", "NaN"],
            vec![
                "--metal-present-mode",
                "minimum-duration60",
                "--metal-present-mode",
                "minimum-duration60",
            ],
            vec![
                "--metal-present-mode",
                "minimum-duration60",
                "--stop-after",
                "1",
            ],
            vec!["--metal-present-mode", "minimum-duration60", "--window"],
            vec![
                "--metal-present-mode",
                "minimum-duration60",
                "--window",
                "--stop-after",
                "0",
            ],
            vec![
                "--metal-present-mode",
                "minimum-duration60",
                "--window",
                "--stop-after",
                "36001",
            ],
            vec![
                "--metal-present-mode",
                "minimum-duration60",
                "--window",
                "--stop-after",
                "18446744073709551616",
            ],
            vec![
                "--metal-present-mode",
                "minimum-duration60",
                "--window",
                "--window",
                "--stop-after",
                "1",
            ],
            vec![
                "--metal-present-mode",
                "minimum-duration60",
                "--window",
                "--stop-after",
                "1",
                "--stop-after",
                "2",
            ],
        ] {
            assert!(
                Mode::split_enabled(args(&values), true).is_err(),
                "{values:?}"
            );
        }
        for flag in [
            "--capture-frames",
            "--capture-at",
            "--capture-size",
            "--load-checkpoint",
            "--save-checkpoint",
            "--record-materials",
            "--prewarm-materials",
            "--material-program",
            "--material-samples",
            "--gpu-offscreen",
            "--lifecycle-probe",
        ] {
            let mut input = args(&[
                "--window",
                "--stop-after",
                "1",
                "--metal-present-mode",
                "minimum-duration60",
            ]);
            input.push(flag.into());
            assert_eq!(Mode::split_enabled(input, true), Err(Error::Conflict));
        }
    }
    #[test]
    fn scene_rate_refusal_does_not_change_ordinary_rates() {
        for rate in [1, 30, 59, 61, 120] {
            assert_eq!(
                Mode::MinimumDuration60.admit_scene(rate),
                Err(Error::SceneRate)
            );
            assert_eq!(Mode::Ordinary.admit_scene(rate), Ok(()));
        }
        assert_eq!(Mode::MinimumDuration60.admit_scene(60), Ok(()));
    }
    #[test]
    fn real_player_parser_refuses_before_package_or_profile_setup() {
        let mut emitted = Vec::new();
        let error = crate::run(
            args(&[
                "--package",
                "does-not-exist",
                "--window",
                "--stop-after",
                "1",
                "--metal-present-mode",
                "unknown",
            ]),
            |s| emitted.push(s.to_owned()),
        )
        .unwrap_err();
        let expected = if cfg!(all(
            feature = "native-minimum-duration",
            target_os = "macos"
        )) {
            Error::Selection
        } else {
            Error::Unsupported
        };
        assert_eq!(error.downcast_ref::<Error>(), Some(&expected));
        assert!(emitted.is_empty());
    }
}

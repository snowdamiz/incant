//! Trusted process selection for the default-off latest-state presentation test.
//! Ordinary/latest selection is unchanged; independent additionally separates
//! rational simulation deadlines from display opportunities. No interpolation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Mode {
    #[default]
    Ordinary,
    Latest,
    Independent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Error {
    Unsupported,
    Selection,
    Bound,
    Conflict,
    IndependentCapability,
    IndependentWake,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::IndependentCapability => "independent presentation requires native-independent-presentation on macOS",
            Self::IndependentWake => "independent presentation requires explicit display-link60",
            Self::Unsupported => "latest presentation requires native-batched-presentation",
            Self::Selection => "expected one --present-batch latest or independent",
            Self::Bound => "latest presentation requires one --window and --stop-after 1..36000",
            Self::Conflict => {
                "latest presentation conflicts with offline, capture, checkpoint, material, lifecycle or minimum-duration modes"
            }
        })
    }
}
impl std::error::Error for Error {}

impl Mode {
    pub(crate) fn split(args: Vec<String>) -> Result<(Vec<String>, Self), Error> {
        Self::split_capabilities(
            args,
            cfg!(feature = "native-batched-presentation"),
            cfg!(all(
                feature = "native-independent-presentation",
                target_os = "macos"
            )),
        )
    }

    #[cfg(test)]
    fn split_enabled(args: Vec<String>, enabled: bool) -> Result<(Vec<String>, Self), Error> {
        Self::split_capabilities(args, enabled, false)
    }

    fn split_capabilities(
        args: Vec<String>,
        enabled: bool,
        independent: bool,
    ) -> Result<(Vec<String>, Self), Error> {
        // Keep the ordinary argument vector and downstream admission unchanged.
        if !args.iter().any(|arg| arg == "--present-batch") {
            return Ok((args, Self::Ordinary));
        }
        if !enabled {
            return Err(Error::Unsupported);
        }
        let mut remaining = Vec::with_capacity(args.len());
        let mut input = args.into_iter();
        let mut selected = None;
        while let Some(arg) = input.next() {
            if arg == "--present-batch" {
                if selected.is_some() {
                    return Err(Error::Selection);
                }
                selected = Some(match input.next().as_deref() {
                    Some("latest") => Self::Latest,
                    Some("independent") if independent => Self::Independent,
                    Some("independent") => return Err(Error::IndependentCapability),
                    _ => return Err(Error::Selection),
                });
            } else {
                remaining.push(arg);
            }
        }
        if remaining.iter().any(|arg| {
            matches!(
                arg.as_str(),
                "--cpu-render-input"
                    | "--gpu-offscreen"
                    | "--ticks"
                    | "--output"
                    | "--ui-viewport"
                    | "--static-module"
                    | "--build-info"
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
                    | "--metal-present-mode"
            )
        }) {
            return Err(Error::Conflict);
        }
        let mut bound = None;
        let mut windows = 0;
        for (index, arg) in remaining.iter().enumerate() {
            if arg == "--window" {
                windows += 1;
            } else if arg == "--stop-after" {
                if bound.is_some() {
                    return Err(Error::Bound);
                }
                let value = remaining.get(index + 1).ok_or(Error::Bound)?;
                if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
                    return Err(Error::Bound);
                }
                let value = value.parse::<u64>().map_err(|_| Error::Bound)?;
                if !(1..=36_000).contains(&value) {
                    return Err(Error::Bound);
                }
                bound = Some(value);
            }
        }
        if windows != 1 || bound.is_none() {
            return Err(Error::Bound);
        }
        let selected = selected.ok_or(Error::Selection)?;
        if selected == Self::Independent
            && !remaining
                .windows(2)
                .any(|pair| pair[0] == "--native-wake-source" && pair[1] == "display-link60")
        {
            return Err(Error::IndependentWake);
        }
        Ok((remaining, selected))
    }

    #[cfg(any(feature = "native-window", test))]
    pub(crate) fn selected(self) -> bool {
        self == Self::Latest
    }
    #[cfg(any(feature = "native-window", test))]
    pub(crate) fn independent(self) -> bool {
        self == Self::Independent
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).into()).collect()
    }
    fn selected(values: &[&str]) -> Vec<String> {
        let mut result = args(values);
        result.extend(args(&["--present-batch", "latest"]));
        result
    }

    #[test]
    fn ordinary_argv_is_preserved_without_enabling_a_policy() {
        for enabled in [false, true] {
            for input in [
                Vec::new(),
                args(&["--package", "missing", "--window"]),
                args(&["--capture-frames", "frames", "--ticks", "2"]),
            ] {
                assert_eq!(
                    Mode::split_enabled(input.clone(), enabled),
                    Ok((input, Mode::Ordinary))
                );
            }
        }
        assert!(!Mode::default().selected());
        assert!(Mode::Latest.selected());
    }

    #[test]
    fn explicit_selection_requires_the_actual_feature() {
        let ordinary = args(&["--package", "data", "--window", "--stop-after", "1"]);
        let selection = selected(&["--package", "data", "--window", "--stop-after", "1"]);
        assert_eq!(
            Mode::split_enabled(selection.clone(), false),
            Err(Error::Unsupported)
        );
        let expected = if cfg!(feature = "native-batched-presentation") {
            Ok((ordinary, Mode::Latest))
        } else {
            Err(Error::Unsupported)
        };
        assert_eq!(Mode::split(selection), expected);
    }

    #[test]
    fn selection_removes_only_its_pair_and_allows_display_link_and_observation() {
        let ordinary = args(&[
            "--package",
            "data",
            "--window",
            "--stop-after",
            "2156",
            "--native-wake-source",
            "display-link60",
            "--input-recording",
            "route.json",
            "--observe",
            "oracle.json",
            "--presentation-output",
            "presentations.json",
            "--presentation-capacity",
            "2156",
        ]);
        for index in [0, 2, ordinary.len()] {
            let mut input = ordinary.clone();
            input.splice(index..index, args(&["--present-batch", "latest"]));
            assert_eq!(
                Mode::split_enabled(input, true),
                Ok((ordinary.clone(), Mode::Latest))
            );
        }
    }

    #[test]
    fn missing_unknown_and_duplicate_selection_is_refused() {
        for value in ["", "ordinary", "Latest", "automatic", "--window"] {
            let mut input = args(&["--window", "--stop-after", "1", "--present-batch"]);
            input.push(value.into());
            assert_eq!(Mode::split_enabled(input, true), Err(Error::Selection));
        }
        assert_eq!(
            Mode::split_enabled(
                args(&["--window", "--stop-after", "1", "--present-batch"]),
                true
            ),
            Err(Error::Selection)
        );
        for tail in [
            args(&["--present-batch"]),
            args(&["--present-batch", "latest"]),
        ] {
            let mut input = selected(&["--window", "--stop-after", "1"]);
            input.extend(tail);
            assert_eq!(Mode::split_enabled(input, true), Err(Error::Selection));
        }
    }

    #[test]
    fn exactly_one_window_and_one_finite_step_bound_are_required() {
        for value in ["1", "36000"] {
            let ordinary = args(&["--window", "--stop-after", value]);
            assert_eq!(
                Mode::split_enabled(selected(&["--window", "--stop-after", value]), true),
                Ok((ordinary, Mode::Latest))
            );
        }
        for value in [
            "",
            "0",
            "36001",
            "-1",
            "+1",
            " 1",
            "1 ",
            "1.0",
            "NaN",
            "18446744073709551616",
        ] {
            assert_eq!(
                Mode::split_enabled(selected(&["--window", "--stop-after", value]), true),
                Err(Error::Bound),
                "{value:?}"
            );
        }
        for values in [
            vec![],
            vec!["--stop-after", "1"],
            vec!["--window"],
            vec!["--window", "--stop-after"],
            vec!["--window", "--window", "--stop-after", "1"],
            vec!["--window", "--stop-after", "1", "--stop-after", "1"],
        ] {
            assert_eq!(
                Mode::split_enabled(selected(&values), true),
                Err(Error::Bound)
            );
        }
    }

    #[test]
    fn incompatible_modes_refuse_before_other_parsers_can_remove_them() {
        for flag in [
            "--cpu-render-input",
            "--gpu-offscreen",
            "--ticks",
            "--output",
            "--ui-viewport",
            "--static-module",
            "--build-info",
            "--capture-frames",
            "--capture-at",
            "--capture-size",
            "--load-checkpoint",
            "--save-checkpoint",
            "--record-materials",
            "--prewarm-materials",
            "--material-program",
            "--material-samples",
            "--lifecycle-probe",
            "--metal-present-mode",
        ] {
            let mut input = selected(&["--window", "--stop-after", "1"]);
            input.push(flag.into());
            assert_eq!(
                Mode::split_enabled(input, true),
                Err(Error::Conflict),
                "{flag}"
            );
        }
    }
    #[test]
    fn independent_selection_is_explicit_and_requires_the_actual_display_route() {
        let input = args(&[
            "--window",
            "--stop-after",
            "2156",
            "--native-wake-source",
            "display-link60",
            "--present-batch",
            "independent",
        ]);
        assert_eq!(
            Mode::split_capabilities(input.clone(), true, false),
            Err(Error::IndependentCapability)
        );
        let (remaining, mode) = Mode::split_capabilities(input, true, true).unwrap();
        assert!(mode.independent());
        assert!(!mode.selected());
        assert_eq!(
            remaining,
            args(&[
                "--window",
                "--stop-after",
                "2156",
                "--native-wake-source",
                "display-link60"
            ])
        );
        assert_eq!(
            Mode::split_capabilities(
                args(&[
                    "--window",
                    "--stop-after",
                    "2156",
                    "--present-batch",
                    "independent"
                ]),
                true,
                true
            ),
            Err(Error::IndependentWake)
        );
        assert_eq!(
            Mode::split_capabilities(
                args(&[
                    "--window",
                    "--stop-after",
                    "0",
                    "--present-batch",
                    "independent"
                ]),
                true,
                true
            ),
            Err(Error::Bound)
        );
        assert_eq!(
            Mode::split_capabilities(
                args(&[
                    "--window",
                    "--stop-after",
                    "1",
                    "--present-batch",
                    "independent",
                    "--gpu-offscreen"
                ]),
                true,
                true
            ),
            Err(Error::Conflict)
        );
        let ordinary = args(&["--window", "--stop-after", "1"]);
        assert_eq!(
            Mode::split_capabilities(ordinary.clone(), true, true),
            Ok((ordinary, Mode::Ordinary))
        );
        let legacy = selected(&["--window", "--stop-after", "1"]);
        assert_eq!(
            Mode::split_capabilities(legacy.clone(), true, true),
            Mode::split_capabilities(legacy, true, false)
        );
    }
}

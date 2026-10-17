//! Default-off trusted wake-source experiment. Simulation time stays unchanged.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Mode {
    #[default]
    Timer,
    DisplayLink60,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Error {
    Unsupported,
    Selection,
    Bound,
    Conflict,
    #[cfg(feature = "native-window")]
    SceneRate,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Unsupported => "display-link60 requires native-display-link on a supported Mac",
            Self::Selection => "expected one --native-wake-source display-link60",
            Self::Bound => "display-link60 requires one --window and --stop-after 1..36000",
            Self::Conflict => {
                "display-link60 conflicts with other presentation interventions and offline modes"
            }
            #[cfg(feature = "native-window")]
            Self::SceneRate => "display-link60 requires a cooked 60Hz scene",
        })
    }
}
impl std::error::Error for Error {}

impl Mode {
    pub(crate) fn split(args: Vec<String>) -> Result<(Vec<String>, Self), Error> {
        Self::split_enabled(
            args,
            cfg!(all(feature = "native-display-link", target_os = "macos")),
        )
    }
    fn split_enabled(args: Vec<String>, enabled: bool) -> Result<(Vec<String>, Self), Error> {
        if !args.iter().any(|a| a == "--native-wake-source") {
            return Ok((args, Self::Timer));
        }
        if !enabled {
            return Err(Error::Unsupported);
        }
        let mut out = Vec::with_capacity(args.len());
        let mut input = args.into_iter();
        let mut selected = false;
        while let Some(arg) = input.next() {
            if arg == "--native-wake-source" {
                if selected || input.next().as_deref() != Some("display-link60") {
                    return Err(Error::Selection);
                }
                selected = true;
            } else {
                out.push(arg);
            }
        }
        if out.iter().any(|a| {
            matches!(
                a.as_str(),
                "--metal-present-mode"
                    | "--gpu-offscreen"
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
        let bounds: Vec<_> = out
            .iter()
            .enumerate()
            .filter(|(_, a)| *a == "--stop-after")
            .collect();
        if out.iter().filter(|a| *a == "--window").count() != 1
            || bounds.len() != 1
            || out
                .get(bounds[0].0 + 1)
                .and_then(|s| s.parse::<u64>().ok())
                .is_none_or(|n| !(1..=36000).contains(&n))
        {
            return Err(Error::Bound);
        }
        Ok((out, Self::DisplayLink60))
    }
    #[cfg(feature = "native-window")]
    pub(crate) fn selected(self) -> bool {
        self == Self::DisplayLink60
    }
    #[cfg(feature = "native-window")]
    pub(crate) fn admit_scene(self, rate: u32) -> Result<(), Error> {
        if self.selected() && rate != 60 {
            Err(Error::SceneRate)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn args(a: &[&str]) -> Vec<String> {
        a.iter().map(|v| (*v).into()).collect()
    }
    #[test]
    fn ordinary_argv_is_identical_without_capability() {
        let a = args(&["--package", "game", "--window"]);
        assert_eq!(Mode::split_enabled(a.clone(), false), Ok((a, Mode::Timer)));
    }
    #[test]
    fn selected_mode_requires_capability_and_retains_other_arguments() {
        let a = args(&["--package", "game", "--window", "--stop-after", "2156"]);
        let mut b = a.clone();
        b.extend(args(&["--native-wake-source", "display-link60"]));
        assert_eq!(
            Mode::split_enabled(b.clone(), false),
            Err(Error::Unsupported)
        );
        assert_eq!(Mode::split_enabled(b, true), Ok((a, Mode::DisplayLink60)));
    }
    #[test]
    fn refuses_duplicates_unbounded_and_mixed_experiments() {
        for a in [
            vec!["--native-wake-source"],
            vec!["--native-wake-source", "automatic"],
            vec!["--native-wake-source", "display-link60"],
            vec![
                "--window",
                "--stop-after",
                "0",
                "--native-wake-source",
                "display-link60",
            ],
            vec![
                "--window",
                "--stop-after",
                "36001",
                "--native-wake-source",
                "display-link60",
            ],
            vec![
                "--window",
                "--stop-after",
                "1",
                "--native-wake-source",
                "display-link60",
                "--native-wake-source",
                "display-link60",
            ],
            vec![
                "--window",
                "--stop-after",
                "1",
                "--native-wake-source",
                "display-link60",
                "--metal-present-mode",
                "minimum-duration60",
            ],
        ] {
            assert!(Mode::split_enabled(args(&a), true).is_err());
        }
    }
}

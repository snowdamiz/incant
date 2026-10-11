pub use incant_observe::{MAX_PLAN_BYTES, Plan};

/// Strip only the optional final pair; the existing mode parser validates the
/// remaining exact argument shape, including duplicate/unknown options.
pub fn split_args(args: &[String]) -> (&[String], Option<&str>) {
    if matches!(args.len(), 10 | 14) && args[args.len() - 2] == "--observe" {
        (&args[..args.len() - 2], Some(&args[args.len() - 1]))
    } else {
        (args, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn observation_options_preserve_strict_legacy_and_spatial_argument_shapes() {
        for count in [8, 12] {
            let plain = vec![String::from("arg"); count];
            assert_eq!(split_args(&plain).1, None);
            let mut with = plain.clone();
            with.extend(["--observe".into(), "plan.json".into()]);
            let (args, path) = split_args(&with);
            assert_eq!(args, plain);
            assert_eq!(path, Some("plan.json"));
            with.extend(["--observe".into(), "other.json".into()]);
            assert!(crate::session::settings(split_args(&with).0).is_err());
        }
    }
}

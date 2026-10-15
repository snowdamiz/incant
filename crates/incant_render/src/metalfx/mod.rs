//! Explicit, default-off Mac MetalFX temporal scaling. The ordinary renderer is
//! the spatial fallback; this feature does not implement the built-in upscaler.
#[cfg(any(feature = "native-metalfx", test))]
mod lifecycle;
#[cfg(all(feature = "native-metalfx", target_os = "macos"))]
mod native;
#[cfg(feature = "native-metalfx")]
mod owner;
mod plan;
#[cfg(feature = "native-metalfx")]
mod render;
#[cfg(feature = "native-metalfx")]
mod shader;
#[cfg(any(
    test,
    all(
        feature = "native-metalfx",
        not(all(feature = "cooked-metalfx-boundary", not(target_arch = "wasm32")))
    )
))]
mod source;
#[cfg(feature = "native-metalfx")]
mod system;
#[cfg(feature = "native-metalfx")]
mod targets;
#[cfg(feature = "native-metalfx")]
pub(crate) use owner::{Owner, Pending};
pub use plan::MAX_METALFX_TRANSITION_BYTES;
#[cfg(feature = "native-metalfx")]
pub(crate) use system::{Draw, System};

// A partial cooked renderer must not quietly parse either boundary program.
// The dedicated feature also requires the complete cooked motion/history route.
#[cfg(any(feature = "native-metalfx", test))]
fn shader_admission() -> Result<(), Fallback> {
    if cfg!(feature = "cooked-output") && !cfg!(feature = "cooked-metalfx-boundary") {
        Err(Fallback::CookedShaders)
    } else {
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Preference {
    #[default]
    Spatial,
    PreferMetalFx,
    RequireMetalFx,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fallback {
    FeatureDisabled,
    Backend,
    OperatingSystem,
    Device,
    Format,
    FractionalViewport,
    Aspect,
    Scale,
    PixelCamera,
    Overlays,
    Bloom,
    CookedShaders,
    UnpreparedExtent,
    Busy,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Selection {
    Spatial(Option<Fallback>),
    MetalFx,
}
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum MetalFxError {
    #[error("required MetalFX path is unavailable: {0:?}")]
    Unavailable(Fallback),
    #[error("MetalFX caller-owned allocation limit")]
    Capacity,
    #[error("invalid native depth support policy")]
    Config,
    #[error("MetalFX raw encoder/resource contract failed")]
    Interop,
    #[error("MetalFX pipelines were not enabled at startup")]
    Disabled,
}
impl Preference {
    #[cfg(any(feature = "native-metalfx", test))]
    fn select(self, reason: Option<Fallback>) -> Result<Selection, MetalFxError> {
        match (self, reason) {
            (Self::Spatial, _) => Ok(Selection::Spatial(None)),
            (Self::RequireMetalFx, Some(r)) => Err(MetalFxError::Unavailable(r)),
            (_, Some(r)) => Ok(Selection::Spatial(Some(r))),
            (_, None) => Ok(Selection::MetalFx),
        }
    }
}

/// Current-footprint depth compatibility only; these fields do not control
/// Apple's private history weights. No implicit visual-policy default.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Config {
    pub relative_depth: f32,
    pub absolute_depth: f32,
}
impl Config {
    pub fn validate(self) -> Result<(), MetalFxError> {
        if !self.relative_depth.is_finite()
            || !self.absolute_depth.is_finite()
            || !(0.0..=0.1).contains(&self.relative_depth)
            || !(0.0..=1.0).contains(&self.absolute_depth)
        {
            return Err(MetalFxError::Config);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cooked_boundary_admission_requires_the_complete_family_without_enabling_vendor_policy() {
        assert_eq!(Preference::default(), Preference::Spatial);
        if cfg!(feature = "cooked-output") && !cfg!(feature = "cooked-metalfx-boundary") {
            assert_eq!(shader_admission(), Err(Fallback::CookedShaders));
        } else {
            assert_eq!(shader_admission(), Ok(()));
        }
        if cfg!(feature = "cooked-metalfx-boundary") {
            assert!(cfg!(feature = "cooked-temporal"));
        }
    }
    #[test]
    fn policy_refusals_do_not_claim_vendor_success() {
        for reason in [
            Fallback::Backend,
            Fallback::OperatingSystem,
            Fallback::Device,
            Fallback::Format,
            Fallback::FractionalViewport,
            Fallback::Aspect,
            Fallback::Scale,
            Fallback::PixelCamera,
            Fallback::Overlays,
            Fallback::Bloom,
            Fallback::CookedShaders,
            Fallback::UnpreparedExtent,
            Fallback::Busy,
        ] {
            assert_eq!(
                Preference::PreferMetalFx.select(Some(reason)),
                Ok(Selection::Spatial(Some(reason)))
            );
            assert_eq!(
                Preference::RequireMetalFx.select(Some(reason)),
                Err(MetalFxError::Unavailable(reason))
            );
        }
    }
    #[test]
    fn depth_policy_rejects_nonfinite_and_excessive_tolerance() {
        for value in [f32::NAN, f32::INFINITY, -0.01, 0.10001] {
            assert!(
                Config {
                    relative_depth: value,
                    absolute_depth: 0.
                }
                .validate()
                .is_err()
            );
        }
        assert!(
            Config {
                relative_depth: 0.01,
                absolute_depth: 1.01
            }
            .validate()
            .is_err()
        );
        Config {
            relative_depth: 0.,
            absolute_depth: 0.,
        }
        .validate()
        .unwrap();
    }
}

#[cfg(test)]
#[test]
fn all_accepted_jitter_pairs_remove_only_raster_motion_in_internal_pixel_units() {
    for size in [[48., 32.], [72., 48.]] {
        for current in crate::temporal::jitter::SAMPLES {
            for previous in crate::temporal::jitter::SAMPLES {
                let current_ndc = [
                    0.3 + 2. * f64::from(current[0]) / size[0],
                    -0.2 - 2. * f64::from(current[1]) / size[1],
                ];
                let previous_ndc = [
                    0.25 + 2. * f64::from(previous[0]) / size[0],
                    -0.1 - 2. * f64::from(previous[1]) / size[1],
                ];
                let raw = [
                    0.5 * (previous_ndc[0] - current_ndc[0]),
                    -0.5 * (previous_ndc[1] - current_ndc[1]),
                ];
                let packed = std::array::from_fn::<_, 2, _>(|i| {
                    raw[i] * size[i] + f64::from(current[i] - previous[i])
                });
                let expected = [-0.025 * size[0], -0.05 * size[1]];
                for i in 0..2 {
                    assert!((packed[i] - expected[i]).abs() < 1e-12);
                }
            }
        }
    }
}

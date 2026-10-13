//! Explicit native temporal choice. The portable path remains the fallback;
//! native internal memory and image quality require separate device evidence.
#[cfg(any(feature = "native-metalfx", test))]
mod lifecycle;
#[cfg(feature = "native-metalfx")]
mod owner;
mod plan;
#[cfg(feature = "native-metalfx")]
mod system;
#[cfg(feature = "native-metalfx")]
mod targets;
#[cfg(feature = "native-metalfx")]
pub(crate) use owner::{Owner, Pending};
#[cfg(feature = "native-metalfx")]
pub(crate) use system::{Draw, System};
#[cfg(all(
    feature = "native-metalfx",
    any(target_os = "macos", target_os = "ios")
))]
mod native;
pub use plan::MAX_METALFX_TRANSITION_BYTES;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Preference {
    #[default]
    Portable,
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
    Busy,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Selection {
    Portable(Option<Fallback>),
    MetalFx,
}
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum MetalFxError {
    #[error("required MetalFX path is unavailable: {0:?}")]
    Unavailable(Fallback),
    #[error("MetalFX caller-owned allocation limit")]
    Capacity,
    #[error("MetalFX raw encoder/resource contract failed")]
    Interop,
    #[error("MetalFX pipelines were not enabled at startup")]
    Disabled,
}
impl Preference {
    #[cfg(any(feature = "native-metalfx", test))]
    pub(crate) fn select(self, reason: Option<Fallback>) -> Result<Selection, MetalFxError> {
        match (self, reason) {
            (Self::Portable, _) => Ok(Selection::Portable(None)),
            // Pixel cameras have no temporal pass in either backend. Requiring
            // native scaling does not override this intentional pixel bypass.
            (_, Some(Fallback::PixelCamera)) => {
                Ok(Selection::Portable(Some(Fallback::PixelCamera)))
            }
            (Self::RequireMetalFx, Some(reason)) => Err(MetalFxError::Unavailable(reason)),
            (_, Some(reason)) => Ok(Selection::Portable(Some(reason))),
            (_, None) => Ok(Selection::MetalFx),
        }
    }
}

#[cfg(test)]
mod numerics;

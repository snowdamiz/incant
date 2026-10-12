//! Checked full-model adapter to the lean shared root-motion primitive.
use super::{Channel, Interpolation, Property};
use crate::{AssetError, CookedModel};
pub use incant_anim::{
    MAX_ROOT_MOTION_CYCLES, MAX_ROOT_MOTION_DURATION, RootMotionDelta, RootMotionTime,
};

#[derive(Debug, thiserror::Error)]
pub enum RootMotionError {
    #[error("invalid root-motion model: {0}")]
    Model(#[from] AssetError),
    #[error("root motion requires animation data")]
    MissingAnimation,
    #[error("root-motion node index")]
    Node,
    #[error("root-motion clip index")]
    Clip,
    #[error("root motion requires a TRS node")]
    StaticMatrix,
    #[error("root motion requires unit rest scale and no selected-node scale track")]
    Scale,
    #[error("root-motion clip duration exceeds admission bound")]
    Duration,
    #[error("root-motion time must be finite")]
    Time,
    #[error("root-motion loop phase must be in [0, positive duration)")]
    Phase,
    #[error("root-motion cycle difference exceeds admission bound")]
    Cycles,
    #[error("invalid root-motion track: {0}")]
    Track(&'static str),
    #[error("root-motion capacity: {0}")]
    Capacity(&'static str),
    #[error("nonfinite root-motion transform")]
    NonFinite,
}
type Result<T> = std::result::Result<T, RootMotionError>;

impl From<incant_anim::RootMotionError> for RootMotionError {
    fn from(error: incant_anim::RootMotionError) -> Self {
        use incant_anim::RootMotionError as E;
        match error {
            E::Track(reason) => Self::Track(reason),
            E::Capacity(reason) => Self::Capacity(reason),
            E::Duration => Self::Duration,
            E::Time => Self::Time,
            E::Phase => Self::Phase,
            E::Cycles => Self::Cycles,
            E::NonFinite => Self::NonFinite,
        }
    }
}

/// Startup validates immutable cooked model data. Queries borrow checked root
/// tracks through incant_anim; no source IO, copied tracks or model dependency
/// enters the native simulation crate. This primitive does not strip poses or
/// apply entity/body motion, and still refuses selected-node scale tracks.
pub struct RootMotionSampler<'a> {
    inner: incant_anim::RootMotionSampler<'a>,
}
impl<'a> RootMotionSampler<'a> {
    pub fn new(model: &'a CookedModel, node: usize, clip: usize) -> Result<Self> {
        let data = model
            .animation
            .as_ref()
            .ok_or(RootMotionError::MissingAnimation)?;
        data.validate(&model.metadata.nodes, &model.meshes, &model.metadata.scenes)?;
        let rest = data.rest.get(node).ok_or(RootMotionError::Node)?;
        let rest = rest.as_ref().ok_or(RootMotionError::StaticMatrix)?;
        let clip = data.clips.get(clip).ok_or(RootMotionError::Clip)?;
        if rest.scale != [1.; 3]
            || clip
                .channels
                .iter()
                .any(|c| c.node == node && c.property == Property::Scale)
        {
            return Err(RootMotionError::Scale);
        }
        let duration = clip
            .channels
            .iter()
            .map(|c| f64::from(*c.times.last().expect("validated keys")))
            .fold(0., f64::max);
        let channel = |property| {
            clip.channels
                .iter()
                .find(|c| c.node == node && c.property == property)
                .map(track)
                .transpose()
        };
        Ok(Self {
            inner: incant_anim::RootMotionSampler::new(
                duration,
                rest.translation,
                rest.rotation,
                channel(Property::Translation)?,
                channel(Property::Rotation)?,
            )?,
        })
    }
    pub fn duration(&self) -> f64 {
        self.inner.duration()
    }
    pub fn clamped_delta(&self, from: f64, to: f64) -> Result<RootMotionDelta> {
        self.inner.clamped_delta(from, to).map_err(Into::into)
    }
    pub fn loop_delta(&self, from: RootMotionTime, to: RootMotionTime) -> Result<RootMotionDelta> {
        self.inner.loop_delta(from, to).map_err(Into::into)
    }
}
fn track(
    channel: &Channel,
) -> std::result::Result<incant_anim::track::Track<'_>, incant_anim::RootMotionError> {
    use incant_anim::track::{Interpolation as I, Kind, Track};
    Track::new(
        &channel.times,
        &channel.values,
        match channel.interpolation {
            Interpolation::Linear => I::Linear,
            Interpolation::Step => I::Step,
        },
        if channel.property == Property::Rotation {
            Kind::Rotation
        } else {
            Kind::Vector
        },
    )
}

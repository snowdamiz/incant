//! Trusted presentation requests, separated from accepted simulation ticks.
use crate::{
    GameError, Result, SceneHost,
    core::{Core, WithHost},
};
use incant_runtime::NativeWorld;
use kira::backend::Backend;

/// An optional compiled host capability. Requests cannot mutate the world or
/// apply input, run scripts, advance audio, drain haptics or change world time.
/// They may draw previously accepted state. Implementations must explicitly
/// refuse unsupported requests. Error/unwind terminalizes the live composition;
/// submitted device effects cannot be rolled back.
pub trait SceneHostPresentation: SceneHost {
    type Presentation;
    type Presented;
    fn present(
        &mut self,
        world: &NativeWorld,
        request: Self::Presentation,
    ) -> std::result::Result<Self::Presented, Self::Error>;
}

pub(crate) fn present<B: Backend, H: SceneHostPresentation>(
    core: &mut Core<B, WithHost<H>>,
    request: H::Presentation,
) -> Result<H::Presented, H::Error> {
    let mut live = core.live.take().ok_or(GameError::Failed)?;
    let result = live
        .host
        .0
        .present(live.native.world()?, request)
        .map_err(GameError::Host)?;
    core.live = Some(live);
    Ok(result)
}

//! Mandatory native spatial hosting, composed with input/VM/physics/audio ticks.
//! The default consumer exposes CPU input only; it does not submit a GPU frame.
use incant_game_runtime::SceneHost;
use incant_render_data::SceneInput;
pub use incant_render_native::TextSettings;
use incant_render_native::{
    NativeRenderError, NativeRenderInput, SyncError, UpdateKind, VisualCaches,
};
use incant_runtime::{NativeWorld, RuntimeError, StableId};
use std::{convert::Infallible, path::PathBuf};
use thiserror::Error;

/// Explicit startup cache locations, camera and consumer settings. No project or
/// source path is read. Settings/resources are transferred into the owned host.
pub struct NativeRenderSettings<S = ()> {
    pub model_cache: PathBuf,
    pub texture_cache: PathBuf,
    pub camera: StableId,
    /// Explicit checked font catalog and bounded CPU atlas settings.
    pub text: Option<TextSettings>,
    pub consumer: S,
}
#[derive(Debug, Error)]
pub enum RenderHostError<E = Infallible> {
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
    #[error(transparent)]
    Input(#[from] NativeRenderError),
    #[error("native render consumer failed: {0}")]
    Consumer(#[source] E),
}

/// A trusted native consumer, owned by the composite rather than invoked at the
/// caller's discretion. Preparation must actually validate/prepare its resources.
/// A GPU implementation owns its target provider and acquires the current target
/// inside `accept`; it must not return an unsubmitted mutable-resource snapshot.
///
/// `accept` runs inside NativeRenderInput's transaction. Preflight errors/unwind
/// restore that CPU input, but cannot undo arbitrary external GPU/device effects.
/// The game composition destroys all live hosts on post-acceptance failure/unwind.
/// A consumer may prepare and submit a replacement before swapping it into its
/// own state. Device loss is terminal; neither sound nor submitted frames rewind.
pub trait RenderInputConsumer: Sized {
    type Settings;
    type Error: std::error::Error + 'static;
    fn prepare(
        input: &SceneInput,
        camera: &str,
        settings: Self::Settings,
    ) -> Result<Self, Self::Error>;
    fn accept(
        &mut self,
        input: &SceneInput,
        camera: &str,
        kind: UpdateKind,
    ) -> Result<(), Self::Error>;
}

/// Explicit CPU observation mode. GPU validity, submission, display and appearance
/// are not implied by its success; those require another actual consumer.
pub struct CpuInputObserver;
impl RenderInputConsumer for CpuInputObserver {
    type Settings = ();
    type Error = Infallible;
    fn prepare(_: &SceneInput, _: &str, (): ()) -> Result<Self, Infallible> {
        Ok(Self)
    }
    fn accept(&mut self, _: &SceneInput, _: &str, _: UpdateKind) -> Result<(), Infallible> {
        Ok(())
    }
}

/// Owns exactly one retained input and one mandatory consumer. Composite startup
/// prepares them before guest/audio setup; tick sync follows simulation/script/
/// audio (including offline PCM rendering) and precedes successful publication.
/// Only read-only observations escape the hosted composition. Standalone adapter
/// retry does not make an already advanced composite tick rollback-capable.
pub struct NativeRenderHost<C: RenderInputConsumer = CpuInputObserver> {
    input: NativeRenderInput,
    consumer: C,
}
impl<C: RenderInputConsumer> NativeRenderHost<C> {
    /// The last fully accepted frame's continuous f64 CPU input.
    pub fn render_input(&self) -> &NativeRenderInput {
        &self.input
    }
    /// Read-only consumer observations; no mutable host/resource escape.
    pub fn consumer(&self) -> &C {
        &self.consumer
    }
}
impl<C: RenderInputConsumer> SceneHost for NativeRenderHost<C> {
    type Settings = NativeRenderSettings<C::Settings>;
    type Error = RenderHostError<C::Error>;
    fn prepare(world: &NativeWorld, settings: Self::Settings) -> Result<Self, Self::Error> {
        let spatial = world.spatial_view()?;
        let caches = VisualCaches {
            models: &settings.model_cache,
            textures: &settings.texture_cache,
        };
        let input = if let Some(text) = settings.text {
            NativeRenderInput::load_with_text(
                &spatial,
                &world.text_view()?,
                caches,
                settings.camera,
                text,
            )?
        } else {
            NativeRenderInput::load(&spatial, caches, settings.camera)?
        };
        let consumer = C::prepare(input.input(), input.camera_label(), settings.consumer)
            .map_err(RenderHostError::Consumer)?;
        Ok(Self { input, consumer })
    }
    fn sync(&mut self, world: &NativeWorld) -> Result<(), Self::Error> {
        let consumer = &mut self.consumer;
        let spatial = world.spatial_view()?;
        let accept = |input: &SceneInput, camera: &str, kind| consumer.accept(input, camera, kind);
        let result = if self.input.has_text_binding() {
            self.input
                .try_sync_with_text(&spatial, &world.text_view()?, accept)
        } else {
            self.input.try_sync(&spatial, accept)
        };
        match result {
            Ok(()) => Ok(()),
            Err(SyncError::Native(error)) => Err(error.into()),
            Err(SyncError::Consumer(error)) => Err(RenderHostError::Consumer(error)),
        }
    }
}

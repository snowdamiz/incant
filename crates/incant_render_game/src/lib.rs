//! Mandatory native spatial hosting, composed with input/VM/physics/audio ticks.
//! The default consumer exposes CPU input only; it does not submit a GPU frame.
mod ui;
use incant_game_runtime::SceneHost;
use incant_render_data::SceneInput;
pub use incant_render_data::UiInput;
pub use incant_render_native::TextSettings;
use incant_render_native::{
    NativeRenderError, NativeRenderInput, SyncError, UpdateKind, VisualCaches,
};
use incant_runtime::{NativeWorld, RuntimeError, StableId};
use std::{convert::Infallible, path::PathBuf, sync::Arc};
use thiserror::Error;
pub use ui::UiSettings;

/// Explicit startup cache locations, camera and consumer settings. No project or
/// source path is read. Settings/resources are transferred into the owned host.
pub struct NativeRenderSettings<S = ()> {
    pub model_cache: PathBuf,
    pub texture_cache: PathBuf,
    pub camera: StableId,
    /// Explicit checked font catalog and bounded CPU atlas settings.
    pub text: Option<TextSettings>,
    pub ui: Option<UiSettings>,
    pub consumer: S,
}
#[derive(Debug, Error)]
pub enum RenderHostError<E = Infallible> {
    #[error(transparent)]
    Ui(#[from] incant_ui::UiError),
    #[error("this consumer has no screen-space UI support")]
    UiUnsupported,
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
///
/// Screen-space UI arrives as the exact candidate `UiInput` of the same
/// transaction (None when the host has no UI owner). A UI-capable consumer must
/// draw it or refuse, and must refuse before consuming a target frame; the CPU
/// UI candidate is published only after `accept`/`admit_ui` succeeds.
pub trait RenderInputConsumer: Sized {
    /// Screen-space input is a separate capability from world-space Text.
    /// Defaults to refusal before resources or GPU submissions are prepared.
    const SUPPORTS_UI: bool = false;
    type Settings;
    type Error: std::error::Error + 'static;
    fn prepare(
        input: &SceneInput,
        camera: &str,
        ui: Option<&Arc<UiInput>>,
        settings: Self::Settings,
    ) -> Result<Self, Self::Error>;
    fn accept(
        &mut self,
        input: &SceneInput,
        camera: &str,
        kind: UpdateKind,
        ui: Option<&Arc<UiInput>>,
    ) -> Result<(), Self::Error>;
    /// Viewport-only relayout: admit the candidate's screen-space resources
    /// without presenting a frame. A refusal leaves the accepted UI drawable.
    fn admit_ui(&mut self, ui: &Arc<UiInput>) -> Result<(), Self::Error>;
}

/// Explicit CPU observation mode. GPU validity, submission, display and appearance
/// are not implied by its success; those require another actual consumer.
pub struct CpuInputObserver;
impl RenderInputConsumer for CpuInputObserver {
    const SUPPORTS_UI: bool = true;
    type Settings = ();
    type Error = Infallible;
    fn prepare(
        _: &SceneInput,
        _: &str,
        _: Option<&Arc<UiInput>>,
        (): (),
    ) -> Result<Self, Infallible> {
        Ok(Self)
    }
    fn accept(
        &mut self,
        _: &SceneInput,
        _: &str,
        _: UpdateKind,
        _: Option<&Arc<UiInput>>,
    ) -> Result<(), Infallible> {
        Ok(())
    }
    fn admit_ui(&mut self, _: &Arc<UiInput>) -> Result<(), Infallible> {
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
    ui: Option<ui::Binding>,
}
impl<C: RenderInputConsumer> NativeRenderHost<C> {
    pub fn ui(&self) -> Option<&incant_ui::UiRuntime> {
        self.ui.as_ref().map(|ui| &ui.runtime)
    }
    pub fn ui_paint(&self) -> Option<&Arc<incant_ui::UiPaint>> {
        self.ui.as_ref().map(|ui| &ui.paint)
    }
    /// The accepted screen-space draw input; its Arc is reused while unchanged.
    pub fn ui_input(&self) -> Option<&Arc<UiInput>> {
        self.ui.as_ref().map(|ui| &ui.input)
    }
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
    const SUPPORTS_UI: bool = C::SUPPORTS_UI;
    fn ui(&self) -> Option<&incant_ui::UiRuntime> {
        self.ui()
    }
    fn stage_ui(
        &mut self,
        world: &NativeWorld,
        events: &[incant_input::InputEvent],
        receipts: &[incant_ui::UiEvent],
    ) -> Result<Option<incant_game_runtime::RoutedUiInput>, Self::Error> {
        self.ui
            .as_mut()
            .map(|ui| ui.stage(world, events, receipts))
            .transpose()
            .map_err(Into::into)
    }
    fn cancel_ui(&mut self) {
        if let Some(ui) = &mut self.ui {
            ui.cancel();
        }
    }
    fn resize_ui(&mut self, world: &NativeWorld, viewport: [f32; 2]) -> Result<(), Self::Error> {
        let ui = self.ui.as_mut().ok_or(RenderHostError::UiUnsupported)?;
        let next = ui.prepare(world, viewport)?;
        // The consumer admits the same candidate before the CPU owner publishes it.
        self.consumer
            .admit_ui(next.input())
            .map_err(RenderHostError::Consumer)?;
        ui.commit(next)?;
        Ok(())
    }
    type Settings = NativeRenderSettings<C::Settings>;
    type Error = RenderHostError<C::Error>;
    fn prepare(world: &NativeWorld, settings: Self::Settings) -> Result<Self, Self::Error> {
        if (settings.ui.is_some() || world.ui_view()?.nodes().len() != 0) && !C::SUPPORTS_UI {
            return Err(RenderHostError::UiUnsupported);
        }
        if world.ui_view()?.nodes().len() != 0 && settings.ui.is_none() {
            return Err(RenderHostError::UiUnsupported);
        }
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
        let ui = settings
            .ui
            .map(|settings| ui::Binding::load(world, &input, settings))
            .transpose()?;
        let consumer = C::prepare(
            input.input(),
            input.camera_label(),
            ui.as_ref().map(|ui| &ui.input),
            settings.consumer,
        )
        .map_err(RenderHostError::Consumer)?;
        Ok(Self {
            input,
            consumer,
            ui,
        })
    }
    fn sync(&mut self, world: &NativeWorld) -> Result<(), Self::Error> {
        if world.ui_view()?.nodes().len() != 0 && self.ui.is_none() {
            return Err(RenderHostError::UiUnsupported);
        }
        let ui_next = self
            .ui
            .as_mut()
            .map(|ui| ui.prepare(world, ui.viewport()))
            .transpose()?;
        let consumer = &mut self.consumer;
        let spatial = world.spatial_view()?;
        let ui_input = ui_next.as_ref().map(|next| next.input());
        let accept =
            |input: &SceneInput, camera: &str, kind| consumer.accept(input, camera, kind, ui_input);
        let result = if self.input.has_text_binding() {
            self.input
                .try_sync_with_text(&spatial, &world.text_view()?, accept)
        } else {
            self.input.try_sync(&spatial, accept)
        };
        match result {
            Ok(()) => {
                if let Some(next) = ui_next {
                    self.ui.as_mut().expect("UI owner").commit(next)?;
                }
                Ok(())
            }
            Err(SyncError::Native(error)) => Err(error.into()),
            Err(SyncError::Consumer(error)) => Err(RenderHostError::Consumer(error)),
        }
    }
}

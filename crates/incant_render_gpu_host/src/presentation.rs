//! Admission of bounded catch-up states, separate from display submission.
use crate::{GpuHostError, GpuRenderConsumer, Live, TargetProvider};
use incant_render::UiStage;
use incant_render_data::SceneInput;
use incant_render_native::UpdateKind;

/// Selected by a trusted native target provider for this host sync only.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresentationMode {
    Present,
    AdmitOnly,
}

impl<P: TargetProvider> GpuRenderConsumer<P> {
    pub(crate) fn admit_without_presenting(
        &self,
        live: &mut Live<P>,
        input: &SceneInput,
        camera: &str,
        kind: UpdateKind,
        ui: Option<UiStage>,
    ) -> Result<(), GpuHostError<P::Error>> {
        // History-enabled policies need their own batch qualification. Refuse
        // rather than silently changing their presentation-based provenance.
        #[cfg(feature = "native-metalfx")]
        if live.metalfx.is_some() {
            return Err(GpuHostError::BatchPolicy);
        }
        if live.resolution_config.is_some()
            || live.motion_epoch.is_some()
            || live.targets.motion_epoch().is_some()
        {
            return Err(GpuHostError::BatchPolicy);
        }
        let replacement = if kind == UpdateKind::Rebuilt {
            Some(
                self.renderer
                    .prepare_retained(input, camera)
                    .map_err(GpuHostError::Render)?,
            )
        } else {
            None
        };
        // Retain rebuilt resources even if the next state is only Motion. An
        // intermediate topology change must survive until the final display.
        if let Some(scene) = replacement {
            live.scene = scene;
        }
        live.scene
            .validate_presentation(&self.renderer, input)
            .map_err(GpuHostError::Render)?;
        if let Some(stage) = ui {
            live.ui.accept(stage).map_err(GpuHostError::Render)?;
        }
        self.renderer.check_device().map_err(GpuHostError::Render)?;
        Ok(())
    }
}

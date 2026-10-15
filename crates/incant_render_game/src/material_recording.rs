use crate::{RenderInputConsumer, UiInput, UpdateKind};
use incant_render_data::{
    SceneInput,
    prewarm::{MaterialManifest, MaterialRecorder, PrewarmError, RecordingContext},
};
use std::sync::Arc;

/// CPU observation of actual committed material selections. It does not create
/// pipelines or claim GPU submission, appearance, cache reuse or timing success.
pub struct MaterialRecordingConsumer(MaterialRecorder);
impl MaterialRecordingConsumer {
    pub fn manifest(&self) -> Result<MaterialManifest, PrewarmError> {
        self.0.manifest()
    }
}
impl RenderInputConsumer for MaterialRecordingConsumer {
    const SUPPORTS_UI: bool = true;
    type Settings = RecordingContext;
    type Error = PrewarmError;
    fn prepare(
        _: &SceneInput,
        _: &str,
        _: Option<&Arc<UiInput>>,
        _: Self::Settings,
    ) -> Result<Self, Self::Error> {
        Err(PrewarmError::Contract(
            "material recording requires the native committed clock",
        ))
    }
    fn prepare_at_tick(
        input: &SceneInput,
        camera: &str,
        _: Option<&Arc<UiInput>>,
        tick: u64,
        settings: Self::Settings,
    ) -> Result<Self, Self::Error> {
        Ok(Self(MaterialRecorder::new(input, camera, tick, settings)?))
    }
    fn accept(
        &mut self,
        _: &SceneInput,
        _: &str,
        _: UpdateKind,
        _: Option<&Arc<UiInput>>,
    ) -> Result<(), Self::Error> {
        Err(PrewarmError::Contract(
            "material recording requires the native committed clock",
        ))
    }
    fn accept_at_tick(
        &mut self,
        input: &SceneInput,
        camera: &str,
        _: UpdateKind,
        _: Option<&Arc<UiInput>>,
        tick: u64,
    ) -> Result<(), Self::Error> {
        self.0.accept(input, camera, tick)
    }
    fn admit_ui(&mut self, _: &Arc<UiInput>) -> Result<(), Self::Error> {
        Ok(())
    }
}

use super::*;
use incant_game_runtime::SceneHostUpdate;
use incant_render_native::StagedTextureRefresh;

/// Separate trusted capability. Preparation has no published effects; validation
/// is read-only. Publication swaps through caller-owned prepared storage, which
/// retains retired handles on success and unwind until after unlocking.
pub trait TextureRefreshConsumer: RenderInputConsumer {
    type Prepared;
    fn prepare_textures(
        &self,
        input: &SceneInput,
        camera: &str,
    ) -> Result<Self::Prepared, Self::Error>;
    fn validate_textures(&self, prepared: &Self::Prepared) -> Result<(), Self::Error>;
    fn publish_textures(&mut self, prepared: &mut Self::Prepared);
}
impl TextureRefreshConsumer for CpuInputObserver {
    type Prepared = ();
    fn prepare_textures(&self, _: &SceneInput, _: &str) -> Result<(), Infallible> {
        Ok(())
    }
    fn validate_textures(&self, _: &()) -> Result<(), Infallible> {
        Ok(())
    }
    fn publish_textures(&mut self, _: &mut ()) {}
}
/// One cold allocation keeps typed requests/refusals small. Its owner remains
/// outside the publication lock, including during consumer unwind.
pub struct PreparedHostTextureRefresh<C: TextureRefreshConsumer>(Box<PreparedHostData<C>>);
struct PreparedHostData<C: TextureRefreshConsumer> {
    native: StagedTextureRefresh,
    consumer: Option<C::Prepared>,
}
pub enum TextureRefreshOperation<C: TextureRefreshConsumer> {
    Prepare(Option<PreparedTextureRefresh>),
    Publish(PreparedHostTextureRefresh<C>),
}
pub enum TextureRefreshReply<C: TextureRefreshConsumer> {
    Prepared(PreparedHostTextureRefresh<C>),
    Published(TextureRefreshReceipt),
    Refused(RenderHostError<C::Error>),
    PreparationRefused(RenderHostError<C::Error>),
}
impl<C: RenderInputConsumer> NativeRenderHost<C> {
    pub fn texture_refresh_ticket(&self) -> TextureRefreshTicket {
        self.input.texture_refresh_ticket()
    }
}
impl<C: TextureRefreshConsumer> SceneHostUpdate for NativeRenderHost<C> {
    type Request = TextureRefreshOperation<C>;
    type Reply = TextureRefreshReply<C>;
    fn update(&mut self, world: &NativeWorld, request: &mut Self::Request) -> Self::Reply {
        match request {
            TextureRefreshOperation::Prepare(prepared) => {
                let prepared = prepared.take().expect("single cold preparation");
                let result = (|| {
                    let native = self
                        .input
                        .stage_texture_refresh(&world.spatial_view()?, prepared)?;
                    let consumer = if native.changed() == 0 {
                        None
                    } else {
                        Some(
                            self.consumer
                                .prepare_textures(native.input(), native.camera_label())
                                .map_err(RenderHostError::Consumer)?,
                        )
                    };
                    Ok::<_, RenderHostError<C::Error>>(PreparedHostTextureRefresh(Box::new(
                        PreparedHostData { native, consumer },
                    )))
                })();
                match result {
                    Ok(prepared) => TextureRefreshReply::Prepared(prepared),
                    Err(error) => TextureRefreshReply::PreparationRefused(error),
                }
            }
            TextureRefreshOperation::Publish(prepared) => {
                let prepared = &mut *prepared.0;
                let view = match world.spatial_view() {
                    Ok(view) => view,
                    Err(error) => {
                        return TextureRefreshReply::Refused(error.into());
                    }
                };
                let validation = (|| {
                    if self.input.has_text_binding()
                        && self.input.text_generation() != world.text_view()?.generation()
                    {
                        return Err(NativeRenderError::TextureRefresh(
                            "text binding changed while textures were staged",
                        )
                        .into());
                    }
                    self.input
                        .validate_texture_publication(&view, &prepared.native)?;
                    if let Some(candidate) = &prepared.consumer {
                        self.consumer
                            .validate_textures(candidate)
                            .map_err(RenderHostError::Consumer)?;
                    }
                    Ok::<_, RenderHostError<C::Error>>(())
                })();
                if let Err(error) = validation {
                    // Keep rejected resources owned until the caller releases its
                    // final authority lock; no GPU/large CPU disposal happens here.
                    return TextureRefreshReply::Refused(error);
                }
                let consumer = &mut self.consumer;
                let result = self
                    .input
                    .try_publish_textures(&view, &mut prepared.native, || {
                        let candidate = prepared
                            .consumer
                            .as_mut()
                            .expect("changed texture candidate");
                        consumer.publish_textures(candidate);
                        Ok::<_, Infallible>(())
                    });
                match result {
                    Ok(receipt) => TextureRefreshReply::Published(receipt),
                    // Same exclusive host/world borrow, with no mutation between
                    // authority validation and the infallible publication move.
                    Err(SyncError::Native(_)) => unreachable!("validated texture publication"),
                    Err(SyncError::Consumer(error)) => match error {},
                }
            }
        }
    }
}

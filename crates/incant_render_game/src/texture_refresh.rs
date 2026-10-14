use super::*;
use incant_game_runtime::SceneHostUpdate;
use incant_render_native::{StagedTextureRefresh, TextureRefreshPublication};

/// Separate trusted capability. Preparation has no published effects; validation
/// is read-only. Publication only moves prepared handles and returns retired
/// handles for destruction after the caller releases its publication lock.
pub trait TextureRefreshConsumer: RenderInputConsumer {
    type Prepared;
    fn prepare_textures(
        &self,
        input: &SceneInput,
        camera: &str,
    ) -> Result<Self::Prepared, Self::Error>;
    fn validate_textures(&self, prepared: &Self::Prepared) -> Result<(), Self::Error>;
    fn publish_textures(&mut self, prepared: Self::Prepared) -> Self::Prepared;
}
impl TextureRefreshConsumer for CpuInputObserver {
    type Prepared = ();
    fn prepare_textures(&self, _: &SceneInput, _: &str) -> Result<(), Infallible> {
        Ok(())
    }
    fn validate_textures(&self, _: &()) -> Result<(), Infallible> {
        Ok(())
    }
    fn publish_textures(&mut self, (): ()) {}
}
pub struct PreparedHostTextureRefresh<C: TextureRefreshConsumer> {
    native: StagedTextureRefresh,
    consumer: Option<C::Prepared>,
}
pub struct TexturePublication<C: TextureRefreshConsumer> {
    native: TextureRefreshPublication,
    _retired_consumer: Option<C::Prepared>,
}
impl<C: TextureRefreshConsumer> TexturePublication<C> {
    pub fn receipt(&self) -> TextureRefreshReceipt {
        self.native.receipt
    }
}
pub enum TextureRefreshOperation<C: TextureRefreshConsumer> {
    Prepare(PreparedTextureRefresh),
    Publish(PreparedHostTextureRefresh<C>),
}
pub enum TextureRefreshReply<C: TextureRefreshConsumer> {
    Prepared(PreparedHostTextureRefresh<C>),
    Published(TexturePublication<C>),
    Refused {
        error: RenderHostError<C::Error>,
        candidate: PreparedHostTextureRefresh<C>,
    },
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
    fn update(&mut self, world: &NativeWorld, request: Self::Request) -> Self::Reply {
        match request {
            TextureRefreshOperation::Prepare(prepared) => {
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
                    Ok::<_, RenderHostError<C::Error>>(PreparedHostTextureRefresh {
                        native,
                        consumer,
                    })
                })();
                match result {
                    Ok(prepared) => TextureRefreshReply::Prepared(prepared),
                    Err(error) => TextureRefreshReply::PreparationRefused(error),
                }
            }
            TextureRefreshOperation::Publish(prepared) => {
                let view = match world.spatial_view() {
                    Ok(view) => view,
                    Err(error) => {
                        return TextureRefreshReply::Refused {
                            error: error.into(),
                            candidate: prepared,
                        };
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
                    return TextureRefreshReply::Refused {
                        error,
                        candidate: prepared,
                    };
                }
                let consumer = &mut self.consumer;
                let result = self.input.try_publish_textures(&view, prepared.native, || {
                    let candidate = prepared.consumer.expect("changed texture candidate");
                    Ok::<_, Infallible>(consumer.publish_textures(candidate))
                });
                match result {
                    Ok((native, retired)) => TextureRefreshReply::Published(TexturePublication {
                        native,
                        _retired_consumer: retired,
                    }),
                    // Same exclusive host/world borrow, with no mutation between
                    // authority validation and the infallible publication move.
                    Err(SyncError::Native(_)) => unreachable!("validated texture publication"),
                    Err(SyncError::Consumer(error)) => match error {},
                }
            }
        }
    }
}

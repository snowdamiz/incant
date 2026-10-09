//! Shadow caster draw packets share the exact immutable imported model versions.
use super::{Batch, RenderScene};
use crate::{
    Renderer, ResourceError, Result,
    shadows::{PipelineKey, PreparedShadows},
};
use std::sync::Arc;
struct Caster {
    batch: Arc<Batch>,
    pipeline: wgpu::RenderPipeline,
}
pub(crate) struct ShadowDraw {
    pub maps: PreparedShadows,
    casters: Vec<Caster>,
}
impl Renderer {
    pub(crate) fn prepare_shadow_draw(
        &self,
        scene: &RenderScene,
        aspect: f32,
    ) -> Result<ShadowDraw> {
        let mut casters = Vec::new();
        let mut bounds = Vec::new();
        if !scene.lights.shadows.is_empty() {
            for batch in scene.batches.iter().filter(|b| b.casts_shadows) {
                let material = &batch.model.materials.materials
                    [batch.model.primitives[batch.primitive].material];
                if material.alpha == incant_assets::AlphaMode::Blend {
                    return Err(ResourceError::TransparentShadowCaster.into());
                }
                bounds.push(batch.bounds);
                casters.push(Caster {
                    batch: Arc::clone(batch),
                    pipeline: self.shadows.pipeline(
                        &self.device,
                        &self.materials.layout,
                        PipelineKey {
                            alpha: material.alpha,
                            double_sided: material.double_sided,
                            mirrored: batch.mirrored,
                        },
                    )?,
                });
            }
        }
        Ok(ShadowDraw {
            maps: self.shadows.prepare(
                &self.device,
                scene.camera,
                aspect,
                &scene.lights.shadows,
                &bounds,
            )?,
            casters,
        })
    }
}
impl ShadowDraw {
    pub fn encode(&self, encoder: &mut wgpu::CommandEncoder) {
        for layer in &self.maps.layers {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Directional shadow cascade"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &layer.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            pass.set_bind_group(0, &layer.camera, &[]);
            for caster in &self.casters {
                let batch = &caster.batch;
                let primitive = &batch.model.primitives[batch.primitive];
                let material = &batch.model.materials.materials[primitive.material];
                pass.set_pipeline(&caster.pipeline);
                pass.set_bind_group(1, &material.bind_group, &[]);
                pass.set_vertex_buffer(0, primitive.vertices.slice(..));
                pass.set_vertex_buffer(1, batch.instances.slice(..));
                pass.set_index_buffer(primitive.indices.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..primitive.index_count, 0, 0..batch.count);
            }
        }
    }
}

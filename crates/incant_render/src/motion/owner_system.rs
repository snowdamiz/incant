//! Additive startup pipelines; the eight existing motion/depth variants stay intact.
use super::{MotionError, MotionSystem, system};
use crate::{Renderer, Result};
pub(crate) const TARGETS: [Option<wgpu::ColorTargetState>; 2] = [
    Some(wgpu::ColorTargetState {
        format: wgpu::TextureFormat::Rgba32Float,
        blend: None,
        write_mask: wgpu::ColorWrites::ALL,
    }),
    Some(wgpu::ColorTargetState {
        format: wgpu::TextureFormat::R32Uint,
        blend: None,
        write_mask: wgpu::ColorWrites::ALL,
    }),
];
impl MotionSystem {
    pub(crate) fn owner_enabled(&self) -> bool {
        self.owner_pipelines.is_some()
    }
}
impl Renderer {
    /// Startup-only opt-in for the integer owner MRT. It attaches no retained
    /// targets until `RetainedScene::set_motion_owner_attachment` is called.
    pub async fn enable_motion_owner(&mut self) -> Result<()> {
        super::admission()?;
        let system = self.motion.as_ref().ok_or(MotionError::Disabled)?;
        if system.owner_enabled() {
            return Ok(());
        }
        self.check_device()?;
        let needed = wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC;
        if !self
            .adapter
            .get_texture_format_features(wgpu::TextureFormat::R32Uint)
            .allowed_usages
            .contains(needed)
        {
            return Err(MotionError::Format.into());
        }
        let validation = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let candidate = (|| -> Result<_> {
            let shader = crate::temporal_shader::stages(
                &self.device,
                self.adapter.get_info().backend,
                crate::temporal_shader::Kind::MotionOwner,
            )?;
            let layout = self
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Motion owner"),
                    bind_group_layouts: &[Some(&system.globals), Some(&self.materials.layout)],
                    immediate_size: 0,
                });
            let buffers = system::owner_buffers();
            Ok(std::array::from_fn(|i| {
                self.device
                    .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                        label: Some("Motion owner"),
                        layout: Some(&layout),
                        vertex: wgpu::VertexState {
                            module: &shader.vertex,
                            entry_point: Some(shader.vertex_entry),
                            compilation_options: Default::default(),
                            buffers: &buffers,
                        },
                        primitive: wgpu::PrimitiveState {
                            front_face: if i & 1 != 0 {
                                wgpu::FrontFace::Cw
                            } else {
                                wgpu::FrontFace::Ccw
                            },
                            cull_mode: if i & 2 != 0 {
                                None
                            } else {
                                Some(wgpu::Face::Back)
                            },
                            ..Default::default()
                        },
                        depth_stencil: Some(wgpu::DepthStencilState {
                            format: wgpu::TextureFormat::Depth32Float,
                            depth_write_enabled: Some(true),
                            depth_compare: Some(wgpu::CompareFunction::Less),
                            stencil: Default::default(),
                            bias: Default::default(),
                        }),
                        multisample: Default::default(),
                        fragment: Some(wgpu::FragmentState {
                            module: &shader.fragment,
                            entry_point: Some(shader.fragment_entry),
                            compilation_options: Default::default(),
                            targets: &TARGETS,
                        }),
                        multiview_mask: None,
                        cache: None,
                    })
            }))
        })();
        if let Some(error) = validation.pop().await {
            #[cfg(target_arch = "wasm32")]
            let error = error.into();
            return Err(crate::ResourceError::Preparation(error).into());
        }
        self.motion
            .as_mut()
            .ok_or(MotionError::Disabled)?
            .owner_pipelines = Some(candidate?);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn motion_owner_layout_is_additive_and_integer_not_float_bits() {
        let old = system::buffers();
        let new = system::owner_buffers();
        assert_eq!(old[..3], new[..3]);
        assert_eq!(old[3].array_stride, 80);
        assert_eq!(new[3].array_stride, 80);
        assert_eq!(old[3].attributes.len(), 5);
        assert_eq!(new[3].attributes.len(), 6);
        assert_eq!(old[3].attributes[4].format, wgpu::VertexFormat::Float32x4);
        let valid = new[3].attributes[4];
        let owner = new[3].attributes[5];
        assert_eq!(
            (valid.shader_location, valid.offset, valid.format),
            (11, 64, wgpu::VertexFormat::Float32)
        );
        assert_eq!(
            (owner.shader_location, owner.offset, owner.format),
            (12, 68, wgpu::VertexFormat::Uint32)
        );
        assert_eq!(
            TARGETS.map(|t| t.unwrap().format),
            [
                wgpu::TextureFormat::Rgba32Float,
                wgpu::TextureFormat::R32Uint
            ]
        );
        assert!(TARGETS.iter().flatten().all(|t| t.blend.is_none()));
    }
}

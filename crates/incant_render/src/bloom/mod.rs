//! Opt-in scene-linear bloom. Screen-space UI is composed after this graph.
mod config;
mod targets;
use crate::{ResourceError, Result, output::OutputPass};
use bytemuck::{Pod, Zeroable};
use config::Plan;
pub use config::{Bloom, BloomError};
use std::sync::{Arc, Mutex};
use targets::Targets;
use wgpu::util::DeviceExt;

pub(super) fn output_source() -> String {
    const ORIGINAL: &str = "var color = mix(display.backdrop.rgb,tone_map(radiance),coverage);";
    let original = include_str!("../output.wgsl");
    assert_eq!(
        original.matches(ORIGINAL).count(),
        1,
        "fixed output seam changed"
    );
    format!(
        "{}\n{}\n{}",
        original.replace(
            ORIGINAL,
            "var color = display.backdrop.rgb; if inside { color = bloom_color(radiance,coverage,position.xy); }"
        ),
        include_str!("../tone_map.wgsl"),
        include_str!("output.wgsl")
    )
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Settings {
    rect: [f32; 4],
    sizes: [u32; 4],
    params: [f32; 4],
}

pub(super) struct BloomSystem {
    pub config: Bloom,
    pub output: OutputPass,
    pipelines: [wgpu::RenderPipeline; 3],
    layout: wgpu::BindGroupLayout,
    cache: Mutex<Option<Arc<Targets>>>,
}
impl BloomSystem {
    pub fn new(device: &wgpu::Device, config: Bloom, cache: Option<&wgpu::PipelineCache>) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Bloom immutable inputs"),
            entries: &[
                texture_entry(0),
                texture_entry(1),
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(48),
                    },
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Bloom filters"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Bloom area filter and reconstruction"),
            source: wgpu::ShaderSource::Wgsl(include_str!("filter.wgsl").into()),
        });
        let pipelines = ["first", "down", "up"].map(|entry| {
            crate::output::build(
                device,
                &crate::shader::Stages {
                    vertex: module.clone(),
                    fragment: module.clone(),
                    vertex_entry: "vertex",
                    fragment_entry: entry,
                },
                Some(&pipeline_layout),
                crate::frame::HDR_FORMAT,
                cache,
            )
        });
        Self {
            config,
            output: OutputPass::bloom(device, cache),
            pipelines,
            layout,
            cache: Mutex::new(None),
        }
    }

    pub fn prepare(
        &self,
        device: &wgpu::Device,
        source: &wgpu::TextureView,
        output: [u32; 2],
        frame: &crate::resolution::FramePlan,
        scale: crate::resolution::ResolutionScale,
    ) -> Result<BloomDraw> {
        let size = frame.size;
        let rect = frame.rect;
        let plan = Plan::new(
            output,
            self.config.levels,
            device.limits().max_texture_dimension_2d,
        )?;
        let source_span = plan.source_span(size, rect, scale)?;
        let targets = {
            let mut cache = self.cache.lock().map_err(|_| BloomError::CacheLock)?;
            if cache.as_ref().is_none_or(|old| old.plan != plan) {
                *cache = Some(Arc::new(Targets::new(device, plan)));
            }
            Arc::clone(cache.as_ref().expect("initialized bloom cache"))
        };
        let count = targets.levels.len();
        let mut stages = Vec::with_capacity(count * 2 - 1);
        let mut previous = source;
        let mut previous_size = size;
        for (i, level) in targets.levels.iter().enumerate() {
            let size = targets.plan.sizes[i];
            let settings = Settings {
                rect,
                sizes: [previous_size[0], previous_size[1], size[0], size[1]],
                params: [
                    self.config.threshold,
                    0.,
                    if i == 0 {
                        source_span[0]
                    } else {
                        previous_size[0] as f32
                    },
                    if i == 0 {
                        source_span[1]
                    } else {
                        previous_size[1] as f32
                    },
                ],
            };
            stages.push(self.stage(
                device,
                usize::from(i != 0),
                previous,
                previous,
                &level.down,
                settings,
            ));
            previous = &level.down;
            previous_size = size;
        }
        for i in (0..count - 1).rev() {
            let level = &targets.levels[i];
            let target = level.up.as_ref().expect("nonterminal reconstruction");
            let size = targets.plan.sizes[i];
            let remaining = (count - i) as f32;
            stages.push(self.stage(
                device,
                2,
                &level.down,
                previous,
                target,
                Settings {
                    rect,
                    sizes: [size[0], size[1], size[0], size[1]],
                    params: [0., (remaining - 1.) / remaining, 0., 0.],
                },
            ));
            previous = target;
        }
        Ok(BloomDraw {
            result: previous.clone(),
            stages,
        })
    }

    fn stage(
        &self,
        device: &wgpu::Device,
        pipeline: usize,
        source: &wgpu::TextureView,
        coarse: &wgpu::TextureView,
        target: &wgpu::TextureView,
        settings: Settings,
    ) -> Stage {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Bloom immutable frame settings"),
            contents: bytemuck::bytes_of(&settings),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Bloom filter inputs"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(source),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(coarse),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: buffer.as_entire_binding(),
                },
            ],
        });
        Stage {
            pipeline: self.pipelines[pipeline].clone(),
            bindings,
            target: target.clone(),
        }
    }
}

fn texture_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: false },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}
struct Stage {
    pipeline: wgpu::RenderPipeline,
    bindings: wgpu::BindGroup,
    target: wgpu::TextureView,
}
pub(super) struct BloomDraw {
    pub result: wgpu::TextureView,
    stages: Vec<Stage>,
}
impl BloomDraw {
    pub fn pass_count(&self) -> usize {
        self.stages.len()
    }
    // The final output pass closes the same Display interval. Interior filter
    // passes have no timestamp descriptor, so each query is written once.
    pub fn encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        queries: Option<&crate::timing::PassQueries>,
    ) {
        for (index, stage) in self.stages.iter().enumerate() {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Bloom normalized pyramid"),
                timestamp_writes: queries
                    .and_then(|q| q.render_range(index, self.pass_count() + 1)),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &stage.target,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_pipeline(&stage.pipeline);
            pass.set_bind_group(0, &stage.bindings, &[]);
            pass.draw(0..3, 0..1);
        }
    }
}

impl crate::Renderer {
    /// Prepare the finite bloom pipeline catalog before drawing. Default is off.
    /// Pixel cameras bypass bloom; footprint is in physical output pixels. This
    /// startup configuration does not enable a temporal upscaler or GPU timer.
    pub async fn enable_bloom(&mut self, config: Bloom) -> Result<()> {
        config::admission(
            config,
            cfg!(all(feature = "cooked-output", not(target_arch = "wasm32"))),
        )?;
        self.check_device()?;
        let validation = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let candidate = BloomSystem::new(&self.device, config, self.pipeline_cache.cache.as_ref());
        if let Some(error) = validation.pop().await {
            #[cfg(target_arch = "wasm32")]
            let error = error.into();
            return Err(ResourceError::Preparation(error).into());
        }
        self.bloom = Some(self.complete_pipeline_preparation(Ok(candidate))?);
        Ok(())
    }
    pub fn disable_bloom(&mut self) {
        self.bloom = None;
    }
}

#[cfg(test)]
mod tests;

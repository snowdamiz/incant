//! Conservative clustered forward lighting; overflow falls back to all local
//! lights rather than silently discarding illumination. Buffers are retained
//! at one grid size; commands preserve replaced buffers across resize.
#[cfg(test)]
#[path = "../../tests/lighting_gpu/mod.rs"]
mod gpu_tests;
pub(crate) mod scene;
use crate::{ResourceError, Result};
use std::sync::{Arc, Mutex};
use wgpu::util::DeviceExt;
pub(crate) const DEPTH_SLICES: u32 = 24;
pub(crate) const CLUSTER_CAPACITY: u32 = 64;
const MAX_CLUSTERS: u32 = 131072;
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct GridUniform {
    pub view: [[f32; 4]; 4],
    pub viewport: [f32; 4],
    pub dimensions: [u32; 4],
    pub depth: [f32; 4],
    pub lights: [u32; 4],
}
pub(crate) struct GpuLights {
    buffer: wgpu::Buffer,
    directional: u32,
    local: u32,
}
impl GpuLights {
    pub fn upload(device: &wgpu::Device, plan: scene::LightPlan) -> Self {
        let (lights, directional) = plan.finish();
        Self {
            buffer: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Immutable authored lights"),
                contents: bytemuck::cast_slice(&lights),
                usage: wgpu::BufferUsages::STORAGE,
            }),
            directional,
            local: lights.len() as u32 - directional,
        }
    }
}
pub(crate) struct GridBuffers {
    dimensions: [u32; 2],
    pub counts: wgpu::Buffer,
    pub indices: wgpu::Buffer,
}
pub(crate) struct LightingSystem {
    pub layout: wgpu::BindGroupLayout,
    compute_layout: wgpu::BindGroupLayout,
    pipeline: wgpu::ComputePipeline,
    cache: Mutex<Option<Arc<GridBuffers>>>,
}
fn layout(device: &wgpu::Device, compute: bool) -> wgpu::BindGroupLayout {
    let entries: Vec<_> = (0..4)
        .map(|binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: if compute {
                wgpu::ShaderStages::COMPUTE
            } else {
                wgpu::ShaderStages::FRAGMENT
            },
            ty: wgpu::BindingType::Buffer {
                ty: if binding == 0 {
                    wgpu::BufferBindingType::Uniform
                } else {
                    wgpu::BufferBindingType::Storage {
                        read_only: !compute || binding == 1,
                    }
                },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        })
        .collect();
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Clustered lighting"),
        entries: &entries,
    })
}
impl LightingSystem {
    pub fn new(device: &wgpu::Device) -> Self {
        let compute_layout = layout(device, true);
        let layout = layout(device, false);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Light cluster assignment"),
            source: wgpu::ShaderSource::Wgsl(include_str!("grid.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Light cluster assignment"),
            bind_group_layouts: &[Some(&compute_layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Light cluster assignment"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("build"),
            compilation_options: Default::default(),
            cache: None,
        });
        Self {
            layout,
            compute_layout,
            pipeline,
            cache: Mutex::new(None),
        }
    }
    fn buffers(&self, device: &wgpu::Device, dimensions: [u32; 2]) -> Result<Arc<GridBuffers>> {
        let count = dimensions[0]
            .checked_mul(dimensions[1])
            .and_then(|n| n.checked_mul(DEPTH_SLICES))
            .filter(|n| *n > 0 && *n <= MAX_CLUSTERS)
            .ok_or(ResourceError::LightGridSize)?;
        let bytes = u64::from(count) * u64::from(CLUSTER_CAPACITY) * 4;
        if bytes > device.limits().max_storage_buffer_binding_size
            || bytes > device.limits().max_buffer_size
        {
            return Err(ResourceError::LightGridSize.into());
        }
        let mut cached = self
            .cache
            .lock()
            .map_err(|_| ResourceError::CacheLock("light grid"))?;
        if let Some(buffers) = cached.as_ref().filter(|b| b.dimensions == dimensions) {
            return Ok(Arc::clone(buffers));
        }
        let make = |label, size| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            })
        };
        let buffers = Arc::new(GridBuffers {
            dimensions,
            counts: make("Cluster light counts", u64::from(count) * 4),
            indices: make("Cluster light indices", bytes),
        });
        *cached = Some(Arc::clone(&buffers));
        Ok(buffers)
    }
    pub fn prepare(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        lights: &GpuLights,
        viewport: [f32; 4],
    ) -> Result<wgpu::BindGroup> {
        let dimensions = [
            (viewport[2] / 64.).ceil() as u32,
            (viewport[3] / 64.).ceil() as u32,
        ];
        let buffers = self.buffers(
            device,
            if lights.local == 0 {
                [1, 1]
            } else {
                dimensions
            },
        )?;
        let grid = GridUniform {
            view: crate::camera_view().to_cols_array_2d(),
            viewport,
            dimensions: [dimensions[0], dimensions[1], DEPTH_SLICES, CLUSTER_CAPACITY],
            depth: [
                crate::CAMERA_NEAR,
                crate::CAMERA_FAR,
                DEPTH_SLICES as f32 / (crate::CAMERA_FAR / crate::CAMERA_NEAR).ln(),
                (crate::CAMERA_FOV * 0.5).tan(),
            ],
            lights: [lights.directional, lights.local, 0, 0],
        };
        let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Immutable light grid frame"),
            contents: bytemuck::bytes_of(&grid),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let entries = [&uniform, &lights.buffer, &buffers.counts, &buffers.indices]
            .iter()
            .enumerate()
            .map(|(binding, buffer)| wgpu::BindGroupEntry {
                binding: binding as u32,
                resource: buffer.as_entire_binding(),
            })
            .collect::<Vec<_>>();
        let compute = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Cluster assignment frame"),
            layout: &self.compute_layout,
            entries: &entries,
        });
        // With no local lights the fragment shader returns before accessing the
        // lists. Otherwise every cluster is overwritten, including empty ones.
        if lights.local > 0 {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Assign local lights to view clusters"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &compute, &[]);
            pass.dispatch_workgroups(dimensions[0], dimensions[1], DEPTH_SLICES);
        }
        Ok(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Cluster shading frame"),
            layout: &self.layout,
            entries: &entries,
        }))
    }
}

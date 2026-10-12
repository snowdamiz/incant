//! Fixed geometry host layouts; no externally supplied shader artifacts.
use crate::{Result, shader::Stages};
#[cfg(all(feature = "cooked-geometry", not(target_arch = "wasm32")))]
mod cooked;

#[derive(Clone, Copy, Debug)]
pub(crate) enum Kind {
    Diagnostic,
    Shadow,
}

pub(crate) fn stages(device: &wgpu::Device, _backend: wgpu::Backend, kind: Kind) -> Result<Stages> {
    #[cfg(all(feature = "cooked-geometry", not(target_arch = "wasm32")))]
    return cooked::stages(device, _backend, kind);
    #[cfg(not(all(feature = "cooked-geometry", not(target_arch = "wasm32"))))]
    {
        let (label, source) = match kind {
            Kind::Diagnostic => ("Phase 0 diagnostic shader", include_str!("fixture.wgsl")),
            Kind::Shadow => (
                "Directional depth caster",
                include_str!("shadows/caster.wgsl"),
            ),
        };
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(label),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        Ok(Stages {
            vertex: module.clone(),
            fragment: module,
            vertex_entry: "vertex",
            fragment_entry: "fragment",
        })
    }
}
const DIAGNOSTIC_ATTRIBUTES: [wgpu::VertexAttribute; 2] =
    wgpu::vertex_attr_array![0=>Float32x4,1=>Float32x3];
const MESH_ATTRIBUTES: [wgpu::VertexAttribute; 4] =
    wgpu::vertex_attr_array![0=>Float32x3,1=>Float32x3,9=>Float32x2,10=>Float32x4];
const INSTANCE_ATTRIBUTES: [wgpu::VertexAttribute; 7] = wgpu::vertex_attr_array![2=>Float32x4,3=>Float32x4,4=>Float32x4,5=>Float32x4,6=>Float32x4,7=>Float32x4,8=>Float32x4];
pub(crate) fn diagnostic_buffers() -> [wgpu::VertexBufferLayout<'static>; 1] {
    [wgpu::VertexBufferLayout {
        array_stride: size_of::<crate::Vertex>() as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &DIAGNOSTIC_ATTRIBUTES,
    }]
}
pub(crate) fn shadow_buffers() -> [wgpu::VertexBufferLayout<'static>; 2] {
    [
        wgpu::VertexBufferLayout {
            array_stride: size_of::<incant_assets_runtime::Vertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &MESH_ATTRIBUTES,
        },
        wgpu::VertexBufferLayout {
            array_stride: size_of::<crate::scene::Instance>() as u64,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &INSTANCE_ATTRIBUTES,
        },
    ]
}

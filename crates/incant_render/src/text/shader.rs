use crate::{Result, shader::Stages};
#[cfg(all(feature = "cooked-text", not(target_arch = "wasm32")))]
mod cooked;

pub(super) fn stages(device: &wgpu::Device, _backend: wgpu::Backend) -> Result<Stages> {
    #[cfg(all(feature = "cooked-text", not(target_arch = "wasm32")))]
    return cooked::stages(device, _backend);
    #[cfg(not(all(feature = "cooked-text", not(target_arch = "wasm32"))))]
    {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("World-space text placement and coverage"),
            source: wgpu::ShaderSource::Wgsl(include_str!("text.wgsl").into()),
        });
        Ok(Stages {
            vertex: module.clone(),
            fragment: module,
            vertex_entry: "vertex",
            fragment_entry: "fragment",
        })
    }
}

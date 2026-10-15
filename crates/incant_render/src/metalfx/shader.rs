//! Private fixed boundary stages. A native cooked selection never parses WGSL.
use crate::{Result, shader::Stages};
#[cfg(all(feature = "cooked-metalfx-boundary", not(target_arch = "wasm32")))]
mod cooked;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    Pack,
    Normalize,
}
pub(super) fn stages(device: &wgpu::Device, _backend: wgpu::Backend, kind: Kind) -> Result<Stages> {
    #[cfg(all(feature = "cooked-metalfx-boundary", not(target_arch = "wasm32")))]
    return cooked::stages(device, _backend, kind);
    #[cfg(not(all(feature = "cooked-metalfx-boundary", not(target_arch = "wasm32"))))]
    {
        super::shader_admission().map_err(|_| {
            crate::ResourceError::CookedMetalFxBoundary("incomplete cooked boundary family")
        })?;
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Fixed MetalFX boundary"),
            source: wgpu::ShaderSource::Wgsl(super::source::source(kind == Kind::Normalize).into()),
        });
        Ok(Stages {
            vertex: module.clone(),
            fragment: module,
            vertex_entry: "vertex",
            fragment_entry: match kind {
                Kind::Pack => "pack",
                Kind::Normalize => "normalize",
            },
        })
    }
}

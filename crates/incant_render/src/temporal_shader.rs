//! Private fixed motion/temporal stages. Native cooked selection never falls back.
use crate::{Result, shader::Stages};
#[cfg(all(feature = "cooked-temporal", not(target_arch = "wasm32")))]
mod cooked;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Motion,
    MotionOwner,
    Depth,
    Resolve,
    Copy,
    Upscale,
    Composite,
    #[cfg(feature = "native-metalfx")]
    MetalFxPack,
    #[cfg(feature = "native-metalfx")]
    MetalFxNormalize,
}
#[cfg(any(
    test,
    not(all(feature = "cooked-temporal", not(target_arch = "wasm32")))
))]
pub(crate) fn source(kind: Kind) -> std::borrow::Cow<'static, str> {
    match kind {
        Kind::MotionOwner => include_str!("motion/owner.wgsl").into(),
        Kind::Motion | Kind::Depth => include_str!("motion/shader.wgsl").into(),
        Kind::Resolve => include_str!("temporal/resolve.wgsl").into(),
        Kind::Copy => include_str!("temporal/copy.wgsl").into(),
        Kind::Upscale => format!(
            "{}\n{}",
            include_str!("upscaling/common.wgsl"),
            include_str!("upscaling/resolve.wgsl")
        )
        .into(),
        #[cfg(feature = "native-metalfx")]
        Kind::MetalFxPack => format!(
            "{}\n{}",
            include_str!("upscaling/common.wgsl"),
            include_str!("metalfx/prepare.wgsl")
        )
        .into(),
        #[cfg(feature = "native-metalfx")]
        Kind::MetalFxNormalize => format!(
            "{}\n{}",
            include_str!("upscaling/common.wgsl"),
            include_str!("metalfx/composite.wgsl")
        )
        .into(),
        Kind::Composite => format!(
            "{}\n{}",
            include_str!("upscaling/common.wgsl"),
            include_str!("upscaling/composite.wgsl")
        )
        .into(),
    }
}
pub(crate) fn stages(device: &wgpu::Device, _backend: wgpu::Backend, kind: Kind) -> Result<Stages> {
    #[cfg(all(feature = "cooked-temporal", not(target_arch = "wasm32")))]
    return cooked::stages(device, _backend, kind);
    #[cfg(not(all(feature = "cooked-temporal", not(target_arch = "wasm32"))))]
    {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Fixed temporal program"),
            source: wgpu::ShaderSource::Wgsl(source(kind)),
        });
        Ok(Stages {
            vertex: module.clone(),
            fragment: module,
            vertex_entry: "vertex",
            fragment_entry: match kind {
                Kind::Motion | Kind::MotionOwner => "fragment",
                Kind::Depth => "depth_only",
                #[cfg(feature = "native-metalfx")]
                Kind::MetalFxPack => "pack",
                #[cfg(feature = "native-metalfx")]
                Kind::MetalFxNormalize => "normalize",
                Kind::Resolve | Kind::Upscale => "resolve",
                Kind::Copy => "copy",
                Kind::Composite => "composite",
            },
        })
    }
}

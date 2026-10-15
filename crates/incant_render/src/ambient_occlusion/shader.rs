//! A complete fixed SSAO route; never silently mix cooked and runtime WGSL.
use crate::{Result, shader::Stages};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    Prepass,
    Forward,
    Sample,
    Horizontal,
    Vertical,
}

pub(super) fn admit() -> Result<()> {
    if cfg!(all(
        feature = "cooked-output",
        not(feature = "cooked-ssao"),
        not(target_arch = "wasm32")
    )) {
        return Err(super::AmbientOcclusionError::CookedMaterials.into());
    }
    Ok(())
}

pub(super) fn validate(renderer: &crate::Renderer) -> Result<()> {
    admit()?;
    #[cfg(all(feature = "cooked-ssao", not(target_arch = "wasm32")))]
    super::cooked::validate(
        renderer.adapter.get_info().backend,
        renderer.device.features(),
    )?;
    #[cfg(not(all(feature = "cooked-ssao", not(target_arch = "wasm32"))))]
    let _ = renderer;
    Ok(())
}

#[cfg(any(test, not(all(feature = "cooked-ssao", not(target_arch = "wasm32")))))]
pub(super) fn source(kind: Kind) -> Result<String> {
    match kind {
        Kind::Prepass => super::pipelines::material_source(true),
        Kind::Forward => super::pipelines::material_source(false),
        Kind::Sample => Ok(include_str!("ao.wgsl").into()),
        Kind::Horizontal | Kind::Vertical => Ok(include_str!("filter.wgsl").into()),
    }
}

pub(super) fn stages(renderer: &crate::Renderer, kind: Kind) -> Result<Stages> {
    #[cfg(all(feature = "cooked-ssao", not(target_arch = "wasm32")))]
    {
        super::cooked::stages(&renderer.device, renderer.adapter.get_info().backend, kind)
    }
    #[cfg(not(all(feature = "cooked-ssao", not(target_arch = "wasm32"))))]
    {
        admit()?;
        let module = renderer
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Engine-owned SSAO program"),
                source: wgpu::ShaderSource::Wgsl(source(kind)?.into()),
            });
        Ok(Stages {
            vertex: module.clone(),
            fragment: module,
            vertex_entry: "vertex",
            fragment_entry: match kind {
                Kind::Horizontal => "horizontal",
                Kind::Vertical => "vertical",
                _ => "fragment",
            },
        })
    }
}

pub(super) fn filters(renderer: &crate::Renderer) -> Result<[Stages; 2]> {
    #[cfg(all(feature = "cooked-ssao", not(target_arch = "wasm32")))]
    {
        Ok([
            stages(renderer, Kind::Horizontal)?,
            stages(renderer, Kind::Vertical)?,
        ])
    }
    #[cfg(not(all(feature = "cooked-ssao", not(target_arch = "wasm32"))))]
    {
        admit()?;
        let module = renderer
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("SSAO plane-aware filter"),
                source: wgpu::ShaderSource::Wgsl(source(Kind::Horizontal)?.into()),
            });
        Ok([Kind::Horizontal, Kind::Vertical].map(|kind| Stages {
            vertex: module.clone(),
            fragment: module.clone(),
            vertex_entry: "vertex",
            fragment_entry: if kind == Kind::Horizontal {
                "horizontal"
            } else {
                "vertical"
            },
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_selection_or_ordinary_wgsl_admits_but_partial_cooked_modes_refuse() {
        // Run in default, cooked-output, cooked-materials and public aggregate
        // configurations. This calls the real pre-allocation admission function.
        if cfg!(all(
            feature = "cooked-output",
            not(feature = "cooked-ssao"),
            not(target_arch = "wasm32")
        )) {
            assert!(matches!(
                admit()
                    .unwrap_err()
                    .downcast_ref::<super::super::AmbientOcclusionError>(),
                Some(super::super::AmbientOcclusionError::CookedMaterials)
            ));
        } else {
            admit().unwrap();
        }
        for kind in [
            Kind::Prepass,
            Kind::Forward,
            Kind::Sample,
            Kind::Horizontal,
            Kind::Vertical,
        ] {
            assert!(!source(kind).unwrap().is_empty());
        }
    }
}

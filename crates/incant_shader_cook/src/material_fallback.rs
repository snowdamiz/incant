//! Fixed pending-material program using the complete ordinary four-group layout.
//! Unlike the SSAO prepass, HLSL special constants remain b5 and the fragment MSL
//! sizes slot remains 7. Unused environment/light bindings do not narrow the host.
use crate::{Artifact, Backend, Bundle, CookError, Result, Stage, geometry};
use geometry::material::Variant;
use naga::{ShaderStage, back::pipeline_constants, valid};
use sha2::{Digest, Sha256};
pub const CONTRACT: &str = "incant-material-fallback-v1;interpolation=centroid;naga=29.0.4;wgpu=29.0.4;layout=ordinary-four-groups;msl=2.4,pull15v14i,stride48-112,vertex-sizes1,fragment-sizes7;hlsl=6.0,frame-b0,material-b1,base-t0,sampler-indices-t5,special-b5;spv=1.0,no-y-flip;bounds=restrict;loops=bounded";
pub fn cook(source: &str) -> Result<Bundle> {
    let module = naga::front::wgsl::parse_str(source)
        .map_err(|e| CookError::Parse(e.emit_to_string(source)))?;
    let info = valid::Validator::new(valid::ValidationFlags::all(), valid::Capabilities::empty())
        .validate(&module)
        .map_err(|e| CookError::Validate(e.to_string()))?;
    geometry::interpolation::validate(geometry::Kind::Material, &module)?;
    geometry::material::validate_variant(Variant::Fallback, &module, &info)?;
    let mut artifacts = Vec::with_capacity(6);
    for (stage, name, tag) in [
        (ShaderStage::Vertex, "vertex", Stage::Vertex),
        (ShaderStage::Fragment, "fragment", Stage::Fragment),
    ] {
        let (compact, compact_info) = pipeline_constants::process_overrides(
            &module,
            &info,
            Some((stage, name)),
            &Default::default(),
        )
        .map_err(|e| CookError::Emit(e.to_string()))?;
        for backend in [Backend::Metal, Backend::Vulkan, Backend::Dx12] {
            let (bytes, entry) = geometry::material_emit::emit_variant(
                Variant::Fallback,
                &compact,
                &compact_info,
                &module,
                stage,
                name,
                backend,
            )?;
            artifacts.push(Artifact {
                backend,
                stage: tag,
                entry,
                bytes,
            });
        }
    }
    let producer = [
        include_str!("material_fallback.rs"),
        include_str!("geometry/material.rs"),
        include_str!("geometry/material_emit.rs"),
        include_str!("geometry/emit.rs"),
        include_str!("geometry/interpolation.rs"),
        include_str!("geometry/mod.rs"),
        include_str!("geometry/sprite.rs"),
        include_str!("audit.rs"),
        include_str!("../Cargo.toml"),
    ]
    .join("\n");
    Ok(Bundle {
        source_hash: Sha256::digest(source).into(),
        producer_hash: Sha256::digest(producer).into(),
        artifacts,
    })
}
pub fn write_bundle(bundle: &Bundle, directory: &std::path::Path) -> Result<()> {
    geometry::write_contract_bundle(CONTRACT, bundle, directory)
}
#[cfg(test)]
#[path = "../../incant_render/src/material_prewarm/source.rs"]
mod test_source;
#[cfg(test)]
mod tests {
    #[test]
    fn fallback_cooks_all_native_backends_and_refuses_abi_changes() {
        let source = super::test_source::fallback().unwrap();
        let bundle = super::cook(&source).unwrap();
        assert_eq!(bundle.artifacts.len(), 6);
        for changed in [
            source.replace("@group(1) @binding(1)", "@group(1) @binding(11)"),
            source.replace("perspective, centroid", "perspective, center"),
        ] {
            assert!(super::cook(&changed).is_err());
        }
        let hlsl = bundle
            .artifacts
            .iter()
            .find(|a| a.backend == crate::Backend::Dx12 && a.stage == crate::Stage::Vertex)
            .unwrap();
        assert!(
            std::str::from_utf8(&hlsl.bytes)
                .unwrap()
                .contains("register(b5)")
        );
    }
}

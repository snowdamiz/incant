use crate::shader::{
    Stages,
    native::{Artifact, Bundle, Stage},
};
include!(concat!(
    env!("OUT_DIR"),
    "/material-fallback-shaders/manifest.rs"
));
const BUNDLE: Bundle = Bundle {
    contract: MANIFEST_CONTRACT,
    expected: "incant-material-fallback-v1;interpolation=centroid;naga=29.0.4;wgpu=29.0.4;layout=ordinary-four-groups;msl=2.4,pull15v14i,stride48-112,vertex-sizes1,fragment-sizes7;hlsl=6.0,frame-b0,material-b1,base-t0,sampler-indices-t5,special-b5;spv=1.0,no-y-flip;bounds=restrict;loops=bounded",
    source: SOURCE_HASH,
    producer: PRODUCER_HASH,
    artifacts: &ARTIFACTS,
};
pub(super) fn stages(device: &wgpu::Device, backend: wgpu::Backend) -> crate::Result<Stages> {
    crate::material_pipeline::validate_cooked_host()
        .map_err(crate::ResourceError::CookedMaterial)?;
    BUNDLE
        .stages(device, backend)
        .map_err(|e| crate::ResourceError::CookedMaterial(e).into())
}

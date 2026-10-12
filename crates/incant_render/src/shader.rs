//! Private fixed shader stages shared by the builtin pipeline constructors.
#[cfg(all(feature = "cooked-output", not(target_arch = "wasm32")))]
pub(crate) mod native;
pub(crate) struct Stages {
    pub vertex: wgpu::ShaderModule,
    pub fragment: wgpu::ShaderModule,
    pub vertex_entry: &'static str,
    pub fragment_entry: &'static str,
}

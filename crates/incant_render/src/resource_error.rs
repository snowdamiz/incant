//! Resource preparation errors remain typed even at the renderer's boxed boundary.
#[derive(Debug, thiserror::Error)]
pub enum ResourceError {
    #[error("render target exceeds dimension or 16-megapixel attachment budget")]
    RenderTargetSize,
    #[error("cluster grid exceeds the 32 MiB index budget or device storage limits")]
    LightGridSize,
    #[error("unsupported display output format: {0:?}")]
    OutputFormat(wgpu::TextureFormat),
    #[error("cooked material image exceeds GPU texture limit")]
    TextureSize,
    #[error("material image exceeds finite RGBA16Float GPU range")]
    TextureRange,
    #[error("unsupported environment source: {0}")]
    EnvironmentSource(&'static str),
    #[error("{0} exceeds GPU buffer limit")]
    BufferSize(&'static str),
    #[error("transformed geometry exceeds GPU numeric range")]
    GeometryRange,
    #[error("{0} cache lock failed")]
    CacheLock(&'static str),
}

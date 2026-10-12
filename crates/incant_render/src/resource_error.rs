//! Resource preparation errors remain typed even at the renderer's boxed boundary.
#[cfg(not(target_arch = "wasm32"))]
pub type PreparationError = wgpu::Error;
#[cfg(target_arch = "wasm32")]
pub use crate::web_error::PreparationError;
#[derive(Debug, thiserror::Error)]
pub enum ResourceError {
    #[error("cooked output shader rejected: {0}")]
    CookedOutput(&'static str),
    #[error("cooked geometry shader rejected: {0}")]
    CookedGeometry(&'static str),
    #[error("cooked sprite shader rejected: {0}")]
    CookedSprite(&'static str),
    #[error("cooked UI shader rejected: {0}")]
    CookedUi(&'static str),
    #[error("renderer initialization failed: {0}")]
    Preparation(#[source] PreparationError),
    #[error("{0} pipeline variant was not prepared during renderer initialization")]
    UnpreparedPipeline(&'static str),
    #[error("native GPU device was lost; the rendering host must stop")]
    DeviceLost,
    #[error(
        "sprite entity {entity}, texture {texture}, source region {region:?}, cooked texture size {texture_size:?}: {source}"
    )]
    SpriteGeometry {
        entity: String,
        texture: String,
        region: [u32; 4],
        texture_size: [u32; 2],
        #[source]
        source: incant_2d::SpriteError,
    },
    #[error("directional shadow projection exceeds finite GPU precision")]
    ShadowRange,
    #[error("directional shadow atlas exceeds the four-light / 64 MiB budget or device limits")]
    ShadowMapSize,
    #[error("BLEND materials cannot cast directional shadows; disable cast_shadows or use MASK")]
    TransparentShadowCaster,
    #[error("render target exceeds dimension or 16-megapixel attachment budget")]
    RenderTargetSize,
    #[error("cluster grid exceeds the 64 MiB membership-mask budget or device storage limits")]
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

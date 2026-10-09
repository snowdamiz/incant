//! Resource preparation errors remain typed even at the renderer's boxed boundary.
#[derive(Debug, thiserror::Error)]
pub enum ResourceError {
    #[error("cooked material image exceeds GPU texture limit")]
    TextureSize,
    #[error("material image exceeds finite RGBA16Float GPU range")]
    TextureRange,
    #[error("{0} exceeds GPU buffer limit")]
    BufferSize(&'static str),
    #[error("transformed geometry exceeds GPU numeric range")]
    GeometryRange,
    #[error("{0} cache lock failed")]
    CacheLock(&'static str),
}

use crate::StableId;

/// Kind of immutable cooked asset consumed by the current spatial renderer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum VisualAssetKind {
    Model,
    Texture,
}

/// Binary startup binding, independent of source paths and authoring registries.
///
/// `fingerprint` is the existing cooked cache/source-recipe identity. It is not
/// necessarily the hash of the complete cooked payload and is not authentication.
/// A loader must match this identity and kind, verify the format's own integrity
/// and resource checks, and retain an immutable version before publishing it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VisualAssetRef {
    pub id: StableId,
    pub kind: VisualAssetKind,
    pub fingerprint: [u8; 32],
}

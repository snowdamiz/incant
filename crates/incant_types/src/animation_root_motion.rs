//! Authored root selection. Cooked topology and physics policy validation are
//! separate cold-boundary checks; no model or solver dependency belongs here.
use crate::Id;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum RootMotionApplication {
    ExtractOnly,
    Transform,
    Kinematic3d,
    Character3d,
}

#[derive(
    Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct AnimationRootMotion {
    pub model: Id,
    /// Captured recipe identity, not a source path or an authentication token.
    /// A changed model revision requires an explicit root binding transaction.
    #[schemars(length(min = 64, max = 64), regex(pattern = "^[0-9a-f]{64}$"))]
    pub fingerprint: String,
    #[schemars(range(min = 0, max = 4095))]
    pub root: u16,
    pub application: RootMotionApplication,
    /// Automatic movement can be stopped without changing animation clocks or
    /// stripping. ExtractOnly always leaves body movement to gameplay.
    pub applying: bool,
}
impl AnimationRootMotion {
    pub fn validate(&self) -> Result<(), &'static str> {
        if !crate::animation_graph::canonical_id(&self.model)
            || self.root >= 4096
            || self.fingerprint.len() != 64
            || !self
                .fingerprint
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err("invalid root-motion model, fingerprint or root node");
        }
        Ok(())
    }
}

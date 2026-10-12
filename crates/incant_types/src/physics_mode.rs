/// Authoring choice for a separately built simulation process. Fast mode does
/// not promise bit-identical results across hardware or solver variants.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum PhysicsMode {
    Fast,
    Deterministic,
}

impl std::fmt::Display for PhysicsMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Fast => "fast",
            Self::Deterministic => "deterministic",
        })
    }
}

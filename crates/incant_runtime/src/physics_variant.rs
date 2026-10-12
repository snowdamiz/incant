//! A cooked scene is admitted to one compiled kernel before a world exists.
//! This keeps NativeWorld construction infallible without a runtime solver toggle.
use crate::SceneError;
pub use incant_types::PhysicsMode;

const _: () = assert!(
    incant_physics_runtime::DETERMINISTIC == incant_physics_2d_runtime::DETERMINISTIC
        && incant_physics_runtime::SIMD_LANES == incant_physics_2d_runtime::SIMD_LANES,
    "2D and 3D physics must select the same process variant"
);

/// Exact kernel identity. Eight-lane fast is a distinct, opt-in candidate, not
/// interchangeable with four-lane fast for cooked scenes or saved simulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PhysicsVariant {
    Deterministic,
    Fast,
    FastSimd8,
}

pub const PHYSICS_VARIANT: PhysicsVariant = if incant_physics_runtime::DETERMINISTIC {
    PhysicsVariant::Deterministic
} else if incant_physics_runtime::SIMD_LANES == 8 {
    PhysicsVariant::FastSimd8
} else {
    PhysicsVariant::Fast
};

impl PhysicsVariant {
    pub const fn mode(self) -> PhysicsMode {
        match self {
            Self::Deterministic => PhysicsMode::Deterministic,
            Self::Fast | Self::FastSimd8 => PhysicsMode::Fast,
        }
    }

    pub(crate) const fn tag(self) -> u8 {
        match self {
            Self::Deterministic => 0,
            Self::Fast => 1,
            Self::FastSimd8 => 2,
        }
    }

    pub(crate) fn from_tag(tag: u8) -> Result<Self, SceneError> {
        match tag {
            0 => Ok(Self::Deterministic),
            1 => Ok(Self::Fast),
            2 => Ok(Self::FastSimd8),
            _ => Err(SceneError::Invalid("unknown physics variant")),
        }
    }

    pub(crate) fn admit(self) -> Result<(), SceneError> {
        if self != PHYSICS_VARIANT {
            return Err(SceneError::PhysicsVariant {
                requested: self,
                compiled: PHYSICS_VARIANT,
            });
        }
        Ok(())
    }
}

impl std::fmt::Display for PhysicsVariant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Deterministic => "deterministic",
            Self::Fast => "fast",
            Self::FastSimd8 => "fast_simd8",
        })
    }
}

/// Reject a project/process mismatch before cooking or initializing simulation.
pub fn require_physics_mode(requested: PhysicsMode) -> Result<(), SceneError> {
    if requested != PHYSICS_VARIANT.mode() {
        return Err(SceneError::PhysicsMode {
            requested,
            compiled: PHYSICS_VARIANT,
        });
    }
    Ok(())
}

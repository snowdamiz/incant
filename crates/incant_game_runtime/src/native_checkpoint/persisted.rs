//! Versioned game envelope around the separately validated native session file.
//! Sample rate and suspension are checked against the retained audio owner.
use super::*;
use incant_input::InputActions;
use incant_runtime::CookedScene;
use incant_script::{CheckpointError, SessionProgram};
use sha2::{Digest, Sha256};

const MAGIC: &[u8; 8] = b"INCNGM01";
const HEADER: usize = 64;
pub const MAX_PERSISTED_GAME_BYTES: usize = incant_script::MAX_PERSISTED_SESSION_BYTES + HEADER;

fn digest(bytes: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(&bytes[..32]);
    h.update(&bytes[HEADER..]);
    h.finalize().into()
}

fn invalid(why: &'static str) -> GameError {
    GameError::Checkpoint(CheckpointError::Invalid(why))
}

impl GameCheckpoint {
    /// Export supported physics, opted-in script data and retained host state.
    /// No executable code, action capabilities or device handles enter this file.
    pub fn persisted_bytes(
        &self,
        payload: &mut CheckpointPayloadBudget,
        planar_work: &mut PlanarCheckpointWorkBudget,
        spatial_work: &mut SpatialCheckpointWorkBudget,
    ) -> Result<Vec<u8>> {
        let native = self
            .native
            .persisted_bytes(payload, planar_work, spatial_work)?;
        let mut bytes = Vec::with_capacity(HEADER + native.len());
        bytes.extend(MAGIC);
        bytes.extend(1u32.to_le_bytes());
        let flags = u32::from(self.suspended) | (u32::from(self.sample_rate.is_some()) << 1);
        bytes.extend(flags.to_le_bytes());
        bytes.extend(self.sample_rate.unwrap_or(0).to_le_bytes());
        bytes.extend(
            u32::try_from(native.len())
                .map_err(|_| invalid("game capacity"))?
                .to_le_bytes(),
        );
        bytes.extend([0; 8]);
        bytes.extend([0; 32]);
        bytes.extend(native);
        let checksum = digest(&bytes);
        bytes[32..HEADER].copy_from_slice(&checksum);
        Ok(bytes)
    }
}

impl PendingGameCheckpoint {
    /// Stage an independently validated game from a file and its trusted package.
    /// Required hosts are prepared by the existing `from_checkpoint` factories;
    /// no guest initialization or device operation happens in this method.
    #[allow(clippy::too_many_arguments)] // Trusted inputs and independent budgets.
    pub fn from_persisted_bytes(
        bytes: &[u8],
        initial: &CookedScene,
        program: impl Into<SessionProgram>,
        actions: InputActions,
        payload: &mut CheckpointPayloadBudget,
        planar_work: &mut PlanarCheckpointWorkBudget,
        spatial_work: &mut SpatialCheckpointWorkBudget,
        allocations: &mut CheckpointAllocationBudget,
    ) -> Result<Self> {
        if !(HEADER..=MAX_PERSISTED_GAME_BYTES).contains(&bytes.len()) || &bytes[..8] != MAGIC {
            return Err(invalid("game framing"));
        }
        let field = |start| u32::from_le_bytes(bytes[start..start + 4].try_into().unwrap());
        let flags = field(12);
        if field(8) != 1
            || flags & !3 != 0
            || bytes[24..32] != [0; 8]
            || field(20) as usize != bytes.len() - HEADER
        {
            return Err(invalid("game header"));
        }
        if bytes[32..HEADER] != digest(bytes) {
            return Err(GameError::Checkpoint(CheckpointError::Integrity));
        }
        let sample_rate = if flags & 2 != 0 {
            crate::offline::validate_rate(field(16))?;
            Some(field(16))
        } else {
            if field(16) != 0 {
                return Err(invalid("backend sample rate"));
            }
            None
        };
        let suspended = flags & 1 != 0;
        let native = PendingSessionCheckpoint::from_persisted_bytes(
            &bytes[HEADER..],
            initial,
            program,
            actions,
            payload,
            planar_work,
            spatial_work,
            allocations,
        )?;
        // Every offline capture retains even its silent mixer, so forged outer
        // metadata cannot alter its actual absolute sample clock or pause state.
        match (sample_rate, native.audio()) {
            (Some(rate), Some(audio))
                if rate == audio.sample_rate() && suspended == audio.suspended() => {}
            (None, None) => {}
            _ => return Err(invalid("game/audio clock pairing")),
        }
        Ok(Self {
            native,
            sample_rate,
            suspended,
        })
    }
}

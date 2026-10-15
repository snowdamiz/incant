//! File entry points for the opt-in native-checkpoint player. Every operation
//! uses cumulative physics budgets; ECS and host sections have separate caps.
use super::*;
use incant_runtime::{
    CheckpointAllocationBudget, CheckpointPayloadBudget, PlanarCheckpointWorkBudget,
    SpatialCheckpointWorkBudget,
};

struct Budgets {
    payload: CheckpointPayloadBudget,
    planar: PlanarCheckpointWorkBudget,
    spatial: SpatialCheckpointWorkBudget,
    allocations: CheckpointAllocationBudget,
}
impl Budgets {
    fn new() -> Result<Self> {
        Ok(Self {
            payload: CheckpointPayloadBudget::new(64 * 1024 * 1024)?,
            planar: PlanarCheckpointWorkBudget::new(100_000_000),
            spatial: SpatialCheckpointWorkBudget::new(100_000_000),
            allocations: CheckpointAllocationBudget::new(64 * 1024 * 1024, 100_000),
        })
    }
}

pub(super) fn save(player: &mut Player) -> Result<Vec<u8>> {
    let mut b = Budgets::new()?;
    let checkpoint = match player {
        Player::Plain(game) => game.checkpoint(&mut b.payload, &mut b.planar, &mut b.spatial)?,
        #[cfg(feature = "render-input")]
        Player::Spatial(game) => game.checkpoint(&mut b.payload, &mut b.planar, &mut b.spatial)?,
        #[cfg(feature = "render-input")]
        Player::Recording(_) => {
            return Err("material recording checkpoint saving is unsupported".into());
        }
        #[cfg(feature = "render-gpu")]
        Player::Gpu(_) | Player::RecordedGpu(_) => {
            return Err("GPU checkpoint saving is unsupported".into());
        }
    };
    // Export has its own cumulative decode/validation/output accounting. It is
    // separate from capture, not a reset inside any partially completed stage.
    let mut b = Budgets::new()?;
    Ok(checkpoint.persisted_bytes(&mut b.payload, &mut b.planar, &mut b.spatial)?)
}

pub(super) fn load(
    bytes: &[u8],
    initial: &CookedScene,
    script: SessionProgram,
    actions: InputActions,
    clips: Vec<ClipBinding>,
    rate: u32,
    settings: Option<Settings>,
) -> Result<Player> {
    let mut b = Budgets::new()?;
    let pending = incant_game_runtime::PendingGameCheckpoint::from_persisted_bytes(
        bytes,
        initial,
        script,
        actions,
        &mut b.payload,
        &mut b.planar,
        &mut b.spatial,
        &mut b.allocations,
    )?;
    if pending.sample_rate() != Some(rate) {
        return Err("checkpoint sample rate differs from requested output".into());
    }
    #[cfg(feature = "render-input")]
    if let Some(settings) = settings {
        return Ok(match settings {
            Settings::Cpu(settings) => Player::Spatial(Box::new(
                HostedOfflineGameSession::from_checkpoint(pending, clips, settings)?,
            )),
            Settings::Recording(_, _) => {
                return Err("material recording checkpoint loading is unsupported".into());
            }
            #[cfg(feature = "render-gpu")]
            Settings::Gpu(_) | Settings::RecordedGpu(_, _) => {
                return Err("GPU checkpoint loading is unsupported".into());
            }
        });
    }
    #[cfg(not(feature = "render-input"))]
    if let Some(settings) = settings {
        match settings {}
    }
    Ok(Player::Plain(Box::new(
        OfflineGameSession::from_checkpoint(pending, clips)?,
    )))
}

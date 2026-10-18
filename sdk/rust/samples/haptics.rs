//! Ephemeral haptics use trusted current generations, never saved authority.
#![no_std]
use core::sync::atomic::{AtomicU32, Ordering};
use incant_module::{Error, HapticEffect, HapticTarget, IDENTITY, Result, Tick, persistent};
static GENERATION: AtomicU32 = AtomicU32::new(0);
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    core::arch::wasm32::unreachable()
}
incant_module::persistent_state!(schema = [17; 32], bytes = 8);
incant_module::module!(simulate);
fn simulate(t: &mut Tick<'_>, _: f64) -> Result<()> {
    let mut state = [0; 8];
    t.read_persistent(0, &mut state)?;
    let count = persistent::read_u64(&state, 0)?
        .checked_add(1)
        .ok_or(Error::Bounds)?;
    persistent::write_u64(&mut state, 0, count)?;
    t.write_persistent(0, &state)?;
    for index in 0..t.chunk_count()? {
        let chunk = t.chunk(index)?;
        let mut storage = [IDENTITY; 16];
        let rows = t.read_transforms(chunk, &mut storage)?;
        for row in rows.iter_mut() {
            row[0] = count as f64;
        }
        t.write_transforms(chunk, rows)?;
    }
    if cfg!(haptic_scratch) {
        let _ = t.haptic_capabilities(&mut []);
        return Ok(());
    }
    let mut scratch = [0; incant_module::HAPTIC_MAX_BYTES];
    let targets = t.haptic_capabilities(&mut scratch)?;
    for mut target in targets.iter() {
        if cfg!(haptic_saved_target)
            && let HapticTarget::Gamepad { id, generation } = target
        {
            let saved = GENERATION.load(Ordering::Relaxed);
            if saved == 0 {
                GENERATION.store(generation, Ordering::Relaxed);
            } else {
                target = HapticTarget::Gamepad {
                    id,
                    generation: saved,
                };
            }
        }
        let effect = match target {
            HapticTarget::Gamepad { .. } => HapticEffect::Rumble {
                strong: if cfg!(haptic_amplitude) {
                    1. + f64::EPSILON
                } else {
                    0.5
                },
                weak: 1.,
                duration_ms: 125,
            },
            HapticTarget::Device { .. } => HapticEffect::Impact { intensity: 0.25 },
        };
        // Failure variants deliberately ignore returned SDK errors. Tick must
        // still trap and the host must discard numeric/persistent/haptic prefixes.
        let _ = t.play_haptic(target, effect);
        let _ = t.stop_haptic(target);
        if cfg!(haptic_quota) {
            for _ in 0..32 {
                let _ = t.stop_haptic(target);
            }
        }
    }
    if cfg!(haptic_owner) {
        t.despawn(&incant_module::EntityId::from_u128(999))?;
    }
    if cfg!(haptic_callback) {
        return Err(Error::Callback);
    }
    if cfg!(haptic_trap) {
        core::arch::wasm32::unreachable();
    }
    Ok(())
}

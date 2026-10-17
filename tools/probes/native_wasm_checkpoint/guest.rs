//! Fixed gameplay fixture: declared state and simulation timers survive a checkpoint.
#![no_std]
use core::sync::atomic::{AtomicU32, Ordering};
use incant_module::{
    AudioCommand, AudioControl, EntityId, Error, IDENTITY, Key, LogLevel, Result, Tick, persistent,
};

static CALLS: AtomicU32 = AtomicU32::new(0);
incant_module::persistent_state!(schema = [23; 32], bytes = 24);
incant_module::module!(simulate);

#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    core::arch::wasm32::unreachable()
}

fn increment(value: u64, add: u64) -> Result<u64> {
    value
        .checked_add(add)
        .filter(|n| *n <= 3600)
        .ok_or(Error::Bounds)
}

fn simulate(t: &mut Tick<'_>, _: f64) -> Result<()> {
    let mut state = [0u8; 24];
    t.read_persistent(0, &mut state)?;
    let count = increment(persistent::read_u64(&state, 0)?, 1)?;
    // Fixed probe has keyboard events only; oversized input refuses explicitly.
    let mut input_bytes = [0u8; 2048];
    let input = t.input(&mut input_bytes)?;
    let moves = increment(
        persistent::read_u64(&state, 8)?,
        u64::from(input.keyboard.held(Key::ArrowRight as u8)),
    )?;
    let score = increment(
        persistent::read_u64(&state, 16)?,
        u64::from(input.keyboard.pressed(Key::Space as u8)),
    )?;
    for (offset, value) in [(0, count), (8, moves), (16, score)] {
        persistent::write_u64(&mut state, offset, value)?;
    }
    t.write_persistent(0, &state)?;
    for n in 0..t.chunk_count()? {
        let chunk = t.chunk(n)?;
        let mut ids = [EntityId([0; 16]); 8];
        let ids = t.read_ids(chunk, &mut ids)?;
        let mut storage = [IDENTITY; 8];
        let rows = t.read_transforms(chunk, &mut storage)?;
        for (id, row) in ids.iter().zip(rows.iter_mut()) {
            if *id == EntityId::from_u128(1) {
                row[..3].copy_from_slice(&[moves as f64 / 16., score as f64, count as f64 / 64.]);
            }
        }
        t.write_transforms(chunk, rows)?;
    }
    // The same audible route is now driven by host-owned simulation timers.
    // A future periodic timer remains pending in both prefix and final saves.
    if count == 1 {
        t.set_timer("gain", 8, Some(11))?;
        t.set_timer("stop", 2, None)?;
        t.set_timer("stop", 25, None)?;
        t.set_timer("restart", 28, None)?;
        t.set_timer("upgain", 33, None)?;
        t.set_timer("future", 100, Some(17))?;
        t.set_timer("cancelled", 1, None)?;
        t.cancel_timer("cancelled")?;
    }
    let mut timer_bytes = [0; 1024];
    let timers = t.timers(&mut timer_bytes)?;
    if timers.tick() != count || timers.len() != usize::from(matches!(count, 9 | 20 | 26 | 29 | 34))
    {
        return Err(Error::Bounds);
    }
    for (id, due) in timers.events() {
        let control = match (id, due) {
            ("gain", 9) => AudioControl::SourceGain(-12.),
            ("gain", 20) => {
                t.cancel_timer("gain")?;
                AudioControl::Seek(0.0021)
            }
            ("stop", 26) => AudioControl::Stop,
            ("restart", 29) => AudioControl::Restart,
            ("upgain", 34) => AudioControl::SourceGain(-4.),
            _ => return Err(Error::Bounds),
        };
        t.audio_commands(
            &[AudioCommand {
                entity: EntityId::from_u128(2),
                control,
            }],
            &mut [0; 128],
        )?;
    }
    t.log(
        if CALLS.fetch_add(1, Ordering::Relaxed) == 0 {
            "fresh-guest"
        } else {
            "continued-guest"
        },
        LogLevel::Info,
    )?;
    Ok(())
}

//! Fixed fixture: coroutine frame, explicit state and a timer survive a checkpoint.
#![no_std]
use core::sync::atomic::{AtomicU32, Ordering};
use incant_module::{
    AudioCommand, AudioControl, EntityId, Error, IDENTITY, Key, LogLevel, Result, Tick, persistent,
};

static CALLS: AtomicU32 = AtomicU32::new(0);
incant_module::persistent_state!(schema = [23; 32], bytes = 24);
incant_module::coroutine_module!(simulate; "audio" => ([24; 32], 8, audio));

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
    if count == 1 {
        t.set_timer("future", 100, Some(17))?;
        let mut scratch = [0; incant_module::coroutines::COROUTINE_MAX_BYTES];
        t.start_coroutine(
            "audio-route",
            "audio",
            &0u64.to_le_bytes(),
            Some(EntityId::from_u128(2)),
            &mut scratch,
        )?;
        t.start_coroutine(
            "cancelled",
            "audio",
            &0u64.to_le_bytes(),
            None,
            &mut scratch,
        )?;
    }
    if count == 2 {
        t.cancel_coroutine("cancelled")?;
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

fn audio(
    t: &mut Tick<'_>,
    task: incant_module::coroutines::Task<'_>,
) -> Result<incant_module::coroutines::Next> {
    use incant_module::coroutines::Next;
    if task.id != "audio-route" || task.entity != Some(EntityId::from_u128(2)) {
        return Err(Error::Bounds);
    }
    let stage = persistent::read_u64(task.state, 0)?;
    let (control, delay) = match stage {
        0 => (None, 7),
        1 => (Some(AudioControl::SourceGain(-12.)), 11),
        2 => (Some(AudioControl::Seek(0.0021)), 6),
        3 => (Some(AudioControl::Stop), 3),
        4 => (Some(AudioControl::Restart), 5),
        5 => (Some(AudioControl::SourceGain(-4.)), 100),
        _ => return Ok(Next::Done),
    };
    if let Some(control) = control {
        t.audio_commands(
            &[AudioCommand {
                entity: EntityId::from_u128(2),
                control,
            }],
            &mut [0; 128],
        )?;
    }
    persistent::write_u64(task.state, 0, stage + 1)?;
    Ok(Next::Wait(delay))
}

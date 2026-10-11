//! Existing accepted-input/physics/audio fixture using the public Rust SDK.
#![no_std]
use incant_module::{
    ActionKind, EntityId, IDENTITY, INPUT_MAX_BYTES, Result, Tick, Transform, Velocity,
};
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    core::arch::wasm32::unreachable()
}
static mut INPUT: [u8; INPUT_MAX_BYTES] = [0; INPUT_MAX_BYTES];
static mut SPAWNED: bool = false;
static mut TICKS: u32 = 0;
incant_module::module!(run);
fn run(tick: &mut Tick<'_>, dt: f64) -> Result<()> {
    // The host executes one non-reentrant guest callback at a time.
    let number = unsafe {
        TICKS += 1;
        TICKS
    };
    let input = tick.input(unsafe { &mut *(&raw mut INPUT) })?;
    let jump = input.action("jump");
    if jump.is_some_and(|a| a.kind != ActionKind::Button) {
        return Err(incant_module::Error::Malformed);
    }
    let active = jump.is_some_and(|a| a.active);
    let pressed = jump.is_some_and(|a| a.pressed);
    let mut ids = [EntityId([0; 16]); 32];
    let mut values: [Transform; 32] = [IDENTITY; 32];
    let mut speeds: [Velocity; 32] = [[0.; 3]; 32];
    for i in 0..tick.chunk_count()? {
        let chunk = tick.chunk(i)?;
        let ids = tick.read_ids(chunk, &mut ids)?;
        let values = tick.read_transforms(chunk, &mut values)?;
        if chunk.has_velocity() {
            tick.read_velocities(chunk, &mut speeds)?;
        }
        for (row, id) in ids.iter().enumerate() {
            if *id == EntityId::from_u128(1) {
                if pressed {
                    values[row][0] += 4.;
                }
                if chunk.has_velocity() {
                    speeds[row][0] = if active { 2. } else { 0. };
                }
                values[row][1] = f64::from(number) * dt;
            }
        }
        tick.write_transforms(chunk, values)?;
        if chunk.has_velocity() {
            tick.write_velocities(chunk, &speeds)?;
        }
    }
    if pressed && !unsafe { SPAWNED } {
        unsafe {
            SPAWNED = true;
        }
        tick.spawn(
            &EntityId::from_u128(3),
            None,
            &IDENTITY,
            Some(&[2., 0., 0.]),
        )?;
    }
    Ok(())
}

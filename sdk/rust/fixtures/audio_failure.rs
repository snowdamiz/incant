//! Caught local errors and late native failures must discard every staged prefix.
#![no_std]
use incant_module::{AudioCommand, AudioControl as A, EntityId, IDENTITY, LogLevel, Result, Tick};
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    core::arch::wasm32::unreachable()
}
incant_module::module!(simulate);
fn command(n: u128, control: A) -> AudioCommand {
    AudioCommand {
        entity: EntityId::from_u128(n),
        control,
    }
}
fn simulate(tick: &mut Tick<'_>, _: f64) -> Result<()> {
    let mut scratch = [0; 256];
    tick.audio_commands(
        &[
            command(2, A::Restart),
            command(3, A::Stop),
            command(1, A::BusGain(-12.)),
        ],
        &mut scratch,
    )?;
    tick.spawn(&EntityId::from_u128(77), None, &IDENTITY, None)?;
    let chunk = tick.chunk(0)?;
    let mut transforms = [IDENTITY; 256];
    let rows = tick.read_transforms(chunk, &mut transforms)?;
    rows[0][0] = 42.;
    tick.write_transforms(chunk, rows)?;
    tick.log("accepted audio prefix", LogLevel::Warn)?;
    tick.audio(&mut [0; 4096])?;
    if cfg!(audio_scratch) {
        let _ = tick.audio_commands(&[command(2, A::Stop)], &mut []);
    }
    if cfg!(audio_snapshot) {
        let _ = tick.audio(&mut []);
    }
    if cfg!(audio_seek) {
        let _ = tick.audio_commands(&[command(2, A::Seek(f64::NAN))], &mut scratch);
    }
    if cfg!(audio_gain) {
        let _ = tick.audio_commands(&[command(2, A::SourceGain(24.000_000_001))], &mut scratch);
    }
    if cfg!(audio_owner) {
        tick.audio_commands(&[command(999, A::Stop)], &mut scratch)?;
    }
    if cfg!(audio_role) {
        tick.audio_commands(&[command(2, A::BusGain(-3.))], &mut scratch)?;
    }
    if cfg!(audio_quota) {
        let commands = [command(2, A::SetPlaying(true)); 1024];
        let _ = tick.audio_commands(&commands, &mut [0; 32784]);
    }
    if cfg!(audio_late) {
        tick.set_velocity(&EntityId::from_u128(999), Some(&[1., 0., 0.]))?;
    }
    if cfg!(audio_callback) {
        return Err(incant_module::Error::Callback);
    }
    if cfg!(audio_trap) {
        core::arch::wasm32::unreachable();
    }
    Ok(())
}

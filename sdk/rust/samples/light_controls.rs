//! Native/Wasm parity fixture: phase lives in committed components, not globals.
#![no_std]
use incant_module::{EntityId, Error, IDENTITY, LightCommand, LightControl as L, Result, Tick};
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    core::arch::wasm32::unreachable()
}
incant_module::module!(simulate);
fn simulate(t: &mut Tick<'_>, _: f64) -> Result<()> {
    let mut phase = None;
    for index in 0..t.chunk_count()? {
        let chunk = t.chunk(index)?;
        let mut ids = [EntityId::from_u128(0); 8];
        let ids = t.read_ids(chunk, &mut ids)?;
        if let Some(row) = ids.iter().position(|id| *id == EntityId::from_u128(1)) {
            if phase.is_some() {
                return Err(Error::Malformed);
            }
            let mut storage = [IDENTITY; 8];
            let rows = t.read_transforms(chunk, &mut storage)?;
            rows[row][0] += 1.;
            phase = Some(rows[row][0]);
            t.write_transforms(chunk, rows)?;
        }
    }
    let phase = phase.ok_or(Error::Malformed)?;
    let command = |n, control| LightCommand {
        entity: EntityId::from_u128(n),
        control,
    };
    let commands = [
        command(1, L::Intensity(phase * 100.)),
        command(2, L::Color([0.25, 0.5, 0.75])),
        command(3, L::Intensity(if phase % 3. == 0. { 0. } else { 10. })),
    ];
    t.light_commands(&commands, &mut [0; 256])?;
    if cfg!(light_scratch) {
        let _ = t.light_commands(&commands, &mut []);
    }
    if cfg!(light_range) {
        let _ = t.light_commands(&[command(1, L::Intensity(f64::NAN))], &mut [0; 64]);
    }
    if cfg!(light_owner) {
        t.light_commands(&[command(999, L::Intensity(1.))], &mut [0; 64])?;
    }
    Ok(())
}

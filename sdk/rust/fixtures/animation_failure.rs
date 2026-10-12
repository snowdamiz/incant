//! Ignored SDK errors and rejected native animation commit roll back everything.
#![no_std]
use incant_module::{AnimationControl as C, EntityId, IDENTITY, LogLevel, Result, Tick};
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    core::arch::wasm32::unreachable()
}
incant_module::module!(simulate);
fn simulate(tick: &mut Tick<'_>, _: f64) -> Result<()> {
    let mut scratch = [0; 256];
    let row = tick
        .animations(&mut scratch)?
        .iter()
        .next()
        .ok_or(incant_module::Error::Malformed)?;
    tick.control_animation(&row.entity, C::Playing(false))?;
    tick.spawn(&EntityId::from_u128(77), None, &IDENTITY, None)?;
    let chunk = tick.chunk(0)?;
    let mut transforms = [IDENTITY; 16];
    let rows = tick.read_transforms(chunk, &mut transforms)?;
    rows[0][0] = 42.;
    tick.write_transforms(chunk, rows)?;
    tick.log("must roll back", LogLevel::Warn)?;
    if cfg!(sdk_animation_capacity) {
        let _ignored = tick.animations(&mut []);
    }
    if cfg!(sdk_animation_number) {
        let _ignored = tick.control_animation(&row.entity, C::Speed(f64::NAN));
    }
    if cfg!(sdk_animation_rest) {
        let _ignored = tick.control_animation(
            &row.entity,
            C::Select {
                clip: None,
                seconds: 1.,
            },
        );
    }
    if cfg!(sdk_animation_clip) {
        tick.control_animation(
            &row.entity,
            C::Select {
                clip: Some(255),
                seconds: 0.,
            },
        )?;
    }
    if cfg!(sdk_animation_entity) {
        tick.control_animation(&EntityId::from_u128(999), C::Playing(true))?;
    }
    if cfg!(sdk_animation_callback) {
        return Err(incant_module::Error::Callback);
    }
    Ok(())
}

//! Actual native animation SDK sample. Commands commit atomically after update.
#![no_std]
use incant_module::{AnimationControl as C, Error, IDENTITY, LogLevel, Result, Tick};
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    core::arch::wasm32::unreachable()
}
incant_module::module!(simulate);
fn simulate(tick: &mut Tick<'_>, _: f64) -> Result<()> {
    let mut a = [0; 256];
    let mut b = [0; 256];
    let before = tick.animations(&mut a)?;
    if before.len() != 2 {
        return Err(Error::Malformed);
    }
    let row = before.iter().next().ok_or(Error::Malformed)?;
    let chunk = tick.chunk(0)?;
    let mut transforms = [IDENTITY; 16];
    let rows = tick.read_transforms(chunk, &mut transforms)?;
    let turn = rows[0][0] as u32;
    rows[0][0] += 1.;
    tick.write_transforms(chunk, rows)?;
    match turn {
        0 => {
            tick.control_animation(&row.entity, C::Playing(false))?;
            tick.control_animation(&row.entity, C::Seek(0.75))?;
        }
        1 => {
            tick.control_animation(&row.entity, C::Speed(-2.))?;
            tick.control_animation(&row.entity, C::Playing(true))?;
        }
        2 => {
            tick.control_animation(&row.entity, C::Looping(true))?;
            tick.control_animation(&row.entity, C::Seek(0.01))?;
        }
        3 => {
            tick.control_animation(
                &row.entity,
                C::Select {
                    clip: Some(1),
                    seconds: 0.,
                },
            )?;
        }
        4 => {
            tick.control_animation(
                &row.entity,
                C::Select {
                    clip: None,
                    seconds: 0.,
                },
            )?;
        }
        5 => {
            tick.control_animation(
                &row.entity,
                C::Select {
                    clip: Some(0),
                    seconds: 0.99,
                },
            )?;
            tick.control_animation(&row.entity, C::Speed(2.))?;
            tick.control_animation(&row.entity, C::Looping(false))?;
            tick.control_animation(&row.entity, C::Playing(true))?;
        }
        7 => {
            tick.control_animation(&row.entity, C::Seek(0.5))?;
            tick.control_animation(&row.entity, C::Playing(true))?;
        }
        _ => {}
    }
    // Queued controls cannot change any row in this callback's observation.
    let after = tick.animations(&mut b)?;
    if !before.iter().eq(after.iter()) {
        return Err(Error::Malformed);
    }
    tick.log("animation snapshot stable", LogLevel::Info)?;
    Ok(())
}

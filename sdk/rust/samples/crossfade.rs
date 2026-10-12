//! Both outgoing and destination clocks are owned observations of one update.
#![no_std]
use incant_module::{AnimationControl as C, EntityId, Error, IDENTITY, LogLevel, Result, Tick};
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
    let entity = EntityId([1; 16]);
    let row = before
        .iter()
        .find(|r| r.entity == entity)
        .ok_or(Error::Malformed)?;
    let chunk = tick.chunk(0)?;
    let mut transforms = [IDENTITY; 16];
    let rows = tick.read_transforms(chunk, &mut transforms)?;
    let turn = rows[0][0] as u32;
    rows[0][0] += 1.;
    rows[1][0] = row.transition.map_or(-1., |t| {
        t.seconds
            + t.elapsed
            + t.duration
            + t.speed
            + f64::from(u8::from(t.playing))
            + f64::from(u8::from(t.looping))
    }) + row.seconds;
    tick.write_transforms(chunk, rows)?;
    match turn {
        0 => {
            tick.control_animation(
                &entity,
                C::Crossfade {
                    clip: Some(0),
                    seconds: 0.75,
                    duration: 0.5,
                },
            )?;
            tick.control_animation(&entity, C::Speed(-1.))?;
            tick.control_animation(&entity, C::Looping(true))?;
        }
        1 => tick.control_animation(&entity, C::Playing(false))?,
        2 => tick.control_animation(&entity, C::Playing(true))?,
        35 => tick.control_animation(
            &entity,
            C::Crossfade {
                clip: None,
                seconds: 0.,
                duration: 0.,
            },
        )?,
        36 => {
            tick.control_animation(&entity, C::Playing(false))?;
            tick.control_animation(
                &entity,
                C::Crossfade {
                    clip: Some(0),
                    seconds: 0.5,
                    duration: 0.25,
                },
            )?;
        }
        37 => tick.control_animation(&entity, C::Playing(true))?,
        38 => tick.control_animation(&entity, C::Speed(2.))?,
        60 => tick.control_animation(
            &entity,
            C::Crossfade {
                clip: None,
                seconds: 0.,
                duration: 0.5,
            },
        )?,
        62 => tick.control_animation(&entity, C::Seek(0.))?,
        _ => {}
    }
    if !before.iter().eq(tick.animations(&mut b)?.iter()) {
        return Err(Error::Malformed);
    }
    tick.log("crossfade snapshot stable", LogLevel::Info)?;
    Ok(())
}

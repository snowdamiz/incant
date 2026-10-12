//! Caught SDK refusals and native retarget validation discard mixed work.
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
    tick.control_animation(
        &row.entity,
        C::Crossfade {
            clip: Some(0),
            seconds: 0.75,
            duration: 0.5,
        },
    )?;
    tick.spawn(&EntityId::from_u128(77), None, &IDENTITY, None)?;
    let chunk = tick.chunk(0)?;
    let mut transforms = [IDENTITY; 16];
    let rows = tick.read_transforms(chunk, &mut transforms)?;
    rows[0][0] = 42.;
    tick.write_transforms(chunk, rows)?;
    tick.log("must roll back", LogLevel::Warn)?;
    if cfg!(sdk_crossfade_capacity) {
        let _ignored = tick.animations(&mut []);
    }
    if cfg!(sdk_crossfade_number) {
        let _ignored = tick.control_animation(
            &row.entity,
            C::Crossfade {
                clip: Some(0),
                seconds: 0.,
                duration: f64::NAN,
            },
        );
    }
    if cfg!(sdk_crossfade_duration) {
        let _ignored = tick.control_animation(
            &row.entity,
            C::Crossfade {
                clip: Some(0),
                seconds: 0.,
                duration: 60.001,
            },
        );
    }
    if cfg!(sdk_crossfade_rest) {
        let _ignored = tick.control_animation(
            &row.entity,
            C::Crossfade {
                clip: None,
                seconds: 1.,
                duration: 0.,
            },
        );
    }
    if cfg!(sdk_crossfade_retarget) {
        tick.control_animation(
            &row.entity,
            C::Crossfade {
                clip: Some(0),
                seconds: 0.,
                duration: 1.,
            },
        )?;
    }
    if cfg!(sdk_crossfade_clip) {
        tick.control_animation(
            &row.entity,
            C::Crossfade {
                clip: Some(255),
                seconds: 0.,
                duration: 0.,
            },
        )?;
    }
    if cfg!(sdk_crossfade_quota) {
        for _ in 0..1024 {
            tick.control_animation(&row.entity, C::Playing(true))?;
        }
    }
    if cfg!(sdk_crossfade_callback) {
        return Err(incant_module::Error::Callback);
    }
    if cfg!(sdk_crossfade_trap) {
        core::arch::wasm32::unreachable();
    }
    Ok(())
}

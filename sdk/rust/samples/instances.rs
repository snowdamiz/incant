//! Two authored owners use immutable parameters; unrelated rows stay unchanged.
#![no_std]
use incant_module::{EntityId, Error, IDENTITY, InitialValue, Result, Tick};
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    core::arch::wasm32::unreachable()
}
incant_module::persistent_state!(schema = [42; 32], bytes = 8);
incant_module::module!(simulate);
fn simulate(t: &mut Tick<'_>, _: f64) -> Result<()> {
    let mut props = [0; 4096];
    let instances = t.instances(&mut props)?;
    let mut timer = [0; incant_module::TIMER_MAX_BYTES];
    let tick = t.timers(&mut timer)?.tick();
    let mut ids = [EntityId::from_u128(0); 32];
    let mut transforms = [IDENTITY; 32];
    for index in 0..t.chunk_count()? {
        let chunk = t.chunk(index)?;
        let owners = t.read_ids(chunk, &mut ids)?;
        let values = t.read_transforms(chunk, &mut transforms)?;
        for (id, value) in owners.iter().zip(values.iter_mut()) {
            if let Some(owner) = instances.iter().find(|owner| owner.entity == id.0) {
                let Some(InitialValue::Number(speed)) = owner.props.get("speed") else {
                    return Err(Error::Malformed);
                };
                value[0] += speed;
            }
        }
        t.write_transforms(chunk, values)?;
    }
    #[cfg(instance_retire)]
    {
        if tick == 1 {
            t.despawn(&EntityId::from_u128(1))?;
        }
        if tick == 2 {
            t.spawn(&EntityId::from_u128(1), None, &IDENTITY, None)?;
        }
    }
    #[cfg(instance_scratch)]
    {
        let _ = t.instances(&mut [0; 1]);
    }
    #[cfg(instance_callback)]
    {
        return Err(Error::Callback);
    }
    #[cfg(instance_trap)]
    {
        core::arch::wasm32::unreachable();
    }
    #[cfg(instance_stale)]
    if tick == 2 {
        #[link(wasm_import_module = "incant_instances_v1")]
        unsafe extern "C" {
            fn snapshot_len(epoch: i64) -> i32;
        }
        // Deliberately old first-callback capability, never a valid SDK pattern.
        let _ = unsafe { snapshot_len(1) };
    }
    let _ = tick;
    t.write_persistent(0, &tick.to_le_bytes())?;
    Ok(())
}

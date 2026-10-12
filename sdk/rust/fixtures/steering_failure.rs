#![no_std]
#![no_main]
use incant_module::{EntityId, Error, LogLevel, Result, Tick, steering::*};
fn simulate(tick: &mut Tick<'_>, _: f64) -> Result<()> {
    let chunk=tick.chunk(0)?;let mut values=[[0.;10];1];tick.read_transforms(chunk,&mut values)?;
    values[0][0]=99.;tick.write_transforms(chunk,&values)?;
    tick.log("must roll back", LogLevel::Info)?;tick.despawn(&EntityId([1;16]))?;
    let agent=SteeringAgent {id:[1;16],position:[0.;3],velocity:[0.;2],preferred_velocity:[1.,0.],radius:0.3,height:1.,max_speed:1.,responsibility:1.};
    let mut scratch=[0;STEERING_MAX_REQUEST_BYTES];let mut output=[0;STEERING_MAX_RESULT_BYTES];
    #[allow(unused_mut)] let mut agents=[agent];
    #[cfg(sdk_steering_number)] {agents[0].radius=f64::NAN;}
    #[cfg(sdk_steering_rounding)] {agents[0].radius=0.0499999999;}
    #[cfg(sdk_steering_domain)] {agents[0].radius=-1.;}
    #[cfg(sdk_steering_duplicate)] let agents=[agent;2];
    #[cfg(sdk_steering_count)] let agents=[agent;129];
    let request=Request {agents:&agents,obstacles:&[],time_horizon:2.,obstacle_time_horizon:2.,margin:0.05};
    #[cfg(sdk_steering_capacity)] let output=&mut output[..0];
    #[cfg(not(sdk_steering_capacity))] let output=&mut output[..];
    #[cfg(sdk_steering_quota)] {
        for _ in 0..33 {let _=tick.steer_agents(Request {agents:&[],..request},&mut scratch,output);}
    }
    #[cfg(not(sdk_steering_quota))] let _=tick.steer_agents(request,&mut scratch,output);
    // Ignored SDK errors must still poison this callback and all its prefixes.
    #[cfg(sdk_steering_callback)] return Err(Error::Callback);
    #[cfg(not(sdk_steering_callback))] {let _=Error::Callback;Ok(())}
}
incant_module::module!(simulate);
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>)->! {core::arch::wasm32::unreachable()}

//! Copied bulk proposals are applied through the same numeric-column lease.
#![no_std]
#![no_main]
use incant_module::{EntityId, Error, LogLevel, Result, Tick, steering::*};
fn simulate(tick: &mut Tick<'_>, _: f64) -> Result<()> {
    let agents = [
        SteeringAgent { id: EntityId::from_u128(9).0, position: [-1.,0.,0.], velocity: [1.,0.], preferred_velocity:[1.,0.],radius:0.3,height:1.8,max_speed:1.,responsibility:1. },
        SteeringAgent { id: EntityId::from_u128(2).0, position: [1.,0.,0.], velocity: [-1.,0.], preferred_velocity:[-1.,0.],radius:0.3,height:1.8,max_speed:1.,responsibility:1. },
    ];
    let obstacles = [Obstacle {id:EntityId::from_u128(7).0,vertices:&[[0.,-2.],[3.,-2.],[3.,-1.],[0.,-1.]],closed:true,min_y:0.,max_y:2.}];
    let mut request = [0; STEERING_MAX_REQUEST_BYTES];
    let mut result = [0; STEERING_MAX_RESULT_BYTES];
    let proposals = tick.steer_agents(Request { agents:&agents,obstacles:&obstacles,time_horizon:2.,obstacle_time_horizon:2.,margin:0.05 },&mut request,&mut result)?;
    let chunk=tick.chunk(0)?;
    let mut transforms=[[0.;10];1];
    tick.read_transforms(chunk,&mut transforms)?;
    let mut rows = proposals.iter();
    let first = rows.next().ok_or(Error::Malformed)?;
    let second = rows.next().ok_or(Error::Malformed)?;
    if first.id != EntityId::from_u128(2).0 || second.id != EntityId::from_u128(9).0 || first.neighbors != 1 || second.neighbors != 1 { return Err(Error::Malformed); }
    transforms[0][..3].copy_from_slice(&[first.velocity[0], first.velocity[1], second.velocity[0]]);
    tick.write_transforms(chunk,&transforms)?;
    tick.log("steering accepted", LogLevel::Info)?;
    Ok(())
}
incant_module::module!(simulate);
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>)->! {core::arch::wasm32::unreachable()}

//! Mixed transaction prefixes must disappear after caught and native failures.
#![no_std]
use incant_module::{navigation::*,EntityId,IDENTITY,LogLevel,Result,Tick};
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>)->!{core::arch::wasm32::unreachable()}
incant_module::module!(simulate);
fn simulate(tick:&mut Tick<'_>,_:f64)->Result<()> {
    let owner=EntityId::from_u128(1);let q=PathRequest{start:[-8.,0.,0.],end:[8.,0.,0.],snap_distance:1.,max_visited:1000};
    let g=GridPathRequest{start:[0,0],end:[2,0],diagonal:true,max_expansions:100};let mut bytes=[0;8192];
    tick.find_path(&owner,q,&mut bytes)?;
    let chunk=tick.chunk(0)?;let mut transforms=[IDENTITY;16];let rows=tick.read_transforms(chunk,&mut transforms)?;rows[0][0]=40.;tick.write_transforms(chunk,rows)?;
    tick.spawn(&EntityId::from_u128(77),None,&IDENTITY,None)?;tick.log("must roll back",LogLevel::Warn)?;
    if cfg!(nav_capacity){let _=tick.find_path(&owner,q,&mut []);}
    if cfg!(nav_grid_capacity){let _=tick.find_grid_path(&owner,g,&mut []);}
    if cfg!(nav_number){let _=tick.find_path(&owner,PathRequest{start:[f64::NAN,0.,0.],..q},&mut bytes);}
    if cfg!(nav_rounding){let _=tick.find_path(&owner,PathRequest{snap_distance:10.000_000_001,..q},&mut bytes);}
    if cfg!(nav_coordinate){let _=tick.find_path(&owner,PathRequest{start:[100_000.000_001,0.,0.],..q},&mut bytes);}
    if cfg!(nav_owner){tick.find_path(&EntityId::from_u128(4),q,&mut bytes)?;}
    if cfg!(nav_grid_owner){tick.find_grid_path(&EntityId::from_u128(4),g,&mut bytes)?;}
    if cfg!(nav_endpoint){tick.find_grid_path(&owner,GridPathRequest{end:[3,0],..g},&mut bytes)?;}
    if cfg!(nav_exhaustion){tick.find_grid_path(&owner,GridPathRequest{max_expansions:1,..g},&mut bytes)?;}
    if cfg!(nav_work){tick.find_grid_path(&owner,GridPathRequest{start:[0,0],end:[0,0],max_expansions:65536,..g},&mut bytes)?;}
    if cfg!(nav_quota){for i in 0..32 {if i%2==0 {
        let mut input=[0;64];let mut output=[0;64];tick.steer_agents(incant_module::steering::Request{agents:&[],obstacles:&[],time_horizon:2.,obstacle_time_horizon:2.,margin:0.},&mut input,&mut output)?;
    } else {tick.find_grid_path(&owner,GridPathRequest{start:[0,0],end:[0,0],max_expansions:1,..g},&mut bytes)?;}}}
    if cfg!(nav_late){tick.set_velocity(&EntityId::from_u128(99),Some(&[1.,0.,0.]))?;}
    if cfg!(nav_callback){return Err(incant_module::Error::Callback);}
    if cfg!(nav_trap){core::arch::wasm32::unreachable();}
    Ok(())
}

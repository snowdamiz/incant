//! Ignored SDK errors and native failures must discard every staged write.
#![no_std]
use incant_module::{physics::*,EntityId,IDENTITY,LogLevel,Result,Tick};
#[panic_handler] fn panic(_: &core::panic::PanicInfo<'_>)->! {core::arch::wasm32::unreachable()}
incant_module::module!(simulate);
fn simulate(tick:&mut Tick<'_>,_:f64)->Result<()> {
    let ray=Ray3 {origin:[-8.,1.,0.],direction:[2.,0.,0.],max_distance:20.,include_sensors:false,exclude_entity:Some(EntityId::from_u128(11)),memberships:u32::MAX,filter:u32::MAX};
    let chara=Character {entity:EntityId::from_u128(11),translation:[5.,-1.,0.],options:CharacterOptions::default()};
    let mut request=[0;PHYSICS_MAX_REQUEST_BYTES];let mut result=[0;PHYSICS_MAX_RESULT_BYTES];
    tick.query_physics(&[Query::Ray3(ray)],&mut request,&mut result)?;
    let chunk=tick.chunk(0)?;let mut transforms=[IDENTITY;16];let rows=tick.read_transforms(chunk,&mut transforms)?;rows[0][0]=40.;tick.write_transforms(chunk,rows)?;tick.spawn(&EntityId::from_u128(77),None,&IDENTITY,None)?;tick.log("must roll back",LogLevel::Warn)?;
    if cfg!(physics_capacity){let _=tick.query_physics(&[Query::Ray3(ray)],&mut request,&mut []);}
    if cfg!(physics_request_capacity){let _=tick.query_physics(&[Query::Ray3(ray)],&mut [],&mut result);}
    if cfg!(physics_events_capacity){let _=tick.physics_events(Dimension::Spatial,&mut []);}
    if cfg!(physics_nan){let _=tick.query_physics(&[Query::Ray3(Ray3 {origin:[f64::NAN,0.,0.],..ray})],&mut request,&mut result);}
    if cfg!(physics_rounding){let _=tick.query_physics(&[Query::Ray3(Ray3 {origin:[1_000_000.000_001,0.,0.],..ray})],&mut request,&mut result);}
    if cfg!(physics_character_rounding){let _=tick.query_physics(&[Query::Character(Character {options:CharacterOptions {offset:0.000_999_999_999,..chara.options},..chara})],&mut request,&mut result);}
    if cfg!(physics_owner){tick.query_physics(&[Query::Ray3(ray),Query::Character(Character {entity:EntityId::from_u128(12),..chara})],&mut request,&mut result)?;}
    if cfg!(physics_velocity){tick.query_physics(&[Query::Character(Character {translation:[9999.,0.,0.],..chara})],&mut request,&mut result)?;}
    if cfg!(physics_units){for _ in 0..16 {tick.query_physics(&[Query::Ray3(ray);64],&mut request,&mut result)?;}}
    if cfg!(physics_character_units){for _ in 0..129 {tick.query_physics(&[Query::Character(chara)],&mut request,&mut result)?;}}
    if cfg!(physics_late){tick.set_velocity(&EntityId::from_u128(123),Some(&[1.,0.,0.]))?;}
    if cfg!(physics_callback){return Err(incant_module::Error::Callback);}
    if cfg!(physics_trap){core::arch::wasm32::unreachable();}
    Ok(())
}

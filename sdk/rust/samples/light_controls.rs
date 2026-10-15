//! Native/Wasm parity fixture: phase lives in committed components, not globals.
#![no_std]
use incant_module::{EntityId,LightCommand,LightControl as L,Result,Tick,IDENTITY};
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>)->! {core::arch::wasm32::unreachable()}
incant_module::module!(simulate);
fn simulate(t:&mut Tick<'_>,_:f64)->Result<()> {
 let chunk=t.chunk(0)?;let mut storage=[IDENTITY;8];let rows=t.read_transforms(chunk,&mut storage)?;rows[0][0]+=1.;let phase=rows[0][0];t.write_transforms(chunk,rows)?;
 let command=|n,control|LightCommand {entity:EntityId::from_u128(n),control};
 let commands=[command(1,L::Intensity(phase*100.)),command(2,L::Color([0.25,0.5,0.75])),command(3,L::Intensity(if phase%3.==0. {0.}else{10.}))];
 t.light_commands(&commands,&mut [0;256])?;
 if cfg!(light_scratch) {let _=t.light_commands(&commands,&mut []);}
 if cfg!(light_range) {let _=t.light_commands(&[command(1,L::Intensity(f64::NAN))],&mut [0;64]);}
 if cfg!(light_owner) {t.light_commands(&[command(999,L::Intensity(1.))],&mut [0;64])?;}
 Ok(())
}

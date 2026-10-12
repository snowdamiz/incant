use super::*;
#[allow(dead_code)]
#[path = "../../incant_runtime/tests/support/hud.rs"]
mod fixture;
#[test]
fn hud_materialization_defines_own_properties_without_guest_setters() {
    let rt = Runtime::new().unwrap();
    let context = Context::full(&rt).unwrap();
    let world = NativeWorld::new(fixture::scene());
    context.with(|ctx| {
        ctx.eval::<(),_>("Object.defineProperty(Array.prototype,'0',{set(){throw Error('array setter')}});Object.defineProperty(Object.prototype,'text',{set(){throw Error('object setter')}});").unwrap();
        for _ in 0..2 {let snapshot=hud::expose(&ctx,&world).unwrap();ctx.globals().set("hud",snapshot).unwrap();ctx.eval::<(),_>("if(hud.widgets[1].text!=='Health 10'||hud.texts[0].text!=='World')throw Error('bad snapshot');").unwrap();}
    });
}

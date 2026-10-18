use super::*;
#[path = "../../incant_runtime/tests/support/localization.rs"]
pub(super) mod fixture;
use fixture::*;
use incant_runtime::{FrameCommands, StructuralCommand};
fn host(body: &str) -> ScriptHost {
    let id = ulid::Ulid::from_bytes([1; 16]).to_string();
    ScriptHost::with_budget(CompiledScript::compile(&format!("exports.default={{update({{chunks,commands,localization,physics}}){{const table='{TABLE}',entity='{id}';{body}}}}};")).unwrap(),Duration::from_secs(3)).unwrap()
}
fn set(world: &mut NativeWorld, locale: &str) {
    let mut c = FrameCommands::default();
    c.push(StructuralCommand::SetLocale(settings(locale)))
        .unwrap();
    world.apply(&mut c).unwrap();
}
#[test]
fn localized_patterns_icu_formats_and_owned_missing_reports_drive_native_motion() {
    let mut world = NativeWorld::new(scene());
    for (locale, key, args, expected) in [
        (
            "en",
            "party",
            r#"{gender:'female',n:3}"#,
            "She and 2 guests",
        ),
        ("ru", "items", "{n:22}", "22 предмета"),
        ("en", "rank", "{n:22}", "22nd"),
        ("fr-CA", "hello", "{name:'Ada'}", "Bonjour Ada"),
        ("zh-Hant-HK", "hello", "{name:'Ada'}", "你好 Ada"),
    ] {
        set(&mut world, locale);
        let date = incant_localization::format_date(
            locale,
            &incant_localization::CalendarDate {
                year: 2024,
                month: 1,
                day: 2,
            },
            incant_localization::DateLength::Medium,
        )
        .unwrap();
        let number = incant_localization::format_number(locale, 1234.5).unwrap();
        host(&format!(r#"
            const a=localization.localize({{table_id:table,key:'{key}',arguments:{args}}});
            if(a.text!=={expected:?})throw Error('pattern');
            if(localization.formatNumber(1234.5)!=={number:?}||localization.formatDate({{year:2024,month:1,day:2}})!=={date:?})throw Error('ICU');
            if(localization.localize({{table_id:table,key:'absent'}}).missing.kind!=='key')throw Error('missing key');
            const report=localization.report({{limit:1}});if(report.entries.length>1||report.total>4)throw Error('page');
            a.text='guest';if(localization.localize({{table_id:table,key:'{key}',arguments:{args}}}).text!=={expected:?})throw Error('alias');
            for(const c of chunks)c.transforms[0]+=1;
        "#)).tick(&mut world,1./60.).unwrap();
    }
    assert_eq!(world.snapshot()[0].transform.translation[0], 5.);
}
#[test]
fn locale_changes_are_next_tick_only_pseudo_preserves_arguments_and_retained_capabilities_expire() {
    let mut world = NativeWorld::new(scene());
    let mut h = host(
        r#"
        if(!globalThis.saved){
            globalThis.saved=localization.localize;globalThis.command=commands.setLocale;
            commands.setLocale({locale:'de',fallbacks:['fr'],pseudo:true});
            if(localization.locale().locale!=='en')throw Error('early locale');
        }else{
            for(const f of [saved,command]){let refused=false;try{f({locale:'en'})}catch{refused=true}if(!refused)throw Error('retained');}
            const s=localization.locale();if(s.locale!=='de'||!s.pseudo||s.fallbacks[0]!=='fr')throw Error('settings');
            const x=localization.localize({table_id:table,key:'hello',arguments:{name:'Ada'}});
            if(!x.text.startsWith('[!! ')||!x.text.includes('Ada')||x.text.includes('Bonjour')||x.resolved_locale!=='fr')throw Error('pseudo');
            s.fallbacks[0]='ru';if(localization.locale().fallbacks[0]!=='fr')throw Error('alias');
        }
    "#,
    );
    h.tick(&mut world, 1. / 60.).unwrap();
    h.tick(&mut world, 1. / 60.).unwrap();
    tests::check(
        &h,
        "let refused=false;try{saved({})}catch{refused=true}if(!refused)throw Error('escaped');",
    );
}
#[test]
fn caught_argument_and_native_errors_poison_the_entire_numeric_locale_and_structural_batch() {
    for bad in [
        "localization.localize({table_id:table,key:'hello',arguments:{name:true}})",
        "localization.localize({table_id:'bad',key:'hello'})",
        "localization.localize({table_id:table,key:'hello',arguments:{name:'\\ud800'}})",
        "localization.localize({table_id:table,key:'items',arguments:{n:NaN}})",
        "localization.localize({table_id:table,key:'hello',get arguments(){throw Error('getter')}})",
        "localization.formatNumber(Infinity)",
        "localization.formatDate({year:2023,month:2,day:29})",
        "localization.formatDate({year:2024.5,month:1,day:2})",
        "localization.report({limit:129})",
        "commands.setLocale({locale:'en-us'})",
        "commands.setLocale({locale:'fr',fallbacks:['en','en']})",
    ] {
        let mut world = NativeWorld::new(scene());
        let before = world.save_bytes().unwrap();
        let mut h = host(&format!(
            "for(const c of chunks)c.transforms[0]=99;commands.setLocale({{locale:'fr'}});commands.setVelocity(entity,[2,0,0]);try{{{bad}}}catch{{}}"
        ));
        assert!(h.tick(&mut world, 1. / 60.).is_err(), "{bad}");
        assert_eq!(world.save_bytes().unwrap(), before, "{bad}");
    }
    for bad in [
        "commands.setVelocity(entity,[NaN,0,0]);",
        "commands.setAudioGain(entity,0);",
    ] {
        let mut world = NativeWorld::new(scene());
        let before = world.save_bytes().unwrap();
        assert!(
            host(&format!(
                "for(const c of chunks)c.transforms[0]=99;commands.setLocale({{locale:'fr'}});{bad}"
            ))
            .tick(&mut world, 1. / 60.)
            .is_err()
        );
        assert_eq!(world.save_bytes().unwrap(), before);
    }
}
#[test]
fn whole_tick_query_input_output_and_locale_command_budgets_cannot_be_caught_away() {
    for excess in [
        "for(let i=0;i<65;i++)try{localization.locale()}catch{}",
        "for(let i=0;i<1000;i++)physics.triggerEvents();for(let i=0;i<7;i++)try{localization.locale()}catch{}",
        "const name='a'.repeat(40000);for(let i=0;i<2;i++)try{localization.localize({table_id:table,key:'hello',arguments:{name}})}catch{}",
        "for(let i=0;i<65;i++)try{commands.setLocale({locale:'fr'})}catch{}",
    ] {
        let mut world = NativeWorld::new(scene());
        let before = world.save_bytes().unwrap();
        assert!(host(&format!("for(const c of chunks)c.transforms[0]=99;commands.setLocale({{locale:'fr'}});{excess}")).tick(&mut world,1./60.).is_err(),"{excess}");
        assert_eq!(world.save_bytes().unwrap(), before);
    }
    // Amplified pattern output consumes the aggregate result budget independently
    // of input bytes: a 7 KiB literal repeats across 38 otherwise valid queries.
    let mut data = tables();
    data.get_mut(TABLE)
        .unwrap()
        .messages
        .get_mut("hello")
        .unwrap()
        .insert("en".into(), "a".repeat(7000));
    // Keep translations compatible after changing the source argument contract.
    data.get_mut(TABLE)
        .unwrap()
        .messages
        .get_mut("hello")
        .unwrap()
        .retain(|k, _| k == "en");
    let cooked = scene()
        .with_localization(incant_runtime::NativeLocalization::new(data, settings("en")).unwrap())
        .unwrap();
    let mut world = NativeWorld::new(cooked);
    let before = world.save_bytes().unwrap();
    assert!(host("for(const c of chunks)c.transforms[0]=99;for(let i=0;i<38;i++)try{localization.localize({table_id:table,key:'hello'})}catch{}").tick(&mut world,1./60.).is_err());
    assert_eq!(world.save_bytes().unwrap(), before);
}
#[test]
fn recursive_getters_share_budget_and_guest_setters_cannot_intercept_owned_results() {
    let mut world = NativeWorld::new(scene());
    set(&mut world, "fr");
    host(r#"
        const r={table_id:table,key:'hello',get arguments(){if(localization.locale().locale!=='fr')throw Error('nested');return {name:'Ada'}}};
        Object.defineProperty(Object.prototype,'text',{set(){throw Error('setter')},configurable:true});
        Object.defineProperty(Array.prototype,'0',{set(){throw Error('setter')},configurable:true});
        const result=localization.localize(r),report=localization.report();
        delete Object.prototype.text;delete Array.prototype[0];
        if(result.text!=='Bonjour Ada'||report.total!==3||report.entries.length!==3)throw Error('value');
    "#).tick(&mut world,1./60.).unwrap();
    let before = world.save_bytes().unwrap();
    assert!(host("for(const c of chunks)c.transforms[0]=99;try{localization.localize({table_id:table,key:'hello',get arguments(){for(let i=0;i<64;i++)try{localization.locale()}catch{};return {name:'Ada'}}})}catch{}").tick(&mut world,1./60.).is_err());
    assert_eq!(world.save_bytes().unwrap(), before);
}

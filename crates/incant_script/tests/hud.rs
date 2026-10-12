#![cfg(feature = "compiler")]
#[path = "../../incant_runtime/tests/support/hud.rs"]
mod fixture;
use fixture::*;
use incant_runtime::*;
use incant_script::{CompiledScript, ScriptError, ScriptHost};
use std::time::Duration;
fn host(body: &str) -> ScriptHost {
    ScriptHost::with_budget(CompiledScript::compile(&format!("exports.default={{update({{chunks,commands,hud,input}}){{const root='00000000000000000000000001',label='00000000000000000000000002',field='00000000000000000000000003',text='00000000000000000000000004';{body}}}}};")).unwrap(),Duration::from_secs(2)).unwrap()
}
#[test]
fn owned_precommand_readback_changes_only_after_acceptance_and_old_functions_expire() {
    let mut w = NativeWorld::new(scene());
    let mut h = host(
        "if(globalThis.old){let refused=false;try{old(label,'old')}catch{refused=true}if(!refused)throw Error('expired');if(hud.widgets[1].text!=='Health 9'||hud.texts[0].text!=='世界')throw Error('committed');if(saved.widgets[1].text!=='Health 10')throw Error('retained snapshot')}else{if(hud.widgets[1].text!=='Health 10')throw Error('initial');globalThis.saved=hud;}globalThis.old=commands.setUiText;const before=hud.widgets[1].text;commands.setUiText(label,'Health 9');commands.setUiDisabled(label,true);commands.setText(text,'世界');if(hud.widgets[1].text!==before)throw Error('precommand');",
    );
    h.tick(&mut w, 1. / 60.).unwrap();
    h.tick(&mut w, 1. / 60.).unwrap();
    assert_eq!(
        declarations(&w).0[1].1.kind.text().unwrap().content,
        "Health 9"
    );
    assert_eq!(declarations(&w).1[0].1.content.as_ref(), "世界");
}
#[test]
fn caught_malformed_and_late_failures_roll_back_numeric_and_all_hud_controls() {
    for bad in [
        "commands.setUiText(label,42)",
        "commands.setUiDisabled(label,0)",
        "commands.setText(text,'x',0)",
        "commands.setUiText('bad','x')",
        "commands.setUiText(label,'\\ud800')",
        "commands.setUiText(label,'x'.repeat(16385))",
        "commands.setUiText(root,'bad')",
        "commands.setText(label,'bad')",
        "commands.despawn(label);commands.setUiText(label,'bad')",
        "throw Error('trap')",
        "for(let i=0;i<257;i++){try{commands.setUiDisabled(label,false)}catch{}}",
        "for(let i=0;i<65;i++){try{commands.setUiText(label,'x'.repeat(16384))}catch{}}",
    ] {
        let mut w = NativeWorld::new(scene());
        let before = declarations(&w);
        let snapshot = w.snapshot();
        let mut h = host(&format!(
            "for(const c of chunks)for(let i=0;i<c.ids.length;i++)if(c.ids[i]==='00000000000000000000000005')c.transforms[i*c.transformStride]=40;commands.setUiText(label,'prefix');commands.setText(text,'prefix');try{{{bad}}}catch{{}}"
        ));
        // An arbitrary caught guest throw is not an API failure; make that case uncaught.
        if bad == "throw Error('trap')" {
            h = host(
                "for(const c of chunks)for(let i=0;i<c.ids.length;i++)if(c.ids[i]==='00000000000000000000000005')c.transforms[i*c.transformStride]=40;commands.setUiText(label,'prefix');throw Error('trap')",
            );
        }
        assert!(h.tick(&mut w, 1. / 60.).is_err(), "{bad}");
        assert_eq!(declarations(&w), before, "{bad}");
        assert_eq!(w.snapshot(), snapshot, "{bad}");
        assert!(matches!(
            h.tick(&mut w, 1. / 60.),
            Err(ScriptError::FailedSession)
        ));
    }
}
#[test]
fn declaration_properties_and_lengths_are_readonly() {
    let mut w = NativeWorld::new(scene());
    let mut h = host(
        "let n=0;try{hud.widgets[1].text='bad'}catch{n++}try{hud.widgets.length=0}catch{n++}try{hud.widgets.push(null)}catch{n++}try{hud.texts[0].text='bad'}catch{n++}if(n!==4||hud.widgets[1].text!=='Health 10')throw Error('readonly');",
    );
    h.tick(&mut w, 1. / 60.).unwrap();
    h.tick(&mut w, 1. / 60.).unwrap();
}

#[test]
fn localized_result_updates_literal_hud_with_precommand_locale_and_declaration_reads() {
    use std::collections::BTreeMap;
    let table = "00000000000000000000000050";
    let tables = BTreeMap::from([(
        table.into(),
        incant_localization::StringTable {
            id: table.into(),
            name: "Game".into(),
            source_locale: "en".into(),
            messages: BTreeMap::from([(
                "health".into(),
                BTreeMap::from([
                    ("en".into(), "Health {n}".into()),
                    ("fr".into(), "Santé {n}".into()),
                ]),
            )]),
        },
    )]);
    let scene = scene()
        .with_localization(
            NativeLocalization::new(
                tables,
                LocaleSettings {
                    locale: "en".into(),
                    ..Default::default()
                },
            )
            .unwrap(),
        )
        .unwrap();
    let mut world = NativeWorld::new(scene);
    let code = format!(
        "exports.default={{update({{commands,localization,hud}}){{const text=localization.localize({{table_id:'{table}',key:'health',arguments:{{n:9}}}}).text;if(globalThis.second&&hud.widgets[1].text!=='Health 9')throw Error('precommand');commands.setLocale({{locale:'fr'}});commands.setUiText('00000000000000000000000002',text);globalThis.second=true;}}}};"
    );
    let mut h = ScriptHost::with_budget(
        CompiledScript::compile(&code).unwrap(),
        Duration::from_secs(2),
    )
    .unwrap();
    h.tick(&mut world, 1. / 60.).unwrap();
    assert_eq!(
        declarations(&world).0[1].1.kind.text().unwrap().content,
        "Health 9"
    );
    h.tick(&mut world, 1. / 60.).unwrap();
    assert_eq!(
        declarations(&world).0[1].1.kind.text().unwrap().content,
        "Santé 9"
    );
}

use incant_cmd::{Actor, Command, CommandBus};
use incant_doc::{LocaleSettings, Project, new_id};
use incant_script::{PlaySession, SaveError, ScriptHost};
use serde_json::{Value, json};
fn project() -> (Project, String) {
    let mut p = Project::empty("Languages");
    let id = new_id();
    p.string_tables.insert(id.clone(), serde_json::from_value(json!({"id":id,"name":"UI","source_locale":"en",
        "messages":{"coins":{"en":"{n, plural, one {# coin} other {# coins}}", "ja":"{n}枚", "ar":"{n} عملات"},
        "only_source":{"en":"Source"}}})).unwrap());
    (p, id)
}
#[test]
fn switching_locale_is_tick_scoped_and_saved_without_changing_authoring() {
    let (p, id) = project();
    let before = p.canonical_text().unwrap();
    let source = format!(
        r#"exports.default={{initialState:{{text:[],locales:[],reports:[],numbers:[],dates:[]}},update(api,dt,s){{
      s.text.push(api.localize({{table_id:{id:?},key:'coins',arguments:{{n:2}}}}).text);
      const settings=api.locale();s.locales.push(settings.locale);settings.locale='broken';
      s.reports.push(api.localizationReport().length);
      s.numbers.push(api.formatNumber(1234.5));s.dates.push(api.formatDate({{year:2024,month:5,day:8}},'short'));
      if(api.clock().tick===1)api.command({{op:'set_locale',settings:{{locale:'ja'}}}});
      if(api.clock().tick===3)api.command({{op:'set_locale',settings:{{locale:'ar'}}}});
    }}}}"#
    );
    let mut original = PlaySession::new(&p, &source).unwrap();
    original.tick().unwrap();
    let save = original.save_text().unwrap();
    let mut restored = PlaySession::from_save(&p, &source, &save).unwrap();
    for _ in 0..4 {
        original.tick().unwrap();
        restored.tick().unwrap();
        assert_eq!(original.save_text().unwrap(), restored.save_text().unwrap());
    }
    assert_eq!(
        original.host.state()["text"],
        json!(["2 coins", "2枚", "2枚", "2 عملات", "2 عملات"])
    );
    assert_eq!(
        original.host.state()["locales"],
        json!(["en", "ja", "ja", "ar", "ar"])
    );
    assert_eq!(original.host.state()["reports"], json!([0, 1, 1, 1, 1]));
    assert_eq!(original.host.state()["numbers"][0], "1,234.5");
    assert_eq!(original.host.state()["dates"][0], "5/8/24");
    assert_eq!(p.canonical_text().unwrap(), before);
    let mut bad: Value = serde_json::from_str(&save).unwrap();
    bad["project"]["string_tables"][&id]["messages"]["coins"]["en"] = json!("Altered resource");
    assert!(matches!(
        PlaySession::from_save(&p, &source, &bad.to_string()),
        Err(SaveError::Manifest)
    ));
}
#[test]
fn catalog_rebuilds_after_commands_and_hot_reload_and_missing_keys_are_explicit() {
    let (p, id) = project();
    let mut bus = CommandBus::new(p).unwrap();
    let source = format!(
        r#"exports.default={{initialState:{{text:'',missing:null}},update(api,dt,s){{
      s.text=api.localize({{table_id:{id:?},key:'coins',arguments:{{n:1}}}}).text;
      s.missing=api.localize({{table_id:{id:?},key:'absent'}}).missing;
    }}}}"#
    );
    let mut host = ScriptHost::new(&source).unwrap();
    host.tick(&mut bus, 1. / 60.).unwrap();
    assert_eq!(host.state()["text"], "1 coin");
    assert_eq!(host.state()["missing"]["kind"], "key");
    let mut changed = bus.project().string_tables[&id].clone();
    changed
        .messages
        .get_mut("coins")
        .unwrap()
        .insert("en".into(), "New {n}".into());
    bus.execute(
        vec![Command::UpsertStringTable { table: changed }],
        Actor::user("test"),
        "Translate",
        None,
    )
    .unwrap();
    host.tick(&mut bus, 1. / 60.).unwrap();
    assert_eq!(host.state()["text"], "New 1");
    host.hot_reload(&source).unwrap();
    bus.execute(
        vec![Command::SetLocale {
            settings: LocaleSettings {
                pseudo: true,
                ..Default::default()
            },
        }],
        Actor::user("test"),
        "Preview",
        None,
    )
    .unwrap();
    host.tick(&mut bus, 1. / 60.).unwrap();
    assert!(
        host.state()["text"]
            .as_str()
            .unwrap()
            .starts_with("[!! Ñééŵ 1")
    );
}
#[test]
fn bad_queries_and_locale_changes_roll_back_commands_state_logs_and_clock() {
    let (p, id) = project();
    for body in [
        format!("api.localize({{table_id:{id:?},key:'coins'}})"),
        "api.formatNumber(Infinity)".into(),
        "api.formatDate({year:2024,month:2,day:30})".into(),
        "api.command({op:'set_locale',settings:{locale:'invalid_locale'}})".into(),
        "for(let i=0;i<65;i++)api.locale()".into(),
    ] {
        let mut bus = CommandBus::new(p.clone()).unwrap();
        let source = format!(
            "exports.default={{initialState:{{n:0}},update(api,dt,s){{s.n++;api.log('prefix');api.command({{op:'set_locale',settings:{{locale:'ja'}}}});{body}}}}}"
        );
        let mut host = ScriptHost::new(&source).unwrap();
        assert!(host.tick(&mut bus, 1. / 60.).is_err(), "{body}");
        assert_eq!(bus.project(), &p);
        assert_eq!(host.state(), &json!({"n":0}));
        assert!(host.take_logs().is_empty());
        assert_eq!(host.clock().tick, 0);
    }
}

#[test]
fn localization_and_physics_share_the_same_native_query_allowance() {
    let (mut p, _) = project();
    let scene = incant_doc::Scene::new("Queries");
    let sid = scene.id.clone();
    p.scenes.insert(sid.clone(), scene);
    let source = format!(
        r#"exports.default={{initialState:{{limited:false}},update(api,dt,s){{
        for(let i=0;i<63;i++)api.locale();
        const ray={{scene_id:{sid:?},origin:[0,1,0],direction:[0,-1,0],max_distance:10,include_sensors:false,exclude_entity:null,memberships:4294967295,filter:4294967295}};
        for(let i=0;i<4;i++)if(api.raycast(ray)!==null)throw Error('unexpected hit');
        try{{api.raycast(ray)}}catch(e){{s.limited=String(e).includes('query budget exceeded')}}
    }}}}"#
    );
    let mut play = PlaySession::new(&p, &source).unwrap();
    play.tick().unwrap();
    assert_eq!(play.host.state()["limited"], true);
}

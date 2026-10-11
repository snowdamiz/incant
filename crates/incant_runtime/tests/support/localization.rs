#![allow(dead_code)]
use incant_localization::{LocaleSettings, LocalizeRequest, StringTable};
use incant_runtime::{CookedEntity, CookedScene, NativeLocalization, StableId};
use std::collections::BTreeMap;
pub const TABLE: &str = "00000000000000000000000002";
pub fn tables() -> BTreeMap<String, StringTable> {
    let messages = [
        ("hello", vec![("en","Hello {name}"),("fr","Bonjour {name}"),("zh-Hant","你好 {name}")]),
        ("items", vec![("en","{n, plural, =0{None} one{# item} other{# items}}"),("ru","{n, plural, one{# предмет} few{# предмета} many{# предметов} other{# предмета}}")]),
        ("party", vec![("en","{gender, select, female{{n, plural, offset:1 =1{She came alone} one{She and # guest} other{She and # guests}}} other{They came}}")]),
        ("rank", vec![("en","{n, selectordinal, one{#st} two{#nd} few{#rd} other{#th}}")]),
    ].into_iter().map(|(k,values)|(k.into(),values.into_iter().map(|(a,b)|(a.into(),b.into())).collect())).collect();
    BTreeMap::from([(
        TABLE.into(),
        StringTable {
            id: TABLE.into(),
            name: "Game".into(),
            source_locale: "en".into(),
            messages,
        },
    )])
}
pub fn settings(locale: &str) -> LocaleSettings {
    LocaleSettings {
        locale: locale.into(),
        ..Default::default()
    }
}
pub fn request(key: &str) -> LocalizeRequest {
    LocalizeRequest {
        table_id: TABLE.into(),
        key: key.into(),
        arguments: Default::default(),
    }
}
pub fn scene() -> CookedScene {
    CookedScene::new(
        StableId([99; 16]),
        60,
        vec![CookedEntity {
            id: StableId([1; 16]),
            parent: None,
            transform: Default::default(),
            velocity: None,
        }],
    )
    .unwrap()
    .with_localization(NativeLocalization::new(tables(), settings("en")).unwrap())
    .unwrap()
}

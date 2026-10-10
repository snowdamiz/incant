//! Exercise compiled ICU data on desktop, WASM and mobile builds.
use incant_localization::{CalendarDate, Catalog, DateLength, LocaleSettings, LocalizeRequest};
use serde_json::{Value, json};

pub(super) fn check() -> Result<Value, String> {
    let id = "00000000000000000000000001";
    let tables = serde_json::from_value(json!({id:{"id":id,"name":"Smoke","source_locale":"en","messages":{
        "count":{"en":"{n, plural, one {# coin} other {# coins}}", "ru":"{n, plural, one {# монета} few {# монеты} many {# монет} other {# монеты}}"},
        "hello":{"en":"Hello {name}","ja":"こんにちは {name}","ar":"مرحبا {name}"}
    }}})).map_err(|e| e.to_string())?;
    let catalog = Catalog::compile(&tables).map_err(|e| e.to_string())?;
    let mut values = vec![];
    for (locale, key, args, expected) in [
        ("en", "count", json!({"n":1}), "1 coin"),
        ("ru", "count", json!({"n":2}), "2 монеты"),
        ("ru", "count", json!({"n":5}), "5 монет"),
        ("ja", "hello", json!({"name":"海"}), "こんにちは 海"),
        ("ar", "hello", json!({"name":"نور"}), "مرحبا نور"),
    ] {
        let result = catalog
            .localize(
                &LocaleSettings {
                    locale: locale.into(),
                    ..Default::default()
                },
                &LocalizeRequest {
                    table_id: id.into(),
                    key: key.into(),
                    arguments: serde_json::from_value(args).map_err(|e| e.to_string())?,
                },
            )
            .map_err(|e| e.to_string())?;
        if result.text != expected || result.missing.is_some() {
            return Err(format!("localization mismatch for {locale}/{key}"));
        }
        values.push(result.text);
    }
    let number = incant_localization::format_number("de", 1234.5).map_err(|e| e.to_string())?;
    let date = incant_localization::format_date(
        "en-US",
        &CalendarDate {
            year: 2024,
            month: 5,
            day: 8,
        },
        DateLength::Short,
    )
    .map_err(|e| e.to_string())?;
    if number != "1.234,5" || date != "5/8/24" {
        return Err("ICU number/date mismatch".into());
    }
    Ok(json!({"text_values":values,"number":number,"date":date,"shaping_verified":false}))
}

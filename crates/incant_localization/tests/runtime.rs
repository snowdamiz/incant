use incant_localization::*;
use std::collections::BTreeMap;
const ID: &str = "00000000000000000000000001";
fn table(entries: &[(&str, &[(&str, &str)])]) -> BTreeMap<String, StringTable> {
    BTreeMap::from([(
        ID.into(),
        StringTable {
            id: ID.into(),
            name: "Game".into(),
            source_locale: "en".into(),
            messages: entries
                .iter()
                .map(|(key, values)| {
                    (
                        key.to_string(),
                        values
                            .iter()
                            .map(|(locale, text)| (locale.to_string(), text.to_string()))
                            .collect(),
                    )
                })
                .collect(),
        },
    )])
}
fn request(key: &str, args: &[(&str, MessageArgument)]) -> LocalizeRequest {
    LocalizeRequest {
        table_id: ID.into(),
        key: key.into(),
        arguments: args.iter().cloned().map(|(k, v)| (k.into(), v)).collect(),
    }
}
fn settings(locale: &str) -> LocaleSettings {
    LocaleSettings {
        locale: locale.into(),
        ..Default::default()
    }
}
fn n(value: f64) -> MessageArgument {
    MessageArgument::Number(value)
}
fn s(value: &str) -> MessageArgument {
    MessageArgument::Text(value.into())
}
#[test]
fn plural_select_gender_offsets_and_ordinal_follow_locale_rules() {
    let data = table(&[
        (
            "items",
            &[
                ("en", "{n, plural, =0{None} one{# item} other{# items}}"),
                (
                    "ru",
                    "{n, plural, one{# предмет} few{# предмета} many{# предметов} other{# предмета}}",
                ),
                (
                    "ar",
                    "{n, plural, zero{zero} one{one} two{two} few{few} many{many} other{other}}",
                ),
            ],
        ),
        (
            "party",
            &[(
                "en",
                "{gender, select, female{{n, plural, offset:1 =1{She came alone} one{She and # guest} other{She and # guests}}} other{They came}}",
            )],
        ),
        (
            "rank",
            &[(
                "en",
                "{n, selectordinal, one{#st} two{#nd} few{#rd} other{#th}}",
            )],
        ),
    ]);
    let catalog = Catalog::compile(&data).unwrap();
    for (language, count, expected) in [
        ("en", 0., "None"),
        ("en", 1., "1 item"),
        ("en", 2., "2 items"),
        ("ru", 21., "21 предмет"),
        ("ru", 22., "22 предмета"),
        ("ru", 25., "25 предметов"),
        ("ar", 0., "zero"),
        ("ar", 2., "two"),
        ("ar", 3., "few"),
        ("ar", 11., "many"),
    ] {
        assert_eq!(
            catalog
                .localize(&settings(language), &request("items", &[("n", n(count))]))
                .unwrap()
                .text,
            expected
        );
    }
    assert_eq!(
        catalog
            .localize(
                &settings("en"),
                &request("party", &[("gender", s("female")), ("n", n(3.))])
            )
            .unwrap()
            .text,
        "She and 2 guests"
    );
    assert_eq!(
        catalog
            .localize(
                &settings("en"),
                &request("party", &[("gender", s("female")), ("n", n(1.))])
            )
            .unwrap()
            .text,
        "She came alone"
    );
    for (count, expected) in [
        (1., "1st"),
        (2., "2nd"),
        (3., "3rd"),
        (11., "11th"),
        (22., "22nd"),
    ] {
        assert_eq!(
            catalog
                .localize(&settings("en"), &request("rank", &[("n", n(count))]))
                .unwrap()
                .text,
            expected
        );
    }
}
#[test]
fn fallback_switching_missing_reports_and_empty_translations_are_explicit() {
    let data = table(&[
        (
            "hello",
            &[
                ("en", "Hello {name}"),
                ("fr", "Bonjour {name}"),
                ("zh-Hant", "你好 {name}"),
            ],
        ),
        ("blank", &[("en", "Empty"), ("fr", "")]),
    ]);
    let catalog = Catalog::compile(&data).unwrap();
    let r = request("hello", &[("name", s("Ada"))]);
    let french = catalog.localize(&settings("fr-CA"), &r).unwrap();
    assert_eq!(french.text, "Bonjour Ada");
    assert_eq!(french.resolved_locale.as_deref(), Some("fr"));
    assert_eq!(french.missing.unwrap().kind, MissingKind::Translation);
    assert_eq!(
        catalog.localize(&settings("zh-Hant-HK"), &r).unwrap().text,
        "你好 Ada"
    );
    let mut fallback = settings("de");
    fallback.fallbacks.push("fr".into());
    assert_eq!(catalog.localize(&fallback, &r).unwrap().text, "Bonjour Ada");
    assert_eq!(
        catalog.localize(&settings("de"), &r).unwrap().text,
        "Hello Ada"
    );
    assert_eq!(
        catalog
            .localize(&settings("fr"), &request("blank", &[]))
            .unwrap()
            .text,
        ""
    );
    assert!(catalog.missing_strings(&settings("fr")).unwrap().is_empty());
    assert_eq!(catalog.missing_strings(&settings("de")).unwrap().len(), 2);
    let missing = catalog
        .localize(&settings("en"), &request("unknown", &[]))
        .unwrap();
    assert_eq!(missing.missing.unwrap().kind, MissingKind::Key);
    assert!(missing.text.contains("unknown"));
}
#[test]
fn icu_numbers_and_calendar_dates_use_locale_data() {
    assert_eq!(format_number("de-DE", 12345.5).unwrap(), "12.345,5");
    assert_eq!(format_number("en-US", 12345.5).unwrap(), "12,345.5");
    assert_eq!(format_number("ar-u-nu-arab", 1234.).unwrap(), "١٬٢٣٤");
    let date = CalendarDate {
        year: 2024,
        month: 5,
        day: 8,
    };
    assert_eq!(
        format_date("en-US", &date, DateLength::Long).unwrap(),
        "May 8, 2024"
    );
    assert_eq!(
        format_date("en-u-ca-hebrew", &date, DateLength::Medium).unwrap(),
        "30 Nisan 5784"
    );
    assert!(
        format_date(
            "en",
            &CalendarDate {
                year: 2023,
                month: 2,
                day: 29
            },
            DateLength::Short
        )
        .is_err()
    );
}
#[test]
fn quotes_unicode_nested_select_and_pseudo_preserve_arguments() {
    let data = table(&[
        (
            "quoted",
            &[(
                "en",
                "This '{isn''t}' obvious; don't change '{name}' or {name}",
            )],
        ),
        (
            "count",
            &[("en", "{n, plural, other{{gender, select, other{# '#'}}}}")],
        ),
        ("pseudo", &[("en", "Hello {name}, {n, number}!")]),
    ]);
    let catalog = Catalog::compile(&data).unwrap();
    assert_eq!(
        catalog
            .localize(
                &settings("en"),
                &request("quoted", &[("name", s("日本語🙂"))])
            )
            .unwrap()
            .text,
        "This {isn't} obvious; don't change {name} or 日本語🙂"
    );
    assert_eq!(
        catalog
            .localize(
                &settings("en"),
                &request("count", &[("n", n(2.)), ("gender", s("any"))])
            )
            .unwrap()
            .text,
        "2 #"
    );
    let mut config = settings("en");
    config.pseudo = true;
    let result = catalog
        .localize(
            &config,
            &request("pseudo", &[("name", s("Ada")), ("n", n(1234.))]),
        )
        .unwrap();
    assert!(result.text.starts_with("[!! ") && result.text.ends_with(" !!]"));
    assert!(result.text.contains("Ada") && result.text.contains("1,234"));
    assert!(!result.text.contains("Hello"));
}
#[test]
fn malformed_patterns_limits_and_bad_values_fail_before_publication() {
    for pattern in [
        "{n,plural,one{One}}",
        "{n,choice,0#no}",
        "{n,number,currency}",
        "{d,date,full}",
        "{n,select,other{x} other{y}}",
        "{n,plural,=1{x} =1{y} other{z}}",
        "{n,plural,four{x} other{y}}",
        "{a",
        "x}",
    ] {
        assert!(
            Catalog::compile(&table(&[("bad", &[("en", pattern)])])).is_err(),
            "{pattern}"
        );
    }
    assert!(
        Catalog::compile(&table(&[(
            "big",
            &[("en", &"x".repeat(MAX_MESSAGE_BYTES + 1))]
        )]))
        .is_err()
    );
    let catalog = Catalog::compile(&table(&[("hello", &[("en", "Hi {name}")])])).unwrap();
    assert!(
        catalog
            .localize(&settings("en"), &request("hello", &[]))
            .is_err()
    );
    assert!(
        catalog
            .localize(&settings("en_US"), &request("hello", &[("name", s("A"))]))
            .is_err()
    );
    assert!(
        catalog
            .localize(
                &settings("en"),
                &request("hello", &[("name", s("A")), ("unused", n(f64::INFINITY))])
            )
            .is_err()
    );
    let mut invalid = table(&[("hello", &[("fr", "Bonjour")])]);
    assert!(Catalog::compile(&invalid).is_err());
    invalid.get_mut(ID).unwrap().source_locale = "fr".into();
    assert!(Catalog::compile(&invalid).is_ok());
    let expanded = Catalog::compile(&table(&[("big", &[("en", "{name}{name}")])])).unwrap();
    assert!(
        expanded
            .localize(
                &settings("en"),
                &request("big", &[("name", s(&"x".repeat(40000)))])
            )
            .is_err()
    );
}

#[test]
fn catalog_and_parser_bounds_reject_excess_work() {
    let mut nested = "leaf".to_string();
    for _ in 0..17 {
        nested = format!("{{n,select,other{{{nested}}}}}");
    }
    let branches = format!(
        "{{n,select,{}other{{fallback}}}}",
        (0..64).map(|i| format!("v{i}{{x}}")).collect::<String>()
    );
    for pattern in [nested, branches, "{n}x".repeat(513)] {
        assert!(Catalog::compile(&table(&[("bound", &[("en", &pattern)])])).is_err());
    }
    let base = table(&[("key", &[("en", "value")])]);
    let mut excess = base.clone();
    let entries = &mut excess.get_mut(ID).unwrap().messages;
    for i in 0..MAX_MESSAGES {
        entries.insert(format!("k{i}"), BTreeMap::from([("en".into(), "x".into())]));
    }
    assert!(Catalog::compile(&excess).is_err());
    let mut excess = base.clone();
    for i in 2..=65_u128 {
        let id = ulid::Ulid::from(i).to_string();
        let mut t = base[ID].clone();
        t.id = id.clone();
        excess.insert(id, t);
    }
    assert!(Catalog::compile(&excess).is_err());
    let mut excess = base.clone();
    for i in 0..32 {
        excess
            .get_mut(ID)
            .unwrap()
            .messages
            .get_mut("key")
            .unwrap()
            .insert(format!("en-x-t{i}"), "x".into());
    }
    assert!(Catalog::compile(&excess).is_err());
    let mut excess = base.clone();
    excess.get_mut(ID).unwrap().messages = (0..512)
        .map(|i| {
            (
                format!("k{i}"),
                BTreeMap::from([("en".into(), "x".repeat(MAX_MESSAGE_BYTES))]),
            )
        })
        .collect();
    assert!(Catalog::compile(&excess).is_err());
    let mut too_many = settings("en");
    too_many.fallbacks = vec!["en".into(); 17];
    assert!(too_many.validate().is_err());
    too_many.fallbacks = vec!["en".into(); 2];
    assert!(too_many.validate().is_err());
}

#[test]
fn translations_preserve_argument_contracts_across_every_branch() {
    for (source, translated) in [
        ("Hello {name}", "{extra}"),
        ("Hello {name}", "{name,number}"),
        ("{n,plural,other{#}}", "{n,select,other{text}}"),
        ("{date,date}", "{date,number}"),
        ("{n,number} {n,select,other{text}}", "text"),
        (
            "{choice,select,other{{n,number}}}",
            "{choice,select,other{{n,date}}}",
        ),
    ] {
        assert!(
            Catalog::compile(&table(&[(
                "invalid",
                &[("en", source), ("fr", translated)]
            )]))
            .is_err(),
            "{source} → {translated}"
        );
    }
    // A translation may omit an argument, or render a typed argument with plain substitution.
    assert!(
        Catalog::compile(&table(&[(
            "valid",
            &[
                ("en", "{n,plural,other{#}}"),
                ("ja", "{n}個"),
                ("fr", "Beaucoup")
            ]
        )]))
        .is_ok()
    );
}

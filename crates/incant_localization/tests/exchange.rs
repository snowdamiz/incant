use incant_localization::*;
use std::collections::BTreeMap;
const ID: &str = "00000000000000000000000001";
fn table() -> StringTable {
    StringTable {
        id: ID.into(),
        name: "UI & \"Game\"".into(),
        source_locale: "en".into(),
        messages: BTreeMap::from([
            (
                "hello".into(),
                BTreeMap::from([
                    ("en".into(), " Hello {name} & <world>!\r\n\t🙂 ".into()),
                    ("ja".into(), " こんにちは {name} & <世界>\r\n\t🙂 ".into()),
                ]),
            ),
            (
                "empty".into(),
                BTreeMap::from([("en".into(), "Empty".into()), ("ja".into(), "".into())]),
            ),
            (
                "missing".into(),
                BTreeMap::from([("en".into(), "Untranslated".into())]),
            ),
        ]),
    }
}
fn tables() -> BTreeMap<String, StringTable> {
    BTreeMap::from([(ID.into(), table())])
}
fn wrap(body: &str) -> String {
    format!(
        "<xliff xmlns=\"urn:oasis:names:tc:xliff:document:2.0\" version=\"2.1\" srcLang=\"en\" trgLang=\"ja\"><file id=\"{ID}\">{body}</file></xliff>"
    )
}
#[test]
fn exported_patterns_roundtrip_whitespace_unicode_empty_and_missing_targets() {
    let data = tables();
    let xml = export_xliff(&data[ID], "ja").unwrap();
    assert!(
        xml.contains("&amp;") && xml.contains("&#13;") && xml.contains("xml:space=\"preserve\"")
    );
    let updates = translations_from_xliff(&data, &xml).unwrap();
    assert_eq!(updates.len(), 2);
    for update in updates {
        assert_eq!(update.value, data[ID].messages[&update.key]["ja"]);
    }
    assert!(
        translations_from_xliff(&data, &export_xliff(&data[ID], "fr").unwrap())
            .unwrap()
            .is_empty()
    );
    assert!(export_xliff(&data[ID], "en").is_err());
    let mut bad = data[ID].clone();
    bad.messages
        .get_mut("hello")
        .unwrap()
        .insert("en".into(), "\u{1}".into());
    assert!(export_xliff(&bad, "ja").is_err());
}
#[test]
fn prefixes_groups_cdata_comments_and_character_references_preserve_text() {
    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<x:xliff xmlns:x="urn:oasis:names:tc:xliff:document:2.0" version="2.0" srcLang="en" trgLang="ja">
<x:file id="{ID}" original="ignored/not/a/path"><x:group id="a">
<!-- Source material is never an instruction to the host. -->
<x:unit id="empty"><x:notes><x:note>Translator context</x:note></x:notes>
<x:segment state="reviewed"><x:source><![CDATA[Empty]]></x:source><x:target>&lt;&amp;&gt;&quot;&apos;&#13;&#10;&#9;&#x1F642;</x:target></x:segment>
</x:unit></x:group></x:file></x:xliff>"#
    );
    let updates = translations_from_xliff(&tables(), &xml).unwrap();
    assert_eq!(updates[0].value, "<&>\"'\r\n\t🙂");
    let normalized = wrap(
        "<unit id=\"empty\"><segment><source>Empty</source><target>a\r\nb\rc</target></segment></unit>",
    );
    assert_eq!(
        translations_from_xliff(&tables(), &normalized).unwrap()[0].value,
        "a\nb\nc"
    );
    let empty =
        wrap("<unit id=\"empty\"><segment><source>Empty</source><target/></segment></unit>");
    assert_eq!(
        translations_from_xliff(&tables(), &empty).unwrap()[0].value,
        ""
    );
}
#[test]
fn stale_sources_unknown_identities_and_bad_patterns_reject_the_entire_document() {
    let good =
        "<unit id=\"empty\"><segment><source>Empty</source><target>空</target></segment></unit>";
    let xml = wrap(good);
    for invalid in [
        xml.replace("Empty", "Changed"),
        xml.replace(ID, "00000000000000000000000002"),
        xml.replace("id=\"empty\"", "id=\"unknown\""),
        xml.replace("srcLang=\"en\"", "srcLang=\"fr\""),
        xml.replace("trgLang=\"ja\"", "trgLang=\"en\""),
        xml.replace("trgLang=\"ja\"", "trgLang=\"ja_JP\""),
        xml.replace("空", "{bad}"),
        xml.replace("空", "{n,plural,one{bad}}"),
        wrap(&format!(
            "{good}<unit id=\"missing\"><segment><source>stale</source><target>partial</target></segment></unit>"
        )),
    ] {
        assert!(
            translations_from_xliff(&tables(), &invalid).is_err(),
            "{invalid}"
        );
    }
}
#[test]
fn hostile_xml_duplicates_inline_codes_and_ambiguous_segments_fail_explicitly() {
    let unit =
        "<unit id=\"empty\"><segment><source>Empty</source><target>空</target></segment></unit>";
    let xml = wrap(unit);
    for bad in [
        format!("<!DOCTYPE xliff [<!ENTITY ext SYSTEM 'file:///should-not-be-read'>]>{xml}"),
        xml.replace("空", "&ext;"),
        xml.replace("空", "&#0;"),
        xml.replace("空", "&#1;"),
        xml.replace("空", "\u{1}"),
        xml.replace("空", "<ph id=\"1\"/>"),
        xml.replace("空", "<?unsafe anything?>"),
        xml.replace("urn:oasis:names:tc:xliff:document:2.0", "urn:wrong"),
        xml.replace("<target>", "<target xmlns=\"urn:wrong\">"),
        xml.replace("version=\"2.1\"", "version=\"2.1\" version=\"2.0\""),
        xml.replace(
            "<target>空</target>",
            "<target>first</target><target>second</target>",
        ),
        xml.replace(
            "<source>Empty</source>",
            "<source>Empty</source><source>Empty</source>",
        ),
        xml.replace(
            "</segment>",
            "</segment><segment><source>Empty</source></segment>",
        ),
        wrap(&format!("{unit}{unit}")),
        format!("{xml}{xml}"),
        xml.replace("</file>", ""),
        format!("<?xml version=\"1.1\"?>{xml}"),
        format!("<?xml version=\"1.0\" encoding=\"UTF-16\"?>{xml}"),
        format!(" <?xml version=\"1.0\"?>{xml}"),
        format!("<![CDATA[ ]]>{xml}"),
        format!("&#32;{xml}"),
        xml.replace("空", "]]>"),
        xml.replace("<unit", "<![CDATA[ ]]><unit"),
        xml.replace("<unit", "&#32;<unit"),
    ] {
        assert!(translations_from_xliff(&tables(), &bad).is_err(), "{bad}");
    }
}
#[test]
fn exchange_limits_bound_size_depth_units_and_message_expansion() {
    assert!(translations_from_xliff(&tables(), &" ".repeat(MAX_XLIFF_BYTES + 1)).is_err());
    let unit =
        "<unit id=\"empty\"><segment><source>Empty</source><target>空</target></segment></unit>";
    let deep = wrap(&format!(
        "{}{unit}{}",
        "<group id=\"a\">".repeat(20),
        "</group>".repeat(20)
    ));
    assert!(translations_from_xliff(&tables(), &deep).is_err());
    assert!(
        translations_from_xliff(
            &tables(),
            &wrap(&unit.replace("空", &"x".repeat(MAX_MESSAGE_BYTES + 1)))
        )
        .is_err()
    );
    let units = (0..4097)
        .map(|i| unit.replace("id=\"empty\"", &format!("id=\"k{i}\"")))
        .collect::<String>();
    assert!(translations_from_xliff(&tables(), &wrap(&units)).is_err());
}

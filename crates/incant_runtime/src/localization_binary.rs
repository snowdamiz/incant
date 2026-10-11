//! Scene-v10 localization suffix: explicit length-prefixed UTF-8 data, never
//! JSON or executable deserialization. Every count/length is bounded before allocation.
use crate::{NativeLocalization, SceneError, binary::Reader};
use incant_localization::{LocaleSettings, StringTable};
use std::collections::BTreeMap;
pub(crate) const MAX_BYTES: usize = 8 * 1024 * 1024;
fn string(bytes: &mut Vec<u8>, text: &str) {
    bytes.extend((text.len() as u32).to_le_bytes());
    bytes.extend(text.as_bytes());
}
pub(crate) fn size(value: &NativeLocalization) -> usize {
    let settings = value.settings();
    4 + 4
        + settings.locale.len()
        + 4
        + settings
            .fallbacks
            .iter()
            .map(|s| 4 + s.len())
            .sum::<usize>()
        + 1
        + 4
        + value
            .tables()
            .iter()
            .map(|(id, t)| {
                4 + id.len()
                    + 4
                    + t.name.len()
                    + 4
                    + t.source_locale.len()
                    + 4
                    + t.messages
                        .iter()
                        .map(|(key, translations)| {
                            4 + key.len()
                                + 4
                                + translations
                                    .iter()
                                    .map(|(locale, text)| 4 + locale.len() + 4 + text.len())
                                    .sum::<usize>()
                        })
                        .sum::<usize>()
            })
            .sum::<usize>()
}
pub(crate) fn write(bytes: &mut Vec<u8>, value: &NativeLocalization) {
    bytes.extend(((size(value) - 4) as u32).to_le_bytes());
    let settings = value.settings();
    string(bytes, &settings.locale);
    bytes.extend((settings.fallbacks.len() as u32).to_le_bytes());
    for s in &settings.fallbacks {
        string(bytes, s);
    }
    bytes.push(u8::from(settings.pseudo));
    bytes.extend((value.tables().len() as u32).to_le_bytes());
    for (id, t) in value.tables() {
        string(bytes, id);
        string(bytes, &t.name);
        string(bytes, &t.source_locale);
        bytes.extend((t.messages.len() as u32).to_le_bytes());
        for (key, translations) in &t.messages {
            string(bytes, key);
            bytes.extend((translations.len() as u32).to_le_bytes());
            for (locale, text) in translations {
                string(bytes, locale);
                string(bytes, text);
            }
        }
    }
}
fn text(r: &mut Reader<'_>, max: usize) -> Result<String, SceneError> {
    let len = r.u32()? as usize;
    if len > max {
        return Err(SceneError::Invalid("localization string length"));
    }
    Ok(std::str::from_utf8(r.bytes(len)?)
        .map_err(|_| SceneError::Invalid("localization UTF-8"))?
        .to_owned())
}
fn count(r: &mut Reader<'_>, max: usize) -> Result<usize, SceneError> {
    let n = r.u32()? as usize;
    if n > max || n > r.remaining_len() / 4 {
        return Err(SceneError::Invalid("localization count"));
    }
    Ok(n)
}
fn ordered<T>(map: &BTreeMap<String, T>, key: &str) -> Result<(), SceneError> {
    if map
        .last_key_value()
        .is_some_and(|(last, _)| last.as_str() >= key)
    {
        return Err(SceneError::Invalid("localization canonical order"));
    }
    Ok(())
}
pub(crate) fn read(reader: &mut Reader<'_>) -> Result<NativeLocalization, SceneError> {
    let len = reader.u32()? as usize;
    if len > MAX_BYTES - 4 {
        return Err(SceneError::Invalid("localization binary size"));
    }
    let mut r = Reader::new(reader.bytes(len)?);
    let locale = text(&mut r, 128)?;
    let mut fallbacks = Vec::new();
    for _ in 0..count(&mut r, 16)? {
        fallbacks.push(text(&mut r, 128)?);
    }
    let pseudo = match r.byte()? {
        0 => false,
        1 => true,
        _ => return Err(SceneError::Invalid("localization boolean")),
    };
    let mut tables = BTreeMap::new();
    let mut keys = 0;
    for _ in 0..count(&mut r, incant_localization::MAX_TABLES)? {
        let id = text(&mut r, 26)?;
        ordered(&tables, &id)?;
        let name = text(&mut r, 256)?;
        let source_locale = text(&mut r, 128)?;
        let n = count(&mut r, incant_localization::MAX_MESSAGES - keys)?;
        keys += n;
        let mut messages = BTreeMap::new();
        for _ in 0..n {
            let key = text(&mut r, 128)?;
            ordered(&messages, &key)?;
            let mut translations = BTreeMap::new();
            for _ in 0..count(&mut r, incant_localization::MAX_LOCALES)? {
                let language = text(&mut r, 128)?;
                ordered(&translations, &language)?;
                translations.insert(
                    language,
                    text(&mut r, incant_localization::MAX_MESSAGE_BYTES)?,
                );
            }
            messages.insert(key, translations);
        }
        tables.insert(
            id.clone(),
            StringTable {
                id,
                name,
                source_locale,
                messages,
            },
        );
    }
    if r.remaining_len() != 0 {
        return Err(SceneError::Invalid("localization trailing bytes"));
    }
    NativeLocalization::new(
        tables,
        LocaleSettings {
            locale,
            fallbacks,
            pseudo,
        },
    )
}

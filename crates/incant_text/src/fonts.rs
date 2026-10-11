use crate::TextError;
use cosmic_text::{Fallback, FontSystem, fontdb};
use std::{collections::BTreeMap, sync::Arc};
use ulid::Ulid;
use unicode_script::Script;

pub const MAX_FONTS: usize = 32;
pub const MAX_FONT_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_TOTAL_FONT_BYTES: usize = 64 * 1024 * 1024;

/// A packaged font asset. Order in the engine constructor is fallback priority.
/// Bytes are owned; no system font enumeration, path resolution or network access.
pub struct FontSource {
    pub id: Ulid,
    pub bytes: Vec<u8>,
}

// cosmic-text's fallback trait takes static family names. Fixed, bounded private
// aliases let each project choose its own ordered asset chain without leaking
// strings, consulting platform fallback lists, or depending on font name aliases.
const FAMILIES: [&str; MAX_FONTS] = [
    "Incant0", "Incant1", "Incant2", "Incant3", "Incant4", "Incant5", "Incant6", "Incant7",
    "Incant8", "Incant9", "Incant10", "Incant11", "Incant12", "Incant13", "Incant14", "Incant15",
    "Incant16", "Incant17", "Incant18", "Incant19", "Incant20", "Incant21", "Incant22", "Incant23",
    "Incant24", "Incant25", "Incant26", "Incant27", "Incant28", "Incant29", "Incant30", "Incant31",
];

struct OrderedFallback(usize);

impl Fallback for OrderedFallback {
    fn common_fallback(&self) -> &[&'static str] {
        &FAMILIES[..self.0]
    }
    fn forbidden_fallback(&self) -> &[&'static str] {
        &[]
    }
    fn script_fallback(&self, _: Script, _: &str) -> &[&'static str] {
        &[]
    }
}

pub(crate) struct Fonts {
    pub system: FontSystem,
    pub ids: BTreeMap<fontdb::ID, Ulid>,
}

impl Fonts {
    pub fn new(fonts: Vec<FontSource>) -> Result<Self, TextError> {
        if fonts.is_empty() || fonts.len() > MAX_FONTS {
            return Err(TextError::Limit("font count"));
        }
        let mut total = 0_usize;
        let mut unique = std::collections::BTreeSet::new();
        for font in &fonts {
            if !unique.insert(font.id) {
                return Err(TextError::Invalid("duplicate font asset"));
            }
            if font.bytes.len() > MAX_FONT_BYTES {
                return Err(TextError::Limit("font bytes"));
            }
            total = total
                .checked_add(font.bytes.len())
                .ok_or(TextError::Limit("font bytes"))?;
            if total > MAX_TOTAL_FONT_BYTES {
                return Err(TextError::Limit("total font bytes"));
            }
            // Collections may declare very many faces before fontdb allocates.
            // Only a single sfnt face is currently supported; do not flatten TTC.
            if !matches!(font.bytes.get(..4), Some(b"\0\x01\0\0" | b"OTTO" | b"true")) {
                return Err(TextError::Font(font.id));
            }
        }
        let count = fonts.len();
        let mut db = fontdb::Database::new();
        let mut assets = Vec::with_capacity(count);
        for (index, font) in fonts.into_iter().enumerate() {
            // This API selects concrete asset faces, not CSS families/styles.
            // Construct bounded matching metadata instead of decoding arbitrary
            // name tables, whose overlapping records can amplify allocations.
            // Normal matching attributes preserve asset order even for italic
            // or bold faces; the actual bytes/outlines remain unchanged.
            db.push_face_info(fontdb::FaceInfo {
                id: fontdb::ID::dummy(),
                source: fontdb::Source::Binary(Arc::new(font.bytes)),
                index: 0,
                families: vec![(
                    FAMILIES[index].into(),
                    fontdb::Language::English_UnitedStates,
                )],
                post_script_name: FAMILIES[index].into(),
                style: fontdb::Style::Normal,
                weight: fontdb::Weight::NORMAL,
                stretch: fontdb::Stretch::Normal,
                monospaced: false,
            });
            assets.push(font.id);
        }
        // push_face_info preserves insertion order, but map by private family
        // rather than assuming fontdb's opaque IDs have any numerical meaning.
        let mut ids = BTreeMap::new();
        for face in db.faces() {
            let index = FAMILIES
                .iter()
                .position(|name| *name == face.families[0].0)
                .ok_or(TextError::Invalid("internal font identity"))?;
            ids.insert(face.id, assets[index]);
        }
        let mut system = FontSystem::new_with_locale_and_db_and_fallback(
            "und".into(),
            db,
            OrderedFallback(count),
        );
        // fontdb admits metadata without requiring a usable shaping face.
        // Validate and retain every actual rustybuzz font before any text can
        // reach the shaper, which assumes that a default face can be loaded.
        for (&face, &asset) in &ids {
            if system.get_font(face).is_none() {
                return Err(TextError::Font(asset));
            }
        }
        Ok(Self { system, ids })
    }
}

pub(crate) fn primary_family() -> cosmic_text::Family<'static> {
    cosmic_text::Family::Name(FAMILIES[0])
}

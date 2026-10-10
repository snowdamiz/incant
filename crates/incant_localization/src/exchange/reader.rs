use super::*;
use quick_xml::{
    NsReader, XmlVersion,
    events::{BytesStart, Event},
    name::{Namespace, NamespaceResolver, ResolveResult},
};

pub(super) struct Message {
    pub source: String,
    pub target: Option<String>,
}
pub(super) struct Parsed {
    pub source: String,
    pub target: String,
    pub files: BTreeMap<String, BTreeMap<String, Message>>,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Element {
    Root,
    File,
    Group,
    Unit,
    Segment,
    Source,
    Target,
    Notes,
    Note,
}
struct Unit {
    id: String,
    source: Option<String>,
    target: Option<String>,
    segment: bool,
}
struct File {
    id: String,
    messages: BTreeMap<String, Message>,
}
#[derive(Default)]
struct State {
    stack: Vec<Element>,
    source: String,
    target: String,
    files: BTreeMap<String, BTreeMap<String, Message>>,
    file: Option<File>,
    unit: Option<Unit>,
    root_seen: bool,
    root_closed: bool,
    units: usize,
    text_bytes: usize,
}
fn attrs(
    e: &BytesStart<'_>,
    resolver: &NamespaceResolver,
    allowed: &[&str],
) -> Result<BTreeMap<String, String>, LocalizationError> {
    let mut result = BTreeMap::new();
    for (i, attr) in e.attributes().enumerate() {
        if i >= 16 {
            return Err(error("at most 16 attributes per element"));
        }
        let attr = attr.map_err(|e| error(e.to_string()))?;
        if attr.key.0 == "xmlns" || attr.key.0.starts_with("xmlns:") {
            continue;
        }
        let (namespace, local) = resolver.resolve_attribute(attr.key);
        let key = match namespace {
            ResolveResult::Unbound => local.as_ref(),
            ResolveResult::Bound(Namespace("http://www.w3.org/XML/1998/namespace"))
                if local.as_ref() == "space" =>
            {
                "xml:space"
            }
            _ => return Err(error("unsupported namespaced attribute")),
        };
        if !allowed.contains(&key) {
            return Err(error(format!("unsupported attribute {key}")));
        }
        let value = attr
            .normalized_value(XmlVersion::Explicit1_0)
            .map_err(|e| error(e.to_string()))?
            .into_owned();
        if value.len() > 512 || !valid_chars(&value) {
            return Err(error("invalid or oversized attribute"));
        }
        if key == "canResegment" && !matches!(value.as_str(), "yes" | "no") {
            return Err(error("invalid canResegment"));
        }
        if key == "xml:space" && !matches!(value.as_str(), "preserve" | "default") {
            return Err(error("invalid xml:space"));
        }
        if result.insert(key.into(), value).is_some() {
            return Err(error("duplicate expanded attribute"));
        }
    }
    Ok(result)
}
fn required(values: &BTreeMap<String, String>, name: &str) -> Result<String, LocalizationError> {
    values
        .get(name)
        .filter(|v| !v.is_empty())
        .cloned()
        .ok_or_else(|| error(format!("missing {name}")))
}
impl State {
    fn open(
        &mut self,
        e: &BytesStart<'_>,
        resolver: &NamespaceResolver,
    ) -> Result<(), LocalizationError> {
        let parent = self.stack.last().copied();
        if self.stack.len() >= 20 {
            return Err(error("XLIFF nesting exceeds 20"));
        }
        let element = match e.local_name().as_ref() {
            "xliff" if parent.is_none() && !self.root_seen => {
                let a = attrs(e, resolver, &["version", "srcLang", "trgLang", "xml:space"])?;
                if !matches!(required(&a, "version")?.as_str(), "2.0" | "2.1") {
                    return Err(error("only XLIFF 2.0/2.1 is supported"));
                }
                self.source = required(&a, "srcLang")?;
                self.target = required(&a, "trgLang")?;
                self.root_seen = true;
                Element::Root
            }
            "file" if parent == Some(Element::Root) => {
                let a = attrs(
                    e,
                    resolver,
                    &["id", "original", "canResegment", "translate", "xml:space"],
                )?;
                if a.get("translate").is_some_and(|v| v != "yes") {
                    return Err(error("nontranslatable files are unsupported"));
                }
                let id = required(&a, "id")?;
                if self.files.contains_key(&id) || self.files.len() >= crate::MAX_TABLES {
                    return Err(error("duplicate or excessive files"));
                }
                self.file = Some(File {
                    id,
                    messages: BTreeMap::new(),
                });
                Element::File
            }
            "group" if matches!(parent, Some(Element::File | Element::Group)) => {
                let a = attrs(
                    e,
                    resolver,
                    &["id", "name", "canResegment", "translate", "xml:space"],
                )?;
                if a.get("translate").is_some_and(|v| v != "yes") {
                    return Err(error("nontranslatable groups are unsupported"));
                }
                required(&a, "id")?;
                Element::Group
            }
            "unit" if matches!(parent, Some(Element::File | Element::Group)) => {
                let a = attrs(
                    e,
                    resolver,
                    &["id", "name", "canResegment", "translate", "xml:space"],
                )?;
                if a.get("translate").is_some_and(|v| v != "yes") {
                    return Err(error("nontranslatable units are unsupported"));
                }
                let id = required(&a, "id")?;
                if !crate::types::key(&id) || self.file.as_ref().unwrap().messages.contains_key(&id)
                {
                    return Err(error("invalid or duplicate unit ID"));
                }
                self.units += 1;
                if self.units > crate::MAX_MESSAGES {
                    return Err(error("too many message units"));
                }
                self.unit = Some(Unit {
                    id,
                    source: None,
                    target: None,
                    segment: false,
                });
                Element::Unit
            }
            "segment" if parent == Some(Element::Unit) => {
                let a = attrs(e, resolver, &["id", "state", "canResegment"])?;
                if a.get("state").is_some_and(|v| {
                    !matches!(v.as_str(), "initial" | "translated" | "reviewed" | "final")
                }) {
                    return Err(error("unsupported segment state"));
                }
                let unit = self.unit.as_mut().unwrap();
                if unit.segment {
                    return Err(error("one segment per message is required"));
                }
                unit.segment = true;
                Element::Segment
            }
            "source" if parent == Some(Element::Segment) => {
                attrs(e, resolver, &["xml:space"])?;
                let unit = self.unit.as_mut().unwrap();
                if unit.source.is_some() || unit.target.is_some() {
                    return Err(error("duplicate or misplaced source"));
                }
                unit.source = Some(String::new());
                Element::Source
            }
            "target" if parent == Some(Element::Segment) => {
                attrs(e, resolver, &["xml:space"])?;
                let unit = self.unit.as_mut().unwrap();
                if unit.source.is_none() || unit.target.is_some() {
                    return Err(error("duplicate or misplaced target"));
                }
                unit.target = Some(String::new());
                Element::Target
            }
            "notes" if matches!(parent, Some(Element::File | Element::Group | Element::Unit)) => {
                attrs(e, resolver, &[])?;
                Element::Notes
            }
            "note" if parent == Some(Element::Notes) => {
                attrs(e, resolver, &["id", "appliesTo", "category", "priority"])?;
                Element::Note
            }
            name => {
                return Err(error(format!(
                    "unsupported or misplaced XLIFF element {name}"
                )));
            }
        };
        self.stack.push(element);
        Ok(())
    }
    fn close(&mut self) -> Result<(), LocalizationError> {
        match self
            .stack
            .pop()
            .ok_or_else(|| error("unexpected end tag"))?
        {
            Element::Root => {
                if self.files.is_empty() {
                    return Err(error("at least one file is required"));
                }
                self.root_closed = true;
            }
            Element::File => {
                let file = self.file.take().unwrap();
                if file.messages.is_empty() {
                    return Err(error("at least one unit per file is required"));
                }
                self.files.insert(file.id, file.messages);
            }
            Element::Unit => {
                let unit = self.unit.take().unwrap();
                let source = unit
                    .source
                    .ok_or_else(|| error("message source is required"))?;
                self.file.as_mut().unwrap().messages.insert(
                    unit.id,
                    Message {
                        source,
                        target: unit.target,
                    },
                );
            }
            Element::Segment if self.unit.as_ref().unwrap().source.is_none() => {
                return Err(error("segment source is required"));
            }
            _ => {}
        }
        Ok(())
    }
    fn text(&mut self, text: &str) -> Result<(), LocalizationError> {
        if !valid_chars(text) {
            return Err(error("character outside XML 1.0"));
        }
        let target = match self.stack.last() {
            Some(Element::Source) => self.unit.as_mut().unwrap().source.as_mut(),
            Some(Element::Target) => self.unit.as_mut().unwrap().target.as_mut(),
            Some(Element::Note) => return Ok(()),
            _ if text
                .bytes()
                .all(|b| matches!(b, b' ' | b'\t' | b'\r' | b'\n')) =>
            {
                return Ok(());
            }
            _ => return Err(error("text outside a source, target or note")),
        }
        .unwrap();
        if target.len() + text.len() > MAX_MESSAGE_BYTES {
            return Err(error("message exceeds 8192 bytes"));
        }
        self.text_bytes += text.len();
        if self.text_bytes > crate::MAX_CATALOG_BYTES {
            return Err(error("decoded message text exceeds 4 MiB"));
        }
        target.push_str(text);
        Ok(())
    }
}
pub(super) fn read(document: &str) -> Result<Parsed, LocalizationError> {
    if document.len() > MAX_XLIFF_BYTES {
        return Err(error("XLIFF exceeds 32 MiB"));
    }
    if !valid_chars(document) {
        return Err(error("character outside XML 1.0"));
    }
    let mut reader = NsReader::from_str(document);
    reader.config_mut().expand_empty_elements = true;
    reader.config_mut().check_comments = true;
    reader.resolver_mut().set_max_namespace_bindings(32);
    let mut state = State::default();
    let mut declared = false;
    let mut events = 0_usize;
    loop {
        let (ns, event) = reader
            .read_resolved_event()
            .map_err(|e| error(e.to_string()))?;
        events += 1;
        if events > 200_000 {
            return Err(error("too many XML events"));
        }
        if matches!(&event, Event::Start(_) | Event::End(_) | Event::Empty(_))
            && !matches!(ns, ResolveResult::Bound(Namespace(NS)))
        {
            return Err(error("incorrect XLIFF namespace"));
        }
        match event {
            Event::Start(e) => state.open(&e, reader.resolver())?,
            Event::End(_) => state.close()?,
            Event::Text(e) => {
                let value = e.xml10_content();
                if value.contains("]]>") {
                    return Err(error("CDATA delimiter in plain text"));
                }
                state.text(&value)?;
            }
            Event::CData(e) => {
                if !matches!(
                    state.stack.last(),
                    Some(Element::Source | Element::Target | Element::Note)
                ) {
                    return Err(error("CDATA outside text element"));
                }
                state.text(&e.xml10_content())?;
            }
            Event::GeneralRef(e) => {
                if !matches!(
                    state.stack.last(),
                    Some(Element::Source | Element::Target | Element::Note)
                ) {
                    return Err(error("reference outside text element"));
                }
                let character = match e.resolve_char_ref().map_err(|e| error(e.to_string()))? {
                    Some(c) => c,
                    None => match &*e {
                        "amp" => '&',
                        "lt" => '<',
                        "gt" => '>',
                        "apos" => '\'',
                        "quot" => '"',
                        _ => return Err(error("unknown entity reference")),
                    },
                };
                state.text(character.encode_utf8(&mut [0; 4]))?;
            }
            Event::Decl(e) => {
                if events != 1
                    || declared
                    || state.root_seen
                    || e.version().map_err(|e| error(e.to_string()))?.as_ref() != "1.0"
                {
                    return Err(error("expected a single XML 1.0 declaration"));
                }
                if let Some(encoding) = e.encoding()
                    && !encoding
                        .map_err(|e| error(e.to_string()))?
                        .eq_ignore_ascii_case("utf-8")
                {
                    return Err(error("XLIFF input must be UTF-8"));
                }
                declared = true;
            }
            Event::Comment(_) => {}
            Event::DocType(_) | Event::PI(_) => {
                return Err(error("DTDs and processing instructions are unsupported"));
            }
            Event::Eof => break,
            Event::Empty(_) => unreachable!("empty elements expanded by reader"),
        }
    }
    if !state.root_closed || !state.stack.is_empty() {
        return Err(error("incomplete XLIFF document"));
    }
    Ok(Parsed {
        source: state.source,
        target: state.target,
        files: state.files,
    })
}

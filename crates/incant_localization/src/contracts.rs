//! Translation patterns cannot demand arguments the source message does not accept.
use crate::{LocalizationError, message::Node};
use std::collections::BTreeMap;
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Any,
    Text,
    Number,
    Date,
}
fn require(
    map: &mut BTreeMap<String, Kind>,
    name: &str,
    kind: Kind,
) -> Result<(), LocalizationError> {
    match map.get(name).copied() {
        Some(old) if old != Kind::Any && kind != Kind::Any && old != kind => Err(
            LocalizationError::Argument(format!("{name} has conflicting pattern types")),
        ),
        Some(old) if old != Kind::Any => Ok(()),
        _ => {
            map.insert(name.into(), kind);
            Ok(())
        }
    }
}
fn visit(nodes: &[Node], output: &mut BTreeMap<String, Kind>) -> Result<(), LocalizationError> {
    for node in nodes {
        match node {
            Node::Text(_) | Node::Pound => {}
            Node::Argument(name) => require(output, name, Kind::Any)?,
            Node::Number(name) => require(output, name, Kind::Number)?,
            Node::Date(name, _) => require(output, name, Kind::Date)?,
            Node::Select { name, branches } => {
                require(output, name, Kind::Text)?;
                for branch in branches.values() {
                    visit(branch, output)?;
                }
            }
            Node::Plural {
                name,
                branches,
                exact,
                ..
            } => {
                require(output, name, Kind::Number)?;
                for branch in branches.values() {
                    visit(branch, output)?;
                }
                for (_, branch) in exact {
                    visit(branch, output)?;
                }
            }
        }
    }
    Ok(())
}
pub(crate) fn validate(source: &[Node], translation: &[Node]) -> Result<(), LocalizationError> {
    let mut source_types = BTreeMap::new();
    visit(source, &mut source_types)?;
    let mut translated_types = BTreeMap::new();
    visit(translation, &mut translated_types)?;
    for (name, kind) in translated_types {
        match source_types.get(&name) {
            Some(source) if kind == Kind::Any || *source == kind => {}
            _ => {
                return Err(LocalizationError::Argument(format!(
                    "translation adds or narrows argument {name}"
                )));
            }
        }
    }
    Ok(())
}

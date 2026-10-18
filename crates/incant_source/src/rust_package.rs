//! Read-only registered package authority. Never a serialized path capability.
//! Rechecking bounded held nodes catches external edits before publication; it
//! does not claim an atomic snapshot against writers racing the final check.
use super::*;
use std::{collections::BTreeSet, path::PathBuf};

const MAX_ENTRIES: usize = 128;
const MAX_FILES: usize = 32;
const MAX_DEPTH: usize = 8;

/// Root comes from the trusted host, entry from validated registration. The only
/// non-Rust entry admitted is that entry's derived `.wasm` sibling. It is bounded
/// and pinned but never returned as compiler input; destination CAS owns bytes.
pub struct RustPackageSnapshot {
    root: PathBuf,
    package: String,
    chain: Vec<Dir>,
    tree: Tree,
    files: Vec<(String, String)>,
}
struct Tree {
    dir: Dir,
    entries: Vec<Entry>,
}
enum Entry {
    Directory {
        name: String,
        tree: Tree,
    },
    Source {
        name: String,
        node: File,
        text: String,
    },
    Output {
        name: String,
        node: File,
    },
}
impl Entry {
    fn name(&self) -> &str {
        match self {
            Self::Directory { name, .. }
            | Self::Source { name, .. }
            | Self::Output { name, .. } => name,
        }
    }
}
fn name(value: &str) -> Result<(), SourceError> {
    if value.is_empty()
        || value.len() > 128
        || !value.as_bytes()[0].is_ascii_alphanumeric() && !value.starts_with('_')
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
    {
        return Err(SourceError::Invalid("portable Rust package name".into()));
    }
    validate_portable_path(value)
}
fn read_file(dir: &Dir, name: &str, limit: usize) -> Result<(File, String), SourceError> {
    let mut node = dir.open(name)?;
    platform::regular(&node)?;
    let before = node.metadata()?;
    if before.len() > limit as u64 {
        return Err(SourceError::Invalid("Rust file bound".into()));
    }
    let mut bytes = Vec::new();
    Read::by_ref(&mut node)
        .take((limit + 1) as u64)
        .read_to_end(&mut bytes)?;
    let after = node.metadata()?;
    platform::regular(&node)?;
    if bytes.len() > limit
        || bytes.len() as u64 != before.len()
        || before.len() != after.len()
        || before.modified()? != after.modified()?
        || platform::identity(&node)? != platform::identity(&dir.open(name)?)?
    {
        return Err(SourceError::Conflict);
    }
    Ok((
        node,
        String::from_utf8(bytes).map_err(|_| SourceError::Invalid("Rust UTF-8 source".into()))?,
    ))
}
fn output(dir: &Dir, name: &str) -> Result<File, SourceError> {
    let node = dir.open(name)?;
    platform::regular(&node)?;
    if node.metadata()?.len() > MAX_WASM_BYTES as u64 {
        return Err(SourceError::Invalid("compiled output bound".into()));
    }
    Ok(node)
}
impl Tree {
    fn capture(
        dir: Dir,
        prefix: &str,
        derived: &str,
        depth: usize,
        count: &mut usize,
        total: &mut usize,
        files: &mut Vec<(String, String)>,
    ) -> Result<Self, SourceError> {
        if depth > MAX_DEPTH {
            return Err(SourceError::Invalid("Rust package depth".into()));
        }
        let names = dir.names(MAX_ENTRIES.saturating_sub(*count))?;
        *count += names.len();
        let mut aliases = BTreeSet::new();
        let mut entries = Vec::new();
        for leaf in &names {
            name(leaf)?;
            if !aliases.insert(leaf.to_ascii_lowercase()) {
                return Err(SourceError::Conflict);
            }
            let path = if prefix.is_empty() {
                leaf.clone()
            } else {
                format!("{prefix}/{leaf}")
            };
            if let Ok(child) = dir.child(leaf) {
                if path == derived {
                    return Err(SourceError::Invalid("derived output is a directory".into()));
                }
                entries.push(Entry::Directory {
                    name: leaf.clone(),
                    tree: Self::capture(child, &path, derived, depth + 1, count, total, files)?,
                });
            } else if path == derived {
                entries.push(Entry::Output {
                    name: leaf.clone(),
                    node: output(&dir, leaf)?,
                });
            } else {
                if !leaf.ends_with(".rs")
                    || leaf.eq_ignore_ascii_case("build.rs")
                    || files.len() == MAX_FILES
                {
                    return Err(SourceError::Invalid(
                        "source-only bounded Rust package required".into(),
                    ));
                }
                let (node, text) = read_file(&dir, leaf, MAX_RUST_SOURCE_BYTES)?;
                *total += text.len();
                if *total > MAX_SOURCE_BYTES {
                    return Err(SourceError::Invalid("Rust package byte bound".into()));
                }
                files.push((path, text.clone()));
                entries.push(Entry::Source {
                    name: leaf.clone(),
                    node,
                    text,
                });
            }
        }
        if dir.names(MAX_ENTRIES)? != names {
            return Err(SourceError::Conflict);
        }
        Ok(Self { dir, entries })
    }
    fn validate(&self) -> Result<(), SourceError> {
        let names = self
            .entries
            .iter()
            .map(|e| e.name().to_owned())
            .collect::<Vec<_>>();
        if self.dir.names(MAX_ENTRIES)? != names {
            return Err(SourceError::Conflict);
        }
        for entry in &self.entries {
            match entry {
                Entry::Directory { name, tree } => {
                    if self.dir.child(name)?.identity()? != tree.dir.identity()? {
                        return Err(SourceError::Conflict);
                    }
                    tree.validate()?;
                }
                Entry::Source { name, node, text } => {
                    platform::regular(node)?;
                    let (current, bytes) = read_file(&self.dir, name, MAX_RUST_SOURCE_BYTES)?;
                    if platform::identity(&current)? != platform::identity(node)? || &bytes != text
                    {
                        return Err(SourceError::Conflict);
                    }
                }
                Entry::Output { name, node } => {
                    platform::regular(node)?;
                    if platform::identity(&output(&self.dir, name)?)? != platform::identity(node)? {
                        return Err(SourceError::Conflict);
                    }
                }
            }
        }
        if self.dir.names(MAX_ENTRIES)? != names {
            return Err(SourceError::Conflict);
        }
        Ok(())
    }
}
impl RustPackageSnapshot {
    pub fn capture(root: &Path, registered_path: &str) -> Result<Self, SourceError> {
        validate_rust_path(registered_path)?;
        let (package, entry) = registered_path
            .rsplit_once('/')
            .ok_or_else(|| SourceError::Invalid("registered Rust package".into()))?;
        if package.split('/').count() > MAX_DEPTH {
            return Err(SourceError::Invalid("package depth".into()));
        }
        let mut chain = vec![Dir::root(root)?];
        for part in package.split('/') {
            name(part)?;
            chain.push(chain.last().unwrap().child(part)?);
        }
        name(entry)?;
        let derived = format!(
            "{}.wasm",
            entry.strip_suffix(".rs").expect("validated entry")
        );
        let mut files = Vec::new();
        let tree = Tree::capture(
            chain.last().unwrap().duplicate()?,
            "",
            derived,
            1,
            &mut 0,
            &mut 0,
            &mut files,
        )?;
        if !files.iter().any(|(path, _)| path == entry) {
            return Err(SourceError::Invalid("missing Rust entry".into()));
        }
        let snapshot = Self {
            root: root.into(),
            package: package.into(),
            chain,
            tree,
            files,
        };
        snapshot.validate()?;
        Ok(snapshot)
    }
    pub fn files(&self) -> &[(String, String)] {
        &self.files
    }
    /// Bounded synchronous data checks only: no process, backend or bus calls.
    pub fn validate(&self) -> Result<(), SourceError> {
        let mut dir = Dir::root(&self.root)?;
        if dir.identity()? != self.chain[0].identity()? {
            return Err(SourceError::Conflict);
        }
        for (part, held) in self.package.split('/').zip(&self.chain[1..]) {
            dir = dir.child(part)?;
            if dir.identity()? != held.identity()? {
                return Err(SourceError::Conflict);
            }
        }
        self.tree.validate()
    }
}

#[cfg(test)]
#[path = "rust_package_tests.rs"]
mod tests;

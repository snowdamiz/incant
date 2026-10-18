//! Reversible publication of one privately staged, missing Rust source subtree.
//!
//! No directory is recursively deleted. Undo retains the complete owned subtree
//! in transaction state. A foreign entry or replaced node blocks movement; an
//! ambiguous post-rename failure retains both the node and recovery marker.
use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OwnedSourceTree {
    token: String,
    root: String,
    directories: Vec<[u64; 2]>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Ownership {
    binding: String,
    root_identity: [u64; 2],
    tree: OwnedSourceTree,
}

impl OwnedSourceTree {
    pub(super) fn validate(&self, edit: &SourceEdit) -> Result<(), SourceError> {
        let prefix = format!("{}/", self.root);
        let suffix = edit.path.strip_prefix(&prefix);
        if edit.kind != SourceKind::Rust
            || edit.before.is_some() == edit.after.is_some()
            || ulid::Ulid::from_string(&self.token).is_err()
            || self.root.is_empty()
            || self.directories.is_empty()
            || self.directories.len() > 7
            || suffix.is_none_or(|suffix| suffix.split('/').count() != self.directories.len())
        {
            return Err(SourceError::Invalid("invalid owned source subtree".into()));
        }
        Ok(())
    }
    fn name(&self) -> String {
        format!("{}.tree", self.token)
    }
    fn receipt(&self) -> String {
        format!("{}.ownership", self.token)
    }
}

impl SourceStore {
    /// Called only by the bus after validating the final project and Missing CAS.
    /// Existing ancestors remain untouched. Before Prepare, only private state
    /// is written; an interrupted stage is inert and never authorizes publication.
    pub fn prepare_creation(
        &self,
        token: &str,
        path: &str,
        source: &str,
    ) -> Result<Option<OwnedSourceTree>, SourceError> {
        self.ensure_ready()?;
        validate_rust_path(path)?;
        SourceKind::Rust.validate_source(source)?;
        ulid::Ulid::from_string(token)
            .map_err(|_| SourceError::Invalid("invalid subtree token".into()))?;
        let parts: Vec<_> = path.split('/').collect();
        let mut parent = self.root.duplicate()?;
        let mut missing = None;
        for (index, part) in parts[..parts.len() - 1].iter().enumerate() {
            match parent.child(part) {
                Ok(next) => parent = next,
                Err(error) if error.kind() == io::ErrorKind::NotFound => {
                    missing = Some(index);
                    break;
                }
                Err(error) => return Err(error.into()),
            }
        }
        let Some(missing) = missing else {
            return Ok(None);
        };
        let mut tree = OwnedSourceTree {
            token: token.into(),
            root: parts[..=missing].join("/"),
            directories: Vec::new(),
        };
        self.state.mkdir(&tree.name())?;
        let mut staged = self.state.child(&tree.name())?;
        tree.directories.push(staged.identity()?);
        for part in &parts[missing + 1..parts.len() - 1] {
            staged.mkdir(part)?;
            staged = staged.child(part)?;
            tree.directories.push(staged.identity()?);
        }
        write_new(&staged, parts[parts.len() - 1], source.as_bytes())?;
        // Release the Windows no-DELETE-sharing chain before a later rename.
        drop(staged);
        let ownership = Ownership {
            binding: self.binding.clone(),
            root_identity: self.root.identity()?,
            tree: tree.clone(),
        };
        let bytes = serde_json::to_vec(&ownership)
            .map_err(|error| SourceError::Invalid(error.to_string()))?;
        write_new(&self.state, &tree.receipt(), &bytes)?;
        Ok(Some(tree))
    }

    /// Also used during recovery of a marker that has no durable Prepare.
    pub fn check_preimage(&self, edit: &SourceEdit) -> Result<(), SourceError> {
        edit.validate()?;
        if edit.created_tree.is_some() {
            self.check_tree_preimage(edit)
        } else if self.read(&edit.path)? == edit.before {
            Ok(())
        } else {
            Err(SourceError::Conflict)
        }
    }

    fn owned_tree<'a>(&self, edit: &'a SourceEdit) -> Result<&'a OwnedSourceTree, SourceError> {
        edit.validate()?;
        let tree = edit
            .created_tree
            .as_ref()
            .ok_or_else(|| SourceError::Invalid("source edit does not own a subtree".into()))?;
        let receipt = optional_read(&self.state, &tree.receipt(), 4096)?
            .ok_or(SourceError::RecoveryRequired)?;
        let ownership: Ownership =
            serde_json::from_str(&receipt).map_err(|_| SourceError::RecoveryRequired)?;
        if ownership.binding != self.binding
            || ownership.root_identity != self.root.identity()?
            || ownership.tree != *tree
        {
            return Err(SourceError::RecoveryRequired);
        }
        Ok(tree)
    }

    fn tree_parent(&self, tree: &OwnedSourceTree) -> Result<(Dir, String), SourceError> {
        let mut parts = tree.root.split('/').peekable();
        let mut parent = self.root.duplicate()?;
        while let Some(part) = parts.next() {
            if parts.peek().is_none() {
                return Ok((parent, part.into()));
            }
            parent = parent.child(part)?;
        }
        Err(SourceError::Invalid("empty subtree root".into()))
    }

    /// Bounded exact-chain inspection: at most seven directories, each with one
    /// child. Enumeration stops at the first extra entry. No link is followed.
    fn tree_at(
        &self,
        parent: &Dir,
        name: &str,
        tree: &OwnedSourceTree,
        edit: &SourceEdit,
    ) -> Result<bool, SourceError> {
        let mut dir = match parent.child(name) {
            Ok(dir) => dir,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
            Err(error) => return Err(error.into()),
        };
        let suffix = edit.path.strip_prefix(&format!("{}/", tree.root)).unwrap();
        let parts: Vec<_> = suffix.split('/').collect();
        for (index, part) in parts.iter().enumerate() {
            if dir.identity()? != tree.directories[index] || !dir.only_child(part)? {
                return Err(SourceError::Conflict);
            }
            if index + 1 < parts.len() {
                dir = dir.child(part)?;
            } else if optional_read(&dir, part, MAX_RUST_SOURCE_BYTES)?.as_deref()
                != edit.before.as_deref().or(edit.after.as_deref())
            {
                return Err(SourceError::Conflict);
            }
        }
        Ok(true)
    }

    pub(super) fn check_tree_preimage(&self, edit: &SourceEdit) -> Result<(), SourceError> {
        let tree = self.owned_tree(edit)?;
        let (parent, name) = self.tree_parent(tree)?;
        let live = self.tree_at(&parent, &name, tree, edit)?;
        let retained = self.tree_at(&self.state, &tree.name(), tree, edit)?;
        if live != edit.before.is_some() || retained != edit.after.is_some() {
            return Err(SourceError::Conflict);
        }
        Ok(())
    }

    pub(super) fn install_tree(&self, edit: &SourceEdit) -> Result<(), SourceError> {
        self.check_tree_preimage(edit)?;
        let tree = self.owned_tree(edit)?;
        let (parent, name) = self.tree_parent(tree)?;
        if edit.after.is_some() {
            self.state.move_to(&tree.name(), &parent, &name)?;
        } else {
            parent.move_to(&name, &self.state, &tree.name())?;
        }
        // This checks the actually moved nodes, not merely a check-and-close
        // observation made before rename. Foreign nodes are never deleted.
        self.check_tree_preimage(&edit.reversed())
    }

    pub(super) fn rollback_tree(&self, edit: &SourceEdit) -> Result<(), SourceError> {
        let tree = self.owned_tree(edit)?;
        let (parent, name) = self.tree_parent(tree)?;
        let live = self.tree_at(&parent, &name, tree, edit)?;
        let retained = self.tree_at(&self.state, &tree.name(), tree, edit)?;
        if live == edit.before.is_some() && retained == edit.after.is_some() {
            return Ok(());
        }
        if live != edit.after.is_some() || retained != edit.before.is_some() {
            return Err(SourceError::RecoveryRequired);
        }
        if edit.before.is_some() {
            self.state.move_to(&tree.name(), &parent, &name)?;
        } else {
            parent.move_to(&name, &self.state, &tree.name())?;
        }
        self.check_tree_preimage(edit)
    }
}

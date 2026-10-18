//! Host-prepared bytes, never an arbitrary candidate-path or filesystem command.
use super::*;
use incant_source::{BinaryPublication, BinaryVersion};

/// An in-memory preparation is bound to this bus session and complete document.
/// It grants no serialized/tool authority and is revalidated at publication.
pub struct PreparedWasmAsset {
    session: String,
    revision: u64,
    project: Project,
    module_id: Id,
    source: SourceVersion,
    destination: BinaryVersion,
    target: incant_source::BinarySnapshot,
    asset: Asset,
    bytes: Vec<u8>,
}
impl CommandBus {
    pub fn check_source_root(&self, root: &std::path::Path) -> Result<(), CommandError> {
        self.ensure_sources_ready()?;
        self.sources
            .as_ref()
            .ok_or(SourceError::Unavailable)?
            .check_root(root)?;
        Ok(())
    }
    pub fn module_binary_version(&self, module_id: &str) -> Result<BinaryVersion, CommandError> {
        let source = self.read_module_source(module_id)?;
        let bytes = self
            .sources
            .as_ref()
            .ok_or(SourceError::Unavailable)?
            .read_binary(&source.path)?;
        Ok(BinaryVersion::of(bytes.as_deref()))
    }
    /// `incant_import` supplies ordinary CapturedModule-validated bytes/asset.
    /// This trusted Rust API does not accept a candidate or destination path.
    pub fn prepare_wasm_asset(
        &self,
        module_id: &str,
        expected_source: SourceVersion,
        expected_destination: BinaryVersion,
        asset: Asset,
        bytes: &[u8],
    ) -> Result<PreparedWasmAsset, CommandError> {
        if bytes.len() > incant_source::MAX_WASM_BYTES {
            return Err(CommandError::Invalid("compiled Wasm exceeds 1 MiB".into()));
        }
        let source = self.read_module_source(module_id)?;
        let target = self
            .sources
            .as_ref()
            .ok_or(SourceError::Unavailable)?
            .capture_binary(&source.path, &expected_destination)?;
        let prepared = PreparedWasmAsset {
            session: self.session_id().into(),
            revision: self.revision,
            project: self.project.clone(),
            module_id: module_id.into(),
            source: expected_source,
            destination: expected_destination,
            target,
            asset,
            bytes: bytes.to_vec(),
        };
        self.check_wasm_asset(&prepared)?;
        Ok(prepared)
    }
    fn check_wasm_asset(&self, prepared: &PreparedWasmAsset) -> Result<(), CommandError> {
        self.ensure_sources_ready()?;
        if self.journal.is_none() || self.sources.is_none() {
            return Err(SourceError::Unavailable.into());
        }
        if self.session_id() != prepared.session
            || self.revision != prepared.revision
            || self.project != prepared.project
        {
            return Err(SourceError::Conflict.into());
        }
        if prepared.bytes.len() > incant_source::MAX_WASM_BYTES
            || prepared.bytes.get(..8) != Some(b"\0asm\x01\0\0\0")
        {
            return Err(CommandError::Invalid(
                "bounded compiled Wasm required".into(),
            ));
        }
        let source = self.read_module_source(&prepared.module_id)?;
        if source.source.is_none() || source.version != prepared.source {
            return Err(SourceError::Conflict.into());
        }
        let path = incant_source::wasm_path(&source.path)?;
        let asset = &prepared.asset;
        if asset.path != path || asset.kind != "wasm" || asset.import_settings.is_some() {
            return Err(CommandError::Invalid(
                "compiled asset must use the registered Rust entry's Wasm sibling".into(),
            ));
        }
        if self.project.assets.values().any(|other| {
            (other.id == asset.id && (other.path != path || other.kind != "wasm"))
                || (other.id != asset.id && other.path.eq_ignore_ascii_case(&path))
        }) || self
            .project
            .scripts
            .values()
            .any(|other| other.path.eq_ignore_ascii_case(&path))
            || self
                .project
                .module_sources
                .values()
                .any(|other| other.path.eq_ignore_ascii_case(&path))
        {
            return Err(CommandError::Invalid(
                "compiled asset aliases registered content".into(),
            ));
        }
        self.sources
            .as_ref()
            .ok_or(SourceError::Unavailable)?
            .check_binary_snapshot(&prepared.target)?;
        let actual = self.module_binary_version(&prepared.module_id)?;
        if actual != prepared.destination {
            return Err(SourceError::Conflict.into());
        }
        if actual != BinaryVersion::Missing && !self.project.assets.contains_key(&asset.id) {
            return Err(CommandError::Invalid(
                "unregistered existing Wasm destination".into(),
            ));
        }
        let mut after = self.project.clone();
        after.assets.insert(asset.id.clone(), asset.clone());
        after.validate()?;
        Ok(())
    }
    pub fn publish_wasm_asset(
        &mut self,
        prepared: PreparedWasmAsset,
        expected_revision: u64,
        actor: Actor,
        description: impl Into<String>,
    ) -> Result<&Transaction, CommandError> {
        if expected_revision != self.revision {
            return Err(CommandError::Conflict {
                expected: expected_revision,
                actual: self.revision,
            });
        }
        self.check_wasm_asset(&prepared)?;
        let id = new_id();
        actor.provenance(&id)?;
        let source_path = &self.project.module_sources[&prepared.module_id].path;
        let edit = self
            .sources
            .as_ref()
            .ok_or(SourceError::Unavailable)?
            .prepare_binary(
                &id,
                BinaryPublication {
                    module_id: &prepared.module_id,
                    asset_id: &prepared.asset.id,
                    source_path,
                    source_sha256: match &prepared.source {
                        SourceVersion::Present { sha256 } => sha256,
                        SourceVersion::Missing => return Err(SourceError::Conflict.into()),
                    },
                    snapshot: &prepared.target,
                    bytes: &prepared.bytes,
                },
            )?;
        let mut after = self.project.clone();
        after
            .assets
            .insert(prepared.asset.id.clone(), prepared.asset.clone());
        let transaction = Transaction {
            id,
            description: description.into(),
            actor,
            commands: vec![Command::UpsertAsset {
                asset: prepared.asset,
            }],
            before: self.project.clone(),
            after,
            source_edit: None,
            project_edit: None,
            binary_edit: Some(edit),
        };
        self.commit_transaction(transaction)?;
        Ok(self.history.last().expect("published binary transaction"))
    }
}
pub(super) fn validate_transaction(tx: &Transaction) -> Result<(), CommandError> {
    let Some(edit) = &tx.binary_edit else {
        return Ok(());
    };
    edit.validate()?;
    let path = edit.path()?;
    let Some(source) = tx.before.module_sources.get(edit.module_id()) else {
        return Err(CommandError::Corrupt("compiled source missing".into()));
    };
    let [Command::UpsertAsset { asset }] = tx.commands.as_slice() else {
        return Err(CommandError::Corrupt(
            "compiled publication commands".into(),
        ));
    };
    let mut expected = tx.before.clone();
    expected.assets.insert(asset.id.clone(), asset.clone());
    if tx.source_edit.is_some()
        || tx.project_edit.is_some()
        || source.path != edit.source_path()
        || asset.id != edit.asset_id()
        || asset.path != path
        || asset.kind != "wasm"
        || asset.import_settings.is_some()
        || expected != tx.after
        || !edit.is_publication()
    {
        return Err(CommandError::Corrupt("compiled publication binding".into()));
    }
    Ok(())
}

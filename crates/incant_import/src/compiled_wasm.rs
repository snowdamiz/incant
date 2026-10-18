//! Shared import admission for host-owned candidate bytes. No candidate path is
//! accepted and no project source is published before incant_cmd's durable WAL.
use super::*;
use incant_cmd::{BinaryVersion, PreparedWasmAsset, SourceVersion};

pub struct CompiledWasmRequest {
    pub module_id: String,
    pub expected_revision: u64,
    pub expected_source: SourceVersion,
    pub expected_destination: BinaryVersion,
}
pub struct PreparedCompiledWasm {
    prepared: PreparedWasmAsset,
    outcome: ImportOutcome,
    revision: u64,
}
impl PreparedCompiledWasm {
    /// Trusted host root only. The future service adapter resolves candidate IDs
    /// and revalidates the complete registered source snapshot before this call.
    pub fn prepare(
        bus: &CommandBus,
        root: &Path,
        request: CompiledWasmRequest,
        bytes: &[u8],
    ) -> Result<Self> {
        bus.check_source_root(root)?;
        if request.expected_revision != bus.revision() {
            return Err(CommandError::Conflict {
                expected: request.expected_revision,
                actual: bus.revision(),
            }
            .into());
        }
        let source = bus.read_module_source(&request.module_id)?;
        if source.source.is_none() || source.version != request.expected_source {
            return Err(CommandError::Source(incant_cmd::SourceError::Conflict).into());
        }
        let path = incant_cmd::wasm_path(&source.path).map_err(CommandError::from)?;
        let previous: Vec<_> = bus
            .project()
            .assets
            .values()
            .filter(|asset| asset.path.eq_ignore_ascii_case(&path))
            .collect();
        if previous.len() > 1
            || previous
                .first()
                .is_some_and(|asset| asset.path != path || asset.kind != "wasm")
        {
            return Err(ImportError::AmbiguousSource(path));
        }
        let previous = previous.first().copied();
        let (captured, cache_hit) =
            super::capture_wasm(root, None, bytes).map_err(|source| ImportError::Cook {
                path: path.clone(),
                source,
            })?;
        let asset = Asset {
            id: previous.map_or_else(new_id, |asset| asset.id.clone()),
            name: previous.map_or_else(
                || {
                    bus.project().module_sources[&request.module_id]
                        .name
                        .clone()
                },
                |asset| asset.name.clone(),
            ),
            path: path.clone(),
            kind: "wasm".into(),
            sha256: captured.fingerprint(),
            import_settings: None,
        };
        let prepared = bus.prepare_wasm_asset(
            &request.module_id,
            request.expected_source,
            request.expected_destination,
            asset.clone(),
            bytes,
        )?;
        let outcome = ImportOutcome {
            changed: true,
            asset,
            cache_hit,
            details: super::wasm_details(&path, &captured),
            diagnostics: vec![],
        };
        Ok(Self {
            prepared,
            outcome,
            revision: request.expected_revision,
        })
    }
    pub fn outcome(&self) -> &ImportOutcome {
        &self.outcome
    }
    pub fn commit(
        self,
        bus: &mut CommandBus,
        actor: Actor,
        description: impl Into<String>,
    ) -> Result<CommittedImports> {
        bus.publish_wasm_asset(self.prepared, self.revision, actor, description)?;
        Ok(CommittedImports {
            imports: vec![self.outcome],
            revision: bus.revision(),
        })
    }
}

//! One bounded compiled module, with exact nodes retained for reversible publication.
//! No binary path or slot is deserialized from a command/tool request.
use super::*;

pub const MAX_WASM_BYTES: usize = 1024 * 1024;
const MAX_BINARY_MARKER: usize = 16 * 1024;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum BinaryVersion {
    Missing,
    Present { sha256: String },
}
impl BinaryVersion {
    pub fn of(bytes: Option<&[u8]>) -> Self {
        bytes.map_or(Self::Missing, |bytes| Self::Present {
            sha256: format!("{:x}", Sha256::digest(bytes)),
        })
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Image {
    slot: String,
    bytes: u64,
    sha256: String,
    identity: [u64; 2],
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BinaryEdit {
    token: String,
    binding: String,
    parents: Vec<[u64; 2]>,
    module_id: String,
    asset_id: String,
    source_path: String,
    source_sha256: String,
    before: Option<Image>,
    after: Option<Image>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BinaryPending {
    pub operation: String,
    pub edit: BinaryEdit,
}
/// Nonserialized destination review: exact parent chain and named file identity.
pub struct BinarySnapshot {
    binding: String,
    source_path: String,
    parents: Vec<[u64; 2]>,
    image: Option<Image>,
}
/// Host/bus-only borrowed preparation inputs; never a wire capability.
pub struct BinaryPublication<'a> {
    pub module_id: &'a str,
    pub asset_id: &'a str,
    pub source_path: &'a str,
    pub source_sha256: &'a str,
    pub snapshot: &'a BinarySnapshot,
    pub bytes: &'a [u8],
}
pub fn wasm_path(source_path: &str) -> Result<String, SourceError> {
    validate_rust_path(source_path)?;
    let path = format!("{}.wasm", source_path.strip_suffix(".rs").unwrap());
    validate_portable_path(&path)?;
    Ok(path)
}
impl BinaryEdit {
    pub fn is_publication(&self) -> bool {
        self.after
            .as_ref()
            .is_some_and(|image| image.slot == format!("{}.wasm-after", self.token))
    }
    pub fn reversed(&self) -> Self {
        let mut result = self.clone();
        std::mem::swap(&mut result.before, &mut result.after);
        result
    }
    pub fn path(&self) -> Result<String, SourceError> {
        wasm_path(&self.source_path)
    }
    pub fn module_id(&self) -> &str {
        &self.module_id
    }
    pub fn asset_id(&self) -> &str {
        &self.asset_id
    }
    pub fn source_path(&self) -> &str {
        &self.source_path
    }
    pub fn validate(&self) -> Result<(), SourceError> {
        for id in [&self.token, &self.module_id, &self.asset_id] {
            if ulid::Ulid::from_string(id)
                .map(|id| id.to_string())
                .ok()
                .as_ref()
                != Some(id)
            {
                return Err(SourceError::Invalid("binary edit identity".into()));
            }
        }
        self.path()?;
        if self.source_sha256.len() != 64
            || !self
                .source_sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(SourceError::Invalid("compiled source digest".into()));
        }
        if self.binding.len() != 64
            || self.parents.len() != self.source_path.split('/').count()
            || self.before.is_none() && self.after.is_none()
        {
            return Err(SourceError::Invalid("binary edit framing".into()));
        }
        for image in [&self.before, &self.after].into_iter().flatten() {
            if ![
                format!("{}.wasm-before", self.token),
                format!("{}.wasm-after", self.token),
            ]
            .contains(&image.slot)
                || image.bytes > MAX_WASM_BYTES as u64
                || image.sha256.len() != 64
                || !image
                    .sha256
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            {
                return Err(SourceError::Invalid("binary image framing".into()));
            }
        }
        if self
            .before
            .as_ref()
            .zip(self.after.as_ref())
            .is_some_and(|(a, b)| a.slot == b.slot || a.identity == b.identity)
        {
            return Err(SourceError::Invalid("binary image alias".into()));
        }
        Ok(())
    }
}
fn read_image(dir: &Dir, name: &str, slot: &str) -> Result<Option<(Image, Vec<u8>)>, SourceError> {
    let mut file = match dir.open(name) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    platform::regular(&file)?;
    let before = file.metadata()?;
    if before.len() > MAX_WASM_BYTES as u64 {
        return Err(SourceError::Invalid("compiled module exceeds 1 MiB".into()));
    }
    let (device, node) = platform::identity(&file)?;
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take(MAX_WASM_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    let after = file.metadata()?;
    platform::regular(&file)?;
    // Re-open the named node too: unchanged bytes through an old handle do not
    // grant replacement authority over a racing path.
    let named = dir.open(name)?;
    platform::regular(&named)?;
    if bytes.len() > MAX_WASM_BYTES
        || before.len() != after.len()
        || before.modified()? != after.modified()?
        || platform::identity(&named)? != (device, node)
    {
        return Err(SourceError::Conflict);
    }
    Ok(Some((
        Image {
            slot: slot.into(),
            bytes: bytes.len() as u64,
            sha256: format!("{:x}", Sha256::digest(&bytes)),
            identity: [device, node],
        },
        bytes,
    )))
}
impl SourceStore {
    pub fn check_root(&self, root: &Path) -> Result<(), SourceError> {
        if Dir::root(root)?.identity()? != self.root.identity()? {
            return Err(SourceError::Conflict);
        }
        Ok(())
    }
    fn binary_parent(
        &self,
        source_path: &str,
    ) -> Result<(Dir, String, Vec<[u64; 2]>), SourceError> {
        let path = wasm_path(source_path)?;
        let mut parts = path.split('/').peekable();
        let mut parent = self.root.duplicate()?;
        let mut identities = vec![parent.identity()?];
        while let Some(part) = parts.next() {
            if parts.peek().is_none() {
                return Ok((parent, part.into(), identities));
            }
            parent = parent.child(part)?;
            identities.push(parent.identity()?);
        }
        Err(SourceError::Invalid("binary path".into()))
    }
    pub fn read_binary(&self, source_path: &str) -> Result<Option<Vec<u8>>, SourceError> {
        self.ensure_ready()?;
        let (parent, leaf, _) = self.binary_parent(source_path)?;
        Ok(read_image(&parent, &leaf, "")?.map(|(_, bytes)| bytes))
    }
    pub fn capture_binary(
        &self,
        source_path: &str,
        expected: &BinaryVersion,
    ) -> Result<BinarySnapshot, SourceError> {
        self.ensure_ready()?;
        let (parent, leaf, parents) = self.binary_parent(source_path)?;
        let actual = read_image(&parent, &leaf, "")?;
        if BinaryVersion::of(actual.as_ref().map(|(_, bytes)| bytes.as_slice())) != *expected {
            return Err(SourceError::Conflict);
        }
        Ok(BinarySnapshot {
            binding: self.binding.clone(),
            source_path: source_path.into(),
            parents,
            image: actual.map(|(image, _)| image),
        })
    }
    pub fn check_binary_snapshot(&self, snapshot: &BinarySnapshot) -> Result<(), SourceError> {
        let (parent, leaf, parents) = self.binary_parent(&snapshot.source_path)?;
        if self.binding != snapshot.binding
            || parents != snapshot.parents
            || !Self::matches_image(&parent, &leaf, snapshot.image.as_ref())?
        {
            return Err(SourceError::Conflict);
        }
        Ok(())
    }
    /// Bus-only preparation, after document/source/destination CAS. All writes
    /// here are confined to private transaction state, never project assets.
    pub fn prepare_binary(
        &self,
        token: &str,
        publication: BinaryPublication<'_>,
    ) -> Result<BinaryEdit, SourceError> {
        let BinaryPublication {
            module_id,
            asset_id,
            source_path,
            source_sha256,
            snapshot,
            bytes,
        } = publication;
        self.ensure_ready()?;
        if bytes.len() > MAX_WASM_BYTES || bytes.get(..8) != Some(b"\0asm\x01\0\0\0") {
            return Err(SourceError::Invalid("bounded Wasm binary required".into()));
        }
        for id in [token, module_id, asset_id] {
            if ulid::Ulid::from_string(id)
                .map(|id| id.to_string())
                .ok()
                .as_deref()
                != Some(id)
            {
                return Err(SourceError::Invalid("binary identity".into()));
            }
        }
        if source_sha256.len() != 64
            || !source_sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(SourceError::Invalid("compiled source digest".into()));
        }
        if snapshot.source_path != source_path {
            return Err(SourceError::Conflict);
        }
        self.check_binary_snapshot(snapshot)?;
        let parents = snapshot.parents.clone();
        let before = snapshot.image.clone().map(|mut image| {
            image.slot = format!("{token}.wasm-before");
            image
        });
        let slot = format!("{token}.wasm-after");
        write_new(&self.state, &slot, bytes)?;
        let after = read_image(&self.state, &slot, &slot)?
            .ok_or(SourceError::Conflict)?
            .0;
        let edit = BinaryEdit {
            token: token.into(),
            binding: self.binding.clone(),
            parents,
            module_id: module_id.into(),
            asset_id: asset_id.into(),
            source_path: source_path.into(),
            source_sha256: source_sha256.into(),
            before,
            after: Some(after),
        };
        edit.validate()?;
        write_new(
            &self.state,
            &format!("{token}.binary-ownership"),
            &bounded_json(&edit, MAX_BINARY_MARKER)?,
        )?;
        Ok(edit)
    }
    fn binary_authority(&self, edit: &BinaryEdit) -> Result<(Dir, String), SourceError> {
        edit.validate()?;
        let (parent, leaf, parents) = self.binary_parent(&edit.source_path)?;
        let raw = optional_read(
            &self.state,
            &format!("{}.binary-ownership", edit.token),
            MAX_BINARY_MARKER,
        )?
        .ok_or(SourceError::RecoveryRequired)?;
        let receipt: BinaryEdit =
            serde_json::from_str(&raw).map_err(|_| SourceError::RecoveryRequired)?;
        if edit.binding != self.binding
            || edit.parents != parents
            || (receipt != *edit && receipt != edit.reversed())
        {
            return Err(SourceError::RecoveryRequired);
        }
        Ok((parent, leaf))
    }
    fn matches_image(dir: &Dir, name: &str, image: Option<&Image>) -> Result<bool, SourceError> {
        let actual = read_image(dir, name, image.map_or("", |image| image.slot.as_str()))?;
        Ok(actual.as_ref().map(|(image, _)| image) == image)
    }
    fn check_binary_source(&self, edit: &BinaryEdit) -> Result<(), SourceError> {
        if edit.is_publication() {
            let source = self.read(&edit.source_path)?.ok_or(SourceError::Conflict)?;
            if format!("{:x}", Sha256::digest(source.as_bytes())) != edit.source_sha256 {
                return Err(SourceError::Conflict);
            }
        }
        Ok(())
    }
    pub fn check_binary_preimage(&self, edit: &BinaryEdit) -> Result<(), SourceError> {
        self.check_binary_source(edit)?;
        let (parent, leaf) = self.binary_authority(edit)?;
        if !Self::matches_image(&parent, &leaf, edit.before.as_ref())? {
            return Err(SourceError::Conflict);
        }
        if let Some(after) = &edit.after
            && !Self::matches_image(&self.state, &after.slot, Some(after))?
        {
            return Err(SourceError::RecoveryRequired);
        }
        if let Some(before) = &edit.before
            && read_image(&self.state, &before.slot, "")?.is_some()
        {
            return Err(SourceError::RecoveryRequired);
        }
        Ok(())
    }
    pub fn binary_pending(&self) -> Result<Option<BinaryPending>, SourceError> {
        let Some(raw) = optional_read(&self.state, "binary.pending", MAX_BINARY_MARKER)
            .map_err(|_| SourceError::RecoveryRequired)?
        else {
            return Ok(None);
        };
        // Recovery must not resolve one protocol while another owns the root.
        // Do not parse or remove the conflicting marker to make progress.
        for name in ["pending", "project.pending"] {
            if !matches!(optional_read(&self.state, name, 0), Ok(None)) {
                return Err(SourceError::RecoveryRequired);
            }
        }
        let pending: BinaryPending =
            serde_json::from_str(&raw).map_err(|_| SourceError::RecoveryRequired)?;
        if ulid::Ulid::from_string(&pending.operation).is_err() {
            return Err(SourceError::RecoveryRequired);
        }
        self.binary_authority(&pending.edit)
            .map_err(|_| SourceError::RecoveryRequired)?;
        Ok(Some(pending))
    }
    fn verify_binary_pending(&self, operation: &str, edit: &BinaryEdit) -> Result<(), SourceError> {
        if self
            .binary_pending()?
            .is_none_or(|p| p.operation != operation || p.edit != *edit)
        {
            return Err(SourceError::RecoveryRequired);
        }
        Ok(())
    }
    pub fn begin_binary(&self, operation: &str, edit: &BinaryEdit) -> Result<(), SourceError> {
        self.ensure_ready()?;
        if ulid::Ulid::from_string(operation).is_err() {
            return Err(SourceError::Invalid("binary operation".into()));
        }
        self.check_binary_preimage(edit)?;
        let pending = BinaryPending {
            operation: operation.into(),
            edit: edit.clone(),
        };
        let name = format!("{operation}.binary-marker");
        write_new(
            &self.state,
            &name,
            &bounded_json(&pending, MAX_BINARY_MARKER)?,
        )?;
        self.state.move_to(&name, &self.state, "binary.pending")?;
        Ok(())
    }
    pub fn install_binary(&self, operation: &str, edit: &BinaryEdit) -> Result<(), SourceError> {
        self.verify_binary_pending(operation, edit)?;
        self.check_binary_preimage(edit)?;
        let (parent, leaf) = self.binary_authority(edit)?;
        if let Some(before) = &edit.before {
            parent.move_to(&leaf, &self.state, &before.slot)?;
            #[cfg(test)]
            crash_point("binary_displaced");
            if !Self::matches_image(&self.state, &before.slot, Some(before))? {
                return Err(SourceError::Conflict);
            }
        }
        if let Some(after) = &edit.after {
            self.state.move_to(&after.slot, &parent, &leaf)?;
        }
        let (parent, leaf) = self.binary_authority(edit)?;
        if !Self::matches_image(&parent, &leaf, edit.after.as_ref())? {
            return Err(SourceError::Conflict);
        }
        self.check_binary_source(edit)?;
        Ok(())
    }
    pub fn rollback_binary(&self, operation: &str, edit: &BinaryEdit) -> Result<(), SourceError> {
        self.verify_binary_pending(operation, edit)?;
        let (parent, leaf) = self.binary_authority(edit)?;
        if Self::matches_image(&parent, &leaf, edit.before.as_ref())? {
            return Ok(());
        }
        if let Some(before) = &edit.before
            && !Self::matches_image(&self.state, &before.slot, Some(before))?
        {
            return Err(SourceError::RecoveryRequired);
        }
        let current = read_image(
            &parent,
            &leaf,
            edit.after.as_ref().map_or("", |image| image.slot.as_str()),
        )?;
        if let Some((current, _)) = current {
            if Some(&current) != edit.after.as_ref() {
                return Err(SourceError::RecoveryRequired);
            }
            parent.move_to(&leaf, &self.state, &current.slot)?;
            if !Self::matches_image(&self.state, &current.slot, Some(&current))? {
                return Err(SourceError::RecoveryRequired);
            }
        }
        if let Some(before) = &edit.before {
            self.state.move_to(&before.slot, &parent, &leaf)?;
        }
        let (parent, leaf) = self.binary_authority(edit)?;
        if !Self::matches_image(&parent, &leaf, edit.before.as_ref())? {
            return Err(SourceError::RecoveryRequired);
        }
        Ok(())
    }
    pub fn finish_binary(&self, operation: &str) -> Result<(), SourceError> {
        if self
            .binary_pending()?
            .is_none_or(|p| p.operation != operation)
        {
            return Err(SourceError::RecoveryRequired);
        }
        self.publish_receipt(operation)?;
        self.state.remove("binary.pending")?;
        Ok(())
    }
}

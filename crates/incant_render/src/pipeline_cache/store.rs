use super::{CacheRead, private::Directory};
use sha2::{Digest, Sha256};
use std::{io, path::PathBuf};
const MAGIC: &[u8; 8] = b"INCPIPC1";
const HEADER: usize = 8 + 32 + 8 + 32;
const MAX_BYTES: usize = 64 * 1024 * 1024;

pub(super) fn application_directory() -> Option<PathBuf> {
    let absolute = |key| {
        std::env::var_os(key)
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
    };
    let base = absolute("XDG_CACHE_HOME").or_else(|| absolute("HOME").map(|p| p.join(".cache")));
    base.map(|p| p.join("incant/pipelines-v1"))
}

pub(super) struct Store {
    directory: PathBuf,
    identity: [u8; 32],
}
impl Store {
    pub fn new(directory: PathBuf, adapter: &wgpu::AdapterInfo) -> Self {
        // Fields are length-delimited; names cannot create path components or
        // ambiguous identities. Driver upgrades get a cold, distinct entry.
        let mut digest = Sha256::new();
        for field in [
            "incant-pipeline-cache-v1",
            "wgpu29",
            &format!("{:?}", adapter.backend),
            &adapter.vendor.to_string(),
            &adapter.device.to_string(),
            &adapter.name,
            &adapter.driver,
            &adapter.driver_info,
        ] {
            digest.update((field.len() as u64).to_le_bytes());
            digest.update(field.as_bytes());
        }
        Self {
            directory,
            identity: digest.finalize().into(),
        }
    }
    fn name(&self) -> String {
        let name: String = self.identity.iter().map(|b| format!("{b:02x}")).collect();
        format!("{name}.cache")
    }
    #[cfg(test)]
    fn path(&self) -> PathBuf {
        self.directory.join(self.name())
    }
    pub fn load(&self) -> (Option<Vec<u8>>, CacheRead) {
        match self.read() {
            Ok(Some(bytes)) => {
                let count = bytes.len();
                (Some(bytes), CacheRead::Supplied { bytes: count })
            }
            Ok(None) => (None, CacheRead::Rejected),
            Err(error) if error.kind() == io::ErrorKind::NotFound => (None, CacheRead::Missing),
            Err(error) => (None, CacheRead::Io(error.kind())),
        }
    }
    fn read(&self) -> io::Result<Option<Vec<u8>>> {
        let directory = Directory::open(&self.directory, false)?;
        let bytes = directory.read(&self.name(), HEADER + MAX_BYTES)?;
        Ok(decode(&bytes, &self.identity).map(<[u8]>::to_vec))
    }
    pub fn save(&self, data: &[u8]) -> io::Result<()> {
        if data.is_empty() || data.len() > MAX_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "driver cache length",
            ));
        }
        let directory = Directory::open(&self.directory, true)?;
        directory.publish(
            &self.name(),
            &[
                MAGIC,
                &self.identity,
                &(data.len() as u64).to_le_bytes(),
                &Sha256::digest(data),
                data,
            ],
        )
    }
}
fn decode<'a>(data: &'a [u8], identity: &[u8; 32]) -> Option<&'a [u8]> {
    if data.len() < HEADER || &data[..8] != MAGIC || &data[8..40] != identity {
        return None;
    }
    let length = u64::from_le_bytes(data[40..48].try_into().ok()?);
    if length == 0 || length > MAX_BYTES as u64 || data.len() != HEADER + length as usize {
        return None;
    }
    let payload = &data[HEADER..];
    (data[48..HEADER] == Sha256::digest(payload)[..]).then_some(payload)
}

#[cfg(test)]
#[path = "store_tests.rs"]
mod tests;

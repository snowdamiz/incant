//! Incant-only credentials. macOS uses private local files at the director's
//! request; other platforms use OS stores with a small manifest committed last.
//! Windows' 2560-byte credential limit is smaller than a complete OAuth record.
//! Bounded chunks also let a failed refresh write preserve the previous session.
use crate::AgentError;
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;
const PREFIX: &str = "incant-keychain:v1:";
const CHUNK: usize = 900; // <=1800 UTF-16 bytes, below Windows' 2560-byte limit.
const MAX_PARTS: usize = 128;
pub struct CredentialStore;
#[derive(Serialize, Deserialize)]
struct Manifest {
    generation: String,
    parts: usize,
}
pub(super) trait Backend {
    fn get(&self, account: &str) -> Result<Option<Zeroizing<String>>, AgentError>;
    fn put(&self, account: &str, secret: &str) -> Result<(), AgentError>;
    fn remove(&self, account: &str) -> Result<(), AgentError>;
}
#[cfg(not(target_os = "macos"))]
struct OsStore;
pub(super) fn error(message: &str) -> AgentError {
    AgentError::Provider(message.into())
}
#[cfg(not(target_os = "macos"))]
fn entry(account: &str) -> Result<keyring::Entry, AgentError> {
    if account.is_empty() || account.len() > 250 || account.contains('\0') {
        return Err(error("invalid credential account ID"));
    }
    keyring::Entry::new("dev.incant.openai", account)
        .map_err(|_| error("OS credential store unavailable"))
}
#[cfg(not(target_os = "macos"))]
impl Backend for OsStore {
    fn get(&self, account: &str) -> Result<Option<Zeroizing<String>>, AgentError> {
        match entry(account)?.get_password() {
            Ok(secret) => Ok(Some(Zeroizing::new(secret))),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(_) => Err(error(
                "OS keychain is unavailable or locked. Unlock it and retry; saved credentials have not been removed.",
            )),
        }
    }
    fn put(&self, account: &str, secret: &str) -> Result<(), AgentError> {
        entry(account)?
            .set_password(secret)
            .map_err(|_| error("Failed to save credential in the OS keychain"))
    }
    fn remove(&self, account: &str) -> Result<(), AgentError> {
        match entry(account)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => Err(error("Failed to delete credential from the OS keychain")),
        }
    }
}
fn manifest(value: &str) -> Result<Option<Manifest>, AgentError> {
    let Some(raw) = value.strip_prefix(PREFIX) else {
        return Ok(None);
    };
    let m: Manifest =
        serde_json::from_str(raw).map_err(|_| error("Invalid Incant credential manifest"))?;
    if m.generation.len() != 32
        || !m.generation.bytes().all(|b| b.is_ascii_hexdigit())
        || !(1..=MAX_PARTS).contains(&m.parts)
    {
        return Err(error("Invalid Incant credential manifest"));
    }
    Ok(Some(m))
}
fn part(account: &str, m: &Manifest, index: usize) -> String {
    format!("{account}:{}:{index}", m.generation)
}
fn remove_parts(backend: &impl Backend, account: &str, m: &Manifest) -> Result<(), AgentError> {
    for n in 0..m.parts {
        backend.remove(&part(account, m, n))?;
    }
    Ok(())
}
fn store(backend: &impl Backend, account: &str, secret: &str) -> Result<(), AgentError> {
    if account.is_empty()
        || account.len() > 200
        || account.contains('\0')
        || secret.is_empty()
        || secret.len() > CHUNK * MAX_PARTS
    {
        return Err(error("Invalid credential size or account ID"));
    }
    let old = backend.get(account)?;
    let old_manifest = old.as_deref().map(|s| manifest(s)).transpose()?.flatten();
    let mut remaining = secret;
    let mut chunks = Vec::new();
    while !remaining.is_empty() {
        let mut end = remaining.len().min(CHUNK);
        while !remaining.is_char_boundary(end) {
            end -= 1;
        }
        chunks.push(&remaining[..end]);
        remaining = &remaining[end..];
    }
    if chunks.len() > MAX_PARTS {
        return Err(error("Credential exceeds supported size"));
    }
    let m = Manifest {
        generation: uuid::Uuid::new_v4().simple().to_string(),
        parts: chunks.len(),
    };
    let result = (|| {
        for (n, chunk) in chunks.into_iter().enumerate() {
            backend.put(&part(account, &m, n), chunk)?;
        }
        backend.put(
            account,
            &format!(
                "{PREFIX}{}",
                serde_json::to_string(&m).expect("manifest serialization")
            ),
        )
    })();
    if result.is_err() {
        let _ = remove_parts(backend, account, &m);
    } else if let Some(old) = old_manifest {
        // A cleanup failure cannot invalidate the newly committed rotating token.
        let _ = remove_parts(backend, account, &old);
    }
    result
}
fn load(backend: &impl Backend, account: &str) -> Result<Option<Zeroizing<String>>, AgentError> {
    let Some(saved) = backend.get(account)? else {
        return Ok(None);
    };
    let Some(m) = manifest(&saved)? else {
        return Ok(Some(saved));
    }; // Read pre-migration entries.
    let mut secret = Zeroizing::new(String::new());
    for n in 0..m.parts {
        let chunk = backend
            .get(&part(account, &m, n))?
            .ok_or_else(|| error("Saved credential is incomplete. Reconnect this account."))?;
        if chunk.len() > CHUNK {
            return Err(error("Invalid credential chunk"));
        }
        secret.push_str(&chunk);
    }
    Ok(Some(secret))
}
#[cfg(target_os = "macos")]
fn backend() -> Result<impl Backend, AgentError> {
    crate::local_credentials::LocalStore::new()
}
#[cfg(not(target_os = "macos"))]
fn backend() -> Result<impl Backend, AgentError> {
    Ok(OsStore)
}
impl CredentialStore {
    pub fn storage_kind() -> &'static str {
        if cfg!(target_os = "macos") {
            "private-local-file"
        } else {
            "os-keychain"
        }
    }
    pub fn save(account: &str, secret: &str) -> Result<(), AgentError> {
        let backend = backend()?;
        if cfg!(target_os = "macos") {
            backend.put(account, secret)
        } else {
            store(&backend, account, secret)
        }
    }
    pub fn load_optional(account: &str) -> Result<Option<Zeroizing<String>>, AgentError> {
        load(&backend()?, account)
    }
    pub fn load(account: &str) -> Result<Zeroizing<String>, AgentError> {
        Self::load_optional(account)?.ok_or_else(|| {
            error("Saved credentials are missing. Continue with ChatGPT to reconnect.")
        })
    }
    pub fn delete(account: &str) -> Result<(), AgentError> {
        let backend = backend()?;
        if let Some(saved) = backend.get(account)?
            && let Some(m) = manifest(&saved)?
        {
            remove_parts(&backend, account, &m)?;
        }
        backend.remove(account)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        cell::{Cell, RefCell},
        collections::BTreeMap,
    };
    #[derive(Default)]
    struct Memory {
        entries: RefCell<BTreeMap<String, String>>,
        writes: Cell<usize>,
        fail_at: Cell<usize>,
    }
    impl Backend for Memory {
        fn get(&self, id: &str) -> Result<Option<Zeroizing<String>>, AgentError> {
            Ok(self.entries.borrow().get(id).cloned().map(Zeroizing::new))
        }
        fn put(&self, id: &str, secret: &str) -> Result<(), AgentError> {
            let n = self.writes.get() + 1;
            self.writes.set(n);
            if n == self.fail_at.get() {
                return Err(error("simulated storage failure"));
            }
            assert!(secret.encode_utf16().count() * 2 <= 2560);
            self.entries.borrow_mut().insert(id.into(), secret.into());
            Ok(())
        }
        fn remove(&self, id: &str) -> Result<(), AgentError> {
            self.entries.borrow_mut().remove(id);
            Ok(())
        }
    }
    #[test]
    fn large_unicode_record_roundtrips_and_replacement_cleans_old_generation() {
        let backend = Memory::default();
        let secret = "OAuth.fake.🔑.not-a-real-token.".repeat(1000);
        store(&backend, "account", &secret).unwrap();
        assert_eq!(load(&backend, "account").unwrap().unwrap().as_str(), secret);
        store(&backend, "account", "replacement").unwrap();
        assert_eq!(
            load(&backend, "account").unwrap().unwrap().as_str(),
            "replacement"
        );
        assert_eq!(backend.entries.borrow().len(), 2);
    }
    #[test]
    fn interrupted_refresh_keeps_original_credential_for_every_failed_write() {
        for fail_at in 1..=5 {
            let backend = Memory::default();
            backend
                .entries
                .borrow_mut()
                .insert("account".into(), "legacy-secret".into());
            backend.fail_at.set(fail_at);
            assert!(store(&backend, "account", &"x".repeat(3000)).is_err());
            assert_eq!(
                load(&backend, "account").unwrap().unwrap().as_str(),
                "legacy-secret"
            );
            assert_eq!(backend.entries.borrow().len(), 1);
        }
    }
    #[test]
    fn invalid_manifest_cannot_read_other_account_parts() {
        let backend = Memory::default();
        backend.entries.borrow_mut().insert(
            "account".into(),
            format!("{PREFIX}{{\"generation\":\"../foreign\",\"parts\":999999}}"),
        );
        assert!(load(&backend, "account").is_err());
    }
}

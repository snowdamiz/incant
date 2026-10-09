//! One persistent account store shared by every Incant binary and checkout.
//! Metadata is outside the project/build tree; credentials stay in the private platform store.
use crate::{
    AgentError,
    auth::{self, AccountMetadata, LoginAttempt},
    credentials::CredentialStore,
    provider::OpenAiProvider,
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

#[derive(Default, Serialize, Deserialize)]
pub struct Accounts {
    pub host_id: String,
    pub active: Option<String>,
    pub accounts: Vec<AccountMetadata>,
    pub api_key_connected: bool,
}
pub struct AccountStore {
    path: PathBuf,
    pub data: Accounts,
    _lock: fs::File,
}
#[derive(Debug, PartialEq, Eq)]
pub enum RemoteRevocation {
    Confirmed,
    Unconfirmed,
    NotApplicable,
}
fn error(message: &str) -> AgentError {
    AgentError::Provider(message.into())
}
impl AccountStore {
    pub fn open() -> Result<Self, AgentError> {
        let dir = dirs::config_dir()
            .ok_or_else(|| error("OS configuration directory unavailable"))?
            .join("Incant");
        Self::open_at(&dir)
    }
    fn open_at(dir: &Path) -> Result<Self, AgentError> {
        fs::create_dir_all(dir).map_err(|_| error("Could not create Incant account directory"))?;
        let lock = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(dir.join("accounts.lock"))
            .map_err(|_| error("Could not open Incant account lock"))?;
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            match lock.try_lock() {
                Ok(()) => break,
                Err(std::fs::TryLockError::WouldBlock) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(25))
                }
                Err(_) => {
                    return Err(error(
                        "Another Incant process is updating the account. Try again shortly.",
                    ));
                }
            }
        }
        let path = dir.join("accounts.json");
        let data = match fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(|_| {
                error("Incant account metadata is damaged; it has not been overwritten")
            })?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Accounts::default(),
            Err(_) => return Err(error("Could not read Incant account metadata")),
        };
        let mut store = Self {
            path,
            data,
            _lock: lock,
        };
        // The original spike emitted incant:<ULID>, rejected by OpenAI. It could
        // not register a client. Migrate only unregistered hosts, never a live one.
        if !auth::valid_host_id(&store.data.host_id) {
            if !store.data.accounts.is_empty() {
                return Err(error(
                    "Saved registration has an unsupported host ID; retained without modification",
                ));
            }
            store.data.host_id = format!("urn:uuid:{}", uuid::Uuid::new_v4());
            store.save()?;
        }
        Ok(store)
    }
    pub fn save(&self) -> Result<(), AgentError> {
        let bytes = serde_json::to_vec_pretty(&self.data)
            .map_err(|_| error("Could not encode account metadata"))?;
        let parent = self.path.parent().expect("account parent");
        let mut file = tempfile::NamedTempFile::new_in(parent)
            .map_err(|_| error("Could not save account metadata"))?;
        // NamedTempFile is owner-only on Unix and replacement is atomic.
        file.write_all(&bytes)
            .and_then(|()| file.as_file().sync_all())
            .map_err(|_| error("Could not save account metadata"))?;
        file.persist(&self.path)
            .map_err(|_| error("Could not replace account metadata"))?;
        Ok(())
    }
    pub fn selected(&self) -> Option<&AccountMetadata> {
        self.data
            .active
            .as_ref()
            .and_then(|id| self.data.accounts.iter().find(|a| &a.id == id))
    }
    pub fn prepare_login(
        &self,
        add: bool,
        account_id: Option<&str>,
    ) -> Result<(LoginAttempt, Option<String>), AgentError> {
        let existing = if add {
            None
        } else if let Some(id) = account_id {
            Some(
                self.data
                    .accounts
                    .iter()
                    .find(|a| a.id == id)
                    .ok_or_else(|| error("Unknown saved account"))?,
            )
        } else {
            self.selected().or_else(|| self.data.accounts.last())
        };
        let mut attempt =
            LoginAttempt::new(&self.data.host_id, existing.map(|a| a.client_id.as_str()))?;
        if let Some(account) = existing
            && let Some(secret) = CredentialStore::load_optional(&account.id)?
        {
            let record: auth::OAuthCredential = serde_json::from_str(&secret)
                .map_err(|_| error("Invalid saved credential record"))?;
            if record.client_id != account.client_id || record.subject != account.subject {
                return Err(error("Stored account identity mismatch"));
            }
            attempt.set_id_token_hint(&record.id_token);
        }
        Ok((attempt, existing.map(|a| a.subject.clone())))
    }
    pub fn activate(&mut self, account: AccountMetadata) -> Result<(), AgentError> {
        self.data.active = Some(account.id.clone());
        self.data.accounts.retain(|a| a.id != account.id);
        self.data.accounts.push(account);
        self.save()
    }
    pub fn switch(&mut self, id: &str) -> Result<bool, AgentError> {
        let account = self
            .data
            .accounts
            .iter()
            .find(|a| a.id == id)
            .ok_or_else(|| error("Unknown saved account"))?;
        if CredentialStore::load_optional(id)?.is_none() {
            return Ok(false);
        }
        auth::access_token(account)?;
        self.data.active = Some(id.into());
        self.save()?;
        Ok(true)
    }
    /// Consumes the store so its lock is released before the provider needs to
    /// reopen it for a request. Retaining that lock would time out token renewal.
    pub fn provider(self, model: String) -> Result<OpenAiProvider, AgentError> {
        if let Some(account) = self.selected() {
            // Validate while this lock is held; the provider reloads/renews the
            // same active registration before each later network request.
            auth::access_token(account)?;
            OpenAiProvider::for_account(account.id.clone(), model)
        } else if self.data.api_key_connected {
            CredentialStore::load("api-key")?;
            OpenAiProvider::for_saved_api_key(model)
        } else {
            Err(error(
                "No OpenAI connection. Continue with ChatGPT in Incant, or run incant auth login.",
            ))
        }
    }
    pub fn disconnect(&mut self) -> Result<RemoteRevocation, AgentError> {
        self.disconnect_with(auth::disconnect, || CredentialStore::delete("api-key"))
    }
    fn disconnect_with(
        &mut self,
        disconnect_oauth: impl FnOnce(&AccountMetadata) -> Result<bool, AgentError>,
        disconnect_key: impl FnOnce() -> Result<(), AgentError>,
    ) -> Result<RemoteRevocation, AgentError> {
        let revoked = if let Some(account) = self.selected() {
            if disconnect_oauth(account)? {
                RemoteRevocation::Confirmed
            } else {
                RemoteRevocation::Unconfirmed
            }
        } else if self.data.api_key_connected {
            disconnect_key()?;
            RemoteRevocation::NotApplicable
        } else {
            RemoteRevocation::NotApplicable
        };
        self.data.active = None;
        // An older API key may still be stored while OAuth is selected. Signing
        // out must not silently activate that fallback or resume billable calls.
        // Other saved credentials/registrations remain inactive for explicit use.
        self.data.api_key_connected = false;
        self.save()?;
        Ok(revoked)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalid_spike_host_migrates_once_and_survives_new_store_instances() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("accounts.json"),
            r#"{"host_id":"incant:01OLD","active":null,"accounts":[],"api_key_connected":false}"#,
        )
        .unwrap();
        let first = AccountStore::open_at(dir.path()).unwrap();
        let id = first.data.host_id.clone();
        assert!(auth::valid_host_id(&id));
        drop(first);
        let second = AccountStore::open_at(dir.path()).unwrap();
        assert_eq!(second.data.host_id, id);
        let metadata = fs::read_to_string(dir.path().join("accounts.json")).unwrap();
        assert!(!metadata.contains("token"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&second.path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
    }
    #[test]
    fn registration_and_selection_persist_without_token_fields() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = AccountStore::open_at(dir.path()).unwrap();
        store
            .activate(AccountMetadata {
                id: "oauth-test".into(),
                client_id: "oaiapp_test".into(),
                subject: "user-test".into(),
                label: "Test account".into(),
            })
            .unwrap();
        drop(store);
        let store = AccountStore::open_at(dir.path()).unwrap();
        assert_eq!(store.selected().unwrap().client_id, "oaiapp_test");
        assert_eq!(store.selected().unwrap().label, "Test account");
    }
    #[test]
    fn damaged_metadata_is_not_silently_replaced() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("accounts.json");
        fs::write(&path, "broken").unwrap();
        assert!(AccountStore::open_at(dir.path()).is_err());
        assert_eq!(fs::read_to_string(path).unwrap(), "broken");
    }

    #[test]
    fn oauth_signout_stays_signed_out_after_reopen_with_an_older_api_key() {
        for remote in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            let mut store = AccountStore::open_at(dir.path()).unwrap();
            store.data.api_key_connected = true;
            store
                .activate(AccountMetadata {
                    id: "oauth-test".into(),
                    client_id: "oaiapp_test".into(),
                    subject: "user-test".into(),
                    label: "Test".into(),
                })
                .unwrap();
            let outcome = store
                .disconnect_with(
                    |account| {
                        assert_eq!(account.id, "oauth-test");
                        Ok(remote)
                    },
                    || panic!("Do not delete an inactive account's credential"),
                )
                .unwrap();
            assert_eq!(
                outcome,
                if remote {
                    RemoteRevocation::Confirmed
                } else {
                    RemoteRevocation::Unconfirmed
                }
            );
            drop(store);
            let store = AccountStore::open_at(dir.path()).unwrap();
            assert!(store.data.active.is_none());
            assert!(!store.data.api_key_connected);
            assert_eq!(
                store.data.accounts.len(),
                1,
                "Registration is retained for explicit reconnection"
            );
            // This must fail before opening any real credential backend or network.
            assert!(store.provider("test".into()).is_err());
        }
    }

    #[test]
    fn failed_local_signout_preserves_selected_account_for_retry() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = AccountStore::open_at(dir.path()).unwrap();
        store
            .activate(AccountMetadata {
                id: "oauth-test".into(),
                client_id: "oaiapp_test".into(),
                subject: "user-test".into(),
                label: "Test".into(),
            })
            .unwrap();
        assert!(
            store
                .disconnect_with(
                    |_| Err(error("Temporary credential-store error")),
                    || panic!("OAuth account is selected"),
                )
                .is_err()
        );
        drop(store);
        assert_eq!(
            AccountStore::open_at(dir.path())
                .unwrap()
                .data
                .active
                .as_deref(),
            Some("oauth-test")
        );
    }

    #[test]
    fn api_key_signout_deletes_locally_without_claiming_remote_revocation() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = AccountStore::open_at(dir.path()).unwrap();
        store.data.api_key_connected = true;
        store.save().unwrap();
        let deleted = std::cell::Cell::new(false);
        let outcome = store
            .disconnect_with(
                |_| panic!("No OAuth account is selected"),
                || {
                    deleted.set(true);
                    Ok(())
                },
            )
            .unwrap();
        assert!(deleted.get());
        assert_eq!(outcome, RemoteRevocation::NotApplicable);
        drop(store);
        assert!(
            !AccountStore::open_at(dir.path())
                .unwrap()
                .data
                .api_key_connected
        );
    }
}

//! macOS development credential storage, per the director's explicit request to
//! stop Keychain prompts after every rebuild. Uses OpenAI's documented protected
//! local-file approach. This module never imports or calls Keychain APIs.
use super::credentials::{Backend, error};
use crate::AgentError;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt},
    path::PathBuf,
};
use zeroize::Zeroizing;
pub(super) struct LocalStore {
    directory: PathBuf,
}
impl LocalStore {
    pub(super) fn new() -> Result<Self, AgentError> {
        let directory = dirs::config_dir()
            .ok_or_else(|| error("OS configuration directory unavailable"))?
            .join("Incant")
            .join("credentials");
        Self::at(directory)
    }
    fn at(directory: PathBuf) -> Result<Self, AgentError> {
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(&directory)
            .map_err(|_| error("Could not create private Incant credential directory"))?;
        let meta = fs::symlink_metadata(&directory)
            .map_err(|_| error("Could not inspect credential directory"))?;
        if !meta.is_dir() || meta.file_type().is_symlink() {
            return Err(error(
                "Credential directory must be a real private directory",
            ));
        }
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))
            .map_err(|_| error("Could not protect credential directory"))?;
        Ok(Self { directory })
    }
    fn path(&self, account: &str) -> Result<PathBuf, AgentError> {
        if account.is_empty() || account.len() > 200 || account.contains('\0') {
            return Err(error("Invalid credential account ID"));
        }
        Ok(self
            .directory
            .join(format!("{:x}.json", Sha256::digest(account.as_bytes()))))
    }
}
impl Backend for LocalStore {
    fn get(&self, account: &str) -> Result<Option<Zeroizing<String>>, AgentError> {
        let path = self.path(account)?;
        let mut file = match fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW)
            .open(path)
        {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(error("Could not read Incant's private credential file")),
        };
        let meta = file
            .metadata()
            .map_err(|_| error("Could not inspect credential file"))?;
        if !meta.is_file() || meta.permissions().mode() & 0o077 != 0 || meta.len() > 128 * 1024 {
            return Err(error(
                "Credential file must be private to this user and within the supported size",
            ));
        }
        let mut secret = Zeroizing::new(String::new());
        file.read_to_string(&mut secret)
            .map_err(|_| error("Could not read credential file"))?;
        Ok(Some(secret))
    }
    fn put(&self, account: &str, secret: &str) -> Result<(), AgentError> {
        if secret.is_empty() || secret.len() > 128 * 1024 {
            return Err(error("Invalid credential size"));
        }
        let destination = self.path(account)?;
        let mut file = tempfile::NamedTempFile::new_in(&self.directory)
            .map_err(|_| error("Could not prepare private credential file"))?;
        file.as_file()
            .set_permissions(fs::Permissions::from_mode(0o600))
            .map_err(|_| error("Could not protect credential file"))?;
        file.write_all(secret.as_bytes())
            .and_then(|()| file.as_file().sync_all())
            .map_err(|_| error("Could not save credential file"))?;
        file.persist(destination)
            .map_err(|_| error("Could not commit credential file"))?;
        fs::File::open(&self.directory)
            .and_then(|d| d.sync_all())
            .map_err(|_| error("Could not sync credential directory"))?;
        Ok(())
    }
    fn remove(&self, account: &str) -> Result<(), AgentError> {
        match fs::remove_file(self.path(account)?) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(_) => Err(error("Could not remove local credential file")),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn private_record_survives_reopen_and_is_replaced_atomically() {
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path().join("private");
        let fake = "fake-oauth-token".repeat(2048);
        LocalStore::at(dir.clone())
            .unwrap()
            .put("account", &fake)
            .unwrap();
        let restarted = LocalStore::at(dir.clone()).unwrap();
        assert_eq!(restarted.get("account").unwrap().unwrap().as_str(), fake);
        assert_eq!(
            fs::metadata(&dir).unwrap().permissions().mode() & 0o777,
            0o700
        );
        assert_eq!(
            fs::metadata(restarted.path("account").unwrap())
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        restarted.put("account", "rotated-fake-token").unwrap();
        assert_eq!(
            restarted.get("account").unwrap().unwrap().as_str(),
            "rotated-fake-token"
        );
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 1);
        restarted.remove("account").unwrap();
        assert!(restarted.get("account").unwrap().is_none());
    }
    #[test]
    fn refuses_symlinks_and_publicly_readable_credentials() {
        let temp = tempfile::tempdir().unwrap();
        let store = LocalStore::at(temp.path().join("private")).unwrap();
        let outside = temp.path().join("outside");
        fs::write(&outside, "do not read").unwrap();
        std::os::unix::fs::symlink(&outside, store.path("account").unwrap()).unwrap();
        assert!(store.get("account").is_err());
        store.put("account", "private").unwrap();
        assert_eq!(fs::read_to_string(&outside).unwrap(), "do not read");
        fs::set_permissions(
            store.path("account").unwrap(),
            fs::Permissions::from_mode(0o644),
        )
        .unwrap();
        assert!(store.get("account").is_err());
    }
}

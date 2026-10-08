use crate::AgentError;
use zeroize::Zeroizing;
/// Stores only under Incant's own service namespace. No discovery or reuse of
/// Codex/Claude credentials is permitted by this API.
pub struct CredentialStore;
impl CredentialStore {
    fn entry(account: &str) -> Result<keyring::Entry, AgentError> {
        if account.is_empty() || account.len() > 200 || account.contains('\0') {
            return Err(AgentError::Provider("invalid credential account ID".into()));
        }
        keyring::Entry::new("dev.incant.openai", account)
            .map_err(|_| AgentError::Provider("OS credential store unavailable".into()))
    }
    pub fn save(account: &str, secret: &str) -> Result<(), AgentError> {
        Self::entry(account)?
            .set_password(secret)
            .map_err(|_| AgentError::Provider("failed to save credential in OS keychain".into()))
    }
    pub fn load(account: &str) -> Result<Zeroizing<String>, AgentError> {
        Self::entry(account)?
            .get_password()
            .map(Zeroizing::new)
            .map_err(|_| {
                AgentError::Provider("credential missing or OS keychain unavailable".into())
            })
    }
    pub fn delete(account: &str) -> Result<(), AgentError> {
        Self::entry(account)?
            .delete_credential()
            .map_err(|_| AgentError::Provider("failed to delete credential".into()))
    }
}

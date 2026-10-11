//! Native provider coordination. OAuth URLs and secrets never enter the webview.
use incant_agent::{
    AgentError,
    accounts::{AccountStore, RemoteRevocation},
    auth,
    credentials::CredentialStore,
};
use serde_json::{Value, json};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use tauri::Emitter;
#[cfg(test)]
mod tests;

pub struct ProviderRuntime {
    agent: Arc<super::agent::Service>,
    state: Mutex<Value>,
    busy: AtomicBool,
    cancelled: AtomicBool,
    browser: Mutex<Option<Arc<auth::BrowserAuthorization>>>,
    browser_reopening: AtomicBool,
}
impl ProviderRuntime {
    pub fn new(agent: Arc<super::agent::Service>) -> Self {
        Self {
            agent,
            state: Mutex::new(json!({"status":"checking","provider":"openai"})),
            busy: AtomicBool::new(false),
            cancelled: AtomicBool::new(false),
            browser: Mutex::new(None),
            browser_reopening: AtomicBool::new(false),
        }
    }
    pub fn snapshot(&self) -> Value {
        self.state.lock().unwrap().clone()
    }
    pub(super) fn connection(&self) -> Option<super::agent::Connection> {
        if self.busy.load(Ordering::Acquire) {
            return None;
        }
        super::agent::Connection::from_public(&self.snapshot())
    }
    fn publish(&self, app: &tauri::AppHandle, state: Value) {
        let changed = {
            let mut previous = self.state.lock().unwrap();
            let changed = super::agent::Connection::from_public(&previous)
                != super::agent::Connection::from_public(&state);
            *previous = state.clone();
            changed
        };
        if changed {
            self.agent.account_operation(|| ());
        }
        // Consumers re-read current state; a delayed hint cannot carry stale
        // connecting/account data over a newer terminal publication.
        let _ = app.emit("incant:provider-changed", ());
    }
    pub fn start(
        self: &Arc<Self>,
        app: tauri::AppHandle,
        action: String,
        account_id: Option<String>,
        add: bool,
    ) -> Result<(), String> {
        if action == "cancel" {
            self.cancelled.store(true, Ordering::Relaxed);
            return Ok(());
        }
        if !["restore", "connect", "disconnect", "switch"].contains(&action.as_str()) {
            return Err("Unknown provider action".into());
        }
        if self
            .busy
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::Relaxed)
            .is_err()
        {
            if action == "connect"
                && account_id.is_none()
                && !add
                && let Some(browser) = self.browser.lock().unwrap().clone()
            {
                return self.reopen_with(browser, auth::BrowserAuthorization::open, move || {
                    let _ = app.emit("incant:provider-changed", ());
                });
            }
            return Err("An account operation is still finishing. Try again shortly.".into());
        }
        self.cancelled.store(false, Ordering::Relaxed);
        let this = self.clone();
        std::thread::spawn(move || {
            let result = this.perform(&app, &action, account_id.as_deref(), add);
            let state = match result {
                Ok(state) => state,
                Err(AgentError::Interrupted) => public_state(false).unwrap_or_else(error_state),
                Err(error) => {
                    let mut state = error_state(error);
                    if let Ok(saved) = public_state(false) {
                        state["accounts"] = saved["accounts"].clone();
                        if let Some(id) = saved.get("activeAccount") {
                            state["activeAccount"] = id.clone();
                        }
                    }
                    state
                }
            };
            *this.browser.lock().unwrap() = None;
            this.publish(&app, state);
            this.busy.store(false, Ordering::Release);
        });
        Ok(())
    }
    fn perform(
        &self,
        app: &tauri::AppHandle,
        action: &str,
        account_id: Option<&str>,
        add: bool,
    ) -> Result<Value, AgentError> {
        if action == "restore" {
            return public_state(true);
        }
        if action == "disconnect" {
            let revoked = AccountStore::open()?.disconnect()?;
            let mut state = public_state(false)?;
            if revoked == RemoteRevocation::Unconfirmed {
                state["message"] = json!(
                    "Signed out on this computer. Remote revocation could not be confirmed; you can also disconnect Incant in ChatGPT Settings."
                );
            }
            return Ok(state);
        }
        if action == "switch"
            && AccountStore::open()?.switch(
                account_id.ok_or_else(|| AgentError::Provider("Choose a saved account".into()))?,
            )?
        {
            return public_state(false);
        }
        let (attempt, subject) = AccountStore::open()?.prepare_login(add, account_id)?;
        let mut pending = public_state(false)?;
        pending["status"] = json!("connecting");
        pending["method"] = json!("oauth");
        pending["phase"] = json!("browser");
        *self.browser.lock().unwrap() = Some(Arc::new(attempt.browser_authorization()));
        self.publish(app, pending.clone());
        attempt.open_browser()?;
        let account = attempt.finish_cancellable(
            subject.as_deref(),
            std::time::Duration::from_secs(300),
            &self.cancelled,
            || {
                *self.browser.lock().unwrap() = None;
                pending["phase"] = json!("validating");
                self.publish(app, pending.clone());
            },
        )?;
        // Publication follows durable credential and metadata writes. Closing the
        // app after success does not discard its selected account.
        AccountStore::open()?.activate(account)?;
        public_state(false)
    }
    /// Reopening an existing browser attempt can block in the OS launcher. Only
    /// the bounded worker holds it; neither admission nor the browser/state locks
    /// are held while launching. A stale failure cannot replace a newer attempt.
    fn reopen_with(
        self: &Arc<Self>,
        browser: Arc<auth::BrowserAuthorization>,
        open: impl FnOnce(&auth::BrowserAuthorization) -> Result<(), AgentError> + Send + 'static,
        changed: impl FnOnce() + Send + 'static,
    ) -> Result<(), String> {
        if self.browser_reopening.swap(true, Ordering::AcqRel) {
            return Ok(());
        }
        let this = self.clone();
        if std::thread::Builder::new()
            .name("incant-browser-reopen".into())
            .spawn(move || {
                struct Reset<'a>(&'a AtomicBool);
                impl Drop for Reset<'_> {
                    fn drop(&mut self) {
                        self.0.store(false, Ordering::Release);
                    }
                }
                let _reset = Reset(&this.browser_reopening);
                if open(&browser).is_err() {
                    let updated = {
                        let pending = this.browser.lock().unwrap();
                        let mut state = this.state.lock().unwrap();
                        if pending
                            .as_ref()
                            .is_some_and(|current| Arc::ptr_eq(current, &browser))
                            && state["status"] == "connecting"
                        {
                            state["message"] =
                                json!("Could not reopen sign-in. Retry Continue with ChatGPT.");
                            true
                        } else {
                            false
                        }
                    };
                    if updated {
                        changed();
                    }
                }
            })
            .is_err()
        {
            self.browser_reopening.store(false, Ordering::Release);
            return Err("Could not start the browser opener. Try again shortly.".into());
        }
        Ok(())
    }
}
fn error_state(error: AgentError) -> Value {
    json!({"status":"error","provider":"openai","error":{"code":"provider.auth","message":error.to_string()}})
}
fn public_state(refresh: bool) -> Result<Value, AgentError> {
    let store = AccountStore::open()?;
    let mut state = json!({"status":"not-connected","provider":"openai","accounts":store.data.accounts.iter().map(|a| json!({"id":a.id,"label":if a.label.is_empty() { format!("ChatGPT · {}", &a.id[a.id.len().saturating_sub(6)..]) } else { a.label.clone() }})).collect::<Vec<_>>()});
    if let Some(account) = store.selected() {
        if CredentialStore::load_optional(&account.id)?.is_none() {
            return Ok(state);
        }
        if refresh {
            auth::access_token(account)?;
        }
        state["status"] = json!("connected");
        state["method"] = json!("oauth");
        state["accountLabel"] = json!(if account.label.is_empty() {
            "ChatGPT account"
        } else {
            &account.label
        });
        state["activeAccount"] = json!(account.id);
    } else if store.data.api_key_connected && CredentialStore::load_optional("api-key")?.is_some() {
        state["status"] = json!("connected");
        state["method"] = json!("api-key");
        state["accountLabel"] = json!("Personal API key");
    }
    Ok(state)
}
#[tauri::command]
pub fn provider_read(state: tauri::State<'_, Arc<super::Editor>>) -> Value {
    state.provider.snapshot()
}
#[tauri::command]
pub fn provider_action(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<super::Editor>>,
    action: String,
    account_id: Option<String>,
    add: Option<bool>,
) -> Result<(), String> {
    if matches!(action.as_str(), "connect" | "disconnect" | "switch") {
        state.agent.account_operation(|| {
            state
                .provider
                .start(app, action, account_id, add.unwrap_or(false))
        })
    } else {
        state
            .provider
            .start(app, action, account_id, add.unwrap_or(false))
    }
}

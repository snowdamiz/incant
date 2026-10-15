use crate::{AgentError, Usage, accounts::AccountSelection};
use reqwest::blocking::Client;
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read};
use std::time::Duration;
use zeroize::Zeroizing;
#[cfg(test)]
mod tests;

pub struct Completion {
    pub output: Vec<Value>,
    pub usage: Usage,
}
pub trait Provider {
    /// Validate the turn's non-secret authorization receipt. Agent calls this
    /// outside the project lock, after charging responses and around tools.
    /// Explicit credentials and deterministic providers may have no account store.
    fn check_active(&self) -> Result<(), AgentError> {
        Ok(())
    }
    fn complete(
        &mut self,
        input: &[Value],
        tools: &[Value],
        max_output_tokens: u64,
        on_text: &mut dyn FnMut(&str),
    ) -> Result<Completion, AgentError>;
    fn model(&self) -> &str;
}
/// Credentials are deliberately neither Serialize nor Debug. Calls go directly
/// to OpenAI; no Incant service is involved and responses are not stored remotely.
pub struct OpenAiProvider {
    client: Client,
    credential: TokenSource,
    renewable: bool,
    model: String,
    base_url: String,
    active: Option<ActiveCheck>,
}
type TokenSource = Box<dyn Fn(bool) -> Result<Zeroizing<String>, AgentError> + Send + Sync>;
type ActiveCheck = Box<dyn Fn() -> Result<(), AgentError> + Send + Sync>;
impl OpenAiProvider {
    pub fn new(credential: String, model: String) -> Result<Self, AgentError> {
        if credential.trim().is_empty() || model.trim().is_empty() {
            return Err(AgentError::Provider("missing credential or model".into()));
        }
        let secret = Zeroizing::new(credential);
        Self::with_token_source(Box::new(move |_| Ok(secret.clone())), false, model)
    }
    pub(crate) fn for_account(
        account_id: String,
        selection: AccountSelection,
        model: String,
    ) -> Result<Self, AgentError> {
        let request_selection = selection.clone();
        let mut provider = Self::with_token_source(
            Box::new(move |force| {
                let store = crate::accounts::AccountStore::open()?;
                store.check_selection(&request_selection)?;
                let account = store
                    .selected()
                    .filter(|account| account.id == account_id)
                    .ok_or_else(|| {
                        AgentError::Provider(
                            "The active account changed or signed out; start a new agent request."
                                .into(),
                        )
                    })?;
                if force {
                    crate::auth::refresh_access_token(account)
                } else {
                    crate::auth::access_token(account)
                }
            }),
            true,
            model,
        )?;
        provider.bind_selection(selection);
        Ok(provider)
    }
    pub(crate) fn for_saved_api_key(
        selection: AccountSelection,
        model: String,
    ) -> Result<Self, AgentError> {
        let request_selection = selection.clone();
        let mut provider = Self::with_token_source(
            Box::new(move |_| {
                let store = crate::accounts::AccountStore::open()?;
                store.check_selection(&request_selection)?;
                if store.data.active.is_some() || !store.data.api_key_connected {
                    return Err(AgentError::Provider(
                        "The active account changed or signed out; start a new agent request."
                            .into(),
                    ));
                }
                crate::credentials::CredentialStore::load("api-key")
            }),
            false,
            model,
        )?;
        provider.bind_selection(selection);
        Ok(provider)
    }
    fn bind_selection(&mut self, selection: AccountSelection) {
        self.active = Some(Box::new(move || {
            // No access_token/load_optional here: checks cannot prompt keychain
            // access, renew tokens or issue a network request.
            crate::accounts::AccountStore::open()?.check_selection(&selection)
        }));
    }
    fn with_token_source(
        credential: TokenSource,
        renewable: bool,
        model: String,
    ) -> Result<Self, AgentError> {
        if model.trim().is_empty() {
            return Err(AgentError::Provider("missing model".into()));
        }
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(120))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| AgentError::Provider("HTTP client initialization failed".into()))?;
        Ok(Self {
            client,
            credential,
            renewable,
            model,
            base_url: "https://api.openai.com/v1".into(),
            active: None,
        })
    }
    pub fn models(&self) -> Result<Value, AgentError> {
        let token = (self.credential)(false)?;
        let response = self
            .client
            .get(format!("{}/models", self.base_url))
            .bearer_auth(token.as_str())
            .send()
            .map_err(|_| AgentError::Provider("model catalog request failed".into()))?;
        if !response.status().is_success() {
            return Err(AgentError::Provider(format!(
                "model catalog returned HTTP {}",
                response.status().as_u16()
            )));
        }
        response
            .json()
            .map_err(|_| AgentError::Provider("invalid model catalog".into()))
    }
}
impl Provider for OpenAiProvider {
    fn check_active(&self) -> Result<(), AgentError> {
        self.active.as_ref().map_or(Ok(()), |check| check())
    }
    fn model(&self) -> &str {
        &self.model
    }
    fn complete(
        &mut self,
        input: &[Value],
        tools: &[Value],
        max_output_tokens: u64,
        on_text: &mut dyn FnMut(&str),
    ) -> Result<Completion, AgentError> {
        let body = json!({"model":self.model,"input":input,"tools":tools,"store":false,"stream":true,"max_output_tokens":max_output_tokens,"parallel_tool_calls":false});
        let mut refreshed_after_unauthorized = false;
        let mut force_refresh = false;
        for attempt in 0..3u32 {
            // Resolve the current record for every request, under the account
            // store's cross-process lock. Long turns must not retain stale tokens.
            let token = (self.credential)(force_refresh)?;
            force_refresh = false;
            let response = self
                .client
                .post(format!("{}/responses", self.base_url))
                .bearer_auth(token.as_str())
                .json(&body)
                .send()
                .map_err(|_| {
                    AgentError::Provider("OpenAI request failed; transport details redacted".into())
                })?;
            let status = response.status();
            if status.as_u16() == 401
                && self.renewable
                && !refreshed_after_unauthorized
                && attempt < 2
            {
                refreshed_after_unauthorized = true;
                force_refresh = true;
                continue;
            }
            if (status.as_u16() == 429 || status.is_server_error()) && attempt < 2 {
                std::thread::sleep(Duration::from_secs(1 << attempt));
                continue;
            }
            if !status.is_success() {
                return Err(AgentError::Provider(format!(
                    "OpenAI returned HTTP {}",
                    status.as_u16()
                )));
            }
            return read_stream(BufReader::new(response), on_text);
        }
        Err(AgentError::Provider("retry limit reached".into()))
    }
}
/// Parse SSE incrementally, including multi-line data and terminal failures.
/// Never treat early text or a dropped connection as a successful turn.
pub fn read_stream(
    reader: impl BufRead,
    on_text: &mut dyn FnMut(&str),
) -> Result<Completion, AgentError> {
    read_stream_limited(reader, on_text, 32 * 1024 * 1024)
}

fn read_stream_limited(
    mut reader: impl BufRead,
    on_text: &mut dyn FnMut(&str),
    limit: usize,
) -> Result<Completion, AgentError> {
    let mut data = String::new();
    let mut completed_items = std::collections::BTreeMap::new();
    let mut total = 0usize;
    loop {
        // Bound the read itself: BufRead::lines allocates an entire line before
        // its caller can check the size. Include delimiters and comments in the
        // same response budget; read at most one extra byte to detect overflow.
        let mut bytes = Vec::new();
        let read = reader
            .by_ref()
            .take((limit - total) as u64 + 1)
            .read_until(b'\n', &mut bytes)
            .map_err(|_| AgentError::Provider("response stream interrupted".into()))?;
        if read == 0 {
            break;
        }
        total += read;
        if total > limit {
            return Err(AgentError::Provider(
                "response stream limit exceeded".into(),
            ));
        }
        if bytes.last() == Some(&b'\n') {
            bytes.pop();
            if bytes.last() == Some(&b'\r') {
                bytes.pop();
            }
        }
        let line = String::from_utf8(bytes)
            .map_err(|_| AgentError::Provider("invalid UTF-8 response stream".into()))?;
        if line.is_empty() {
            if data.is_empty() {
                continue;
            }
            if data.trim() == "[DONE]" {
                break;
            }
            let event: Value = serde_json::from_str(&data)
                .map_err(|_| AgentError::Provider("invalid stream event".into()))?;
            data.clear();
            match event["type"].as_str().unwrap_or("") {
                "response.output_text.delta" => {
                    if let Some(text) = event["delta"].as_str() {
                        on_text(text);
                    }
                }
                "response.output_item.done" => {
                    let index = event["output_index"].as_u64().ok_or_else(|| {
                        AgentError::Provider("output item is missing its index".into())
                    })?;
                    let item = event.get("item").filter(|v| v.is_object()).ok_or_else(|| {
                        AgentError::Provider("invalid completed output item".into())
                    })?;
                    completed_items.insert(index, item.clone());
                }
                "response.completed" => {
                    let response = &event["response"];
                    let usage = &response["usage"];
                    let output = response["output"].as_array().ok_or_else(|| {
                        AgentError::Provider("completed response has no output array".into())
                    })?;
                    // ChatGPT's direct route can send item.done events followed
                    // by an empty terminal output array. Only use fully completed
                    // items, and never execute them before response.completed.
                    let output: Vec<Value> = if output.is_empty() {
                        completed_items.into_values().collect()
                    } else {
                        output.clone()
                    };
                    if output.is_empty() {
                        return Err(AgentError::Provider(
                            "provider completed without any output items".into(),
                        ));
                    }
                    return Ok(Completion {
                        output,
                        usage: Usage {
                            input_tokens: usage["input_tokens"].as_u64().ok_or_else(|| {
                                AgentError::Provider("missing input usage".into())
                            })?,
                            output_tokens: usage["output_tokens"].as_u64().ok_or_else(|| {
                                AgentError::Provider("missing output usage".into())
                            })?,
                        },
                    });
                }
                "response.failed" | "response.incomplete" | "error" => {
                    let response = &event["response"];
                    // Only emit known, static diagnostics. Provider error text
                    // may echo project contents or other sensitive inputs.
                    let reason = match event["type"].as_str() {
                        Some("response.incomplete") => {
                            match response["incomplete_details"]["reason"].as_str() {
                                Some("max_output_tokens") => {
                                    "response reached its output-token limit (including reasoning); increase --max-output-tokens within the session budget"
                                }
                                Some("content_filter") => {
                                    "response was stopped by the content filter"
                                }
                                _ => "provider returned an incomplete response",
                            }
                        }
                        _ => match response["error"]["code"]
                            .as_str()
                            .or(event["code"].as_str())
                        {
                            Some("server_error") => "provider reported a server error",
                            Some("rate_limit_exceeded") => "provider rate limit exceeded",
                            _ => "provider reported a failed response",
                        },
                    };
                    let usage = response["usage"]["input_tokens"]
                        .as_u64()
                        .zip(response["usage"]["output_tokens"].as_u64())
                        .map(|(input_tokens, output_tokens)| Usage {
                            input_tokens,
                            output_tokens,
                        });
                    return Err(AgentError::Response { reason, usage });
                }
                _ => {}
            }
        } else if let Some(part) = line.strip_prefix("data:") {
            data.push_str(part.trim_start());
            data.push('\n');
        }
    }
    Err(AgentError::Provider(
        "stream ended without response.completed".into(),
    ))
}

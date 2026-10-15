//! Explicit host configuration for loopback Responses-compatible servers.
//! Never reads the OpenAI account store, follows redirects or uses HTTP proxies.
use super::{Completion, OpenAiProvider, Provider};
use crate::AgentError;
use reqwest::blocking::Client;
use serde_json::Value;
use std::time::Duration;
use url::{Host, Url};
use zeroize::Zeroizing;

pub struct LocalResponsesProvider(OpenAiProvider);

impl LocalResponsesProvider {
    /// `endpoint` is the API base, e.g. http://127.0.0.1:8000/v1. The caller
    /// supplies a separate optional local-server credential, never OAuth tokens.
    /// This provider requires streaming Responses and engine tool-call support;
    /// it does not silently fall back to a different protocol or hosted service.
    pub fn new(
        endpoint: &str,
        credential: Option<String>,
        model: String,
    ) -> Result<Self, AgentError> {
        let credential = credential.map(Zeroizing::new);
        let endpoint = endpoint_url(endpoint)?;
        if credential.as_ref().is_some_and(|key| key.trim().is_empty()) {
            return Err(AgentError::Provider("empty local-server credential".into()));
        }
        let send_authorization = credential.is_some();
        let mut inner = OpenAiProvider::with_token_source(
            Box::new(move |_| Ok(credential.clone().unwrap_or_default())),
            false,
            model,
        )?;
        inner.client = Client::builder()
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(120))
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .build()
            .map_err(|_| AgentError::Provider("local HTTP client initialization failed".into()))?;
        inner.base_url = endpoint;
        inner.send_authorization = send_authorization;
        inner.service = "Local Responses server";
        Ok(Self(inner))
    }
}

fn endpoint_url(endpoint: &str) -> Result<String, AgentError> {
    let invalid = || {
        AgentError::Provider(
        "local endpoint must be an HTTP(S) loopback IP URL without credentials, query or fragment".into(),
    )
    };
    let url = Url::parse(endpoint).map_err(|_| invalid())?;
    let loopback = match url.host() {
        Some(Host::Ipv4(ip)) => ip.is_loopback(),
        Some(Host::Ipv6(ip)) => ip.is_loopback(),
        _ => false,
    };
    if !loopback
        || !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(invalid());
    }
    Ok(url.as_str().trim_end_matches('/').to_owned())
}

impl Provider for LocalResponsesProvider {
    fn model(&self) -> &str {
        self.0.model()
    }
    fn complete(
        &mut self,
        input: &[Value],
        tools: &[Value],
        max_output_tokens: u64,
        on_text: &mut dyn FnMut(&str),
    ) -> Result<Completion, AgentError> {
        self.0.complete(input, tools, max_output_tokens, on_text)
    }
}

#[cfg(test)]
mod tests;

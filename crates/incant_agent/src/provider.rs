use crate::{AgentError, Usage};
use reqwest::blocking::Client;
use serde_json::{Value, json};
use std::io::{BufRead, BufReader};
use std::time::Duration;
use zeroize::Zeroizing;

pub struct Completion {
    pub output: Vec<Value>,
    pub usage: Usage,
}
pub trait Provider {
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
    credential: Zeroizing<String>,
    model: String,
}
impl OpenAiProvider {
    pub fn new(credential: String, model: String) -> Result<Self, AgentError> {
        if credential.trim().is_empty() || model.trim().is_empty() {
            return Err(AgentError::Provider("missing credential or model".into()));
        }
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(120))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| AgentError::Provider("HTTP client initialization failed".into()))?;
        Ok(Self {
            client,
            credential: Zeroizing::new(credential),
            model,
        })
    }
    pub fn models(&self) -> Result<Value, AgentError> {
        let response = self
            .client
            .get("https://api.openai.com/v1/models")
            .bearer_auth(self.credential.as_str())
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
        for attempt in 0..3u32 {
            let response = self
                .client
                .post("https://api.openai.com/v1/responses")
                .bearer_auth(self.credential.as_str())
                .json(&body)
                .send()
                .map_err(|_| {
                    AgentError::Provider("OpenAI request failed; transport details redacted".into())
                })?;
            let status = response.status();
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
    let mut data = String::new();
    let mut completed_items = std::collections::BTreeMap::new();
    let mut total = 0usize;
    for line in reader.lines() {
        let line = line.map_err(|_| AgentError::Provider("response stream interrupted".into()))?;
        total += line.len();
        if total > 32 * 1024 * 1024 {
            return Err(AgentError::Provider(
                "response stream limit exceeded".into(),
            ));
        }
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
                    return Err(AgentError::Provider(
                        "provider reported a failed or incomplete response".into(),
                    ));
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

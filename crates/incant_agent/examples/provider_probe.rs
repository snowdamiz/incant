//! Synthetic tool-call probe using Incant's own saved account. Never prints secrets.
use incant_agent::{accounts::AccountStore, provider::Provider};
use serde_json::json;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut provider = AccountStore::open()?.provider("gpt-6-astra".into())?;
    let response = provider.complete(
        &[json!({"role":"user","content":"Call doc_query with an empty path to read the current project. This is a tool wiring test."})],
        &incant_agent::tools(), 1024, &mut |_| {})?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({"output":response.output,"usage":response.usage}))?
    );
    Ok(())
}

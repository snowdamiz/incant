//! Prints actual generated contracts for a create-only external comparison.
fn main() {
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "CoroutineValue": schemars::schema_for!(incant_types::CoroutineValue),
            "CoroutineResult": schemars::schema_for!(incant_types::CoroutineResult),
        }))
        .unwrap()
    );
}

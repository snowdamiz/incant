//! Temporary fake data only: verifies OS persistence across independent builds.
use incant_agent::credentials::CredentialStore;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).ok_or("save, verify or delete required")?;
    let id = args.get(2).ok_or("probe UUID required")?;
    uuid::Uuid::parse_str(id)?;
    let account = format!("persistence-probe-{id}");
    let fake = "Incant synthetic OAuth record; no real token.🔑".repeat(300);
    match mode.as_str() {
        "save" => CredentialStore::save(&account, &fake)?,
        "verify" => {
            if CredentialStore::load(&account)?.as_str() != fake {
                return Err("probe mismatch".into());
            }
        }
        "delete" => {
            CredentialStore::delete(&account)?;
            if CredentialStore::load_optional(&account)?.is_some() {
                return Err("probe not deleted".into());
            }
        }
        _ => return Err("unknown probe action".into()),
    }
    println!(
        "{mode}: passed (synthetic data only; build {})",
        option_env!("INCANT_PROBE_BUILD").unwrap_or("default")
    );
    Ok(())
}

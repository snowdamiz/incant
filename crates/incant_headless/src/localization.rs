//! Translation exchange keeps file IO in the CLI and project edits in incant_cmd.
use crate::{Result, print, read_project, save};
use incant_cmd::{Actor, CommandBus, PreparedTranslations};
use std::{
    fs,
    io::{Read, Write},
    path::Path,
};

pub(super) fn export(project: &Path, table_id: &str, locale: &str, output: &Path) -> Result<()> {
    let project = read_project(project)?;
    let table = project
        .string_tables
        .get(table_id)
        .ok_or("unknown string table")?;
    let document = incant_localization::export_xliff(table, locale)?;
    // Publish only a fully written file, and never replace an existing path.
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    file.write_all(document.as_bytes())?;
    file.as_file().sync_all()?;
    file.persist_noclobber(output).map_err(|e| e.error)?;
    print(
        serde_json::json!({"table_id":table_id,"locale":locale,"messages":table.messages.len(),"bytes":document.len(),"output":output}),
    )
}
pub(super) fn import(project: &Path, input: &Path) -> Result<()> {
    let mut bytes = vec![];
    fs::File::open(input)?
        .take((incant_localization::MAX_XLIFF_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > incant_localization::MAX_XLIFF_BYTES {
        return Err("XLIFF exceeds 32 MiB".into());
    }
    let document = String::from_utf8(bytes)?;
    let mut bus = CommandBus::persistent(
        project.with_extension("journal.jsonl"),
        read_project(project)?,
    )?;
    let prepared = PreparedTranslations::prepare(&bus, &document)?;
    let result = prepared.commit(&mut bus, Actor::import("xliff"))?;
    save(project, &bus.project().canonical_text()?)?;
    print(serde_json::to_value(result)?)
}

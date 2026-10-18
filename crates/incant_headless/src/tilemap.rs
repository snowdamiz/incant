use crate::{Result, print};
use incant_cmd::{Actor, CommandBus};
use incant_import::{ImportSnapshot, TilemapPaintRequest};
use std::{fs::File, io::Read, path::Path};

pub fn paint(project: &Path, request: &Path, cache: Option<&Path>) -> Result<()> {
    const CAP: u64 = 1024 * 1024;
    let metadata = std::fs::symlink_metadata(request)?;
    if !metadata.is_file() {
        return Err("tilemap request must be a regular file".into());
    }
    if metadata.len() > CAP {
        return Err("tilemap request exceeds 1 MiB".into());
    }
    let mut bytes = Vec::new();
    File::open(request)?.take(CAP + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > CAP {
        return Err("tilemap request exceeds 1 MiB".into());
    }
    let request: TilemapPaintRequest = serde_json::from_slice(&bytes)?;
    let root = project
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut bus = CommandBus::open_project_file_with_journal(
        project,
        project.with_extension("journal.jsonl"),
    )?;
    let prepared = ImportSnapshot::capture(&bus).prepare_tilemap_paint(root, cache, &request)?;
    let result = prepared.commit(&mut bus, Actor::user("cli"))?;
    // Saving failure retains the recoverable journal; never report a saved edit.
    // No-op does not create a save checkpoint or touch existing file bytes.
    if result.changed {
        bus.save_project_file(bus.revision(), Actor::user("cli"))?;
    }
    print(result)
}

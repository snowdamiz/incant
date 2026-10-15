use crate::{Result, Template, TemplateError, plan};
use incant_cmd::{Actor, CommandBus};
use incant_doc::{Id, Project};
use incant_import::{ImportRequest, ImportSnapshot};
use serde::Serialize;
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Component, Path, PathBuf},
};

const PROJECT: &str = "project.incant.json";
const JOURNAL: &str = "project.incant.journal.jsonl";
const MODEL: &[u8] = include_bytes!("../../../templates/builtin/assets/template-cube.gltf");
const ATLAS: &[u8] =
    include_bytes!("../../../templates/builtin/assets/template-sprites.atlas.json");
const FRAMES: [(&str, &[u8]); 4] = [
    (
        "template-frame-0.png",
        include_bytes!("../../../templates/builtin/assets/template-frame-0.png"),
    ),
    (
        "template-frame-1.png",
        include_bytes!("../../../templates/builtin/assets/template-frame-1.png"),
    ),
    (
        "template-frame-2.png",
        include_bytes!("../../../templates/builtin/assets/template-frame-2.png"),
    ),
    (
        "template-tile.png",
        include_bytes!("../../../templates/builtin/assets/template-tile.png"),
    ),
];

#[derive(Debug, Serialize)]
pub struct CreatedProject {
    pub template: Template,
    pub project: PathBuf,
    pub journal: PathBuf,
    pub project_id: Id,
    pub roles: BTreeMap<String, Id>,
    pub recipe_transaction: Id,
    pub revision: u64,
}

fn destination(path: &Path) -> Result<PathBuf> {
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let mut prefix = PathBuf::new();
    for component in path.components() {
        if matches!(component, Component::ParentDir | Component::CurDir) {
            return Err(TemplateError::Destination);
        }
        prefix.push(component);
        if prefix != path {
            let metadata = fs::symlink_metadata(&prefix)?;
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(TemplateError::Destination);
            }
        }
    }
    if path.file_name().is_none() {
        return Err(TemplateError::Destination);
    }
    match fs::symlink_metadata(&path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
        Ok(_) => return Err(TemplateError::Destination),
    }
    Ok(path)
}
fn raw(root: &Path, name: &str, bytes: &[u8]) -> Result<()> {
    // Fixed, single-component bundled names only, never project/agent data.
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join(name))?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

/// Host-only creation into a new directory; no serialized path capability.
/// Raw bundled inputs are create-only files. Imports and the recipe use the
/// ordinary bus/source protocol. The final project filename is published last;
/// earlier failures retain an explicitly incomplete directory for inspection.
/// This is not an atomic directory operation and never deletes existing files.
pub fn create(path: &Path, template: Template, name: &str) -> Result<CreatedProject> {
    if name.trim().is_empty() || name.len() > 256 || name.chars().any(char::is_control) {
        return Err(TemplateError::Name);
    }
    let path = destination(path)?;
    let directory = {
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            let mut directory = fs::DirBuilder::new();
            directory.mode(0o700);
            directory
        }
        #[cfg(not(unix))]
        {
            fs::DirBuilder::new()
        }
    };
    directory.create(&path)?; // Exclusive; private from creation on Unix.
    create_in(&path, template, name).map_err(|source| TemplateError::Incomplete {
        path,
        source: Box::new(source),
    })
}

fn create_in(root: &Path, template: Template, name: &str) -> Result<CreatedProject> {
    #[cfg(unix)]
    fs::File::open(root.parent().ok_or(TemplateError::Destination)?)?.sync_all()?;
    let initial = Project::empty(name);
    let project_id = initial.id.clone();
    let journal = root.join(JOURNAL);
    let mut bus = CommandBus::persistent_with_sources(&journal, initial.clone(), root)?;
    let source = if template == Template::Empty {
        None
    } else if template.planar() {
        for (name, bytes) in FRAMES {
            raw(root, name, bytes)?;
        }
        raw(root, "template-sprites.atlas.json", ATLAS)?;
        Some("template-sprites.atlas.json")
    } else {
        raw(root, "template-cube.gltf", MODEL)?;
        Some("template-cube.gltf")
    };
    let asset = if let Some(source) = source {
        let prepared = ImportSnapshot::capture(&bus).prepare(
            root,
            None,
            &[ImportRequest {
                source: source.into(),
                texture_usage: None,
            }],
        )?;
        let committed = prepared.commit(
            &mut bus,
            Actor::import("starter asset"),
            "Import bundled starter asset",
        )?;
        Some(
            committed
                .imports
                .into_iter()
                .next()
                .expect("one request")
                .asset,
        )
    } else {
        None
    };
    let recipe = plan(template, asset.as_ref())?;
    let revision = bus.revision();
    let transaction = bus
        .execute(
            recipe.commands,
            Actor::user("project-template"),
            format!("Create {} starter scene", template.name()),
            Some(revision),
        )?
        .id
        .clone();
    let complete = bus.project().clone();
    let revision = bus.revision();
    let canonical = complete.canonical_text()?;
    bus.ensure_sources_ready()?;
    if let Some(script) = recipe.roles.get("script")
        && bus.read_script(script)?.matches_history != Some(true)
    {
        return Err(TemplateError::Verification("source identity"));
    }
    drop(bus);
    // Reopen at the permanent journal/root location before publication. A
    // populated staging directory cannot be renamed: source binding is rooted.
    let recovered = CommandBus::persistent_with_sources(&journal, complete.clone(), root)?;
    if recovered.project() != &complete || recovered.revision() != revision {
        return Err(TemplateError::Verification("journal recovery"));
    }
    if let Some(script) = recipe.roles.get("script")
        && recovered.read_script(script)?.matches_history != Some(true)
    {
        return Err(TemplateError::Verification("recovered source"));
    }
    if incant_import::validate_cached_sprites(&complete, root, None)?
        .iter()
        .any(|d| d.severity == incant_import::SpriteDiagnosticSeverity::Error)
    {
        return Err(TemplateError::Verification("cached sprite bindings"));
    }
    let project = publish(root, &canonical)?;
    // Existing command journal remains locked through initial publication.
    // All future edits/checkpoints use open_project_file[_with_journal].
    drop(recovered);
    Ok(CreatedProject {
        template,
        project,
        journal,
        project_id,
        roles: recipe.roles,
        recipe_transaction: transaction,
        revision,
    })
}

fn publish(root: &Path, canonical: &str) -> Result<PathBuf> {
    let project = root.join(PROJECT);
    let mut staged = tempfile::NamedTempFile::new_in(root)?;
    staged.write_all(canonical.as_bytes())?;
    staged.as_file().sync_all()?;
    #[cfg(test)]
    if FAIL_PUBLICATION.with(std::cell::Cell::get) {
        return Err(TemplateError::Verification(
            "injected pre-publication refusal",
        ));
    }
    staged
        .persist_noclobber(&project)
        .map_err(|error| TemplateError::Io(error.error))?;
    #[cfg(unix)]
    fs::File::open(root)?.sync_all()?;
    Ok(project)
}

#[cfg(test)]
thread_local! { static FAIL_PUBLICATION: std::cell::Cell<bool> = const { std::cell::Cell::new(false) }; }

#[cfg(test)]
mod tests;

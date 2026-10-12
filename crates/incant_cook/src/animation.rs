#[path = "animation_ik.rs"]
mod ik;
// Startup-only animation metadata derived from checked cooked model bytes.
// Sealed tables cannot be populated with caller-invented model metadata.
use crate::{CookError, stable_id};
use incant_doc::{Animation, Entity, MeshRenderer, Project};
use incant_runtime::{NativeAnimation, StableId, VisualAssetRef};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

struct Resource {
    fingerprint: [u8; 32],
    durations: Arc<[f32]>,
    path: PathBuf,
    ik: BTreeMap<StableId, Arc<incant_runtime::NativeIk>>,
}
#[derive(Default)]
pub struct AnimationResources {
    models: BTreeMap<StableId, Resource>,
    root_motion: crate::RootMotionResources,
}
impl AnimationResources {
    pub(crate) fn bind_root_motion(
        &self,
        asset: &VisualAssetRef,
        entity: StableId,
        authored: &incant_doc::AnimationRootMotion,
        animation: &NativeAnimation,
    ) -> Result<Arc<incant_runtime::NativeRootMotion>, CookError> {
        let error = || CookError::RootMotionResource {
            entity: ulid::Ulid::from_bytes(entity.0).to_string(),
            reason: "missing or stale captured root catalog",
        };
        let catalog = self.root_motion.catalog(entity).ok_or_else(error)?;
        if stable_id(&authored.model)? != asset.id
            || catalog.fingerprint() != asset.fingerprint
            || catalog.root() != authored.root
        {
            return Err(error());
        }
        let root = incant_runtime::NativeRootMotion {
            catalog: Arc::clone(catalog),
            application: authored.application,
            applying: authored.applying,
        };
        root.validate(animation)?;
        Ok(Arc::new(root))
    }
    /// Validated cache inputs for build-system invalidation, never source paths.
    pub fn cache_paths(&self) -> impl Iterator<Item = &Path> {
        self.models.values().map(|r| r.path.as_path())
    }
    pub(crate) fn bind_ik(
        &self,
        asset: &VisualAssetRef,
        entity: StableId,
        authored: &incant_types::AnimationIk,
    ) -> Result<Arc<incant_runtime::NativeIk>, CookError> {
        let error = || CookError::AnimationResource(ulid::Ulid::from_bytes(entity.0).to_string());
        let resource = self.models.get(&asset.id).ok_or_else(error)?;
        let compiled = resource.ik.get(&entity).ok_or_else(error)?;
        // Resources are a snapshot of checked authoring; never reuse against a changed declaration.
        if resource.fingerprint != asset.fingerprint
            || compiled.as_ref() != &ik::compile(authored, asset.fingerprint)?
        {
            return Err(error());
        }
        Ok(compiled.clone())
    }
    pub(crate) fn bind(
        &self,
        asset: &VisualAssetRef,
        initial: Animation,
        graph: Option<&incant_doc::AnimationGraph>,
    ) -> Result<Arc<NativeAnimation>, CookError> {
        let missing =
            || CookError::AnimationResource(ulid::Ulid::from_bytes(asset.id.0).to_string());
        let resource = self.models.get(&asset.id).ok_or_else(missing)?;
        if resource.fingerprint != asset.fingerprint {
            return Err(missing());
        }
        let animation = NativeAnimation {
            graph: graph
                .map(|g| crate::animation_graph::compile(g, &resource.durations))
                .transpose()?,
            mesh_fingerprint: asset.fingerprint,
            clip_durations: resource.durations.clone(),
            initial,
        };
        animation.validate()?;
        Ok(Arc::new(animation))
    }
}
fn mesh(entity: &Entity) -> Result<MeshRenderer, CookError> {
    let value = entity
        .components
        .get("MeshRenderer")
        .ok_or_else(|| CookError::AnimationResource(entity.id.clone()))?;
    Ok(serde_json::from_value(value.clone())?)
}

/// Load only models used by Animation components in the selected scene. This
/// checks the document once, rejects redirected project-cache directories, and
/// drops each bounded model after extracting its small immutable duration table.
/// Source meshes and TypeScript are never read here; ticks do no filesystem I/O.
/// A scene with no Animation components requires no cache directory at all.
pub fn load_project_animation_resources(
    project: &Project,
    scene_id: &str,
    root: &Path,
) -> Result<AnimationResources, CookError> {
    project.validated()?;
    let scene = project
        .scenes
        .get(scene_id)
        .ok_or_else(|| CookError::MissingScene(scene_id.into()))?;
    let mut resources = AnimationResources::default();
    let mut cache = None;
    let mut clips = 0usize;
    let mut ik_budget = 0usize;
    for entity in scene
        .entities
        .values()
        .filter(|e| e.components.contains_key("Animation"))
    {
        let mesh = mesh(entity)?;
        let id = stable_id(&mesh.mesh)?;
        if resources.models.contains_key(&id) {
            continue;
        }
        let error = || CookError::AnimationResource(mesh.mesh.clone());
        let asset = project.assets.get(&mesh.mesh).ok_or_else(error)?;
        if asset.kind != "model" {
            return Err(error());
        }
        if cache.is_none() {
            cache = Some(model_cache(root).map_err(|_| error())?);
        }
        let directory = cache.as_deref().expect("resolved model cache");
        let key = asset.sha256.to_ascii_lowercase();
        let model = incant_assets_runtime::load_model(directory, &key).map_err(|_| error())?;
        let animation = model.animation.as_ref().ok_or_else(error)?;
        let durations: Arc<[f32]> = animation
            .clips
            .iter()
            .map(|clip| {
                clip.channels
                    .iter()
                    .filter_map(|c| c.times.last().copied())
                    .fold(0.0f32, f32::max)
            })
            .collect();
        clips = clips.checked_add(durations.len()).ok_or_else(error)?;
        if clips > incant_runtime::MAX_SCENE_ANIMATION_CLIPS {
            return Err(error());
        }
        let mut fingerprint = [0; 32];
        for (byte, pair) in fingerprint
            .iter_mut()
            .zip(key.as_bytes().as_chunks::<2>().0)
        {
            // load_model has already checked exactly 64 lowercase hex bytes.
            let hex = |b: u8| char::from(b).to_digit(16).expect("checked model key") as u8;
            *byte = hex(pair[0]) << 4 | hex(pair[1]);
        }
        let ik = ik::admit(scene, &mesh.mesh, &model, fingerprint, &mut ik_budget)?;
        resources.models.insert(
            id,
            Resource {
                fingerprint,
                durations,
                ik,
                path: incant_assets_runtime::model::key_path(directory, &key)
                    .map_err(|_| error())?,
            },
        );
    }
    resources.root_motion = crate::load_project_root_motion_resources(project, scene_id, root)?;
    Ok(resources)
}

// The cook crate is a build-time dependency of shipped players. Keep its
// directory check local rather than importing source importers/ufbx into builds.
// As with asset project caches, every descended component must be a real folder.
pub(crate) fn model_cache(root: &Path) -> std::io::Result<PathBuf> {
    let root = root.canonicalize()?;
    let mut path = root.clone();
    for part in [".incant", "cache", "models"] {
        path.push(part);
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink()
            || !metadata.is_dir()
            || !path.canonicalize()?.starts_with(&root)
        {
            return Err(std::io::Error::other(
                "model cache must stay in real project directories",
            ));
        }
    }
    Ok(path)
}

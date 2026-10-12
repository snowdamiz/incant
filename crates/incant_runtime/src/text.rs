//! Immutable native text configuration. Shaping and asset I/O are host startup
//! work, never part of a world tick. Shared strings survive numeric relocation.
use crate::{NativeWorld, RuntimeError, SceneError, StableId};
use bevy_ecs::prelude::Component;
pub use incant_types::{
    FontAssetRef, MAX_SCENE_TEXT_BYTES, MAX_TEXT_BYTES, MAX_TEXT_FONTS, MAX_TEXT_NODES, TextLayout,
    TextWrap,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

#[derive(Debug, Clone, PartialEq)]
pub struct NativeText {
    pub content: Arc<str>,
    pub fonts: Arc<[StableId]>,
    pub layout: TextLayout,
}
#[derive(Debug, Clone, PartialEq)]
pub struct CookedText {
    pub entity: StableId,
    pub text: Arc<NativeText>,
}
#[derive(Component)]
pub(crate) struct TextComponent(pub Arc<NativeText>);
impl NativeText {
    pub fn validate(&self) -> Result<(), SceneError> {
        self.layout.validate().map_err(SceneError::Invalid)?;
        if self.content.len() > MAX_TEXT_BYTES
            || self.fonts.is_empty()
            || self.fonts.len() > MAX_TEXT_FONTS
            || self
                .fonts
                .iter()
                .enumerate()
                .any(|(i, id)| self.fonts[..i].contains(id))
        {
            return Err(SceneError::Invalid("text content or font chain"));
        }
        Ok(())
    }
}
pub(crate) fn validate<'a>(
    fonts: &[FontAssetRef],
    nodes: impl IntoIterator<Item = (StableId, &'a NativeText)>,
    scene: StableId,
    exists: impl Fn(StableId) -> bool,
) -> Result<(), SceneError> {
    if fonts.len() > MAX_TEXT_FONTS
        || fonts.windows(2).any(|a| a[0].id >= a[1].id)
        || fonts.iter().any(|a| a.id == scene || exists(a.id))
    {
        return Err(SceneError::Invalid("font asset catalog"));
    }
    let mut ids = BTreeSet::new();
    let mut bytes = 0usize;
    for (id, node) in nodes {
        if !exists(id) || !ids.insert(id) || ids.len() > MAX_TEXT_NODES {
            return Err(SceneError::Invalid("text entity references or count"));
        }
        node.validate()?;
        bytes = bytes
            .checked_add(node.content.len())
            .filter(|&n| n <= MAX_SCENE_TEXT_BYTES)
            .ok_or(SceneError::Invalid("scene text bytes"))?;
        if node
            .fonts
            .iter()
            .any(|id| fonts.binary_search_by_key(id, |a| a.id).is_err())
        {
            return Err(SceneError::Invalid("missing font asset reference"));
        }
    }
    Ok(())
}

pub struct TextView<'a> {
    world: &'a NativeWorld,
}
impl NativeWorld {
    pub fn text_view(&self) -> Result<TextView<'_>, RuntimeError> {
        self.ensure_usable()?;
        Ok(TextView { world: self })
    }
    pub(crate) fn install_text(&mut self, id: StableId, text: Option<Arc<NativeText>>) {
        if let Some(text) = text {
            self.text_ids.insert(id);
            self.world
                .entity_mut(self.entities[&id])
                .insert(TextComponent(text));
        } else {
            self.text_ids.remove(&id);
            if let Some(&entity) = self.entities.get(&id) {
                self.world.entity_mut(entity).remove::<TextComponent>();
            }
        }
    }
    pub(crate) fn prepare_text(
        &self,
        commands: &crate::FrameCommands,
    ) -> Result<Option<PendingText>, RuntimeError> {
        use crate::StructuralCommand as C;
        if !commands.iter().any(|c| {
            matches!(c, C::SetText { .. } | C::ControlText { .. })
                || matches!(c,C::Despawn(id) if self.text_ids.contains(id))
        }) {
            return Ok(None);
        }
        let mut spawned = BTreeSet::new();
        let mut removed = BTreeSet::new();
        let mut changes: BTreeMap<StableId, Option<Arc<NativeText>>> = BTreeMap::new();
        // Track command-order lifetime; full structural staging additionally
        // rejects duplicate spawn/despawn and invalid unrelated components.
        for command in commands.iter() {
            match command {
                C::Spawn(entity) => {
                    spawned.insert(entity.id);
                }
                C::Despawn(id) => {
                    removed.insert(*id);
                    changes.insert(*id, None);
                }
                C::ControlText {
                    entity,
                    control: crate::TextControl::Content(content),
                } => {
                    if removed.contains(entity) {
                        return Err(RuntimeError::MissingEntity(*entity));
                    }
                    let node = changes.entry(*entity).or_insert_with(|| {
                        self.entities
                            .get(entity)
                            .and_then(|&e| self.world.get::<TextComponent>(e))
                            .map(|t| Arc::clone(&t.0))
                    });
                    let node = node.as_mut().ok_or(SceneError::Invalid(
                        "text control requires existing world text",
                    ))?;
                    if node.content != *content {
                        Arc::make_mut(node).content = Arc::clone(content);
                    }
                }
                C::SetText { entity, text } => {
                    if removed.contains(entity)
                        || (!self.entities.contains_key(entity) && !spawned.contains(entity))
                    {
                        return Err(RuntimeError::MissingEntity(*entity));
                    }
                    changes.insert(*entity, text.clone());
                }
                _ => {}
            }
        }
        let exists = |id| {
            !removed.contains(&id) && (self.entities.contains_key(&id) || spawned.contains(&id))
        };
        let current = self
            .text_ids
            .iter()
            .filter(|id| !changes.contains_key(id))
            .map(|id| {
                (
                    *id,
                    self.world
                        .get::<TextComponent>(self.entities[id])
                        .unwrap()
                        .0
                        .as_ref(),
                )
            });
        let added = changes
            .iter()
            .filter_map(|(&id, node)| node.as_deref().map(|n| (id, n)));
        validate(
            &self.font_assets,
            current.chain(added),
            self.scene_id(),
            exists,
        )?;
        changes.retain(|id, node| {
            node.as_deref()
                != self
                    .entities
                    .get(id)
                    .and_then(|&e| self.world.get::<TextComponent>(e))
                    .map(|t| t.0.as_ref())
        });
        let generation = if changes.is_empty() {
            self.text_generation
        } else {
            self.text_generation
                .checked_add(1)
                .ok_or(RuntimeError::GenerationOverflow)?
        };
        Ok(Some(PendingText {
            changes,
            generation,
        }))
    }
    pub(crate) fn publish_text(&mut self, pending: Option<PendingText>) {
        if let Some(p) = pending {
            for (id, text) in p.changes {
                self.install_text(id, text);
            }
            self.text_generation = p.generation;
        }
    }
}
pub(crate) struct PendingText {
    changes: BTreeMap<StableId, Option<Arc<NativeText>>>,
    generation: u64,
}
impl<'a> TextView<'a> {
    pub fn identity(&self) -> &'a crate::WorldIdentity {
        &self.world.identity
    }
    pub fn generation(&self) -> u64 {
        self.world.text_generation
    }
    pub fn fonts(&self) -> &'a [FontAssetRef] {
        &self.world.font_assets
    }
    pub fn nodes(&self) -> impl ExactSizeIterator<Item = (StableId, &'a NativeText)> + 'a {
        let world = self.world;
        world.text_ids.iter().map(move |id| {
            (
                *id,
                world
                    .world
                    .get::<TextComponent>(world.entities[id])
                    .expect("text index")
                    .0
                    .as_ref(),
            )
        })
    }
}

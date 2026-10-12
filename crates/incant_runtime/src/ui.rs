//! Screen-space declarations share native entity lifetimes and parents. Mutable
//! focus, composition, layout and atlas state belong to the composed host.
use crate::{CookedScene, NativeWorld, RuntimeError, SceneError, StableId};
use incant_types::{UiLink, UiNode, validate_ui_tree};
use std::{collections::BTreeMap, sync::Arc};
pub type NativeUi = UiNode<StableId>;
#[derive(Clone, Debug, PartialEq)]
pub struct CookedUi {
    pub entity: StableId,
    pub node: Arc<NativeUi>,
}

pub(crate) fn validate(
    nodes: &[CookedUi],
    fonts: &[crate::FontAssetRef],
    parent: impl Fn(StableId) -> Result<Option<StableId>, SceneError>,
    compatible: impl Fn(StableId) -> bool,
    children: impl IntoIterator<Item = (StableId, Option<StableId>)>,
) -> Result<(), SceneError> {
    if nodes.is_empty() {
        return Ok(());
    }
    if nodes.len() > incant_types::MAX_UI_NODES
        || nodes.windows(2).any(|n| n[0].entity >= n[1].entity)
    {
        return Err(SceneError::Invalid("UI count or canonical IDs"));
    }
    let mut links = Vec::with_capacity(nodes.len());
    for n in nodes {
        if !compatible(n.entity) {
            return Err(SceneError::Invalid(
                "UI requires identity transform and no world components",
            ));
        }
        if n.node.kind.text().is_some_and(|t| {
            t.fonts
                .iter()
                .any(|id| fonts.binary_search_by_key(id, |f| f.id).is_err())
        }) {
            return Err(SceneError::Invalid("UI font reference"));
        }
        links.push(UiLink {
            id: n.entity,
            parent: parent(n.entity)?,
            node: &n.node,
        });
    }
    validate_ui_tree(&links).map_err(SceneError::Invalid)?;
    for (id, parent) in children {
        if parent.is_some_and(|p| nodes.binary_search_by_key(&p, |n| n.entity).is_ok())
            && nodes.binary_search_by_key(&id, |n| n.entity).is_err()
        {
            return Err(SceneError::Invalid("UI containers require UI children"));
        }
    }
    Ok(())
}
impl CookedScene {
    pub fn with_ui(mut self, mut nodes: Vec<CookedUi>) -> Result<Self, SceneError> {
        if nodes.len() > incant_types::MAX_UI_NODES {
            return Err(SceneError::Invalid("UI node count"));
        }
        nodes.sort_unstable_by_key(|n| n.entity);
        self.ui = nodes;
        self.validate_physics_2d()?;
        Ok(self)
    }
    pub fn ui(&self) -> &[CookedUi] {
        &self.ui
    }
    pub fn has_ui(&self) -> bool {
        !self.ui.is_empty()
    }
    pub(crate) fn validate_ui(
        &self,
        entities: &BTreeMap<StableId, &crate::CookedEntity>,
    ) -> Result<(), SceneError> {
        let bytes = self
            .ui
            .iter()
            .filter_map(|n| n.node.kind.text())
            .map(|t| t.content.len())
            .chain(self.texts.iter().map(|n| n.text.content.len()))
            .sum::<usize>();
        if bytes > incant_types::MAX_SCENE_TEXT_BYTES {
            return Err(SceneError::Invalid("combined scene text bytes"));
        }
        validate(
            &self.ui,
            &self.font_assets,
            |id| {
                entities
                    .get(&id)
                    .map(|e| e.parent)
                    .ok_or(SceneError::Invalid("UI entity reference"))
            },
            |id| {
                entities
                    .get(&id)
                    .is_some_and(|e| e.transform == Default::default() && e.velocity.is_none())
                    && !self.spatial.iter().any(|n| n.entity == id)
                    && !self.audio.iter().any(|n| n.entity == id)
                    && !self.physics.iter().any(|n| n.entity == id)
                    && !self.physics_2d.iter().any(|n| n.entity == id)
                    && !self.navigation.iter().any(|n| n.entity == id)
                    && !self.texts.iter().any(|n| n.entity == id)
            },
            self.entities.iter().map(|e| (e.id, e.parent)),
        )
    }
}

pub struct UiView<'a> {
    world: &'a NativeWorld,
}
pub struct UiViewNode<'a> {
    pub id: StableId,
    pub parent: Option<StableId>,
    pub incarnation: u64,
    pub node: &'a Arc<NativeUi>,
}
impl NativeWorld {
    pub fn ui_view(&self) -> Result<UiView<'_>, RuntimeError> {
        self.ensure_usable()?;
        Ok(UiView { world: self })
    }
    pub(crate) fn prepare_ui(
        &self,
        commands: &crate::FrameCommands,
    ) -> Result<Option<PendingUi>, RuntimeError> {
        use crate::StructuralCommand as C;
        if commands.is_empty()
            || (self.ui.is_empty() && !commands.iter().any(|c| matches!(c, C::SetUi { .. })))
        {
            return Ok(None);
        }
        // Numeric/control-only ticks cannot change membership or hierarchy.
        if !commands.iter().any(|c| match c {
            C::SetUi { .. } | C::SetText { .. } => true,
            C::Spawn(e) => e.parent.is_some_and(|p| self.ui.contains_key(&p)),
            C::Despawn(id) => self.ui.contains_key(id),
            C::SetParent { entity, parent } => {
                self.ui.contains_key(entity) || parent.is_some_and(|p| self.ui.contains_key(&p))
            }
            C::SetSpatial { entity, .. }
            | C::SetAudio { entity, .. }
            | C::SetPhysics { entity, .. }
            | C::SetPhysics2D { entity, .. }
            | C::SetNavigation { entity, .. }
            | C::SetVelocity { entity, .. } => self.ui.contains_key(entity),
            _ => false,
        }) {
            return Ok(None);
        }
        let mut nodes = self.ui.clone();
        let mut born = BTreeMap::new();
        let mut removed = std::collections::BTreeSet::new();
        let mut parents = BTreeMap::new();
        for command in commands.iter() {
            match command {
                C::Spawn(e) => {
                    born.insert(e.id, e);
                    parents.insert(e.id, e.parent);
                }
                C::Despawn(id) => {
                    removed.insert(*id);
                    nodes.remove(id);
                }
                C::SetParent { entity, parent } => {
                    parents.insert(*entity, *parent);
                }
                C::SetUi { entity, node } => {
                    if removed.contains(entity)
                        || (!self.entities.contains_key(entity) && !born.contains_key(entity))
                    {
                        return Err(RuntimeError::MissingEntity(*entity));
                    }
                    if let Some(node) = node {
                        nodes.insert(*entity, Arc::clone(node));
                    } else {
                        nodes.remove(entity);
                    }
                    if nodes.len() > incant_types::MAX_UI_NODES {
                        return Err(SceneError::Invalid("UI node count").into());
                    }
                }
                _ => {}
            }
        }
        let exists = |id: &StableId| {
            !removed.contains(id) && (self.entities.contains_key(id) || born.contains_key(id))
        };
        let parent = |id: StableId| {
            parents
                .get(&id)
                .copied()
                .or_else(|| self.parents.get(&id).copied())
        };
        let declarations: Vec<_> = nodes
            .iter()
            .map(|(&entity, node)| CookedUi {
                entity,
                node: Arc::clone(node),
            })
            .collect();
        validate(
            &declarations,
            &self.font_assets,
            |id| {
                if exists(&id) {
                    parent(id).ok_or(SceneError::Invalid("UI entity reference"))
                } else {
                    Err(SceneError::Invalid("UI entity reference"))
                }
            },
            |id| {
                let live = self.entities.get(&id).map(|&e| self.world.entity(e));
                let transform = born.get(&id).map(|e| &e.transform).or_else(|| {
                    live.as_ref()
                        .and_then(|e| e.get::<crate::world::Local>())
                        .map(|t| &t.0)
                });
                if transform != Some(&Default::default()) {
                    return false;
                }
                let mut velocity = born.get(&id).is_some_and(|e| e.velocity.is_some())
                    || live
                        .as_ref()
                        .is_some_and(|e| e.contains::<crate::world::Motion>());
                let mut text = self.text_ids.contains(&id);
                let mut spatial = self.spatial_ids.contains(&id);
                let mut audio = self.audio_ids.contains(&id);
                let mut physics = live
                    .as_ref()
                    .is_some_and(|e| e.contains::<crate::physics::Binding>());
                let mut planar = live
                    .as_ref()
                    .is_some_and(|e| e.contains::<crate::physics_2d::Binding2D>());
                let mut navigation = self.navigation.nodes.iter().any(|n| n.entity == id);
                for c in commands.iter() {
                    match c {
                        C::SetVelocity {
                            entity,
                            velocity: v,
                        } if *entity == id => velocity = v.is_some(),
                        C::SetText { entity, text: n } if *entity == id => text = n.is_some(),
                        C::SetSpatial { entity, spatial: n } if *entity == id => {
                            spatial = n.is_some()
                        }
                        C::SetAudio { entity, audio: n } if *entity == id => audio = n.is_some(),
                        C::SetPhysics { entity, physics: n } if *entity == id => {
                            physics = n.is_some()
                        }
                        C::SetPhysics2D { entity, physics: n } if *entity == id => {
                            planar = n.is_some()
                        }
                        C::SetNavigation {
                            entity,
                            navigation: n,
                        } if *entity == id => navigation = n.is_some(),
                        _ => {}
                    }
                }
                !(velocity || text || spatial || audio || physics || planar || navigation)
            },
            self.parents
                .keys()
                .chain(born.keys())
                .filter(|id| exists(id))
                .map(|&id| (id, parent(id).flatten())),
        )?;
        let mut text_bytes: BTreeMap<_, _> = self
            .text_ids
            .iter()
            .map(|id| {
                (
                    *id,
                    self.world
                        .get::<crate::text::TextComponent>(self.entities[id])
                        .expect("text index")
                        .0
                        .content
                        .len(),
                )
            })
            .collect();
        for c in commands.iter() {
            match c {
                C::Despawn(id) => {
                    text_bytes.remove(id);
                }
                C::SetText { entity, text } => {
                    if let Some(t) = text {
                        text_bytes.insert(*entity, t.content.len());
                    } else {
                        text_bytes.remove(entity);
                    }
                }
                _ => {}
            }
        }
        let bytes = declarations
            .iter()
            .filter_map(|n| n.node.kind.text())
            .map(|t| t.content.len())
            .chain(text_bytes.values().copied())
            .try_fold(0usize, |n, len| {
                n.checked_add(len)
                    .filter(|n| *n <= incant_types::MAX_SCENE_TEXT_BYTES)
                    .ok_or(SceneError::Invalid("combined scene text bytes"))
            })?;
        let _ = bytes;
        let changed = nodes != self.ui
            || nodes.keys().any(|id| {
                parents
                    .get(id)
                    .is_some_and(|p| self.parents.get(id) != Some(p))
            });
        if !changed {
            return Ok(None);
        }
        let generation = self
            .ui_generation
            .checked_add(1)
            .ok_or(RuntimeError::GenerationOverflow)?;
        Ok(Some(PendingUi { nodes, generation }))
    }
    pub(crate) fn publish_ui(&mut self, p: Option<PendingUi>) {
        if let Some(p) = p {
            self.ui = p.nodes;
            self.ui_generation = p.generation;
            self.ui_used |= !self.ui.is_empty();
        }
    }
}
pub(crate) struct PendingUi {
    nodes: BTreeMap<StableId, Arc<NativeUi>>,
    generation: u64,
}
impl<'a> UiView<'a> {
    pub fn identity(&self) -> &'a crate::WorldIdentity {
        &self.world.identity
    }
    pub fn generation(&self) -> u64 {
        self.world.ui_generation
    }
    pub fn fonts(&self) -> &'a [crate::FontAssetRef] {
        &self.world.font_assets
    }
    pub fn nodes(&self) -> impl ExactSizeIterator<Item = UiViewNode<'a>> + 'a {
        let w = self.world;
        w.ui.iter().map(move |(&id, node)| UiViewNode {
            id,
            parent: w.parents[&id],
            incarnation: w.entities[&id].to_bits(),
            node,
        })
    }
}

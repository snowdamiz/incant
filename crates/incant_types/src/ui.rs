//! Screen-space UI declarations. World-space `Text` keeps its separate contract.
use crate::{Id, MAX_TEXT_BYTES, MAX_TEXT_FONTS, TextWrap};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const MAX_UI_NODES: usize = 1024;
pub const MAX_UI_DEPTH: usize = 64;
pub const MAX_UI_EXTENT: f32 = 65536.0;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum UiLength {
    Auto,
    Pixels(f32),
    /// Fraction of the containing block's dimension, from zero through one.
    Fraction(f32),
}
impl UiLength {
    pub fn validate(self) -> Result<(), &'static str> {
        match self {
            Self::Auto => Ok(()),
            Self::Pixels(value) if extent(value) => Ok(()),
            Self::Fraction(value) if value.is_finite() && (0.0..=1.0).contains(&value) => Ok(()),
            _ => Err("invalid UI dimension"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum UiDirection {
    Row,
    RowReverse,
    Column,
    ColumnReverse,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum UiAlign {
    Start,
    End,
    Center,
    Stretch,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum UiJustify {
    Start,
    End,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

/// Small typed flexbox subset. All lengths use logical pixels. No CSS strings,
/// implicit physical-pixel rounding, absolute positioning or scroll state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UiStyle {
    pub size: [UiLength; 2],
    pub min_size: [f32; 2],
    pub max_size: [Option<f32>; 2],
    /// Left, top, right, bottom, in logical pixels.
    pub padding: [f32; 4],
    /// Horizontal and vertical gaps.
    pub gap: [f32; 2],
    pub direction: UiDirection,
    pub wrap: bool,
    pub align: UiAlign,
    pub justify: UiJustify,
    pub grow: f32,
    pub shrink: f32,
    pub basis: UiLength,
}
impl Default for UiStyle {
    fn default() -> Self {
        Self {
            size: [UiLength::Auto; 2],
            min_size: [0.0; 2],
            max_size: [None; 2],
            padding: [0.0; 4],
            gap: [0.0; 2],
            direction: UiDirection::Column,
            wrap: false,
            align: UiAlign::Stretch,
            justify: UiJustify::Start,
            grow: 0.0,
            shrink: 1.0,
            basis: UiLength::Auto,
        }
    }
}
impl UiStyle {
    pub fn validate(&self) -> Result<(), &'static str> {
        for dimension in self.size.iter().copied().chain([self.basis]) {
            dimension.validate()?;
        }
        if !self
            .padding
            .iter()
            .chain(&self.gap)
            .chain(&self.min_size)
            .copied()
            .all(extent)
            || self.max_size.iter().flatten().copied().any(|n| !extent(n))
            || (0..2).any(|i| self.max_size[i].is_some_and(|max| max < self.min_size[i]))
            || [self.grow, self.shrink]
                .iter()
                .any(|n| !n.is_finite() || !(0.0..=100.0).contains(n))
        {
            return Err("invalid UI flexbox bounds");
        }
        Ok(())
    }
}

/// UI text has no authored width: the flexbox content box is its sole width
/// authority. IDs are authoring strings or already checked native StableIds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UiText<I = Id> {
    #[schemars(length(max = 16384))]
    pub content: String,
    #[schemars(length(min = 1, max = 32))]
    pub fonts: Vec<I>,
    pub font_size: f32,
    pub line_height: f32,
    pub wrap: TextWrap,
}
impl<I: PartialEq> UiText<I> {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.content.len() > MAX_TEXT_BYTES
            || self.fonts.is_empty()
            || self.fonts.len() > MAX_TEXT_FONTS
            || self
                .fonts
                .iter()
                .enumerate()
                .any(|(i, font)| self.fonts[..i].contains(font))
            || !self.font_size.is_finite()
            || !(1.0..=512.0).contains(&self.font_size)
            || !self.line_height.is_finite()
            || !(1.0..=4096.0).contains(&self.line_height)
        {
            return Err("invalid UI text or font chain");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "role", rename_all = "snake_case", deny_unknown_fields)]
pub enum UiKind<I = Id> {
    Root,
    Container,
    Label { text: UiText<I> },
    Button { text: UiText<I> },
    TextInput { text: UiText<I> },
}
impl<I> UiKind<I> {
    pub fn text(&self) -> Option<&UiText<I>> {
        match self {
            Self::Label { text } | Self::Button { text } | Self::TextInput { text } => Some(text),
            Self::Root | Self::Container => None,
        }
    }
    pub fn is_container(&self) -> bool {
        matches!(self, Self::Root | Self::Container)
    }
    pub fn is_focusable(&self) -> bool {
        matches!(self, Self::Button { .. } | Self::TextInput { .. })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UiNode<I = Id> {
    pub kind: UiKind<I>,
    pub style: UiStyle,
    /// Siblings sort by (order, StableId), independent of map/insertion order.
    pub order: i32,
    /// Disabled subtrees do not accept focus or input; layout is retained.
    pub disabled: bool,
}
impl<I: PartialEq> UiNode<I> {
    pub fn validate(&self) -> Result<(), &'static str> {
        self.style.validate()?;
        if matches!(self.kind, UiKind::Root)
            && (self.style.size != [UiLength::Auto; 2]
                || self.style.min_size != [0.0; 2]
                || self.style.max_size != [None; 2]
                || self.style.basis != UiLength::Auto)
        {
            return Err("UI root dimensions belong to the host viewport");
        }
        if let Some(text) = self.kind.text() {
            text.validate()?;
        }
        Ok(())
    }
}

fn extent(value: f32) -> bool {
    value.is_finite() && (0.0..=MAX_UI_EXTENT).contains(&value)
}

/// Borrowed final-state links: callers must supply the entity's actual parent,
/// not an independently retained UI parent. Indices below refer to this slice.
pub struct UiLink<'a, I> {
    pub id: crate::StableId,
    pub parent: Option<crate::StableId>,
    pub node: &'a UiNode<I>,
}
pub struct UiTopology {
    pub root: usize,
    pub parents: Vec<Option<usize>>,
    pub preorder: Vec<usize>,
}

/// Check one complete UI hierarchy, including ordinary entity reparent/despawn
/// effects. Empty UI is represented outside this function, not by a fake root.
pub fn validate_ui_tree<I: PartialEq>(links: &[UiLink<'_, I>]) -> Result<UiTopology, &'static str> {
    if links.is_empty() || links.len() > MAX_UI_NODES {
        return Err("UI node count");
    }
    let mut by_id: Vec<_> = (0..links.len()).collect();
    by_id.sort_unstable_by_key(|&i| links[i].id);
    if by_id
        .windows(2)
        .any(|pair| links[pair[0]].id == links[pair[1]].id)
    {
        return Err("duplicate UI entity");
    }
    let mut parents = Vec::with_capacity(links.len());
    let mut root = None;
    let mut text_bytes = 0usize;
    for (i, link) in links.iter().enumerate() {
        link.node.validate()?;
        if let Some(text) = link.node.kind.text() {
            text_bytes = text_bytes
                .checked_add(text.content.len())
                .ok_or("UI text bytes")?;
            if text_bytes > crate::MAX_SCENE_TEXT_BYTES {
                return Err("UI aggregate text bytes");
            }
        }
        match (&link.node.kind, link.parent) {
            (UiKind::Root, None) => {
                if root.replace(i).is_some() {
                    return Err("multiple UI roots");
                }
                parents.push(None);
            }
            (UiKind::Root, Some(_)) | (_, None) => return Err("invalid UI root parent"),
            (_, Some(parent)) => {
                let slot = by_id
                    .binary_search_by_key(&parent, |&index| links[index].id)
                    .map_err(|_| "UI parent is not a UI entity")?;
                let parent = by_id[slot];
                if !links[parent].node.kind.is_container() {
                    return Err("UI leaf cannot own children");
                }
                parents.push(Some(parent));
            }
        }
    }
    let root = root.ok_or("missing UI root")?;
    // Bound depth before recursion in the layout library. A disconnected cycle
    // cannot terminate at the sole root, so it is also refused here.
    for i in 0..links.len() {
        let mut at = i;
        let mut depth = 1;
        while let Some(parent) = parents[at] {
            depth += 1;
            if depth > MAX_UI_DEPTH {
                return Err("UI hierarchy cycle or depth limit");
            }
            at = parent;
        }
        if at != root {
            return Err("disconnected UI hierarchy");
        }
    }
    // Store each parent's direct children together, in authored order. The
    // explicit stack visits each node once and is bounded by node admission.
    let mut children: Vec<_> = (0..links.len()).filter(|&i| i != root).collect();
    children.sort_unstable_by_key(|&i| (parents[i], links[i].node.order, links[i].id));
    let mut stack = Vec::with_capacity(links.len());
    let mut preorder = Vec::with_capacity(links.len());
    stack.push(root);
    while let Some(at) = stack.pop() {
        preorder.push(at);
        let start = children.partition_point(|&i| parents[i] < Some(at));
        let end = children.partition_point(|&i| parents[i] <= Some(at));
        stack.extend(children[start..end].iter().rev().copied());
    }
    if preorder.len() != links.len() {
        return Err("disconnected UI hierarchy");
    }
    Ok(UiTopology {
        root,
        parents,
        preorder,
    })
}

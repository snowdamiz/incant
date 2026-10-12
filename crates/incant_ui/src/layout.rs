use crate::{MAX_UI_DEPTH, MAX_UI_EXTENT, StableId, UiError, UiNode};
use incant_types::{UiLink, UiTopology, validate_ui_tree};
use std::sync::Arc;
use taffy::{AvailableSpace, NodeId, Size, TaffyTree};

pub const MAX_MEASURE_CALLS: usize = 8192;

#[derive(Clone)]
pub struct Widget {
    pub id: StableId,
    /// Lifetime supplied by the native entity owner, never an authored ID.
    pub incarnation: u64,
    /// Copied from the same committed entity hierarchy as `node`.
    pub parent: Option<StableId>,
    pub node: Arc<UiNode<StableId>>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub origin: [f32; 2],
    pub size: [f32; 2],
}
impl Rect {
    pub fn contains(self, point: [f32; 2]) -> bool {
        (0..2).all(|i| point[i] >= self.origin[i] && point[i] < self.origin[i] + self.size[i])
    }
    pub fn intersect(self, other: Self) -> Self {
        let origin = std::array::from_fn(|i| self.origin[i].max(other.origin[i]));
        let size = std::array::from_fn(|i| {
            ((self.origin[i] + self.size[i]).min(other.origin[i] + other.size[i]) - origin[i])
                .max(0.0)
        });
        Self { origin, size }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetLayout {
    pub id: StableId,
    pub bounds: Rect,
    pub content: Rect,
    pub clip: Rect,
    pub hit: Rect,
    /// Includes disabled ancestors.
    pub disabled: bool,
}

/// An immutable layout in logical pixels. Iteration uses stable paint/focus
/// preorder, not an internal Taffy node index or entity insertion order.
pub struct LayoutFrame {
    focusable: bool,
    // Both directions are constructed with the canonical preorder. Input never
    // joins two arrays by repeated stable-ID searches inside its node scans.
    layout_to_widget: Vec<usize>,
    widget_to_layout: Vec<usize>,
    pub viewport: [f32; 2],
    pub generation: u64,
    pub content_generation: u64,
    pub nodes: Vec<WidgetLayout>,
}

impl LayoutFrame {
    pub(crate) fn widget_index(&self, position: usize) -> usize {
        self.layout_to_widget[position]
    }
    pub(crate) fn layout_index(&self, widget: usize) -> usize {
        self.widget_to_layout[widget]
    }
    pub(crate) fn widget_layout(&self, widget: usize) -> &WidgetLayout {
        &self.nodes[self.layout_index(widget)]
    }
    /// Cached during the same bounded layout pass; routing never rescans the tree.
    pub fn has_focusable(&self) -> bool {
        self.focusable
    }
}

#[derive(Clone, Copy, Debug)]
pub enum MeasureSpace {
    Definite(f32),
    MinContent,
    MaxContent,
}
/// Trusted packaged-font integration. Failures poison this entire candidate;
/// no zero-size fallback is published. Inputs use content-box dimensions.
pub trait TextMeasure {
    fn measure(
        &mut self,
        widget: &Widget,
        display: &str,
        known: [Option<f32>; 2],
        available: [MeasureSpace; 2],
    ) -> Result<[f32; 2], UiError>;
}

pub struct LayoutEngine {
    widgets: Arc<[Widget]>,
    topology: Option<UiTopology>,
    tree: TaffyTree<usize>,
    slots: Vec<NodeId>,
    published: Option<Arc<LayoutFrame>>,
    generation: u64,
}

/// Private tree state is not exposed through the read-only frame. A caller can
/// prepare before a game tick and discard on refusal without undoing any layout.
pub struct LayoutCandidate {
    owner: Arc<[Widget]>,
    previous_generation: u64,
    tree: Option<TaffyTree<usize>>,
    frame: Arc<LayoutFrame>,
}
impl LayoutCandidate {
    pub fn frame(&self) -> &Arc<LayoutFrame> {
        &self.frame
    }
}
impl LayoutEngine {
    pub fn new(widgets: Vec<Widget>) -> Result<Self, UiError> {
        if widgets.len() > crate::MAX_UI_NODES {
            return Err(UiError::Limit("widget count"));
        }
        let links: Vec<_> = widgets
            .iter()
            .map(|w| UiLink {
                id: w.id,
                parent: w.parent,
                node: &*w.node,
            })
            .collect();
        let topology = if links.is_empty() {
            None
        } else {
            Some(validate_ui_tree(&links).map_err(UiError::Invalid)?)
        };
        let mut tree = TaffyTree::with_capacity(widgets.len());
        tree.disable_rounding();
        let mut slots = Vec::with_capacity(widgets.len());
        for (i, widget) in widgets.iter().enumerate() {
            slots.push(
                tree.new_leaf_with_context(super::style::convert(&widget.node.style), i)
                    .map_err(|_| UiError::Layout)?,
            );
        }
        for (i, widget) in widgets.iter().enumerate() {
            if widget.node.kind.is_container() {
                let topology = topology.as_ref().expect("nonempty container");
                let children: Vec<_> = topology
                    .preorder
                    .iter()
                    .filter(|&&child| topology.parents[child] == Some(i))
                    .map(|&child| slots[child])
                    .collect();
                tree.set_children(slots[i], &children)
                    .map_err(|_| UiError::Layout)?;
            }
        }
        Ok(Self {
            widgets: widgets.into(),
            topology,
            tree,
            slots,
            published: None,
            generation: 0,
        })
    }

    pub fn widgets(&self) -> &[Widget] {
        &self.widgets
    }
    pub fn frame(&self) -> Option<&Arc<LayoutFrame>> {
        self.published.as_ref()
    }

    /// Unchanged generation/viewport returns the exact retained frame without
    /// layout, measurement, tree cloning or allocation. Content generations are
    /// supplied by the owning widget state, never by project code.
    pub fn prepare(
        &self,
        viewport: [f32; 2],
        content_generation: u64,
        measure: &mut impl TextMeasure,
    ) -> Result<LayoutCandidate, UiError> {
        self.prepare_display(viewport, content_generation, None, measure)
    }

    pub(crate) fn prepare_display(
        &self,
        viewport: [f32; 2],
        content_generation: u64,
        buffers: Option<&[Option<Arc<incant_text::editing::TextBuffer>>]>,
        measure: &mut impl TextMeasure,
    ) -> Result<LayoutCandidate, UiError> {
        if buffers.is_some_and(|b| b.len() != self.widgets.len()) {
            return Err(UiError::Invalid("widget buffer count"));
        }
        if viewport
            .iter()
            .any(|n| !n.is_finite() || !(1.0..=MAX_UI_EXTENT).contains(n))
        {
            return Err(UiError::Invalid("viewport bounds"));
        }
        if let Some(frame) = &self.published
            && frame.viewport == viewport
            && frame.content_generation == content_generation
        {
            return Ok(LayoutCandidate {
                owner: Arc::clone(&self.widgets),
                previous_generation: self.generation,
                tree: None,
                frame: Arc::clone(frame),
            });
        }
        let generation = self
            .generation
            .checked_add(1)
            .ok_or(UiError::Limit("layout generation"))?;
        let Some(topology) = &self.topology else {
            return Ok(LayoutCandidate {
                owner: Arc::clone(&self.widgets),
                previous_generation: self.generation,
                tree: None,
                frame: Arc::new(LayoutFrame {
                    focusable: false,
                    layout_to_widget: Vec::new(),
                    widget_to_layout: Vec::new(),
                    viewport,
                    generation,
                    content_generation,
                    nodes: Vec::new(),
                }),
            });
        };
        // Changed layout may overlap one previous bounded tree and output. We do
        // not copy the scene, font bytes, text strings or immutable declarations.
        let mut tree = self.tree.clone();
        let root = self.slots[topology.root];
        let mut style = super::style::convert(&self.widgets[topology.root].node.style);
        style.size = Size {
            width: taffy::Dimension::length(viewport[0]),
            height: taffy::Dimension::length(viewport[1]),
        };
        tree.set_style(root, style).map_err(|_| UiError::Layout)?;
        if self
            .published
            .as_ref()
            .is_none_or(|frame| frame.content_generation != content_generation)
        {
            for (i, widget) in self.widgets.iter().enumerate() {
                if widget.node.kind.text().is_some() {
                    tree.mark_dirty(self.slots[i])
                        .map_err(|_| UiError::Layout)?;
                }
            }
        }
        let mut failure = None;
        let mut calls = 0usize;
        tree.compute_layout_with_measure(
            root,
            Size {
                width: AvailableSpace::Definite(viewport[0]),
                height: AvailableSpace::Definite(viewport[1]),
            },
            |inputs, _, context, style| {
                taffy::compute_leaf_layout(
                    inputs,
                    style,
                    |_, _| 0.0,
                    |known, available| {
                        if failure.is_some() {
                            return Size::ZERO;
                        }
                        let Some(index) = context.as_deref() else {
                            return Size::ZERO;
                        };
                        let widget = &self.widgets[*index];
                        if widget.node.kind.is_container() {
                            return Size::ZERO;
                        }
                        calls += 1;
                        if calls > MAX_MEASURE_CALLS {
                            failure = Some(UiError::Limit("text measurement work"));
                            return Size::ZERO;
                        }
                        match measure.measure(
                            widget,
                            buffers.and_then(|b| b[*index].as_ref()).map_or_else(
                                || widget.node.kind.text().map_or("", |t| t.content.as_str()),
                                |buffer| buffer.display_text(),
                            ),
                            [known.width, known.height],
                            [space(available.width), space(available.height)],
                        ) {
                            Ok(size)
                                if size.iter().all(|n| {
                                    n.is_finite() && (0.0..=MAX_UI_EXTENT).contains(n)
                                }) =>
                            {
                                Size {
                                    width: size[0],
                                    height: size[1],
                                }
                            }
                            Ok(_) => {
                                failure = Some(UiError::Invalid("measured text extent"));
                                Size::ZERO
                            }
                            Err(error) => {
                                failure = Some(error);
                                Size::ZERO
                            }
                        }
                    },
                )
            },
        )
        .map_err(|_| UiError::Layout)?;
        if let Some(error) = failure {
            return Err(error);
        }
        let mut positions = vec![Rect::default(); self.widgets.len()];
        let mut clips = vec![
            Rect {
                origin: [0.0; 2],
                size: viewport
            };
            self.widgets.len()
        ];
        let mut disabled = vec![false; self.widgets.len()];
        let mut nodes = Vec::with_capacity(self.widgets.len());
        let mut widget_to_layout = vec![0; self.widgets.len()];
        let mut focusable = false;
        for &i in &topology.preorder {
            widget_to_layout[i] = nodes.len();
            let layout = tree.layout(self.slots[i]).map_err(|_| UiError::Layout)?;
            let parent = topology.parents[i];
            let base = parent.map_or([0.0; 2], |p| positions[p].origin);
            let bounds = Rect {
                origin: [base[0] + layout.location.x, base[1] + layout.location.y],
                size: [layout.size.width, layout.size.height],
            };
            if bounds
                .origin
                .iter()
                .any(|n| !n.is_finite() || n.abs() > MAX_UI_EXTENT * MAX_UI_DEPTH as f32)
                || bounds
                    .size
                    .iter()
                    .any(|n| !n.is_finite() || !(0.0..=MAX_UI_EXTENT).contains(n))
            {
                return Err(UiError::Invalid("computed layout bounds"));
            }
            let content = Rect {
                origin: [
                    bounds.origin[0] + layout.padding.left,
                    bounds.origin[1] + layout.padding.top,
                ],
                size: [
                    (bounds.size[0] - layout.padding.left - layout.padding.right).max(0.0),
                    (bounds.size[1] - layout.padding.top - layout.padding.bottom).max(0.0),
                ],
            };
            let inherited = parent.map_or(
                Rect {
                    origin: [0.0; 2],
                    size: viewport,
                },
                |p| clips[p],
            );
            positions[i] = bounds;
            clips[i] = inherited.intersect(content);
            disabled[i] = self.widgets[i].node.disabled || parent.is_some_and(|p| disabled[p]);
            let hit = inherited.intersect(bounds);
            focusable |= !disabled[i]
                && self.widgets[i].node.kind.is_focusable()
                && hit.size.iter().all(|v| *v > 0.0);
            nodes.push(WidgetLayout {
                id: self.widgets[i].id,
                bounds,
                content,
                clip: clips[i],
                hit,
                disabled: disabled[i],
            });
        }
        let frame = Arc::new(LayoutFrame {
            focusable,
            layout_to_widget: topology.preorder.clone(),
            widget_to_layout,
            viewport,
            generation,
            content_generation,
            nodes,
        });
        Ok(LayoutCandidate {
            owner: Arc::clone(&self.widgets),
            previous_generation: self.generation,
            tree: Some(tree),
            frame,
        })
    }

    pub fn commit(&mut self, candidate: LayoutCandidate) -> Result<(), UiError> {
        if !Arc::ptr_eq(&candidate.owner, &self.widgets)
            || candidate.previous_generation != self.generation
        {
            return Err(UiError::Stale);
        }
        if let Some(tree) = candidate.tree {
            self.tree = tree;
        }
        self.generation = candidate.frame.generation;
        self.published = Some(candidate.frame);
        Ok(())
    }
}
fn space(value: AvailableSpace) -> MeasureSpace {
    match value {
        AvailableSpace::Definite(value) => MeasureSpace::Definite(value),
        AvailableSpace::MinContent => MeasureSpace::MinContent,
        AvailableSpace::MaxContent => MeasureSpace::MaxContent,
    }
}

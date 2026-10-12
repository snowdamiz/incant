//! Focus and composition belong to one retained widget owner. Platform events
//! carry an observed token; a delayed event cannot acquire the current focus.
use crate::{
    LayoutCandidate, LayoutEngine, LayoutFrame, StableId, TextMeasure, UiError, UiKind, Widget,
};
use incant_text::editing::{Movement, Selection, TextBuffer};
use std::{ops::Range, sync::Arc};

pub const MAX_UI_EVENTS: usize = 256;
pub const MAX_UI_PACKET_BYTES: usize = 64 * 1024;

#[derive(Clone, Debug)]
pub struct FocusToken {
    pub(crate) owner: Arc<()>,
    pub(crate) epoch: u64,
    pub(crate) widget: StableId,
}
impl FocusToken {
    pub fn widget(&self) -> StableId {
        self.widget
    }
    pub fn epoch(&self) -> u64 {
        self.epoch
    }
}

/// A pointer receipt is bound to the displayed owner, including world and
/// widget-tree incarnation. Equal numeric generations are not interchangeable.
#[derive(Clone, Debug)]
pub struct PointerToken {
    owner: Arc<()>,
    generation: u64,
    delivery_epoch: u64,
}

#[derive(Clone, Copy, Debug)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}
/// Logical UI input. Physical routing and platform IME delivery remain outside
/// this module. No key position is interpreted as a character.
pub enum UiEvent {
    Focus(StableId),
    Traverse {
        reverse: bool,
    },
    Navigate(Direction),
    Activate,
    /// A pointer event refers to the layout which was actually displayed.
    Pointer {
        position: [f32; 2],
        pressed: bool,
        target: PointerToken,
    },
    WindowFocus(bool),
    Preedit {
        target: FocusToken,
        text: String,
        cursor: Option<Range<usize>>,
    },
    Commit {
        target: FocusToken,
        text: String,
    },
    Cancel {
        target: FocusToken,
    },
    Select {
        target: FocusToken,
        selection: Selection,
    },
    Move {
        target: FocusToken,
        movement: Movement,
        extend: bool,
    },
    Backspace {
        target: FocusToken,
    },
    Delete {
        target: FocusToken,
    },
    Submit {
        target: FocusToken,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiAction {
    Activated(StableId),
    ValueChanged { widget: StableId, value: Arc<str> },
    Submitted { widget: StableId, value: Arc<str> },
}

#[derive(Clone)]
pub(crate) struct State {
    pub(crate) buffers: Vec<Option<Arc<TextBuffer>>>,
    pub(crate) focus: Option<usize>,
    pub(crate) pointer: Option<usize>,
    pub(crate) epoch: u64,
    pub(crate) delivery_epoch: u64,
    pub(crate) content_generation: u64,
    pub(crate) window_focused: bool,
}

/// CPU widget owner, prepared from the same checked scene as its composed game.
/// Starting a new/rebuilt owner expires every prior focus token. Public mutation
/// is prepare/commit only; dropping a candidate preserves all accepted state.
pub struct UiRuntime {
    pub(crate) owner: Arc<()>,
    pub(crate) layout: LayoutEngine,
    pub(crate) state: State,
    pub(crate) revision: u64,
    pub(crate) actions: Vec<UiAction>,
    pub(crate) discarded_events: usize,
}
pub struct UiPending {
    pub(crate) owner: Arc<()>,
    pub(crate) previous_revision: u64,
    pub(crate) revision: u64,
    pub(crate) state: Option<State>,
    pub(crate) replacement: Option<(Arc<()>, LayoutEngine)>,
    pub(crate) layout: LayoutCandidate,
    pub(crate) actions: Vec<UiAction>,
    pub(crate) discarded_events: usize,
}
impl UiPending {
    pub fn layout(&self) -> &Arc<LayoutFrame> {
        self.layout.frame()
    }
    pub fn actions(&self) -> &[UiAction] {
        &self.actions
    }
    pub fn discarded_events(&self) -> usize {
        self.discarded_events
    }
    pub(crate) fn prepared<'a>(
        &'a self,
        runtime: &'a UiRuntime,
    ) -> Result<(&'a [Widget], &'a State), UiError> {
        if !Arc::ptr_eq(&self.owner, &runtime.owner) || self.previous_revision != runtime.revision {
            return Err(UiError::Stale);
        }
        Ok((
            self.replacement
                .as_ref()
                .map_or_else(|| runtime.widgets(), |(_, layout)| layout.widgets()),
            self.state.as_ref().unwrap_or(&runtime.state),
        ))
    }
}

impl UiRuntime {
    pub fn new(widgets: Vec<Widget>) -> Result<Self, UiError> {
        let layout = LayoutEngine::new(widgets)?;
        let buffers = layout
            .widgets()
            .iter()
            .map(|widget| {
                if let UiKind::TextInput { text } = &widget.node.kind {
                    Ok(Some(Arc::new(TextBuffer::new(&text.content)?)))
                } else {
                    Ok(None)
                }
            })
            .collect::<Result<Vec<_>, UiError>>()?;
        Ok(Self {
            owner: Arc::new(()),
            layout,
            state: State {
                buffers,
                focus: None,
                pointer: None,
                epoch: 0,
                delivery_epoch: 0,
                content_generation: 0,
                window_focused: true,
            },
            revision: 0,
            actions: Vec::new(),
            discarded_events: 0,
        })
    }
    pub fn layout(&self) -> Option<&Arc<LayoutFrame>> {
        self.layout.frame()
    }
    pub fn actions(&self) -> &[UiAction] {
        &self.actions
    }
    /// Well-formed delayed receipts discarded in the last accepted UI packet.
    pub fn discarded_events(&self) -> usize {
        self.discarded_events
    }
    pub fn focus(&self) -> Option<FocusToken> {
        self.state.focus.map(|i| FocusToken {
            owner: Arc::clone(&self.owner),
            epoch: self.state.epoch,
            widget: self.layout.widgets()[i].id,
        })
    }
    pub fn pointer_token(&self) -> Option<PointerToken> {
        self.layout().map(|frame| PointerToken {
            owner: Arc::clone(&self.owner),
            generation: frame.generation,
            delivery_epoch: self.state.delivery_epoch,
        })
    }
    pub fn widgets(&self) -> &[Widget] {
        self.layout.widgets()
    }
    pub fn text(&self, id: StableId) -> Option<&TextBuffer> {
        let i = self.layout.widgets().iter().position(|w| w.id == id)?;
        self.state.buffers[i].as_deref()
    }

    pub fn prepare(
        &self,
        events: &[UiEvent],
        viewport: [f32; 2],
        measure: &mut impl TextMeasure,
    ) -> Result<UiPending, UiError> {
        validate_packet(events)?;
        let mut actions = Vec::new();
        let mut discarded_events = 0;
        let mut state = if events.is_empty() {
            None
        } else {
            Some(self.state.clone())
        };
        if let Some(state) = &mut state {
            let frame = self
                .layout
                .frame()
                .ok_or(UiError::Invalid("UI input before initial layout"))?;
            let context = Context {
                owner: &self.owner,
                widgets: self.layout.widgets(),
                layout: frame,
                pointer_current: frame.viewport == viewport,
            };
            for event in events {
                match context.apply(state, &mut actions, event) {
                    Ok(()) => {}
                    Err(UiError::Stale) => discarded_events += 1,
                    Err(error) => return Err(error),
                }
            }
        }
        self.finish(state, actions, discarded_events, viewport, measure)
    }

    pub(crate) fn finish(
        &self,
        mut state: Option<State>,
        actions: Vec<UiAction>,
        discarded_events: usize,
        viewport: [f32; 2],
        measure: &mut impl TextMeasure,
    ) -> Result<UiPending, UiError> {
        if let Some(state) = &mut state {
            let bytes = state.buffers.iter().zip(self.layout.widgets()).try_fold(
                0usize,
                |total, (buffer, widget)| {
                    let len = buffer.as_ref().map_or_else(
                        || widget.node.kind.text().map_or(0, |text| text.content.len()),
                        |buffer| buffer.display_text().len(),
                    );
                    total
                        .checked_add(len)
                        .ok_or(UiError::Limit("widget text bytes"))
                },
            )?;
            if bytes > incant_types::MAX_SCENE_TEXT_BYTES {
                return Err(UiError::Limit("widget text bytes"));
            }
            if state.buffers.iter().zip(&self.state.buffers).any(|(a, b)| {
                a.as_ref().map(|v| v.display_text()) != b.as_ref().map(|v| v.display_text())
            }) {
                state.content_generation = self
                    .state
                    .content_generation
                    .checked_add(1)
                    .ok_or(UiError::Limit("UI text generation"))?;
            }
        }
        let current = state.as_ref().unwrap_or(&self.state);
        let layout = self.layout.prepare_display(
            viewport,
            current.content_generation,
            Some(&current.buffers),
            measure,
        )?;
        let revision = self
            .revision
            .checked_add(1)
            .ok_or(UiError::Limit("UI revision"))?;
        Ok(UiPending {
            owner: Arc::clone(&self.owner),
            previous_revision: self.revision,
            replacement: None,
            revision,
            state,
            layout,
            actions,
            discarded_events,
        })
    }

    /// Relayout accepted widget state without clearing the last tick notifications.
    pub fn prepare_refresh(
        &self,
        viewport: [f32; 2],
        measure: &mut impl TextMeasure,
    ) -> Result<UiPending, UiError> {
        self.finish(
            None,
            self.actions.clone(),
            self.discarded_events,
            viewport,
            measure,
        )
    }
    pub fn commit(&mut self, pending: UiPending) -> Result<(), UiError> {
        if !Arc::ptr_eq(&pending.owner, &self.owner) || pending.previous_revision != self.revision {
            return Err(UiError::Stale);
        }
        // No observable widget mutation occurs unless layout accepts the exact
        // same owner's candidate. The remaining assignments cannot fail.
        if let Some((owner, mut layout)) = pending.replacement {
            layout.commit(pending.layout)?;
            self.owner = owner;
            self.layout = layout;
        } else {
            self.layout.commit(pending.layout)?;
        }
        if let Some(state) = pending.state {
            self.state = state;
        }
        self.actions = pending.actions;
        self.discarded_events = pending.discarded_events;
        self.revision = pending.revision;
        Ok(())
    }
}

pub(crate) struct Context<'a> {
    pub(crate) owner: &'a Arc<()>,
    pub(crate) widgets: &'a [Widget],
    pub(crate) layout: &'a LayoutFrame,
    pub(crate) pointer_current: bool,
}
impl Context<'_> {
    fn index(&self, id: StableId) -> Option<usize> {
        self.widgets.iter().position(|w| w.id == id)
    }
    fn eligible(&self, i: usize) -> bool {
        let node = self.layout.widget_layout(i);
        self.widgets[i].node.kind.is_focusable()
            && !node.disabled
            && node.hit.size.iter().all(|v| *v > 0.0)
    }
    fn focus(&self, state: &mut State, next: Option<usize>) -> Result<(), UiError> {
        if next.is_some_and(|i| !state.window_focused || !self.eligible(i)) {
            return Err(UiError::Invalid("widget cannot receive focus"));
        }
        if state.focus == next {
            return Ok(());
        }
        state.epoch = state
            .epoch
            .checked_add(1)
            .ok_or(UiError::Limit("focus epoch"))?;
        if let Some(i) = state.focus
            && state.buffers[i].is_some()
        {
            editable(state, i)?.set_focused(false);
        }
        if let Some(i) = next
            && state.buffers[i].is_some()
        {
            editable(state, i)?.set_focused(true);
        }
        state.focus = next;
        state.pointer = None;
        Ok(())
    }
    fn target(&self, state: &State, target: &FocusToken) -> Result<usize, UiError> {
        if !Arc::ptr_eq(self.owner, &target.owner)
            || state.epoch != target.epoch
            || !state.window_focused
        {
            return Err(UiError::Stale);
        }
        let i = state.focus.ok_or(UiError::Stale)?;
        if self.widgets[i].id != target.widget || state.buffers[i].is_none() {
            return Err(UiError::Stale);
        }
        Ok(i)
    }
    fn activate(&self, state: &State, actions: &mut Vec<UiAction>) -> Result<(), UiError> {
        if let Some(i) = state.focus
            && self.eligible(i)
            && state.window_focused
        {
            match &self.widgets[i].node.kind {
                UiKind::Button { .. } => actions.push(UiAction::Activated(self.widgets[i].id)),
                UiKind::TextInput { .. } => {
                    let buffer = state.buffers[i].as_ref().expect("text input buffer");
                    // Platform IME owns Enter while a composition is active.
                    if buffer.preedit().is_none() {
                        emit_value(actions, self.widgets[i].id, buffer.text(), true)?;
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
    fn traverse(&self, state: &mut State, reverse: bool) -> Result<(), UiError> {
        if !state.window_focused {
            return Ok(());
        }
        let current = state.focus;
        let mut ordered = (0..self.layout.nodes.len())
            .map(|position| self.layout.widget_index(position))
            .filter(|&i| self.eligible(i));
        let first = if reverse {
            ordered.next_back()
        } else {
            ordered.next()
        };
        let Some(first) = first else {
            return self.focus(state, None);
        };
        let next = if let Some(current) = current {
            let position = self.layout.layout_index(current);
            let count = self.layout.nodes.len();
            (1..=count)
                .map(|offset| {
                    let index = if reverse {
                        (position + count - offset) % count
                    } else {
                        (position + offset) % count
                    };
                    self.layout.widget_index(index)
                })
                .find(|&i| self.eligible(i))
                .unwrap_or(first)
        } else {
            first
        };
        self.focus(state, Some(next))
    }
    fn navigate(&self, state: &mut State, direction: Direction) -> Result<(), UiError> {
        if !state.window_focused {
            return Ok(());
        }
        let Some(current) = state.focus else {
            return self.traverse(state, false);
        };
        let bounds = self.layout.widget_layout(current).bounds;
        let center = [
            bounds.origin[0] + bounds.size[0] * 0.5,
            bounds.origin[1] + bounds.size[1] * 0.5,
        ];
        let axis = usize::from(matches!(direction, Direction::Up | Direction::Down));
        let sign = if matches!(direction, Direction::Left | Direction::Up) {
            -1.0
        } else {
            1.0
        };
        let mut best: Option<(f32, f32, usize)> = None;
        for (position, node) in self.layout.nodes.iter().enumerate() {
            let i = self.layout.widget_index(position);
            if i == current || !self.eligible(i) {
                continue;
            }
            let other = [
                node.bounds.origin[0] + node.bounds.size[0] * 0.5,
                node.bounds.origin[1] + node.bounds.size[1] * 0.5,
            ];
            let primary = (other[axis] - center[axis]) * sign;
            let secondary = (other[1 - axis] - center[1 - axis]).abs();
            if primary > 0.0
                && best.is_none_or(|(p, s, _)| primary < p || (primary == p && secondary < s))
            {
                best = Some((primary, secondary, i));
            }
        }
        if let Some((_, _, i)) = best {
            self.focus(state, Some(i))?;
        }
        Ok(())
    }
    pub(crate) fn apply(
        &self,
        state: &mut State,
        actions: &mut Vec<UiAction>,
        event: &UiEvent,
    ) -> Result<(), UiError> {
        match event {
            UiEvent::WindowFocus(focused) => {
                if state.window_focused != *focused {
                    state.delivery_epoch = state
                        .delivery_epoch
                        .checked_add(1)
                        .ok_or(UiError::Limit("input delivery epoch"))?;
                }
                state.window_focused = *focused;
                if !focused {
                    self.focus(state, None)?;
                    state.pointer = None;
                }
            }
            UiEvent::Focus(id) => self.focus(
                state,
                Some(
                    self.index(*id)
                        .ok_or(UiError::Invalid("unknown focus widget"))?,
                ),
            )?,
            UiEvent::Traverse { reverse } => self.traverse(state, *reverse)?,
            UiEvent::Navigate(direction) => self.navigate(state, *direction)?,
            UiEvent::Activate => self.activate(state, actions)?,
            UiEvent::Pointer {
                position,
                pressed,
                target,
            } => {
                if !self.pointer_current
                    || !Arc::ptr_eq(self.owner, &target.owner)
                    || target.generation != self.layout.generation
                    || target.delivery_epoch != state.delivery_epoch
                {
                    return Err(UiError::Stale);
                }
                if !state.window_focused {
                    return Ok(());
                }
                let hit = self
                    .layout
                    .nodes
                    .iter()
                    .enumerate()
                    .rev()
                    .filter(|(_, n)| n.hit.contains(*position))
                    .map(|(position, _)| self.layout.widget_index(position))
                    .find(|&i| self.eligible(i));
                if *pressed {
                    self.focus(state, hit)?;
                    state.pointer = hit;
                } else if let Some(captured) = state.pointer.take()
                    && hit == Some(captured)
                    && state.focus == Some(captured)
                {
                    self.activate(state, actions)?;
                }
            }
            _ => {
                let target = match event {
                    UiEvent::Preedit { target, .. }
                    | UiEvent::Commit { target, .. }
                    | UiEvent::Cancel { target }
                    | UiEvent::Select { target, .. }
                    | UiEvent::Move { target, .. }
                    | UiEvent::Backspace { target }
                    | UiEvent::Delete { target }
                    | UiEvent::Submit { target } => target,
                    _ => unreachable!(),
                };
                let i = self.target(state, target)?;
                if matches!(event, UiEvent::Submit { .. }) {
                    self.activate(state, actions)?;
                    return Ok(());
                }
                let before = Arc::clone(state.buffers[i].as_ref().expect("target buffer"));
                let buffer = editable(state, i)?;
                match event {
                    UiEvent::Preedit { text, cursor, .. } => {
                        buffer.set_preedit(text, cursor.clone())?
                    }
                    UiEvent::Commit { text, .. } => buffer.commit(text)?,
                    UiEvent::Cancel { .. } => buffer.cancel_preedit(),
                    UiEvent::Select { selection, .. } => buffer.select(*selection)?,
                    UiEvent::Move {
                        movement, extend, ..
                    } => buffer.move_selection(*movement, *extend)?,
                    UiEvent::Backspace { .. } => buffer.backspace()?,
                    UiEvent::Delete { .. } => buffer.delete_forward()?,
                    _ => unreachable!(),
                }
                if before.text() != buffer.text() {
                    emit_value(actions, self.widgets[i].id, buffer.text(), false)?;
                }
            }
        }
        Ok(())
    }
}

fn emit_value(
    actions: &mut Vec<UiAction>,
    widget: StableId,
    text: &str,
    submit: bool,
) -> Result<(), UiError> {
    let bytes = actions.iter().try_fold(text.len(), |sum, action| {
        let len = match action {
            UiAction::Activated(_) => 0,
            UiAction::ValueChanged { value, .. } | UiAction::Submitted { value, .. } => value.len(),
        };
        sum.checked_add(len)
            .filter(|n| *n <= MAX_UI_PACKET_BYTES)
            .ok_or(UiError::Limit("UI output bytes"))
    })?;
    if bytes > MAX_UI_PACKET_BYTES {
        return Err(UiError::Limit("UI output bytes"));
    }
    let value = Arc::from(text);
    actions.push(if submit {
        UiAction::Submitted { widget, value }
    } else {
        UiAction::ValueChanged { widget, value }
    });
    Ok(())
}

pub(crate) fn editable(state: &mut State, i: usize) -> Result<&mut TextBuffer, UiError> {
    let buffer = state.buffers[i]
        .as_mut()
        .ok_or(UiError::Invalid("widget is not text input"))?;
    if Arc::get_mut(buffer).is_none() {
        let mut candidate = TextBuffer::new(buffer.text())?;
        candidate.select(buffer.selection())?;
        candidate.set_focused(buffer.focused());
        if let Some(preedit) = buffer.preedit() {
            candidate.set_preedit(preedit.text, preedit.cursor)?;
        }
        *buffer = Arc::new(candidate);
    }
    Ok(Arc::get_mut(buffer).expect("unique prepared text buffer"))
}
pub(crate) fn validate_packet(events: &[UiEvent]) -> Result<(), UiError> {
    if events.len() > MAX_UI_EVENTS {
        return Err(UiError::Limit("UI events"));
    }
    let mut bytes = 0usize;
    for event in events {
        match event {
            UiEvent::Preedit { text, cursor, .. } => {
                if cursor.as_ref().is_some_and(|r| {
                    r.start > r.end
                        || !text.is_char_boundary(r.start)
                        || !text.is_char_boundary(r.end)
                }) {
                    return Err(UiError::Invalid("preedit UTF-8 cursor"));
                }
                bytes = bytes
                    .checked_add(text.len())
                    .ok_or(UiError::Limit("UI input bytes"))?;
                if text.len() > incant_text::MAX_TEXT_BYTES {
                    return Err(UiError::Limit("preedit bytes"));
                }
            }
            UiEvent::Commit { text, .. } => {
                bytes = bytes
                    .checked_add(text.len())
                    .ok_or(UiError::Limit("UI input bytes"))?;
                if text.len() > incant_text::MAX_TEXT_BYTES {
                    return Err(UiError::Limit("commit bytes"));
                }
            }
            UiEvent::Pointer { position, .. }
                if position
                    .iter()
                    .any(|n| !n.is_finite() || n.abs() > 1_000_000.0) =>
            {
                return Err(UiError::Invalid("pointer coordinates"));
            }
            _ => {}
        }
        if bytes > MAX_UI_PACKET_BYTES {
            return Err(UiError::Limit("UI input bytes"));
        }
    }
    Ok(())
}

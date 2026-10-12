//! Final native-tree reconciliation happens after a successful native tick,
//! against the pending UI state from that same input packet. No live publication
//! occurs until the composed host accepts both candidates.
use crate::focus::{State, editable};
use crate::{LayoutEngine, TextMeasure, UiError, UiKind, UiPending, UiRuntime, Widget};
use incant_text::editing::TextBuffer;
use std::sync::Arc;

impl UiRuntime {
    pub fn reconcile(
        &self,
        mut pending: UiPending,
        widgets: Vec<Widget>,
        measure: &mut impl TextMeasure,
    ) -> Result<UiPending, UiError> {
        if !Arc::ptr_eq(&pending.owner, &self.owner)
            || pending.previous_revision != self.revision
            || pending.replacement.is_some()
        {
            return Err(UiError::Stale);
        }
        if widgets.len() == self.widgets().len()
            && widgets.iter().zip(self.widgets()).all(|(a, b)| {
                a.id == b.id
                    && a.incarnation == b.incarnation
                    && a.parent == b.parent
                    && a.node == b.node
            })
        {
            return Ok(pending);
        }
        let layout = LayoutEngine::new(widgets)?;
        let prior = pending.state.as_ref().unwrap_or(&self.state);
        let mut state = State {
            buffers: Vec::with_capacity(layout.widgets().len()),
            focus: None,
            pointer: None,
            epoch: prior
                .epoch
                .checked_add(1)
                .ok_or(UiError::Limit("focus epoch"))?,
            delivery_epoch: prior.delivery_epoch,
            content_generation: prior
                .content_generation
                .checked_add(1)
                .ok_or(UiError::Limit("UI text generation"))?,
            window_focused: prior.window_focused,
        };
        for (i, widget) in layout.widgets().iter().enumerate() {
            let previous = self
                .widgets()
                .iter()
                .position(|old| old.id == widget.id && old.incarnation == widget.incarnation);
            let buffer = if let UiKind::TextInput { text } = &widget.node.kind {
                if let Some(old) = previous
                    && let UiKind::TextInput { text: old_text } = &self.widgets()[old].node.kind
                    && old_text.content == text.content
                {
                    prior.buffers[old].clone()
                } else {
                    Some(Arc::new(TextBuffer::new(&text.content)?))
                }
            } else {
                None
            };
            state.buffers.push(buffer);
            // Preserve focus only for the same actual native entity and role.
            // Composition never survives a rebuilt delivery target; text edits
            // accepted earlier in this same tick do survive.
            if previous == prior.focus
                && previous.is_some()
                && std::mem::discriminant(&widget.node.kind)
                    == std::mem::discriminant(&self.widgets()[previous.unwrap()].node.kind)
            {
                state.focus = Some(i);
            }
            if state.buffers[i]
                .as_ref()
                .is_some_and(|b| b.focused() || b.preedit().is_some())
            {
                editable(&mut state, i)?.set_focused(false);
            }
        }
        let _bytes = state
            .buffers
            .iter()
            .zip(layout.widgets())
            .try_fold(0usize, |n, (b, w)| {
                n.checked_add(b.as_ref().map_or_else(
                    || w.node.kind.text().map_or(0, |t| t.content.len()),
                    |b| b.display_text().len(),
                ))
                .filter(|n| *n <= incant_types::MAX_SCENE_TEXT_BYTES)
                .ok_or(UiError::Limit("widget text bytes"))
            })?;
        // New configuration can reduce clipping or disable a focused ancestor.
        // Shape without composition first, then decide whether focus is usable.
        let candidate = layout.prepare_display(
            pending.layout.frame().viewport,
            state.content_generation,
            Some(&state.buffers),
            measure,
        )?;
        if let Some(i) = state.focus {
            let node = candidate
                .frame()
                .nodes
                .iter()
                .find(|n| n.id == layout.widgets()[i].id)
                .expect("layout membership");
            if !state.window_focused || node.disabled || node.hit.size.iter().any(|v| *v <= 0.0) {
                state.focus = None;
            } else if state.buffers[i].is_some() {
                editable(&mut state, i)?.set_focused(true);
            }
        }
        pending.state = Some(state);
        pending.layout = candidate;
        pending.replacement = Some((Arc::new(()), layout));
        Ok(pending)
    }
}

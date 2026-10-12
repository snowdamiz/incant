//! One transactional physical-to-widget route. IME/pointer receipts are applied
//! first, then ordered physical events; callers must not duplicate pointer input
//! as both a receipt and a gameplay mouse event. Physical keys never create text.
use crate::focus::{Context, State, validate_packet};
use crate::{
    Direction, FocusToken, TextMeasure, UiAction, UiError, UiEvent, UiPending, UiRuntime, Widget,
};
use incant_input::{GamepadButton, InputEvent, InputRuntime, KeyCode, UiNotification, UiSnapshot};
use incant_text::editing::Movement;
use std::{collections::BTreeSet, sync::Arc};

#[derive(Clone, Default)]
struct RoutingState {
    raw: InputRuntime,
    forwarded: BTreeSet<KeyCode>,
    suppressed: BTreeSet<KeyCode>,
    forwarded_pad: BTreeSet<(u32, GamepadButton)>,
    suppressed_pad: BTreeSet<(u32, GamepadButton)>,
}
/// This owner must be retained beside its UiRuntime and committed in the same
/// composed tick. It validates even events subsequently consumed by widgets.
pub struct UiRouter {
    owner: Arc<()>,
    revision: u64,
    state: RoutingState,
}
impl Default for UiRouter {
    fn default() -> Self {
        Self {
            owner: Arc::new(()),
            revision: 0,
            state: RoutingState::default(),
        }
    }
}
pub struct RoutedUi {
    owner: Arc<()>,
    previous_revision: u64,
    revision: u64,
    state: RoutingState,
    pending: UiPending,
    gameplay: Vec<InputEvent>,
    snapshot: UiSnapshot,
}
impl RoutedUi {
    pub fn gameplay(&self) -> &[InputEvent] {
        &self.gameplay
    }
    pub fn snapshot(&self) -> &UiSnapshot {
        &self.snapshot
    }
    pub fn take_input(&mut self) -> (Vec<InputEvent>, UiSnapshot) {
        (
            std::mem::take(&mut self.gameplay),
            std::mem::take(&mut self.snapshot),
        )
    }
    pub fn pending(&self) -> &UiPending {
        &self.pending
    }
    pub fn reconcile(
        mut self,
        ui: &UiRuntime,
        widgets: Vec<Widget>,
        measure: &mut impl TextMeasure,
    ) -> Result<Self, UiError> {
        self.pending = ui.reconcile(self.pending, widgets, measure)?;
        Ok(self)
    }
}
impl UiRouter {
    pub fn prepare(
        &self,
        ui: &UiRuntime,
        physical: &[InputEvent],
        receipts: &[UiEvent],
        dt: f64,
        viewport: [f32; 2],
        measure: &mut impl TextMeasure,
    ) -> Result<RoutedUi, UiError> {
        validate_packet(receipts)?;
        let mut route = self.state.clone();
        // Full physical admission first, including events owned by UI.
        let global: Vec<_> = receipts
            .iter()
            .filter_map(|e| match e {
                UiEvent::WindowFocus(focused) => Some(InputEvent::Focus { focused: *focused }),
                _ => None,
            })
            .collect();
        if global.is_empty() {
            route.raw.advance(physical, dt)?;
        } else {
            if global.len() + physical.len() > incant_input::MAX_EVENTS_PER_TICK {
                return Err(UiError::Limit("combined physical events"));
            }
            let mut combined = global;
            combined.extend_from_slice(physical);
            route.raw.advance(&combined, dt)?;
        }
        let frame = ui
            .layout()
            .ok_or(UiError::Invalid("UI input before initial layout"))?;
        let context = Context {
            owner: &ui.owner,
            widgets: ui.widgets(),
            layout: frame,
            pointer_current: frame.viewport == viewport,
        };
        let mut state = ui.state.clone();
        let mut actions = Vec::new();
        let mut discarded = 0;
        let mut operations = 0;
        let mut gameplay = Vec::new();
        let mut keys = self.state.raw.frame().keyboard.held.clone();
        let mut pads: BTreeSet<_> = self
            .state
            .raw
            .frame()
            .gamepads
            .iter()
            .flat_map(|(id, pad)| pad.buttons.held.iter().map(move |button| (*id, *button)))
            .collect();
        let mut active_pads: BTreeSet<_> = self
            .state
            .raw
            .frame()
            .gamepads
            .iter()
            .flat_map(|(id, pad)| {
                pad.values
                    .iter()
                    .filter(|(_, value)| **value != 0.0)
                    .map(move |(button, _)| (*id, *button))
            })
            .collect();
        for event in receipts {
            if let UiEvent::WindowFocus(focused) = event {
                if !focused {
                    keys.clear();
                    pads.clear();
                    route.forwarded.clear();
                    route.suppressed.clear();
                    route.forwarded_pad.clear();
                    route.suppressed_pad.clear();
                    active_pads.clear();
                }
                push(&mut gameplay, InputEvent::Focus { focused: *focused })?;
            }
            apply(
                &context,
                &mut state,
                &mut actions,
                event,
                &mut operations,
                &mut discarded,
            )?;
        }
        release_owned(ui, &state, &keys, &active_pads, &mut route, &mut gameplay)?;
        for event in physical {
            match event {
                InputEvent::Focus { focused } => {
                    apply(
                        &context,
                        &mut state,
                        &mut actions,
                        &UiEvent::WindowFocus(*focused),
                        &mut operations,
                        &mut discarded,
                    )?;
                    if !focused {
                        keys.clear();
                        pads.clear();
                        route.forwarded.clear();
                        route.suppressed.clear();
                        route.forwarded_pad.clear();
                        route.suppressed_pad.clear();
                        active_pads.clear();
                    }
                    push(&mut gameplay, event.clone())?;
                }
                InputEvent::Key { code, down } => {
                    let pressed = if !state.window_focused {
                        false
                    } else if *down {
                        keys.insert(*code)
                    } else {
                        keys.remove(code);
                        false
                    };
                    let captured = owns(ui, &state, *code);
                    if captured || route.suppressed.contains(code) {
                        if *down {
                            route.suppressed.insert(*code);
                        } else {
                            route.suppressed.remove(code);
                        }
                        if route.forwarded.remove(code) {
                            push(
                                &mut gameplay,
                                InputEvent::Key {
                                    code: *code,
                                    down: false,
                                },
                            )?;
                        }
                        if captured
                            && pressed
                            && let Some(logical) = key_event(ui, &state, *code, &keys)
                        {
                            apply(
                                &context,
                                &mut state,
                                &mut actions,
                                &logical,
                                &mut operations,
                                &mut discarded,
                            )?;
                        }
                    } else {
                        if *down {
                            route.forwarded.insert(*code);
                        } else {
                            route.forwarded.remove(code);
                        }
                        push(&mut gameplay, event.clone())?;
                    }
                    release_owned(ui, &state, &keys, &active_pads, &mut route, &mut gameplay)?;
                }
                InputEvent::GamepadButton { id, button, value } => {
                    let key = (*id, *button);
                    let pressed = if !state.window_focused {
                        false
                    } else {
                        if *value != 0.0 {
                            active_pads.insert(key);
                        } else {
                            active_pads.remove(&key);
                        }
                        if *value >= 0.5 {
                            pads.insert(key)
                        } else {
                            pads.remove(&key);
                            false
                        }
                    };
                    let captured = owns_pad(ui, &state, *button);
                    if captured || route.suppressed_pad.contains(&key) {
                        if *value != 0.0 {
                            route.suppressed_pad.insert(key);
                        } else {
                            route.suppressed_pad.remove(&key);
                        }
                        if route.forwarded_pad.remove(&key) {
                            push(
                                &mut gameplay,
                                InputEvent::GamepadButton {
                                    id: *id,
                                    button: *button,
                                    value: 0.0,
                                },
                            )?;
                        }
                        if captured
                            && pressed
                            && let Some(logical) = pad_event(*button)
                        {
                            apply(
                                &context,
                                &mut state,
                                &mut actions,
                                &logical,
                                &mut operations,
                                &mut discarded,
                            )?;
                        }
                    } else {
                        if state.window_focused && *value != 0.0 {
                            route.forwarded_pad.insert(key);
                        } else {
                            route.forwarded_pad.remove(&key);
                        }
                        push(&mut gameplay, event.clone())?;
                    }
                    release_owned(ui, &state, &keys, &active_pads, &mut route, &mut gameplay)?;
                }
                InputEvent::GamepadDisconnected { id } => {
                    pads.retain(|(pad, _)| pad != id);
                    active_pads.retain(|(pad, _)| pad != id);
                    route.forwarded_pad.retain(|(pad, _)| pad != id);
                    route.suppressed_pad.retain(|(pad, _)| pad != id);
                    push(&mut gameplay, event.clone())?;
                }
                _ => push(&mut gameplay, event.clone())?,
            }
        }
        let snapshot = snapshot(ui, &state, &actions, discarded)?;
        let pending = ui.finish(Some(state), actions, discarded, viewport, measure)?;
        Ok(RoutedUi {
            owner: Arc::clone(&self.owner),
            previous_revision: self.revision,
            revision: self
                .revision
                .checked_add(1)
                .ok_or(UiError::Limit("UI input revision"))?,
            state: route,
            pending,
            gameplay,
            snapshot,
        })
    }
    /// Both owner checks precede publication. A rejected or dropped routed
    /// candidate leaves raw input, focus, text and layouts unchanged.
    pub fn commit(&mut self, ui: &mut UiRuntime, routed: RoutedUi) -> Result<(), UiError> {
        if !Arc::ptr_eq(&self.owner, &routed.owner) || self.revision != routed.previous_revision {
            return Err(UiError::Stale);
        }
        ui.commit(routed.pending)?;
        self.state = routed.state;
        self.revision = routed.revision;
        Ok(())
    }
}
fn push(out: &mut Vec<InputEvent>, event: InputEvent) -> Result<(), UiError> {
    if out.len() == incant_input::MAX_EVENTS_PER_TICK {
        return Err(UiError::Limit("routed physical events"));
    }
    out.push(event);
    Ok(())
}
fn apply(
    context: &Context<'_>,
    state: &mut State,
    actions: &mut Vec<UiAction>,
    event: &UiEvent,
    operations: &mut usize,
    discarded: &mut usize,
) -> Result<(), UiError> {
    if *operations == crate::MAX_UI_EVENTS {
        return Err(UiError::Limit("routed widget events"));
    }
    *operations += 1;
    match context.apply(state, actions, event) {
        Err(UiError::Stale) => {
            *discarded += 1;
            Ok(())
        }
        other => other,
    }
}
fn focusable(ui: &UiRuntime) -> bool {
    ui.layout().is_some_and(|frame| frame.has_focusable())
}
fn owns_pad(ui: &UiRuntime, state: &State, button: GamepadButton) -> bool {
    state.window_focused
        && match button {
            GamepadButton::South => state.focus.is_some(),
            GamepadButton::DpadLeft
            | GamepadButton::DpadRight
            | GamepadButton::DpadUp
            | GamepadButton::DpadDown => focusable(ui),
            _ => false,
        }
}
fn owns(ui: &UiRuntime, state: &State, code: KeyCode) -> bool {
    state.window_focused
        && (code == KeyCode::Tab && focusable(ui)
            || state.focus.is_some_and(|i| {
                state.buffers[i].is_some()
                    || matches!(
                        code,
                        KeyCode::ArrowLeft
                            | KeyCode::ArrowRight
                            | KeyCode::ArrowUp
                            | KeyCode::ArrowDown
                            | KeyCode::Enter
                            | KeyCode::Space
                    ) && ui.widgets()[i].node.kind.is_focusable()
            }))
}
fn release_owned(
    ui: &UiRuntime,
    state: &State,
    keys: &BTreeSet<KeyCode>,
    active_pads: &BTreeSet<(u32, GamepadButton)>,
    route: &mut RoutingState,
    gameplay: &mut Vec<InputEvent>,
) -> Result<(), UiError> {
    let mut release = Vec::new();
    for key in &route.forwarded {
        if owns(ui, state, *key) {
            release.push(*key);
        }
    }
    for key in release {
        route.forwarded.remove(&key);
        if keys.contains(&key) {
            route.suppressed.insert(key);
        }
        push(
            gameplay,
            InputEvent::Key {
                code: key,
                down: false,
            },
        )?;
    }
    let release: Vec<_> = route
        .forwarded_pad
        .iter()
        .copied()
        .filter(|(_, button)| owns_pad(ui, state, *button))
        .collect();
    for key in release {
        route.forwarded_pad.remove(&key);
        if active_pads.contains(&key) {
            route.suppressed_pad.insert(key);
        }
        push(
            gameplay,
            InputEvent::GamepadButton {
                id: key.0,
                button: key.1,
                value: 0.0,
            },
        )?;
    }
    Ok(())
}
fn key_event(
    ui: &UiRuntime,
    state: &State,
    code: KeyCode,
    keys: &BTreeSet<KeyCode>,
) -> Option<UiEvent> {
    let extend = keys.contains(&KeyCode::ShiftLeft) || keys.contains(&KeyCode::ShiftRight);
    if code == KeyCode::Tab {
        return Some(UiEvent::Traverse { reverse: extend });
    }
    let index = state.focus?;
    if state.buffers[index].is_some() {
        let target = FocusToken {
            owner: Arc::clone(&ui.owner),
            epoch: state.epoch,
            widget: ui.widgets()[index].id,
        };
        Some(match code {
            KeyCode::ArrowLeft => UiEvent::Move {
                target,
                movement: Movement::PreviousGrapheme,
                extend,
            },
            KeyCode::ArrowRight => UiEvent::Move {
                target,
                movement: Movement::NextGrapheme,
                extend,
            },
            KeyCode::Home => UiEvent::Move {
                target,
                movement: Movement::Start,
                extend,
            },
            KeyCode::End => UiEvent::Move {
                target,
                movement: Movement::End,
                extend,
            },
            KeyCode::Backspace => UiEvent::Backspace { target },
            KeyCode::Delete => UiEvent::Delete { target },
            KeyCode::Enter => UiEvent::Submit { target },
            KeyCode::Escape => UiEvent::Cancel { target },
            _ => return None,
        })
    } else {
        Some(match code {
            KeyCode::ArrowLeft => UiEvent::Navigate(Direction::Left),
            KeyCode::ArrowRight => UiEvent::Navigate(Direction::Right),
            KeyCode::ArrowUp => UiEvent::Navigate(Direction::Up),
            KeyCode::ArrowDown => UiEvent::Navigate(Direction::Down),
            KeyCode::Enter | KeyCode::Space => UiEvent::Activate,
            _ => return None,
        })
    }
}
fn pad_event(button: GamepadButton) -> Option<UiEvent> {
    Some(match button {
        GamepadButton::DpadLeft => UiEvent::Navigate(Direction::Left),
        GamepadButton::DpadRight => UiEvent::Navigate(Direction::Right),
        GamepadButton::DpadUp => UiEvent::Navigate(Direction::Up),
        GamepadButton::DpadDown => UiEvent::Navigate(Direction::Down),
        GamepadButton::South => UiEvent::Activate,
        _ => return None,
    })
}
fn snapshot(
    ui: &UiRuntime,
    state: &State,
    actions: &[UiAction],
    discarded: usize,
) -> Result<UiSnapshot, UiError> {
    fn id(id: crate::StableId) -> String {
        ulid::Ulid::from_bytes(id.0).to_string()
    }
    let frame = UiSnapshot {
        focused: state.focus.map(|i| id(ui.widgets()[i].id)),
        text_input: state.focus.is_some_and(|i| state.buffers[i].is_some()),
        events: actions
            .iter()
            .map(|action| match action {
                UiAction::Activated(entity) => UiNotification::Activated {
                    entity_id: id(*entity),
                },
                UiAction::ValueChanged { widget, value } => UiNotification::ValueChanged {
                    entity_id: id(*widget),
                    value: value.to_string(),
                },
                UiAction::Submitted { widget, value } => UiNotification::Submitted {
                    entity_id: id(*widget),
                    value: value.to_string(),
                },
            })
            .collect(),
        discarded_events: discarded as u16,
    };
    frame.validate()?;
    Ok(frame)
}

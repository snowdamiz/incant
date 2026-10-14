//! Bounded logical-point touch delivery shared by native mobile adapters.
use incant_input::{InputEvent, MAX_EVENTS_PER_TICK, MAX_TOUCHES, TouchPhase};
use incant_ui::{MAX_UI_EVENTS, PointerToken, UiEvent, UiRuntime};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum TouchError {
    #[error("native touch settings or sequence are invalid")]
    Invalid,
    #[error("native touch packet is full")]
    Capacity,
}
enum Touch {
    Gameplay([f32; 2]),
    Ui(PointerToken),
    Suppressed,
}

/// A UI gesture never becomes a gameplay gesture, including after owner/layout
/// replacement. The platform must separately fence callbacks by delivery epoch.
pub struct NativeTouchInput {
    physical: Vec<InputEvent>,
    ui: Vec<UiEvent>,
    touches: [Option<Touch>; MAX_TOUCHES],
    observed: Option<PointerToken>,
}
impl Default for NativeTouchInput {
    fn default() -> Self {
        Self {
            physical: Vec::with_capacity(MAX_EVENTS_PER_TICK),
            ui: Vec::with_capacity(MAX_UI_EVENTS),
            touches: std::array::from_fn(|_| None),
            observed: None,
        }
    }
}
impl NativeTouchInput {
    pub fn physical(&self) -> &[InputEvent] {
        &self.physical
    }
    pub fn receipts(&self) -> &[UiEvent] {
        &self.ui
    }
    /// Only after the composed tick accepts the entire packet.
    pub fn accepted(&mut self) {
        self.physical.clear();
        self.ui.clear();
    }
    /// Paired with platform epoch revocation and composed focus neutralization.
    pub fn reset(&mut self) {
        self.accepted();
        self.touches = std::array::from_fn(|_| None);
        self.observed = None;
    }
    /// Call immediately after accepted presentation, before accepting callbacks.
    /// A published replacement retires captures; old releases are suppressed.
    pub fn presented(&mut self, runtime: Option<&UiRuntime>) -> Result<(), TouchError> {
        let token = runtime.and_then(UiRuntime::pointer_token);
        if token == self.observed {
            return Ok(());
        }
        let physical_count = self
            .touches
            .iter()
            .filter(|t| matches!(t, Some(Touch::Gameplay(_))))
            .count();
        let cancel_ui =
            token.is_some() && self.touches.iter().any(|t| matches!(t, Some(Touch::Ui(_))));
        if self.physical.len() + physical_count > MAX_EVENTS_PER_TICK - 2
            || self.ui.len() + usize::from(cancel_ui) > MAX_UI_EVENTS
        {
            return Err(TouchError::Capacity);
        }
        self.ui.retain(|event| {
            !matches!(
                event,
                UiEvent::Pointer { .. } | UiEvent::CancelPointer { .. }
            )
        });
        for (id, touch) in self.touches.iter_mut().enumerate() {
            if let Some(Touch::Gameplay(position)) = touch {
                self.physical.push(InputEvent::Touch {
                    id: id as u32,
                    phase: TouchPhase::Cancel,
                    position: position.map(f64::from),
                });
            }
            if touch.is_some() {
                *touch = Some(Touch::Suppressed);
            }
        }
        if cancel_ui {
            self.ui.push(UiEvent::CancelPointer {
                target: token.clone().expect("checked above"),
            });
        }
        self.observed = token;
        Ok(())
    }
    pub fn touch(
        &mut self,
        runtime: Option<&UiRuntime>,
        id: u32,
        phase: TouchPhase,
        position: [f32; 2],
    ) -> Result<(), TouchError> {
        let slot = usize::try_from(id)
            .ok()
            .filter(|id| *id < MAX_TOUCHES)
            .ok_or(TouchError::Invalid)?;
        if position
            .iter()
            .any(|n| !n.is_finite() || n.abs() > 1_000_000.)
        {
            return Err(TouchError::Invalid);
        }
        // The adapter publishes owner changes before callbacks, never retargets
        // an already queued gesture to an unpublished replacement.
        if runtime.and_then(UiRuntime::pointer_token) != self.observed {
            return Err(TouchError::Invalid);
        }
        if (phase == TouchPhase::Down) == self.touches[slot].is_some() {
            return Err(TouchError::Invalid);
        }
        // Reserve before changing capture state. Two physical slots belong to
        // the composed owner's lifecycle-neutralizing focus events.
        if self.physical.len() >= MAX_EVENTS_PER_TICK - 2 || self.ui.len() >= MAX_UI_EVENTS {
            return Err(TouchError::Capacity);
        }
        if phase == TouchPhase::Down {
            let target = runtime
                .filter(|ui| ui.pointer_hit(position).is_some())
                .and_then(UiRuntime::pointer_token);
            self.touches[slot] = Some(if let Some(target) = target {
                if self.touches.iter().any(|t| matches!(t, Some(Touch::Ui(_)))) {
                    Touch::Suppressed
                } else {
                    self.ui.push(UiEvent::Pointer {
                        position,
                        pressed: true,
                        target: target.clone(),
                    });
                    Touch::Ui(target)
                }
            } else {
                Touch::Gameplay(position)
            });
        }
        match self.touches[slot].as_mut().expect("validated sequence") {
            Touch::Gameplay(last) => {
                *last = position;
                self.physical.push(InputEvent::Touch {
                    id,
                    phase,
                    position: position.map(f64::from),
                });
            }
            Touch::Ui(target) if phase == TouchPhase::Up => self.ui.push(UiEvent::Pointer {
                position,
                pressed: false,
                target: target.clone(),
            }),
            Touch::Ui(target) if phase == TouchPhase::Cancel => {
                self.ui.push(UiEvent::CancelPointer {
                    target: target.clone(),
                })
            }
            _ => {}
        }
        if matches!(phase, TouchPhase::Up | TouchPhase::Cancel) {
            self.touches[slot] = None;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;

//! Bounded native contact capture. Platform IDs never enter runtime/replay state.
use super::*;
use incant_input::{MAX_TOUCHES, TouchPhase};
use tao::event::{DeviceId, Touch, TouchPhase as NativePhase};

#[derive(Clone)]
enum Owner {
    Gameplay,
    Ui(PointerToken),
    Suppressed,
}
#[derive(Clone)]
struct Contact {
    device: DeviceId,
    native_id: u64,
    id: u32,
    position: [f64; 2],
    owner: Owner,
}
#[derive(Clone)]
pub(super) struct Contacts {
    slots: [Option<Contact>; MAX_TOUCHES],
    next_id: u32,
}
impl Default for Contacts {
    fn default() -> Self {
        Self {
            slots: std::array::from_fn(|_| None),
            next_id: 0,
        }
    }
}
impl Contacts {
    pub(super) fn clear(&mut self) {
        self.slots.fill(None);
        // Never recycle runtime identities at a surface/focus/session boundary.
    }
}
impl UiInput {
    pub(super) fn ui_touch_held(&self) -> bool {
        self.touches
            .slots
            .iter()
            .flatten()
            .any(|c| matches!(c.owner, Owner::Ui(_)))
    }
    /// The entire native event, including ownership-fence cancellations, is
    /// atomic on a range/sequence/queue refusal. This fixed-size snapshot never
    /// copies a project, input history, native payload or widget tree.
    pub(super) fn touch(
        &mut self,
        ui: Option<&UiRuntime>,
        physical: &mut PendingInput,
        touch: &Touch,
        scale: f64,
    ) -> Result<(), Error> {
        let position = logical_position(touch.location, scale)?;
        if !position
            .iter()
            .all(|v| v.is_finite() && v.abs() <= 1_000_000.)
        {
            return Err(Error::InputRange);
        }
        let phase = match touch.phase {
            NativePhase::Started => TouchPhase::Down,
            NativePhase::Moved => TouchPhase::Move,
            NativePhase::Ended => TouchPhase::Up,
            NativePhase::Cancelled => TouchPhase::Cancel,
            _ => return Err(Error::InputCapability),
        };
        let contacts = self.touches.clone();
        let left = self.left.clone();
        let observed = self.observed.clone();
        let receipts = self.receipts.len();
        let bytes = self.bytes;
        let packet = physical.checkpoint();
        let result = self
            .synchronize(ui, physical)
            .and_then(|()| self.touch_inner(ui, physical, touch, phase, position));
        if result.is_err() {
            self.touches = contacts;
            self.left = left;
            self.observed = observed;
            self.receipts.truncate(receipts);
            self.bytes = bytes;
            physical.rollback(packet);
        }
        result
    }
    fn touch_inner(
        &mut self,
        ui: Option<&UiRuntime>,
        physical: &mut PendingInput,
        touch: &Touch,
        phase: TouchPhase,
        position: [f64; 2],
    ) -> Result<(), Error> {
        let existing = self.touches.slots.iter().position(|slot| {
            slot.as_ref()
                .is_some_and(|c| c.device == touch.device_id && c.native_id == touch.id)
        });
        let index = if phase == TouchPhase::Down {
            if existing.is_some() {
                return Err(Error::TouchSequence);
            }
            self.touches
                .slots
                .iter()
                .position(Option::is_none)
                .ok_or(Error::TouchCapacity)?
        } else if let Some(index) = existing {
            index
        } else {
            // Native continuations after a lifecycle cancellation carry no
            // authority. In particular, they cannot synthesize a new press.
            return Ok(());
        };
        let mut contact = if phase == TouchPhase::Down {
            let next = self
                .touches
                .next_id
                .checked_add(1)
                .ok_or(Error::TouchIdentity)?;
            let owner = if !self.pointer_ready || self.surface_retry {
                Owner::Suppressed
            } else if ui.is_some_and(|u| u.pointer_hit(position.map(|v| v as f32)).is_some()) {
                if self.ui_touch_held() || matches!(self.left, Left::Ui(_)) {
                    Owner::Suppressed
                } else {
                    Owner::Ui(ui.and_then(UiRuntime::pointer_token).expect("hit layout"))
                }
            } else {
                Owner::Gameplay
            };
            Contact {
                device: touch.device_id,
                native_id: touch.id,
                id: next,
                position,
                owner,
            }
        } else {
            self.touches.slots[index]
                .as_ref()
                .expect("active contact")
                .clone()
        };
        // Gameplay capture may be cancelled by newly occupied UI coordinates;
        // neither crossing direction can turn the same contact into a UI press.
        let fenced = matches!(contact.owner, Owner::Gameplay)
            && (self.surface_retry
                || !self.pointer_ready
                || ui.is_some_and(|u| u.pointer_hit(position.map(|v| v as f32)).is_some()));
        match &contact.owner {
            Owner::Gameplay => physical.push(InputEvent::Touch {
                id: contact.id,
                phase: if fenced { TouchPhase::Cancel } else { phase },
                position,
            })?,
            Owner::Ui(target) => match phase {
                TouchPhase::Down | TouchPhase::Up => self.push(
                    UiEvent::Pointer {
                        position: position.map(|v| v as f32),
                        pressed: phase == TouchPhase::Down,
                        target: target.clone(),
                    },
                    0,
                )?,
                TouchPhase::Cancel => self.push(
                    UiEvent::CancelPointer {
                        target: target.clone(),
                    },
                    0,
                )?,
                TouchPhase::Move => {}
            },
            Owner::Suppressed => {}
        }
        if phase == TouchPhase::Down {
            self.touches.next_id = contact.id;
        }
        contact.position = position;
        if fenced {
            contact.owner = Owner::Suppressed;
        }
        self.touches.slots[index] =
            (!matches!(phase, TouchPhase::Up | TouchPhase::Cancel)).then_some(contact);
        Ok(())
    }
    /// A new displayed owner/layout cancels, rather than retargets, held input.
    pub(super) fn synchronize_touches(
        &mut self,
        ui: Option<&UiRuntime>,
        physical: &mut PendingInput,
        changed: bool,
    ) -> Result<(), Error> {
        let token = ui.and_then(UiRuntime::pointer_token);
        let cancel = std::array::from_fn::<_, MAX_TOUCHES, _>(|i| {
            self.touches.slots[i]
                .as_ref()
                .is_some_and(|c| match &c.owner {
                    Owner::Gameplay => {
                        changed
                            || ui.is_some_and(|u| {
                                u.pointer_hit(c.position.map(|v| v as f32)).is_some()
                            })
                    }
                    Owner::Ui(target) => Some(target) != token.as_ref(),
                    Owner::Suppressed => false,
                })
        });
        self.cancel_contacts(physical, token, &cancel, false)
    }
    /// Surface and viewport changes cancel all captured contacts. A later native
    /// move/release is ignored until an explicit new Started event arrives.
    pub(crate) fn cancel_touches(
        &mut self,
        ui: Option<&UiRuntime>,
        physical: &mut PendingInput,
    ) -> Result<(), Error> {
        self.cancel_contacts(
            physical,
            ui.and_then(UiRuntime::pointer_token),
            &[true; MAX_TOUCHES],
            true,
        )
    }
    fn cancel_contacts(
        &mut self,
        physical: &mut PendingInput,
        token: Option<PointerToken>,
        cancel: &[bool; MAX_TOUCHES],
        clear: bool,
    ) -> Result<(), Error> {
        let contacts = self
            .touches
            .slots
            .iter()
            .zip(cancel)
            .filter_map(|(c, &cancel)| cancel.then_some(c.as_ref()).flatten());
        let mut gameplay = 0;
        let mut ui = false;
        for contact in contacts {
            match contact.owner {
                Owner::Gameplay => gameplay += 1,
                Owner::Ui(_) => ui = true,
                Owner::Suppressed => {}
            }
        }
        physical.reserve_events(gameplay)?;
        if ui && token.is_some() && self.receipts.len() == MAX_UI_EVENTS {
            return Err(Error::UiCapacity);
        }
        if ui && let Some(target) = token {
            self.push(UiEvent::CancelPointer { target }, 0)?;
        }
        for (slot, &cancel) in self.touches.slots.iter_mut().zip(cancel) {
            if !cancel {
                continue;
            }
            if let Some(contact) = slot {
                if matches!(contact.owner, Owner::Gameplay) {
                    physical.push(InputEvent::Touch {
                        id: contact.id,
                        phase: TouchPhase::Cancel,
                        position: contact.position,
                    })?;
                }
                contact.owner = Owner::Suppressed;
            }
            if clear {
                *slot = None;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;

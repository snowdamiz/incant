use super::{
    Gesture, InputError, InputRuntime, MAX_TOUCHES, TouchChange, TouchPhase, TouchPoint, add,
    subtract,
};
use std::collections::BTreeMap;

const TAP_SECONDS: f64 = 0.3;
const HOLD_SECONDS: f64 = 0.5;
const TAP_DISTANCE: f64 = 12.;
const SWIPE_DISTANCE: f64 = 50.;
const SWIPE_SECONDS: f64 = 0.5;

#[derive(Debug, Clone)]
pub(super) struct Contact {
    start: [f64; 2],
    started: f64,
    max_distance: f64,
    long_pressed: bool,
    multi_touch: bool,
}
pub(super) type Pair = ([u32; 2], [[f64; 2]; 2]);
pub(super) fn pair(touches: &BTreeMap<u32, TouchPoint>) -> Option<Pair> {
    if touches.len() != 2 {
        return None;
    }
    let mut entries = touches.iter();
    let (a, p) = entries.next()?;
    let (b, q) = entries.next()?;
    Some(([*a, *b], [p.position, q.position]))
}
fn length(v: [f64; 2]) -> f64 {
    v[0].hypot(v[1])
}

impl InputRuntime {
    pub(super) fn touch(
        &mut self,
        id: u32,
        phase: TouchPhase,
        position: [f64; 2],
        dt: f64,
    ) -> Result<(), InputError> {
        if phase == TouchPhase::Down {
            if self.contacts.contains_key(&id) {
                return Err(InputError::DuplicateTouch);
            }
            if self.contacts.len() >= MAX_TOUCHES {
                return Err(InputError::Devices);
            }
            self.contacts.insert(
                id,
                Contact {
                    start: position,
                    started: self.time,
                    max_distance: 0.,
                    long_pressed: false,
                    multi_touch: false,
                },
            );
            self.frame.touches.insert(
                id,
                TouchPoint {
                    position,
                    delta: [0.; 2],
                },
            );
            if self.contacts.len() > 1 {
                for contact in self.contacts.values_mut() {
                    contact.multi_touch = true;
                }
            }
        } else {
            let contact = self.contacts.get_mut(&id).ok_or(InputError::MissingTouch)?;
            let point = self
                .frame
                .touches
                .get_mut(&id)
                .expect("matching touch state");
            add(&mut point.delta, subtract(position, point.position));
            point.position = position;
            let offset = subtract(position, contact.start);
            contact.max_distance = contact.max_distance.max(length(offset));
            if matches!(phase, TouchPhase::Up | TouchPhase::Cancel) {
                let duration = (self.time - contact.started).max(dt);
                if phase == TouchPhase::Up && !contact.long_pressed && !contact.multi_touch {
                    if duration <= TAP_SECONDS + 1e-9 && contact.max_distance <= TAP_DISTANCE {
                        self.frame.gestures.push(Gesture::Tap { id, position });
                    } else if duration <= SWIPE_SECONDS + 1e-9 && length(offset) >= SWIPE_DISTANCE {
                        let distance = length(offset);
                        self.frame.gestures.push(Gesture::Swipe {
                            id,
                            direction: [offset[0] / distance, offset[1] / distance],
                            distance,
                            duration_seconds: duration,
                        });
                    }
                }
                self.contacts.remove(&id);
                self.frame.touches.remove(&id);
            }
        }
        self.frame.touch_changes.push(TouchChange {
            id,
            phase,
            position,
        });
        Ok(())
    }
    pub(super) fn finish_gestures(&mut self, previous: Option<Pair>) {
        for (&id, contact) in &mut self.contacts {
            if !contact.long_pressed
                && !contact.multi_touch
                && contact.max_distance <= TAP_DISTANCE
                && self.time - contact.started + 1e-9 >= HOLD_SECONDS
            {
                contact.long_pressed = true;
                self.frame.gestures.push(Gesture::LongPress {
                    id,
                    position: self.frame.touches[&id].position,
                });
            }
        }
        if let (Some((old_ids, old)), Some((ids, current))) = (previous, pair(&self.frame.touches))
            && old_ids == ids
            && old != current
            && !self
                .frame
                .touch_changes
                .iter()
                .any(|change| ids.contains(&change.id) && change.phase != TouchPhase::Move)
        {
            let before = subtract(old[1], old[0]);
            let after = subtract(current[1], current[0]);
            let a = length(before);
            let b = length(after);
            // Coincident/noisy points cannot define a stable pinch or rotation.
            if a >= 1. && b >= 1. {
                let angle = after[1].atan2(after[0]) - before[1].atan2(before[0]);
                let rotation = angle.sin().atan2(angle.cos());
                self.frame.gestures.push(Gesture::Pinch {
                    ids,
                    center: [
                        (current[0][0] + current[1][0]) / 2.,
                        (current[0][1] + current[1][1]) / 2.,
                    ],
                    scale_delta: b / a,
                    rotation_delta: rotation,
                });
            }
        }
    }
}

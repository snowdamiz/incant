//! Canonical copied device authority and ephemeral output; never native handles.
use crate::{
    HAPTIC_COMMAND_MAGIC, HAPTIC_MAGIC, HAPTIC_MAX_DURATION_MS, HAPTIC_MAX_REQUESTS,
    HAPTIC_MAX_TARGETS, HapticCommand, HapticCommandHeader, HapticEffectKind, HapticHeader,
    HapticTarget, HapticTargetKind, INPUT_MAX_GAMEPADS,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Malformed;
type Result<T> = core::result::Result<T, Malformed>;

pub fn validate_target(row: &HapticTarget) -> Result<HapticTargetKind> {
    let kind = u8::try_from(row.kind)
        .ok()
        .and_then(HapticTargetKind::from_tag)
        .ok_or(Malformed)?;
    if row.generation == 0
        || match kind {
            HapticTargetKind::Gamepad => row.id as usize >= INPUT_MAX_GAMEPADS,
            HapticTargetKind::Device => row.id != 0,
        }
    {
        return Err(Malformed);
    }
    Ok(kind)
}

#[derive(Clone, Copy)]
pub struct Targets<'a> {
    rows: &'a [u8],
}
impl<'a> Targets<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self> {
        let h = HapticHeader::decode(bytes).ok_or(Malformed)?;
        if &h.magic != HAPTIC_MAGIC
            || h.count as usize > HAPTIC_MAX_TARGETS
            || h.stride as usize != HapticTarget::SIZE
            || bytes.len() != HapticHeader::SIZE + h.count as usize * HapticTarget::SIZE
        {
            return Err(Malformed);
        }
        let result = Self {
            rows: &bytes[HapticHeader::SIZE..],
        };
        let mut previous = None;
        for row in result.iter() {
            validate_target(&row)?;
            let key = (row.kind, row.id);
            if previous.is_some_and(|old| old >= key) {
                return Err(Malformed);
            }
            previous = Some(key);
        }
        Ok(result)
    }
    pub fn len(&self) -> usize {
        self.rows.len() / HapticTarget::SIZE
    }
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
    pub fn iter(&self) -> impl ExactSizeIterator<Item = HapticTarget> + '_ {
        self.rows
            .as_chunks::<{ HapticTarget::SIZE }>()
            .0
            .iter()
            .map(|r| HapticTarget::decode(r).unwrap())
    }
}

/// Validate original f64 values before narrowing; unused fields require +0 bits.
pub fn validate_command(row: &HapticCommand) -> Result<HapticEffectKind> {
    let target = validate_target(&HapticTarget {
        kind: row.target_kind,
        id: row.id,
        generation: row.generation,
    })?;
    let kind = u8::try_from(row.effect_kind)
        .ok()
        .and_then(HapticEffectKind::from_tag)
        .ok_or(Malformed)?;
    let amplitude = |value: f64| value.is_finite() && (0. ..=1.).contains(&value);
    if row.reserved != 0
        || !match (target, kind) {
            (HapticTargetKind::Gamepad, HapticEffectKind::Rumble) => {
                amplitude(row.strong)
                    && amplitude(row.weak)
                    && (1..=HAPTIC_MAX_DURATION_MS as u32).contains(&row.duration_ms)
            }
            (HapticTargetKind::Device, HapticEffectKind::Impact) => {
                amplitude(row.strong) && row.weak.to_bits() == 0 && row.duration_ms == 0
            }
            (_, HapticEffectKind::Stop) => {
                row.strong.to_bits() == 0 && row.weak.to_bits() == 0 && row.duration_ms == 0
            }
            _ => false,
        }
    {
        return Err(Malformed);
    }
    Ok(kind)
}
#[derive(Clone, Copy)]
pub struct Commands<'a> {
    rows: &'a [u8],
}
impl<'a> Commands<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self> {
        let h = HapticCommandHeader::decode(bytes).ok_or(Malformed)?;
        if &h.magic != HAPTIC_COMMAND_MAGIC
            || h.count as usize > HAPTIC_MAX_REQUESTS
            || h.stride as usize != HapticCommand::SIZE
            || bytes.len() != HapticCommandHeader::SIZE + h.count as usize * HapticCommand::SIZE
        {
            return Err(Malformed);
        }
        let result = Self {
            rows: &bytes[HapticCommandHeader::SIZE..],
        };
        for row in result.iter() {
            validate_command(&row)?;
        }
        Ok(result)
    }
    pub fn iter(&self) -> impl ExactSizeIterator<Item = HapticCommand> + '_ {
        self.rows
            .as_chunks::<{ HapticCommand::SIZE }>()
            .0
            .iter()
            .map(|r| HapticCommand::decode(r).unwrap())
    }
}

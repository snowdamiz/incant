use super::Tick;
use crate::{AnimationIkSnapshot, EntityId, Error, Result, raw};
use incant_wasm_abi as abi;

impl Tick<'_> {
    /// Complete copied pose plus ordered IK. The snapshot borrows only caller
    /// scratch, never the native world. Legacy pose methods explicitly refuse IK.
    pub fn animation_ik<'a>(&mut self, scratch: &'a mut [u8]) -> Result<AnimationIkSnapshot<'a>> {
        self.ready()?;
        // SAFETY: synchronous scalar import under this callback's epoch.
        let len = unsafe { raw::incant_animation_ik_v1_snapshot_len(self.epoch) };
        let len = self.check(usize::try_from(len).map_err(|_| Error::Failed))?;
        self.check(
            if len <= abi::ANIMATION_IK_MAX_BYTES && len <= scratch.len() {
                Ok(())
            } else {
                Err(Error::Capacity)
            },
        )?;
        // SAFETY: initialized exclusive output scratch is valid for len bytes.
        let copied = unsafe {
            raw::incant_animation_ik_v1_read_snapshot(
                self.epoch,
                scratch.as_mut_ptr() as usize as i32,
                len as i32,
            )
        };
        self.check(if copied == len as i32 {
            Ok(())
        } else {
            Err(Error::Failed)
        })?;
        self.check(AnimationIkSnapshot::parse(&scratch[..len]))
    }
    /// Change one existing stable constraint, in model space before the entity
    /// transform. Chain/identity changes are authored structural work. Controls
    /// share the transaction quota and publish only after the accepted callback.
    pub fn set_animation_ik(
        &mut self,
        entity: EntityId,
        constraint: EntityId,
        target: [f64; 3],
        pole: [f64; 3],
        weight: f64,
    ) -> Result<()> {
        self.ready()?;
        self.check(if crate::animation_ik::numeric(target, pole, weight) {
            Ok(())
        } else {
            Err(Error::Bounds)
        })?;
        let bytes = abi::AnimationIkCommand {
            entity: entity.0,
            constraint: constraint.0,
            target,
            pole,
            weight,
        }
        .encode();
        // SAFETY: synchronous copy of the complete bounded initialized record.
        let result = unsafe {
            raw::incant_animation_ik_v1_control(
                self.epoch,
                bytes.as_ptr() as usize as i32,
                bytes.len() as i32,
            )
        };
        self.check(if result == 0 {
            Ok(())
        } else {
            Err(Error::Failed)
        })
    }
}

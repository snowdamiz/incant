use super::Tick;
use crate::{
    AnimationGraphSnapshotV2, AnimationIkSnapshotV2, AnimationPoseSnapshotV5, Error, Result, raw,
};
use incant_wasm_abi as abi;
impl Tick<'_> {
    /// Complete ordinary/compound local pose. Refuses committed IK; use animation_ik_v2 for that owner.
    pub fn animation_poses_v5<'a>(
        &mut self,
        scratch: &'a mut [u8],
    ) -> Result<AnimationPoseSnapshotV5<'a>> {
        self.ready()?;
        // SAFETY: synchronous scalar import under this callback's epoch.
        let len = unsafe { raw::incant_animation_v5_snapshot_len(self.epoch) };
        let len = self.check(usize::try_from(len).map_err(|_| Error::Failed))?;
        self.check(
            if len <= abi::ANIMATION_V5_MAX_BYTES && len <= scratch.len() {
                Ok(())
            } else {
                Err(Error::Capacity)
            },
        )?;
        // SAFETY: initialized exclusive output scratch remains live for the call.
        let copied = unsafe {
            raw::incant_animation_v5_read_snapshot(
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
        self.check(AnimationPoseSnapshotV5::parse(&scratch[..len]))
    }
    /// Complete ordinary/compound pose plus ordered IK, all captured before this callback's controls.
    pub fn animation_ik_v2<'a>(
        &mut self,
        scratch: &'a mut [u8],
    ) -> Result<AnimationIkSnapshotV2<'a>> {
        self.ready()?;
        // SAFETY: synchronous scalar import under this callback's epoch.
        let len = unsafe { raw::incant_animation_ik_v2_snapshot_len(self.epoch) };
        let len = self.check(usize::try_from(len).map_err(|_| Error::Failed))?;
        self.check(
            if len <= abi::ANIMATION_IK_V2_MAX_BYTES && len <= scratch.len() {
                Ok(())
            } else {
                Err(Error::Capacity)
            },
        )?;
        // SAFETY: initialized exclusive output scratch remains live for the call.
        let copied = unsafe {
            raw::incant_animation_ik_v2_read_snapshot(
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
        self.check(AnimationIkSnapshotV2::parse(&scratch[..len]))
    }
    /// Current graph parameters and captured outgoing fade clocks. Existing graph controls remain transactional.
    pub fn animation_graphs_v2<'a>(
        &mut self,
        scratch: &'a mut [u8],
    ) -> Result<AnimationGraphSnapshotV2<'a>> {
        self.ready()?;
        // SAFETY: synchronous scalar import under this callback's epoch.
        let len = unsafe { raw::incant_animation_graph_v2_snapshot_len(self.epoch) };
        let len = self.check(usize::try_from(len).map_err(|_| Error::Failed))?;
        self.check(
            if len <= abi::ANIMATION_GRAPH_V2_MAX_BYTES && len <= scratch.len() {
                Ok(())
            } else {
                Err(Error::Capacity)
            },
        )?;
        // SAFETY: initialized exclusive output scratch remains live for the call.
        let copied = unsafe {
            raw::incant_animation_graph_v2_read_snapshot(
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
        self.check(AnimationGraphSnapshotV2::parse(&scratch[..len]))
    }
}

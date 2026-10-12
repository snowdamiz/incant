//! One publication boundary for animation, extracted-root removal and IK.
use super::*;

#[derive(Debug, thiserror::Error)]
pub enum AnimationPoseError {
    #[error(transparent)]
    Sample(#[from] AssetError),
    #[error(transparent)]
    Ik(#[from] TwoBoneIkError),
    #[error("root removal requires an existing editable TRS node")]
    Root,
}

impl AnimationPlayer {
    /// Sample and crossfade local poses, optionally restore an admitted root's
    /// translation/rotation to rest, then solve IK in slice order. Local matrices,
    /// globals and palettes publish only after every stage succeeds. An invalid
    /// later constraint cannot leave a new sampled pose behind. Accepted calls
    /// allocate nothing and reuse the existing staging buffers.
    ///
    /// Root catalog identity and rigid topology are the caller's cold-admission
    /// responsibility. This primitive checks only the selected editable node;
    /// it preserves sampled scale and does not apply motion to an entity/body.
    pub fn sample_pose_pipeline(
        &mut self,
        from: PoseSource<'_>,
        to: PoseSource<'_>,
        weight: f64,
        strip_root: Option<u16>,
        constraints: &[TwoBoneIk],
    ) -> std::result::Result<(), AnimationPoseError> {
        self.validate_ik_constraints(constraints)?;
        if strip_root.is_some_and(|root| {
            self.data
                .rest
                .get(usize::from(root))
                .and_then(Option::as_ref)
                .is_none()
        }) {
            return Err(AnimationPoseError::Root);
        }
        self.sample_pose_crossfade_trs(from, to, weight)?;
        if let Some(root) = strip_root.map(usize::from) {
            let rest = self.data.rest[root].as_ref().expect("admitted root");
            let value = self.trs[root]
                .as_mut()
                .expect("sample preserves TRS topology");
            value.translation = rest.translation;
            value.rotation = rest.rotation;
        }
        self.stage_trs()?;
        self.solve_staged_ik(constraints)?;
        std::mem::swap(&mut self.live, &mut self.staged);
        Ok(())
    }
}

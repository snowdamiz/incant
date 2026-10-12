use super::*;
use crate::Vertex;
use glam::DMat3;

/// A borrowed skin pose for one checked node/primitive binding. No copying or
/// allocation occurs here, and sampling cannot overlap this borrow. Returned
/// vertices are in model space: the skinned mesh node transform is ignored.
pub struct SkinPose<'a> {
    joints: &'a [DMat4],
    influences: &'a [Influences],
}
impl AnimationPlayer {
    pub fn skin_pose(&self, node: usize, primitive: usize) -> Result<SkinPose<'_>> {
        if !self
            .nodes
            .get(node)
            .is_some_and(|n| n.meshes.contains(&primitive))
        {
            return Err(invalid("skin primitive binding"));
        }
        let skin = self.data.node_skins[node].ok_or_else(|| invalid("node has no skin"))?;
        Ok(SkinPose {
            joints: self.joint_palette(skin)?,
            influences: &self.data.influences[primitive],
        })
    }
}
impl SkinPose<'_> {
    pub fn vertex_count(&self) -> usize {
        self.influences.len()
    }

    /// Linear-blend position, inverse-transpose normal, linear tangent, and
    /// determinant-corrected tangent handedness. UVs are unchanged. The shader
    /// performs its usual final normalization/orthogonalization. Singular blended
    /// transforms and nonfinite/f32-overflow results refuse, never emit NaNs.
    /// Each result is owned; a consumer fills private staging before publication.
    pub fn deform_vertex(&self, vertex: usize, source: Vertex) -> Result<Vertex> {
        self.deform_vertex_oriented(vertex, source)
            .map(|(vertex, _)| vertex)
    }

    /// Also returns whether the blended linear basis reflects orientation. A
    /// renderer must account for this independently from the outer entity frame
    /// when choosing its culling policy. Mixed orientation needs an explicit
    /// primitive policy; this value alone does not repair folded triangles.
    pub fn deform_vertex_oriented(&self, vertex: usize, source: Vertex) -> Result<(Vertex, bool)> {
        if source.iter().any(|v| !v.is_finite()) {
            return Err(invalid("nonfinite skin vertex"));
        }
        let weights = self
            .influences
            .get(vertex)
            .ok_or_else(|| invalid("skin vertex index"))?;
        let mut transform = DMat4::ZERO;
        for lane in 0..4 {
            transform +=
                self.joints[weights.joints[lane] as usize] * f64::from(weights.weights[lane]);
        }
        let linear = DMat3::from_mat4(transform);
        let determinant = linear.determinant();
        if determinant == 0. || !determinant.is_finite() {
            return Err(invalid("singular blended skin transform"));
        }
        let normal = linear.inverse().transpose();
        let vector = |offset| {
            DVec3::new(
                f64::from(source[offset]),
                f64::from(source[offset + 1]),
                f64::from(source[offset + 2]),
            )
        };
        // Explicit homogeneous multiplication preserves the weighted sum even
        // when normalized f32 weights have a tiny rounding residual from one.
        let position = (transform * vector(0).extend(1.)).truncate();
        let n = normal * vector(3);
        let tangent = linear * vector(8);
        let mut out = source;
        out[..3].copy_from_slice(&position.as_vec3().to_array());
        out[3..6].copy_from_slice(&n.as_vec3().to_array());
        out[8..11].copy_from_slice(&tangent.as_vec3().to_array());
        out[11] *= if determinant < 0. { -1. } else { 1. };
        if out.iter().any(|v| !v.is_finite()) {
            return Err(invalid("skinned vertex exceeds numeric range"));
        }
        Ok((out, determinant < 0.))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blended_nonuniform_reflection_transforms_the_full_vertex_basis() {
        let joints = [
            DMat4::from_scale_rotation_translation(
                DVec3::new(2., 1., 1.),
                DQuat::IDENTITY,
                DVec3::new(4., 0., 0.),
            ),
            DMat4::from_scale(DVec3::new(1., 3., -1.)),
        ];
        let influences = [Influences {
            joints: [0, 1, 0, 0],
            weights: [0.25, 0.75, 0., 0.],
        }];
        let pose = SkinPose {
            joints: &joints,
            influences: &influences,
        };
        let source = [1., 2., 3., 1., 1., 1., 0.125, 0.75, 1., -1., 0., 1.];
        let got = pose.deform_vertex(0, source).unwrap();
        let expected = [
            2.25, 5., -1.5, 0.8, 0.4, -2., 0.125, 0.75, 1.25, -2.5, 0., -1.,
        ];
        for (a, b) in got.into_iter().zip(expected) {
            assert!((a - b).abs() < 1e-6, "{a} != {b}");
        }
        assert!(pose.deform_vertex(1, source).is_err());
    }

    #[test]
    fn collapsed_blend_and_unrepresentable_results_refuse() {
        let influences = [Influences {
            joints: [0; 4],
            weights: [1., 0., 0., 0.],
        }];
        let source = [1.; 12];
        for matrix in [DMat4::ZERO, DMat4::from_scale(DVec3::splat(1e100))] {
            let joints = [matrix];
            let pose = SkinPose {
                joints: &joints,
                influences: &influences,
            };
            assert!(pose.deform_vertex(0, source).is_err());
        }
    }
}

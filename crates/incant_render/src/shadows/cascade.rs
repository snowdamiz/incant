//! Camera-fitted, texel-stabilized directional cascade projections.
use crate::{ResourceError, camera::CameraView};
use glam::{DMat4, DVec3, Mat4, Vec3};

pub(crate) const CASCADES: usize = 4;
pub(crate) const MAP_SIZE: u32 = 1024;

#[derive(Clone, Copy, Debug)]
pub(crate) struct Bounds {
    pub minimum: Vec3,
    pub maximum: Vec3,
}
impl Bounds {
    fn corners(self) -> impl Iterator<Item = DVec3> {
        (0..8).map(move |i| {
            DVec3::new(
                if i & 1 == 0 {
                    self.minimum.x
                } else {
                    self.maximum.x
                } as f64,
                if i & 2 == 0 {
                    self.minimum.y
                } else {
                    self.maximum.y
                } as f64,
                if i & 4 == 0 {
                    self.minimum.z
                } else {
                    self.maximum.z
                } as f64,
            )
        })
    }
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct Cascade {
    pub matrix: Mat4,
    pub far: f32,
    pub blend_start: f32,
    pub texel_world: f32,
    pub inverse_depth: f32,
}

/// Empty receiver intervals intentionally request no shadow sampling. All caster
/// bounds participate in depth fitting, including casters outside the view.
pub(crate) fn fit(
    camera: CameraView,
    aspect: f32,
    direction: Vec3,
    distance: f32,
    casters: &[Bounds],
) -> Result<Option<[Cascade; CASCADES]>, ResourceError> {
    if !aspect.is_finite() || aspect <= 0. || !distance.is_finite() || distance <= 0. {
        return Err(ResourceError::ShadowRange);
    }
    let near = f64::from(camera.near);
    let far = f64::from(camera.far.min(distance));
    if far <= near {
        return Ok(None);
    }
    let forward = direction.as_dvec3().normalize();
    if !forward.is_finite() {
        return Err(ResourceError::ShadowRange);
    }
    let up = if forward.dot(DVec3::Y).abs() < 0.95 {
        DVec3::Y
    } else {
        DVec3::X
    };
    let orientation = DMat4::look_to_rh(DVec3::ZERO, forward, up);
    let camera_world = camera.view.as_dmat4().inverse();
    if !camera_world.is_finite() {
        return Err(ResourceError::ShadowRange);
    }
    let vertical = camera.half_height_at(1.);
    let mut boundaries = [near; CASCADES + 1];
    for (i, split) in boundaries.iter_mut().enumerate().skip(1) {
        let t = i as f64 / CASCADES as f64;
        *split = 0.5 * (near * (far / near).powf(t) + near + (far - near) * t);
    }
    boundaries[CASCADES] = far;
    let mut caster_z = (f64::INFINITY, f64::NEG_INFINITY);
    for point in casters.iter().flat_map(|b| b.corners()) {
        let z = orientation.transform_point3(point).z;
        if !z.is_finite() {
            return Err(ResourceError::ShadowRange);
        }
        caster_z.0 = caster_z.0.min(z);
        caster_z.1 = caster_z.1.max(z);
    }
    let mut cascades = Vec::with_capacity(CASCADES);
    for i in 0..CASCADES {
        // The next cascade includes the previous split's blend interval.
        let start = if i == 0 {
            near
        } else {
            boundaries[i] - 0.1 * (boundaries[i] - boundaries[i - 1])
        };
        let end = boundaries[i + 1];
        let corners: Vec<_> = [start, end]
            .into_iter()
            .flat_map(|depth| {
                let scale = if camera.orthographic_half_height.is_some() {
                    1.
                } else {
                    depth
                };
                [-1., 1.].into_iter().flat_map(move |y| {
                    [-1., 1.].into_iter().map(move |x| {
                        camera_world.transform_point3(DVec3::new(
                            x * vertical * f64::from(aspect) * scale,
                            y * vertical * scale,
                            -depth,
                        ))
                    })
                })
            })
            .collect();
        let center = corners.iter().copied().sum::<DVec3>() / 8.;
        let radius = corners
            .iter()
            .map(|p| p.distance(center))
            .fold(0., f64::max);
        // Stable square extent and two texels of margin per edge accommodate
        // half-texel snapping and the comparison filter's neighboring samples.
        let radius = (radius * 16.).ceil() / 16. * f64::from(MAP_SIZE) / f64::from(MAP_SIZE - 4);
        let texel = 2. * radius / f64::from(MAP_SIZE);
        if !texel.is_finite() || texel <= 0. {
            return Err(ResourceError::ShadowRange);
        }
        let center_light = orientation.transform_point3(center);
        let x = (center_light.x / texel).round() * texel;
        let y = (center_light.y / texel).round() * texel;
        let mut depth = caster_z;
        for point in &corners {
            let z = orientation.transform_point3(*point).z;
            depth.0 = depth.0.min(z);
            depth.1 = depth.1.max(z);
        }
        let padding = texel.max(1.);
        let near_z = -depth.1 - padding;
        let far_z = -depth.0 + padding;
        let span = far_z - near_z;
        // Do not publish a map whose normalized depth cannot distinguish a
        // quarter texel in world space at worst-case f32 depth precision.
        if !span.is_finite() || span <= 0. || span * f64::from(f32::EPSILON) > texel * 0.25 {
            return Err(ResourceError::ShadowRange);
        }
        let matrix = (DMat4::orthographic_rh(
            x - radius,
            x + radius,
            y - radius,
            y + radius,
            near_z,
            far_z,
        ) * orientation)
            .as_mat4();
        if !matrix.is_finite() || !matrix.inverse().is_finite() {
            return Err(ResourceError::ShadowRange);
        }
        // Guard large translations that collapse otherwise finite f32 matrices.
        for point in &corners {
            let p = matrix.project_point3(point.as_vec3());
            if !p.is_finite()
                || p.x.abs() > 1.001
                || p.y.abs() > 1.001
                || !(-0.001..=1.001).contains(&p.z)
            {
                return Err(ResourceError::ShadowRange);
            }
        }
        cascades.push(Cascade {
            matrix,
            far: end as f32,
            blend_start: (end - 0.1 * (end - boundaries[i])) as f32,
            texel_world: texel as f32,
            inverse_depth: (1. / span) as f32,
        });
    }
    Ok(Some(cascades.try_into().expect("four cascades")))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn camera(x: f64) -> CameraView {
        CameraView::authored(
            &incant_doc::Camera {
                fov_degrees: 60.,
                near: 0.1,
                far: 100.,
                projection: Default::default(),
            },
            DMat4::from_translation(DVec3::new(x, 0., 8.)),
        )
        .unwrap()
    }
    #[test]
    fn receiver_frustum_and_offscreen_caster_depth_are_covered() {
        for orthographic in [false, true] {
            let mut view = camera(0.);
            if orthographic {
                view.orthographic_half_height = Some(8.);
            }
            check_receiver_coverage(view);
        }
    }
    fn check_receiver_coverage(view: CameraView) {
        let bounds = Bounds {
            minimum: Vec3::new(99., -1., 39.),
            maximum: Vec3::new(101., 1., 41.),
        };
        let cascades = fit(view, 1.5, Vec3::NEG_Z, 40., &[bounds])
            .unwrap()
            .unwrap();
        let mut previous = view.near;
        for c in cascades {
            assert!(c.far > previous && c.blend_start > previous && c.blend_start < c.far);
            assert!(c.texel_world > 0. && c.inverse_depth > 0.);
            for z in [previous, c.far] {
                let half_height = view.half_height_at(f64::from(z)) as f32;
                for (x, y) in [(-1., -1.), (-1., 1.), (1., -1.), (1., 1.)] {
                    let p = c.matrix.project_point3(Vec3::new(
                        x * half_height * 1.5,
                        y * half_height,
                        8. - z,
                    ));
                    assert!(
                        p.x.abs() <= 1. && p.y.abs() <= 1. && (0.0..=1.).contains(&p.z),
                        "{p:?}"
                    );
                }
            }
            for p in bounds.corners() {
                let z = c.matrix.project_point3(p.as_vec3()).z;
                assert!((0.0..=1.).contains(&z));
            }
            previous = c.far;
        }
        assert_eq!(previous, 40.);
    }
    #[test]
    fn subtexel_camera_motion_keeps_xy_projection_and_invalid_precision_fails() {
        let original = fit(camera(0.), 1., Vec3::NEG_Z, 40., &[]).unwrap().unwrap();
        let moved = fit(
            camera(f64::from(original[0].texel_world) * 0.1),
            1.,
            Vec3::NEG_Z,
            40.,
            &[],
        )
        .unwrap()
        .unwrap();
        for (a, b) in original.iter().zip(moved) {
            assert_eq!(a.matrix.row(0), b.matrix.row(0));
            assert_eq!(a.matrix.row(1), b.matrix.row(1));
        }
        assert!(fit(camera(0.), 1., Vec3::Y, 40., &[]).unwrap().is_some());
        assert!(fit(camera(0.), 1., Vec3::ZERO, 40., &[]).is_err());
        assert!(fit(camera(0.), 0., Vec3::NEG_Z, 40., &[]).is_err());
        assert!(
            fit(camera(0.), 1., Vec3::NEG_Z, 0.05, &[])
                .unwrap()
                .is_none()
        );
        assert!(
            fit(
                camera(0.),
                1.,
                Vec3::NEG_Z,
                40.,
                &[Bounds {
                    minimum: Vec3::splat(-1e30),
                    maximum: Vec3::splat(1e30)
                }]
            )
            .is_err()
        );
    }
}

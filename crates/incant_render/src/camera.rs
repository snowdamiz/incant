//! Authored cameras share one projection with geometry, clusters and shadows.
use crate::SceneError;
use glam::{DMat4, DVec3, Mat4, Vec3};

#[derive(Clone, Copy)]
pub(crate) struct CameraView {
    pub view: Mat4,
    pub eye: Vec3,
    pub forward: Vec3,
    pub near: f32,
    pub far: f32,
    pub fov: f32,
    pub orthographic_half_height: Option<f32>,
    pub jitter: [f32; 2],
    pub jitter_guard: [f32; 2],
    pub pixel_resolution: Option<[u32; 2]>,
}
impl CameraView {
    pub fn preview() -> Self {
        let eye = Vec3::from_array(crate::studio::EYE);
        Self {
            jitter: [0.; 2],
            jitter_guard: [0.; 2],
            view: Mat4::look_at_rh(eye, Vec3::ZERO, Vec3::Y),
            eye,
            forward: (-eye).normalize(),
            near: crate::CAMERA_NEAR,
            far: crate::CAMERA_FAR,
            fov: crate::CAMERA_FOV,
            orthographic_half_height: None,
            pixel_resolution: None,
        }
    }
    pub fn authored(camera: &incant_types::Camera, world: DMat4) -> Result<Self, SceneError> {
        if !(0.1..179.).contains(&camera.fov_degrees) {
            return Err(SceneError::CameraProjection);
        }
        // Local -Z looks forward, +Y supplies roll. Normalize before converting
        // to GPU precision; scale changes inherited position/aim, not clip units.
        let mut eye64 = world.transform_point3(DVec3::ZERO);
        let pixel_resolution = if let incant_types::CameraProjection::PixelPerfect {
            pixels_per_unit,
            reference_resolution,
        } = camera.projection
        {
            if !pixels_per_unit.is_finite()
                || !(1.0..=4096.0).contains(&pixels_per_unit)
                || reference_resolution
                    .iter()
                    .any(|d| !(16..=4096).contains(d))
                || u64::from(reference_resolution[0]) * u64::from(reference_resolution[1])
                    > 4 * 1024 * 1024
            {
                return Err(SceneError::CameraProjection);
            }
            // Snap only camera-plane translation. Simulation state and forward
            // depth stay authored; inherited roll defines the grid's axes.
            let forward = world.transform_vector3(-DVec3::Z).normalize();
            let right = forward.cross(world.transform_vector3(DVec3::Y)).normalize();
            let up = right.cross(forward).normalize();
            for (axis, is_right) in [(right, true), (up, false)] {
                let coordinate = eye64.dot(axis);
                let pixel = coordinate * pixels_per_unit;
                // Orthonormal basis dot products can put an exact mathematical
                // half-pixel a few f64 ULPs on its other side after a roll.
                // Stabilize only that tiny band, capped far below a GPU pixel.
                let half = pixel.floor() + 0.5;
                let tolerance = (8. * f64::EPSILON * pixel.abs().max(1.)).min(1e-7);
                let pixel = if (pixel - half).abs() <= tolerance {
                    half
                } else {
                    pixel
                };
                // Align exact half-pixel ties with top-left raster coverage:
                // right ties round down, up ties round up (screen Y is inverted).
                // A grid-aligned object following the camera then stays still.
                let snapped = if is_right {
                    (pixel - 0.5).ceil()
                } else {
                    (pixel + 0.5).floor()
                };
                // Odd logical dimensions place the image centre on a pixel
                // centre. Offset framing by half a pixel so world-grid edges
                // still land on pixel boundaries, as they do for even sizes.
                let dimension = reference_resolution[usize::from(!is_right)];
                let phase = if dimension % 2 == 1 {
                    if is_right { 0.5 } else { -0.5 }
                } else {
                    0.
                };
                eye64 += axis * ((snapped + phase) / pixels_per_unit - coordinate);
            }
            Some(reference_resolution)
        } else {
            None
        };
        let eye = eye64.as_vec3();
        let forward = world.transform_vector3(-DVec3::Z).normalize().as_vec3();
        let up = world.transform_vector3(DVec3::Y).normalize().as_vec3();
        if !eye.is_finite()
            || !forward.is_finite()
            || !up.is_finite()
            || forward.cross(up).length_squared() < 1e-12
        {
            return Err(SceneError::CameraTransform);
        }
        let result = Self {
            jitter: [0.; 2],
            jitter_guard: [0.; 2],
            view: Mat4::look_to_rh(eye, forward, up),
            eye,
            forward,
            near: camera.near as f32,
            far: camera.far as f32,
            fov: camera.fov_degrees.to_radians() as f32,
            pixel_resolution,
            orthographic_half_height: match camera.projection {
                incant_types::CameraProjection::Perspective {} => None,
                incant_types::CameraProjection::Orthographic { vertical_size } => {
                    Some((vertical_size * 0.5) as f32)
                }
                incant_types::CameraProjection::PixelPerfect {
                    pixels_per_unit,
                    reference_resolution,
                } => Some((f64::from(reference_resolution[1]) / pixels_per_unit * 0.5) as f32),
            },
        };
        let ratio = result.far / result.near;
        if !result.view.is_finite()
            || !result.near.is_finite()
            || result.near <= 0.
            || !result.far.is_finite()
            || result.far <= result.near
            || !ratio.is_finite()
            || ratio <= 1.
            || !result.fov.is_finite()
            || result
                .orthographic_half_height
                .is_some_and(|h| !h.is_finite() || h <= 0.)
        {
            return Err(SceneError::CameraProjection);
        }
        result.matrix(1.)?;
        Ok(result)
    }
    /// Current-only overlays and their opaque depth use the same stable grid.
    pub fn unjittered(mut self) -> Self {
        self.jitter = [0.; 2];
        self.jitter_guard = [0.; 2];
        self
    }
    pub fn matrix(self, aspect: f32) -> Result<Mat4, SceneError> {
        if !aspect.is_finite() || aspect <= 0. {
            return Err(SceneError::CameraProjection);
        }
        let aspect = self
            .pixel_resolution
            .map_or(aspect, |[w, h]| w as f32 / h as f32);
        let projection = if let Some(h) = self.orthographic_half_height {
            let w = h * aspect;
            Mat4::orthographic_rh(-w, w, -h, h, self.near, self.far)
        } else {
            Mat4::perspective_rh(self.fov, aspect, self.near, self.far)
        };
        let mut matrix = projection * self.view;
        if self.jitter != [0.; 2] {
            matrix = Mat4::from_translation(Vec3::new(self.jitter[0], self.jitter[1], 0.)) * matrix;
        }
        if !matrix.is_finite() {
            return Err(SceneError::CameraProjection);
        }
        Ok(matrix)
    }
    /// View-space half height at a forward distance, for fitting shadow receivers.
    pub fn half_height_at(self, depth: f64) -> f64 {
        self.orthographic_half_height
            .map_or_else(|| (f64::from(self.fov) * 0.5).tan() * depth, f64::from)
    }
    /// Cluster shader parameter: perspective tangent or fixed orthographic extent.
    pub fn vertical_parameter(self) -> f32 {
        self.orthographic_half_height
            .unwrap_or_else(|| (self.fov * 0.5).tan())
    }
    /// Perspective uses an eye position (w=1); parallel rays use a direction (w=0).
    pub fn shading_eye(self) -> [f32; 4] {
        if self.orthographic_half_height.is_some() {
            (-self.forward).extend(0.).to_array()
        } else {
            self.eye.extend(1.).to_array()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn parameters() -> incant_types::Camera {
        incant_types::Camera {
            fov_degrees: 60.,
            near: 0.2,
            far: 500.,
            projection: Default::default(),
        }
    }
    #[test]
    fn camera_world_basis_uses_inherited_pose_but_not_scale_for_clip_units() {
        let world = DMat4::from_translation(DVec3::new(4., 5., 6.))
            * DMat4::from_rotation_y(std::f64::consts::FRAC_PI_2)
            * DMat4::from_scale(DVec3::new(2., 3., 4.));
        let camera = CameraView::authored(&parameters(), world).unwrap();
        assert!(camera.view.transform_point3(Vec3::new(4., 5., 6.)).length() < 1e-5);
        let point = camera.view.transform_point3(Vec3::new(1., 5., 6.));
        assert!((point - Vec3::new(0., 0., -3.)).length() < 1e-5);
        assert_eq!(camera.near, 0.2);
        assert_eq!(camera.far, 500.);
    }
    #[test]
    fn unrepresentable_cameras_fail_before_encoding_gpu_work() {
        for (near, far) in [
            (1e-100, 100.),
            (1., 1. + 1e-10),
            (1e-30, 1e30),
            (0.1, 1e100),
        ] {
            let mut c = parameters();
            c.near = near;
            c.far = far;
            assert!(matches!(
                CameraView::authored(&c, DMat4::IDENTITY),
                Err(SceneError::CameraProjection)
            ));
        }
        assert!(matches!(
            CameraView::authored(&parameters(), DMat4::from_translation(DVec3::splat(1e100))),
            Err(SceneError::CameraTransform)
        ));
        assert!(CameraView::preview().matrix(0.).is_err());
        assert!(CameraView::preview().matrix(f32::from_bits(1)).is_err());
    }
    #[test]
    fn orthographic_extent_depth_and_view_direction_are_independent_of_distance() {
        let mut parameters = parameters();
        parameters.projection = incant_types::CameraProjection::Orthographic { vertical_size: 8. };
        let camera = CameraView::authored(&parameters, DMat4::IDENTITY).unwrap();
        let projection = camera.matrix(1.5).unwrap();
        for depth in [0.2, 4., 100., 500.] {
            let edge = projection.project_point3(Vec3::new(6., 4., -depth));
            assert!((edge.x - 1.).abs() < 1e-6 && (edge.y - 1.).abs() < 1e-6);
            assert_eq!(camera.half_height_at(f64::from(depth)), 4.);
        }
        assert!(projection.project_point3(Vec3::new(0., 0., -0.2)).z.abs() < 1e-6);
        assert!((projection.project_point3(Vec3::new(0., 0., -500.)).z - 1.).abs() < 1e-6);
        assert_eq!(camera.shading_eye(), [0., 0., 1., 0.]);
        assert_eq!(camera.vertical_parameter(), 4.);
        let square = camera
            .matrix(1.)
            .unwrap()
            .project_point3(Vec3::new(4., 4., -20.));
        assert!((square.x - 1.).abs() < 1e-6 && (square.y - 1.).abs() < 1e-6);
    }
    #[test]
    fn orthographic_size_rejects_unrepresentable_gpu_extents() {
        let mut parameters = parameters();
        for size in [0., -1., 1e-100, 1e100, f64::INFINITY, f64::NAN] {
            parameters.projection = incant_types::CameraProjection::Orthographic {
                vertical_size: size,
            };
            assert!(matches!(
                CameraView::authored(&parameters, DMat4::IDENTITY),
                Err(SceneError::CameraProjection)
            ));
        }
    }

    #[test]
    fn pixel_grid_preserves_logical_framing_and_snaps_only_rendered_camera_motion() {
        let mut p = parameters();
        p.projection = incant_types::CameraProjection::PixelPerfect {
            pixels_per_unit: 32.,
            reference_resolution: [320, 180],
        };
        let at = |x, y, z| {
            CameraView::authored(&p, DMat4::from_translation(DVec3::new(x, y, z))).unwrap()
        };
        let first = at(0., 0., 8.);
        assert_eq!(first.view, at(0.49 / 32., -0.49 / 32., 8.).view);
        let moved = at(0.51 / 32., -0.51 / 32., 8.);
        assert_eq!(moved.eye, Vec3::new(1. / 32., -1. / 32., 8.));
        assert_eq!(at(0., 0., 8.01).eye.z, 8.01);
        assert_eq!(
            at(0.5 / 32., 0.5 / 32., 8.).eye,
            Vec3::new(0., 1. / 32., 8.)
        );
        assert_eq!(
            at(-0.5 / 32., -0.5 / 32., 8.).eye,
            Vec3::new(-1. / 32., 0., 8.)
        );
        assert_eq!(first.matrix(1.).unwrap(), first.matrix(2.).unwrap());
        let edge = first
            .matrix(1.)
            .unwrap()
            .project_point3(Vec3::new(5., 180. / 64., 0.));
        assert!((edge.x - 1.).abs() < 1e-6 && (edge.y - 1.).abs() < 1e-6);
        let rotation = DMat4::from_rotation_z(0.37);
        let pose = DMat4::from_translation(rotation.transform_vector3(DVec3::new(
            0.49 / 32.,
            -0.49 / 32.,
            8.,
        ))) * rotation;
        let rolled = CameraView::authored(&p, pose).unwrap();
        assert!((rolled.eye - Vec3::new(0., 0., 8.)).length() < 1e-6);
    }
}

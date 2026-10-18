//! Conservative static evidence, independent of sampled motion/coverage.
//! Failed preparation never publishes this snapshot. No hashes, allocations or
//! asset reads occur per submission; immutable bindings are admitted upstream.
use crate::{camera::CameraView, lighting::scene::Light};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct Camera {
    view: [u32; 16],
    eye: [u32; 3],
    forward: [u32; 3],
    clip_fov: [u32; 3],
    ortho: Option<u32>,
    pixels: Option<[u32; 2]>,
    rect: [u32; 4],
}
impl Camera {
    pub fn new(c: CameraView, rect: [f32; 4]) -> Self {
        Self {
            view: c.view.to_cols_array().map(f32::to_bits),
            eye: c.eye.to_array().map(f32::to_bits),
            forward: c.forward.to_array().map(f32::to_bits),
            clip_fov: [c.near, c.far, c.fov].map(f32::to_bits),
            ortho: c.orthographic_half_height.map(f32::to_bits),
            pixels: c.pixel_resolution,
            rect: rect.map(f32::to_bits),
        }
    }
}
pub(super) struct Continuity {
    camera: Option<Camera>,
    lights: Vec<Light>,
}
impl Continuity {
    pub fn new(light_capacity: usize) -> Self {
        Self {
            camera: None,
            lights: Vec::with_capacity(light_capacity),
        }
    }
    pub fn admits(&self, camera: Camera, lights: &[Light], geometry_stationary: bool) -> bool {
        geometry_stationary
            && self.camera == Some(camera)
            && bytemuck::cast_slice::<_, u8>(&self.lights) == bytemuck::cast_slice::<_, u8>(lights)
    }
    pub fn fits(&self, lights: &[Light]) -> bool {
        lights.len() <= self.lights.capacity()
    }
    pub fn accept(&mut self, camera: Camera, lights: &[Light]) {
        assert!(self.fits(lights), "prepared light capacity");
        self.lights.clear();
        self.lights.extend_from_slice(lights);
        self.camera = Some(camera);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn moved_authored_light_changes_packed_continuity_even_with_identical_typed_parameters() {
        let light = incant_render_data::SceneLight::Point(incant_types::PointLight {
            color: [1.; 3],
            intensity: 3.,
            range: 10.,
        });
        let pack = |world| {
            let mut plan = crate::lighting::scene::LightPlan::default();
            plan.add_typed(&light, world).unwrap();
            plan.finish().0
        };
        let old = pack(glam::DMat4::IDENTITY);
        let moved = pack(glam::DMat4::from_translation(glam::DVec3::X));
        let camera = Camera::new(CameraView::preview(), [0., 0., 48., 48.]);
        let mut state = Continuity::new(old.len());
        state.accept(camera, &old);
        assert!(state.admits(camera, &old, true));
        assert!(!state.admits(camera, &moved, true));
    }
    #[test]
    fn static_admission_compares_camera_geometry_and_actual_packed_light_without_publishing() {
        let view = CameraView::preview();
        let rect = [0., 0., 48., 48.];
        let camera = Camera::new(view, rect);
        let light = Light {
            position_range: [0., 0., 1., 10.],
            direction_outer: [0., 0., -1., 1.],
            radiance_inner: [1.; 4],
            kind: [1, 0, 0, 0],
        };
        let mut state = Continuity::new(1);
        assert!(!state.admits(camera, &[light], true));
        state.accept(camera, &[light]);
        assert!(state.admits(camera, &[light], true));
        assert!(!state.admits(camera, &[light], false));
        let mut moved_light = light;
        moved_light.position_range[0] = 1.;
        assert!(!state.admits(camera, &[moved_light], true));
        let mut moved_camera = view;
        moved_camera.eye.x += 1.;
        assert!(!state.admits(Camera::new(moved_camera, rect), &[light], true));
        assert!(!state.admits(Camera::new(view, [0., 0., 47., 48.]), &[light], true));
        assert!(!state.fits(&[light, light]));
        // All preceding rejected candidates leave the accepted state intact.
        assert!(state.admits(camera, &[light], true));
        state.accept(camera, &[moved_light]);
        assert!(!state.admits(camera, &[light], true));
        assert!(state.admits(camera, &[moved_light], true));
    }
}

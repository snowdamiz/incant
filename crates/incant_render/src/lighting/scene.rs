//! Immutable world-space punctual lights resolved from validated components.
use crate::SceneError;
use glam::{DMat4, DVec3, Vec3};
use incant_doc::{DirectionalLight, PointLight, SpotLight};
use std::collections::BTreeMap;

pub(crate) const MAX_DIRECTIONAL: usize = 16;
pub(crate) const MAX_LOCAL: usize = 4096;
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct Light {
    pub position_range: [f32; 4],
    pub direction_outer: [f32; 4],
    pub radiance_inner: [f32; 4],
    pub kind: [u32; 4],
}
#[derive(Default)]
pub(crate) struct LightPlan {
    directional: Vec<Light>,
    local: Vec<Light>,
}
impl LightPlan {
    pub fn add(
        &mut self,
        components: &BTreeMap<String, serde_json::Value>,
        world: DMat4,
    ) -> Result<(), SceneError> {
        let decode = |kind| components.get(kind).cloned();
        let (color, intensity, range, inner, outer, kind) =
            if let Some(value) = decode("DirectionalLight") {
                let l: DirectionalLight =
                    serde_json::from_value(value).map_err(incant_doc::DocumentError::from)?;
                (l.color, l.intensity, 0., 1., 1., 0)
            } else if let Some(value) = decode("PointLight") {
                let l: PointLight =
                    serde_json::from_value(value).map_err(incant_doc::DocumentError::from)?;
                (l.color, l.intensity, l.range, 1., -1., 1)
            } else if let Some(value) = decode("SpotLight") {
                let l: SpotLight =
                    serde_json::from_value(value).map_err(incant_doc::DocumentError::from)?;
                (
                    l.color,
                    l.intensity,
                    l.range,
                    l.inner_degrees.to_radians().cos(),
                    l.outer_degrees.to_radians().cos(),
                    2,
                )
            } else {
                return Ok(());
            };
        let position = world.transform_point3(DVec3::ZERO).as_vec3();
        let direction = world.transform_vector3(-DVec3::Z).normalize().as_vec3();
        if !position.is_finite()
            || !direction.is_finite()
            || !crate::camera_view().transform_point3(position).is_finite()
        {
            return Err(SceneError::LightTransform);
        }
        if kind == 2 && inner as f32 <= outer as f32 {
            return Err(SceneError::LightCone);
        }
        let light = Light {
            position_range: position.extend(range as f32).to_array(),
            direction_outer: direction.extend(outer as f32).to_array(),
            radiance_inner: Vec3::from_array(color.map(|v| (v * intensity) as f32))
                .extend(inner as f32)
                .to_array(),
            kind: [kind, 0, 0, 0],
        };
        if kind == 0 {
            self.directional.push(light);
        } else {
            self.local.push(light);
        }
        if self.directional.len() > MAX_DIRECTIONAL || self.local.len() > MAX_LOCAL {
            return Err(SceneError::LightLimit);
        }
        Ok(())
    }
    pub fn finish(mut self) -> (Vec<Light>, u32) {
        // Explicitly authored zero-intensity lights still disable the preview key.
        if self.directional.is_empty() && self.local.is_empty() {
            self.directional.push(Light {
                position_range: [0.; 4],
                direction_outer: (-Vec3::from_array(crate::studio::LIGHT_DIRECTION).normalize())
                    .extend(1.)
                    .to_array(),
                radiance_inner: Vec3::from_array(crate::studio::LIGHT_RADIANCE)
                    .extend(1.)
                    .to_array(),
                kind: [0; 4],
            });
        }
        let directional = self.directional.len() as u32;
        self.directional.extend(self.local);
        (self.directional, directional)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn world_transform_affects_position_direction_but_not_light_range_or_intensity() {
        let world = DMat4::from_translation(DVec3::new(4., 5., 6.))
            * DMat4::from_rotation_y(std::f64::consts::FRAC_PI_2)
            * DMat4::from_scale(DVec3::new(2., 3., 4.));
        let mut p = LightPlan::default();
        p.add(&BTreeMap::from([("SpotLight".into(),json!({"color":[1,0.5,0.25],"intensity":8,"range":10,"inner_degrees":10,"outer_degrees":20}))]),world).unwrap();
        let (lights, directional) = p.finish();
        assert_eq!(directional, 0);
        assert_eq!(lights.len(), 1);
        assert_eq!(lights[0].position_range, [4., 5., 6., 10.]);
        assert!((lights[0].direction_outer[0] + 1.).abs() < 1e-6);
        assert!(lights[0].direction_outer[2].abs() < 1e-6);
        assert_eq!(&lights[0].radiance_inner[..3], &[8., 4., 2.]);
    }
    #[test]
    fn explicit_zero_lights_disable_preview_and_budgets_fail_without_dropping_lights() {
        let (_, directional) = LightPlan::default().finish();
        assert_eq!(directional, 1);
        let c = BTreeMap::from([(
            "DirectionalLight".into(),
            json!({"color":[1,1,1],"intensity":0}),
        )]);
        let mut p = LightPlan::default();
        for _ in 0..MAX_DIRECTIONAL {
            p.add(&c, DMat4::IDENTITY).unwrap();
        }
        assert!(matches!(
            p.add(&c, DMat4::IDENTITY),
            Err(SceneError::LightLimit)
        ));
        let c = BTreeMap::from([(
            "PointLight".into(),
            json!({"color":[1,1,1],"intensity":0,"range":1}),
        )]);
        let mut p = LightPlan::default();
        p.add(&c, DMat4::IDENTITY).unwrap();
        let (lights, d) = p.finish();
        assert_eq!(d, 0);
        assert_eq!(lights[0].radiance_inner[..3], [0.; 3]);
        let mut p = LightPlan::default();
        for _ in 0..MAX_LOCAL {
            p.add(&c, DMat4::IDENTITY).unwrap();
        }
        assert!(matches!(
            p.add(&c, DMat4::IDENTITY),
            Err(SceneError::LightLimit)
        ));
    }
    #[test]
    fn unrepresentable_light_transforms_and_cones_fail_explicitly() {
        let mut p = LightPlan::default();
        let c = BTreeMap::from([(
            "SpotLight".into(),
            json!({
                "color":[1,1,1],"intensity":1,"range":1,
                "inner_degrees":10,"outer_degrees":10.000000001
            }),
        )]);
        assert!(matches!(
            p.add(&c, DMat4::IDENTITY),
            Err(SceneError::LightCone)
        ));
        let c = BTreeMap::from([(
            "PointLight".into(),
            json!({
                "color":[1,1,1],"intensity":1,"range":1
            }),
        )]);
        assert!(matches!(
            p.add(&c, DMat4::from_translation(DVec3::splat(1e100))),
            Err(SceneError::LightTransform)
        ));
    }
}

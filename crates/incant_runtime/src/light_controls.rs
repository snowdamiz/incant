//! Scalar controls over existing punctual lights, staged before any publication.
use crate::{
    CookedSpatial, FrameCommands, NativeWorld, RuntimeError, SceneError, StableId,
    StructuralCommand,
};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LightControl {
    /// Linear RGB multiplier, each finite in [0,1].
    Color([f64; 3]),
    /// Finite [0,1_000_000], lux for directional or candela for point/spot.
    Intensity(f64),
}
impl LightControl {
    pub fn validate(self) -> Result<(), SceneError> {
        let valid = match self {
            Self::Color(color) => color.iter().all(|v| (0. ..=1.).contains(v)),
            Self::Intensity(value) => (0. ..=1_000_000.).contains(&value),
        };
        if !valid {
            return Err(SceneError::Invalid("light control numeric range"));
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LightValues {
    color: [f64; 3],
    intensity: f64,
}
impl LightValues {
    fn read(node: &CookedSpatial) -> Option<Self> {
        if let Some(v) = &node.directional {
            Some(Self {
                color: v.color,
                intensity: v.intensity,
            })
        } else if let Some(v) = &node.point {
            Some(Self {
                color: v.color,
                intensity: v.intensity,
            })
        } else {
            node.spot.as_ref().map(|v| Self {
                color: v.color,
                intensity: v.intensity,
            })
        }
    }
    fn install(self, node: &mut CookedSpatial) {
        let (color, intensity) = if let Some(v) = &mut node.directional {
            (&mut v.color, &mut v.intensity)
        } else if let Some(v) = &mut node.point {
            (&mut v.color, &mut v.intensity)
        } else {
            let v = node.spot.as_mut().expect("prepared final punctual light");
            (&mut v.color, &mut v.intensity)
        };
        *color = self.color;
        *intensity = self.intensity;
    }
}
impl NativeWorld {
    /// Controls use the batch's final spatial declaration and compose in order.
    /// Only touched scalar values are copied; kind/range/cone/shadows stay bound.
    pub(crate) fn prepare_light_controls(
        &self,
        commands: &FrameCommands,
        spatial: &BTreeMap<StableId, Option<CookedSpatial>>,
        exists: impl Fn(StableId) -> bool,
    ) -> Result<BTreeMap<StableId, LightValues>, RuntimeError> {
        let mut changes = BTreeMap::new();
        for command in commands.iter() {
            let StructuralCommand::ControlLight { entity, control } = command else {
                continue;
            };
            control.validate()?;
            if !exists(*entity) {
                return Err(RuntimeError::MissingEntity(*entity));
            }
            let value = match changes.entry(*entity) {
                std::collections::btree_map::Entry::Occupied(v) => v.into_mut(),
                std::collections::btree_map::Entry::Vacant(v) => {
                    let node = spatial.get(entity).map(Option::as_ref).unwrap_or_else(|| {
                        self.entities
                            .get(entity)
                            .and_then(|h| self.world.get::<CookedSpatial>(*h))
                    });
                    v.insert(node.and_then(LightValues::read).ok_or(SceneError::Invalid(
                        "light control requires final punctual light",
                    ))?)
                }
            };
            match *control {
                LightControl::Color(color) => value.color = color,
                LightControl::Intensity(intensity) => value.intensity = intensity,
            }
        }
        Ok(changes)
    }
    pub(crate) fn install_light_controls(&mut self, values: BTreeMap<StableId, LightValues>) {
        for (id, value) in values {
            value.install(
                &mut self
                    .world
                    .get_mut::<CookedSpatial>(self.entities[&id])
                    .expect("prepared final light"),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scalar_updates_do_not_consume_exhausted_binding_or_topology_generations() {
        let id = StableId([1; 16]);
        let mut node = CookedSpatial::empty(id);
        node.point = Some(incant_types::PointLight {
            color: [1.; 3],
            intensity: 0.,
            range: 2.,
        });
        let scene = crate::CookedScene::new(
            StableId([0; 16]),
            60,
            vec![crate::CookedEntity {
                id,
                parent: None,
                transform: Default::default(),
                velocity: None,
            }],
        )
        .unwrap()
        .with_spatial(vec![], vec![node])
        .unwrap();
        let mut w = NativeWorld::new(scene);
        w.binding_generation = u64::MAX;
        w.topology_generation = u64::MAX;
        for n in [0., 2., 0.] {
            let mut q = FrameCommands::default();
            q.push(StructuralCommand::ControlLight {
                entity: id,
                control: LightControl::Intensity(n),
            })
            .unwrap();
            w.apply(&mut q).unwrap();
            assert_eq!(
                w.spatial_view()
                    .unwrap()
                    .nodes()
                    .next()
                    .unwrap()
                    .declaration
                    .unwrap()
                    .point
                    .as_ref()
                    .unwrap()
                    .intensity,
                n
            );
            assert_eq!(w.binding_generation, u64::MAX);
            assert_eq!(w.topology_generation, u64::MAX);
        }
    }
}

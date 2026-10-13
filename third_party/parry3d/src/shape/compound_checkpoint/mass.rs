//! Replay pinned native Sum arithmetic without its temporary Vec.
use super::*;
use crate::mass_properties::MassProperties;
use crate::utils;
impl Compound {
    /// Compute exact native compound mass arithmetic using two ordered passes.
    /// The caller must first admit this witness through `checkpoint_plan` and
    /// separately check density and charge both child walks to its work budget.
    /// No BVH, SharedShape, temporary Vec or native compound is constructed.
    pub fn checkpoint_mass_properties(
        value: &s::ValidatedCompound<'_>,
        density: f32,
    ) -> MassProperties {
        let properties = || {
            value.geometry().record().parts.iter().map(|part| {
                primitive::Primitive::new(part.shape)
                    .shape()
                    .mass_properties(density)
                    .transform_by(&primitive::pose(part))
            })
        };
        let mut total_mass = 0.0;
        let mut total_com = Vector::ZERO;
        for props in properties() {
            let mass = utils::inv(props.inv_mass);
            total_mass += mass;
            total_com += props.local_com * mass;
        }
        if total_mass > 0.0 {
            total_com /= total_mass;
        }
        #[cfg(feature = "dim2")]
        let mut total_inertia = 0.0;
        #[cfg(feature = "dim3")]
        let mut total_inertia = crate::math::Matrix::ZERO;
        for props in properties() {
            total_inertia += props.construct_shifted_inertia_matrix(total_com - props.local_com);
        }
        #[cfg(feature = "dim2")]
        {
            MassProperties {
                local_com: total_com,
                inv_mass: utils::inv(total_mass),
                inv_principal_inertia: utils::inv(total_inertia),
            }
        }
        #[cfg(feature = "dim3")]
        {
            MassProperties::with_inertia_matrix(total_com, total_mass, total_inertia)
        }
    }
}

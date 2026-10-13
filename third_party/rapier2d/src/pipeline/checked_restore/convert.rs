use crate::dynamics::{MassProperties, RigidBodyType};
use crate::math::{AngVector, Pose, Rotation, Vector};
use incant_physics_snapshot::{BodyKind, LocalMass, LocalPose};
pub(crate) fn v(b: [u32; 3]) -> Vector {
    #[cfg(feature = "dim2")]
    {
        Vector::new(f32::from_bits(b[0]), f32::from_bits(b[1]))
    }
    #[cfg(feature = "dim3")]
    {
        Vector::from_array(b.map(f32::from_bits))
    }
}
pub(crate) fn bits(v: Vector) -> [u32; 3] {
    #[cfg(feature = "dim2")]
    {
        [v.x.to_bits(), v.y.to_bits(), 0]
    }
    #[cfg(feature = "dim3")]
    {
        v.to_array().map(f32::to_bits)
    }
}
pub(crate) fn pose(p: LocalPose) -> Pose {
    #[cfg(feature = "dim2")]
    let r = Rotation::from_cos_sin_unchecked(
        f32::from_bits(p.rotation[0]),
        f32::from_bits(p.rotation[1]),
    );
    #[cfg(feature = "dim3")]
    let r = Rotation::from_array(p.rotation.map(f32::from_bits));
    Pose::from_parts(v(p.translation), r)
}
pub(crate) fn pose_bits(p: Pose) -> LocalPose {
    #[cfg(feature = "dim2")]
    let rotation = [p.rotation.re.to_bits(), p.rotation.im.to_bits(), 0, 0];
    #[cfg(feature = "dim3")]
    let rotation = p.rotation.to_array().map(f32::to_bits);
    LocalPose {
        translation: bits(p.translation),
        rotation,
    }
}
pub(crate) fn mass(m: LocalMass) -> MassProperties {
    MassProperties {
        local_com: v(m.center),
        inv_mass: f32::from_bits(m.inverse_mass),
        #[cfg(feature = "dim2")]
        inv_principal_inertia: f32::from_bits(m.inverse_principal_inertia[0]),
        #[cfg(feature = "dim3")]
        inv_principal_inertia: v(m.inverse_principal_inertia),
        #[cfg(feature = "dim3")]
        principal_inertia_local_frame: Rotation::from_array(m.principal_frame.map(f32::from_bits)),
    }
}
pub(crate) fn mass_bits(m: MassProperties) -> LocalMass {
    LocalMass {
        center: bits(m.local_com),
        inverse_mass: m.inv_mass.to_bits(),
        #[cfg(feature = "dim2")]
        inverse_principal_inertia: [m.inv_principal_inertia.to_bits(), 0, 0],
        #[cfg(feature = "dim3")]
        inverse_principal_inertia: bits(m.inv_principal_inertia),
        #[cfg(feature = "dim2")]
        principal_frame: [0; 4],
        #[cfg(feature = "dim3")]
        principal_frame: m.principal_inertia_local_frame.to_array().map(f32::to_bits),
    }
}
pub(crate) fn kind(b: BodyKind) -> RigidBodyType {
    match b {
        BodyKind::Dynamic => RigidBodyType::Dynamic,
        BodyKind::Fixed => RigidBodyType::Fixed,
        BodyKind::KinematicPosition => RigidBodyType::KinematicPositionBased,
        BodyKind::KinematicVelocity => RigidBodyType::KinematicVelocityBased,
    }
}

pub(crate) fn angular(b: [u32; 3]) -> AngVector {
    #[cfg(feature = "dim2")]
    {
        f32::from_bits(b[0])
    }
    #[cfg(feature = "dim3")]
    {
        v(b)
    }
}

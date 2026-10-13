use super::*;
use crate::math::Rotation;
use crate::shape::{Ball, Capsule, Cuboid, TypedShape};
use core::alloc::Layout;

pub(super) fn bits(v: Vector) -> [u32; 3] {
    let mut out = [0; 3];
    for i in 0..crate::math::DIM {
        out[i] = v[i].to_bits();
    }
    out
}
pub(super) fn vector(b: [u32; 3]) -> Vector {
    #[cfg(feature = "dim2")]
    {
        Vector::new(f32::from_bits(b[0]), f32::from_bits(b[1]))
    }
    #[cfg(feature = "dim3")]
    {
        Vector::from_array(b.map(f32::from_bits))
    }
}
pub(super) fn bounds(a: Aabb) -> s::CompoundAabb {
    s::CompoundAabb {
        mins: bits(a.mins),
        maxs: bits(a.maxs),
    }
}
pub(super) fn aabb(a: s::CompoundAabb) -> Aabb {
    Aabb::new(vector(a.mins), vector(a.maxs))
}
pub(super) fn part(pose: Pose, shape: &SharedShape) -> s::Result<s::Part> {
    #[cfg(feature = "dim2")]
    let rotation = {
        // PlanarTiles deliberately supports only the native tile factory's
        // exact identity rotation, including the signs of its zero fields.
        if pose.rotation.re.to_bits() != 1f32.to_bits() || pose.rotation.im.to_bits() != 0 {
            return Err(s::Error::Geometry);
        }
        [0, 0, 0, 1f32.to_bits()]
    };
    #[cfg(feature = "dim3")]
    let rotation = pose.rotation.to_array().map(f32::to_bits);
    let shape = match shape.as_typed_shape() {
        TypedShape::Cuboid(c) if shape.as_shape::<Cuboid>().is_some() => s::Primitive::Box {
            half_extents: bits(c.half_extents),
        },
        #[cfg(feature = "dim3")]
        TypedShape::Ball(b) if shape.as_shape::<Ball>().is_some() => s::Primitive::Ball {
            radius: b.radius.to_bits(),
        },
        #[cfg(feature = "dim3")]
        TypedShape::Capsule(c) if shape.as_shape::<Capsule>().is_some() => {
            let canonical = Capsule::new_y(c.segment.b.y, c.radius);
            if bits(c.segment.a) != bits(canonical.segment.a)
                || bits(c.segment.b) != bits(canonical.segment.b)
            {
                return Err(s::Error::Geometry);
            }
            s::Primitive::CapsuleY {
                half_height: c.segment.b.y.to_bits(),
                radius: c.radius.to_bits(),
            }
        }
        _ => return Err(s::Error::Geometry),
    };
    Ok(s::Part {
        translation: bits(pose.translation),
        rotation,
        shape,
    })
}
pub(super) fn pose(p: &s::Part) -> Pose {
    #[cfg(feature = "dim2")]
    let rotation = Rotation::IDENTITY;
    #[cfg(feature = "dim3")]
    let rotation = Rotation::from_array(p.rotation.map(f32::from_bits));
    Pose::from_parts(vector(p.translation), rotation)
}
pub(super) enum Primitive {
    Box(Cuboid),
    Ball(Ball),
    Capsule(Capsule),
}
impl Primitive {
    pub(super) fn new(p: s::Primitive) -> Self {
        match p {
            s::Primitive::Box { half_extents } => Self::Box(Cuboid::new(vector(half_extents))),
            s::Primitive::Ball { radius } => Self::Ball(Ball::new(f32::from_bits(radius))),
            s::Primitive::CapsuleY {
                half_height,
                radius,
            } => Self::Capsule(Capsule::new_y(
                f32::from_bits(half_height),
                f32::from_bits(radius),
            )),
        }
    }
    pub(super) fn shape(&self) -> &dyn Shape {
        match self {
            Self::Box(v) => v,
            Self::Ball(v) => v,
            Self::Capsule(v) => v,
        }
    }
    pub(super) fn shared(self, permit: &mut AllocationPermit) -> ConstructionResult<SharedShape> {
        permit.charge(self.plan()?)?;
        // Stable Arc allocation is infallible; logical precharge does not make
        // allocator exhaustion recoverable. No source Arc or shape is cloned.
        Ok(match self {
            Self::Box(v) => SharedShape::new(v),
            Self::Ball(v) => SharedShape::new(v),
            Self::Capsule(v) => SharedShape::new(v),
        })
    }
    pub(super) fn plan(&self) -> ConstructionResult<AllocationPlan> {
        fn arc<T>() -> ConstructionResult<AllocationPlan> {
            let (layout, _) = Layout::array::<usize>(2)
                .map_err(|_| ConstructionError::Limit)?
                .extend(Layout::new::<T>())
                .map_err(|_| ConstructionError::Limit)?;
            AllocationPlan::array::<u8>(layout.pad_to_align().size())
        }
        match self {
            Self::Box(_) => arc::<Cuboid>(),
            Self::Ball(_) => arc::<Ball>(),
            Self::Capsule(_) => arc::<Capsule>(),
        }
    }
}

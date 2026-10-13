//! A bounded feasibility diagnostic, not a save implementation.
//! Only engine-constructed typed worlds are cloned. No serialized Rapier state
//! is ever read or deserialized. JSON is diagnostic output, not runtime data.
#![forbid(unsafe_code)]
mod planar {
    use rapier2d::prelude::*;
    const DIMENSION: u32 = 2;
    fn vector(x: f32, y: f32) -> Vector {
        Vector::new(x, y)
    }
    fn cuboid(x: f32, y: f32) -> SharedShape {
        SharedShape::cuboid(x, y)
    }
    include!("probe.rs");
}
mod spatial {
    use rapier3d::prelude::*;
    const DIMENSION: u32 = 3;
    fn vector(x: f32, y: f32) -> Vector {
        Vector::new(x, y, 0.)
    }
    fn cuboid(x: f32, y: f32) -> SharedShape {
        SharedShape::cuboid(x, y, 0.5)
    }
    include!("probe.rs");
}
fn main() {
    rayon::ThreadPoolBuilder::new()
        .num_threads(2)
        .build_global()
        .unwrap();
    let rows: Vec<_> = planar::run().into_iter().chain(spatial::run()).collect();
    println!("{}", serde_json::to_string_pretty(&rows).unwrap());
}

// The format word retains the historical layout in its low half and binds the
// compiled physics variant in its high half. Keep layout assertions exact.
pub fn scene_format(layout: u32) -> u32 {
    use incant_runtime::{PHYSICS_VARIANT, PhysicsVariant};
    layout
        | match PHYSICS_VARIANT {
            PhysicsVariant::Deterministic => 0,
            PhysicsVariant::Fast => 1 << 16,
            PhysicsVariant::FastSimd8 => 2 << 16,
        }
}

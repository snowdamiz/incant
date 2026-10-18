//! Test-only MRT discriminator; semantic motion and the geometry oracle stay fixed.
use super::readback::Sample;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Storage {
    Original,
    Redundant,
    Rgba,
    Slots,
}
impl Storage {
    pub fn name(self) -> &'static str {
        match self {
            Self::Original => "original-forward",
            Self::Redundant => "redundant-forward",
            Self::Rgba => "rgba-forward",
            Self::Slots => "rgba-swapped",
        }
    }
    /// Semantic reader bindings -> physical render attachment slots.
    pub fn readback_slots(self) -> [usize; 4] {
        if self == Self::Slots {
            [0, 2, 1, 3]
        } else {
            [0, 1, 2, 3]
        }
    }
}
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct Record {
    pub sample: Sample,
    pub raw_motion_zw: [u32; 4],
    pub raw_owner: [u32; 4],
}
const _: () = assert!(size_of::<Record>() == 80);

fn replace(text: String, from: &str, to: &str) -> String {
    assert_eq!(text.matches(from).count(), 1, "storage diagnostic anchor");
    text.replace(from, to)
}
pub(super) fn source(storage: Storage) -> String {
    let text = super::source();
    match storage {
        Storage::Original => text,
        Storage::Redundant => {
            let mut text = replace(text, "pack(m.zw)", "redundant(m.zw,in.owner)");
            text.push_str(REDUNDANT);
            text
        }
        Storage::Rgba => replace(
            replace(text, "@location(2) owner:vec2u", "@location(2) owner:vec4u"),
            "vec2u(in.owner&65535u,in.owner>>16u)",
            "vec4u(in.owner&65535u,in.owner>>16u,0u,0u)",
        ),
        Storage::Slots => swap_locations(source(Storage::Rgba), "owner:vec4u"),
    }
}
fn swap_locations(text: String, owner: &str) -> String {
    replace(
        replace(text, "@location(1) motion_zw:", "@location(2) motion_zw:"),
        &format!("@location(2) {owner}"),
        &format!("@location(1) {owner}"),
    )
}
const REDUNDANT: &str = r#"
fn redundant(v:vec2f,owner:u32)->vec4u {
    let b=bitcast<vec2u>(v);
    // Fixture-only, lossless: finite nonnegative depth (including positive zero),
    // validity exactly positive zero or one. Refuse rather than canonicalize -0.
    if (b.x&2147483648u)!=0u || !(v.x>=0.0 && v.x<=1.0) || (b.y!=0u && b.y!=1065353216u) {
        return vec4u(0u,32704u,65535u,65535u); // decoded NaN: original oracle refuses
    }
    let depth_valid=b.x|select(0u,2147483648u,b.y==1065353216u);
    return vec4u(depth_valid&65535u,depth_valid>>16u,owner&65535u,owner>>16u);
}
"#;
pub(super) fn calibration(storage: Storage) -> String {
    let text = format!("{}\n{}", super::CALIBRATION, super::PACK);
    if matches!(storage, Storage::Rgba | Storage::Slots) {
        let text = replace(
            replace(text, "sample_id:vec2u", "sample_id:vec4u"),
            "vec2u(i+1u,0u)",
            "vec4u(i+1u,0u,0u,0u)",
        );
        if storage == Storage::Slots {
            swap_locations(text, "sample_id:vec4u")
        } else {
            text
        }
    } else {
        text
    }
}
pub(super) fn copy(storage: Storage) -> String {
    assert_ne!(storage, Storage::Original);
    let mut text = replace(
        super::diagnostic::copy(false),
        "struct Sample {motion:vec4f,position_depth:vec4f,owner:vec4u}",
        "struct Sample {motion:vec4f,position_depth:vec4f,owner:vec4u,raw_motion_zw:vec4u,raw_owner:vec4u}",
    );
    text = replace(
        text,
        "vec4u(o.x|(o.y<<16u),o.x,o.y,raw_zw.z));",
        "vec4u(o.x|(o.y<<16u),o.x,o.y,raw_zw.z),raw_zw,textureLoad(owner,p,s));",
    );
    if storage == Storage::Redundant {
        text = replace(
            text,
            "unpack(raw_zw)",
            "unpack(vec4u(raw_zw.x,raw_zw.y&32767u,0u,select(0u,16256u,(raw_zw.y&32768u)!=0u)))",
        );
    }
    text
}

#[test]
fn visibility_storage_variants_validate_and_lossless_fixture_packing_refuses_invalid() {
    for storage in [
        Storage::Original,
        Storage::Redundant,
        Storage::Rgba,
        Storage::Slots,
    ] {
        let mut texts = vec![source(storage), calibration(storage)];
        if storage != Storage::Original {
            texts.push(copy(storage));
        }
        for text in texts {
            let module = naga::front::wgsl::parse_str(&text).unwrap();
            naga::valid::Validator::new(
                naga::valid::ValidationFlags::all(),
                naga::valid::Capabilities::all(),
            )
            .validate(&module)
            .unwrap();
        }
    }
    // Independent bit-level reference, including owner epoch bit and endpoints.
    let encode = |z: f32, valid: f32, owner: u32| {
        (z.is_finite()
            && (0. ..=1.).contains(&z)
            && z.to_bits() >> 31 == 0
            && [0, 1f32.to_bits()].contains(&valid.to_bits()))
        .then_some([z.to_bits() | (u32::from(valid == 1.) << 31), owner])
    };
    for z in [0f32, f32::from_bits(1), f32::from_bits(0x3e36a324), 0.5, 1.] {
        for valid in [0., 1.] {
            for owner in [1, 2, 0x80000001, 0x80000002, u32::MAX] {
                let packed = encode(z, valid, owner).unwrap();
                assert_eq!(packed[0] & 0x7fffffff, z.to_bits());
                assert_eq!((packed[0] >> 31) as f32, valid);
                assert_eq!(packed[1], owner);
            }
        }
    }
    for z in [-0., -1., 1.001, f32::NAN, f32::INFINITY] {
        assert!(encode(z, 1., 1).is_none());
    }
    for valid in [-0., -1., 0.5, 2., f32::NAN] {
        assert!(encode(0.5, valid, 1).is_none());
    }
}

#[test]
fn visibility_slot_mapping_matches_compiled_fragment_interfaces() {
    for storage in [Storage::Rgba, Storage::Slots] {
        let mapping = storage.readback_slots();
        let mut sorted = mapping;
        sorted.sort_unstable();
        assert_eq!(sorted, [0, 1, 2, 3], "no duplicated or missing attachment");
        for (source, names) in [
            (
                source(storage),
                ["motion_xy", "motion_zw", "owner", "sample_position"],
            ),
            (
                calibration(storage),
                ["motion_xy", "motion_zw", "sample_id", "xy"],
            ),
        ] {
            let module = naga::front::wgsl::parse_str(&source).unwrap();
            let fragment = module
                .entry_points
                .iter()
                .find(|e| e.stage == naga::ShaderStage::Fragment)
                .unwrap();
            let result = fragment.function.result.as_ref().unwrap();
            let naga::TypeInner::Struct { members, .. } = &module.types[result.ty].inner else {
                panic!("fragment output must retain all semantic fields");
            };
            for (name, slot) in names.into_iter().zip(mapping) {
                let member = members
                    .iter()
                    .find(|m| m.name.as_deref() == Some(name))
                    .unwrap();
                let Some(naga::Binding::Location { location, .. }) = &member.binding else {
                    panic!("missing fragment color location");
                };
                assert_eq!(
                    *location as usize, slot,
                    "writer/reader mismatch for {name}"
                );
            }
        }
    }
}

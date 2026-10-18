//! Private three-plane discriminator. No independent duplicate-X witness exists.
//! Motion XY and sample XY remain exact; depth/validity and owner share 64 bits.
use super::storage::replace;

pub(super) fn source() -> String {
    let text = replace(
        super::source(),
        "@location(2) owner:vec2u, @location(3) sample_position:vec4u",
        "@location(2) sample_position:vec4u",
    );
    let text = replace(
        text,
        "pack(m.zw),vec2u(in.owner&65535u,in.owner>>16u)",
        "compact(m.zw,in.owner)",
    );
    format!("{text}\n{ENCODE}")
}

// Every finite depth in [0,1] has magnitude bits <= 0x3f800000: bit 30 is free.
// Store the original sign there (including -0), and binary validity in bit 31.
// Negative nonzero depth and nonbinary validity were never admitted by motion().
const ENCODE: &str = r#"
fn compact(v:vec2f,owner:u32)->vec4u {
    let b=bitcast<vec2u>(v);
    if !(v.x>=0.0 && v.x<=1.0) || (b.y!=0u && b.y!=1065353216u) {
        return vec4u(65535u,32767u,owner&65535u,owner>>16u);
    }
    let word=(b.x&1073741823u)|((b.x&2147483648u)>>1u)
        |select(0u,2147483648u,b.y==1065353216u);
    return vec4u(word&65535u,word>>16u,owner&65535u,owner>>16u);
}
"#;

pub(super) fn calibration() -> String {
    let text = replace(
        super::CALIBRATION.to_string(),
        "@location(2) sample_id:vec2u, @location(3) xy:vec4u",
        "@location(2) xy:vec4u",
    );
    let text = replace(
        text,
        "vec4u(0),vec4u(0),vec2u(i+1u,0u),pack(v.xy)",
        "vec4u(0),compact(vec2f(0),i+1u),pack(v.xy)",
    );
    format!("{text}\n{}\n{ENCODE}", super::PACK)
}

pub(super) const COPY: &str = r#"
@group(0) @binding(0) var motion_xy:texture_multisampled_2d<u32>;
@group(0) @binding(1) var depth_owner:texture_multisampled_2d<u32>;
@group(0) @binding(2) var position:texture_multisampled_2d<u32>;
@group(0) @binding(3) var depth:texture_depth_multisampled_2d;
struct Sample {motion:vec4f,position_depth:vec4f,owner:vec4u,raw_motion_zw:vec4u,raw_owner:vec4u}
@group(0) @binding(4) var<storage,read_write> output:array<Sample>;
fn unpack(v:vec4u)->vec2f {return bitcast<vec2f>(vec2u(v.x|(v.y<<16u),v.z|(v.w<<16u)));}
fn previous(v:vec4u)->vec2f {
    let word=v.x|(v.y<<16u);let magnitude=word&1073741823u;
    if magnitude>1065353216u || ((word&1073741824u)!=0u && magnitude!=0u) {
        return vec2f(bitcast<f32>(2143289344u),0.0);
    }
    let z=magnitude|((word&1073741824u)<<1u);
    return vec2f(bitcast<f32>(z),select(0.0,1.0,(word&2147483648u)!=0u));
}
@compute @workgroup_size(64) fn copy(@builtin(global_invocation_id) id:vec3u) {
    let i=id.x;if i>=16384u {return;}
    let pixel=i/4u;let s=i32(i%4u);let p=vec2i(vec2u(pixel%64u,pixel/64u));
    let v=textureLoad(depth_owner,p,s);let owner=v.z|(v.w<<16u);
    // raw_owner is explicitly absent, not a fabricated duplicate sample-X word.
    output[i]=Sample(vec4f(unpack(textureLoad(motion_xy,p,s)),previous(v)),
        vec4f(unpack(textureLoad(position,p,s)),textureLoad(depth,p,s),f32(s)),
        vec4u(owner,v.z,v.w,0u),v,vec4u(0u));
}
"#;

/// Independent host decoder for retained raw words; never admits GPU history.
pub(crate) fn decode(v: [u32; 4]) -> Option<([u32; 2], u32)> {
    if v.iter().any(|&lane| lane > u16::MAX as u32) {
        return None;
    }
    let word = v[0] | (v[1] << 16);
    let magnitude = word & 0x3fff_ffff;
    if magnitude > 1f32.to_bits() || (word & 0x4000_0000 != 0 && magnitude != 0) {
        return None;
    }
    Some((
        [
            magnitude | ((word & 0x4000_0000) << 1),
            if word >> 31 == 0 { 0 } else { 1f32.to_bits() },
        ],
        v[2] | (v[3] << 16),
    ))
}

#[test]
fn compact_visibility_preserves_signed_zero_depth_validity_and_full_owner() {
    let encode = |z: f32, valid: f32, owner: u32| {
        if !(0. ..=1.).contains(&z) || ![0, 1f32.to_bits()].contains(&valid.to_bits()) {
            return None;
        }
        let word = (z.to_bits() & 0x3fff_ffff)
            | ((z.to_bits() & 0x8000_0000) >> 1)
            | (u32::from(valid == 1.) << 31);
        Some([word & 65535, word >> 16, owner & 65535, owner >> 16])
    };
    // Fixed expected encodings catch a coupled encoder/decoder sign-bit error.
    for (z, valid, expected) in [
        (0., 0., [0, 0, 1, 32768]),
        (-0., 0., [0, 16384, 1, 32768]),
        (-0., 1., [0, 49152, 1, 32768]),
        (1., 1., [0, 49024, 1, 32768]),
    ] {
        assert_eq!(encode(z, valid, 0x8000_0001), Some(expected));
    }
    for z in [0., -0., f32::from_bits(1), f32::MIN_POSITIVE, 0.5, 1.] {
        for valid in [0., 1.] {
            for owner in [0, 1, 0x7fff_ffff, 0x8000_0001, u32::MAX] {
                let words = encode(z, valid, owner).unwrap();
                assert_eq!(decode(words), Some(([z.to_bits(), valid.to_bits()], owner)));
            }
        }
    }
    for z in [
        -f32::from_bits(1),
        -1.,
        f32::from_bits(1f32.to_bits() + 1),
        f32::NAN,
        f32::INFINITY,
    ] {
        assert!(encode(z, 1., 1).is_none());
    }
    for valid in [-0., -1., 0.5, 2., f32::NAN] {
        assert!(encode(0., valid, 1).is_none());
    }
    for invalid in [
        [1, 16384, 1, 0],
        [1, 16256, 1, 0],
        [65535, 32767, 1, 0],
        [65536, 0, 0, 0],
    ] {
        assert!(decode(invalid).is_none());
    }
}

#[test]
fn compact_visibility_shader_interfaces_validate_three_physical_planes() {
    for text in [source(), calibration(), COPY.into()] {
        let module = naga::front::wgsl::parse_str(&text).unwrap();
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .unwrap();
        if let Some(fragment) = module
            .entry_points
            .iter()
            .find(|e| e.stage == naga::ShaderStage::Fragment)
        {
            let result = fragment.function.result.as_ref().unwrap();
            let naga::TypeInner::Struct { members, .. } = &module.types[result.ty].inner else {
                panic!("MRT output");
            };
            let locations: Vec<_> = members
                .iter()
                .map(|m| match &m.binding {
                    Some(naga::Binding::Location { location, .. }) => *location,
                    _ => panic!("color attachment"),
                })
                .collect();
            assert_eq!(locations, [0, 1, 2]);
        }
    }
}

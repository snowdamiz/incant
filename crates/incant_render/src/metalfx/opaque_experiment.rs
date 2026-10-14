//! Private, explicit scene-linear opaque-domain experiment; never a public mode.
use super::MetalFxError;
pub(crate) const BACKGROUND: [f32; 3] = [0.02, 0.03, 0.05];
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Domain {
    pub background: [f32; 3],
    pub stable_background: bool,
}
impl Domain {
    pub fn validate(self) -> Result<(), MetalFxError> {
        if self
            .background
            .iter()
            .any(|v| !v.is_finite() || !(0.0..=65504.0).contains(v))
        {
            return Err(MetalFxError::Config);
        }
        Ok(())
    }
    pub fn uniform(self) -> [f32; 4] {
        [
            self.background[0],
            self.background[1],
            self.background[2],
            f32::from(u8::from(self.stable_background)),
        ]
    }
}
pub(crate) fn source(normalize: bool) -> String {
    let body = if normalize {
        include_str!("opaque_normalize.wgsl").to_owned()
    } else {
        let original = include_str!("prepare.wgsl");
        let at = "    if !admitted(c) || c.a!=1.0 || !(z>=0.0 && z<1.0) {return out;}";
        assert_eq!(original.matches(at).count(), 1);
        original.replace(
            at,
            &format!("{}\n{at}", include_str!("opaque_prepare.wgsl")),
        )
    };
    format!(
        "{}\n{}\n{}\n{body}",
        include_str!("common.wgsl"),
        include_str!("alpha.wgsl"),
        include_str!("opaque_common.wgsl")
    )
}
pub(crate) fn output_source(background: [f32; 3]) -> String {
    Domain {
        background,
        stable_background: false,
    }
    .validate()
    .unwrap();
    let original = include_str!("../output.wgsl");
    let at = "    let coverage = clamp(hdr.a,0.0,1.0);";
    assert_eq!(original.matches(at).count(), 1);
    // Only the known opaque/MASK fixture uses this output. Complete the scene
    // domain before the scene transform; native output already has alpha one.
    let replacement = format!(
        "    hdr = vec4f(hdr.rgb + vec3f({:?},{:?},{:?})*(1.0-hdr.a),1.0);\n{at}",
        background[0], background[1], background[2]
    );
    format!(
        "{}\n{}",
        original.replace(at, &replacement),
        include_str!("../tone_map.wgsl")
    )
}
#[test]
fn opaque_shaders_validate_and_background_is_pre_transform() {
    for s in [source(false), source(true), output_source(BACKGROUND)] {
        let m = naga::front::wgsl::parse_str(&s).unwrap();
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        )
        .validate(&m)
        .unwrap();
    }
    let s = output_source(BACKGROUND);
    assert!(s.find("hdr = vec4f(hdr.rgb +").unwrap() < s.find("tone_map(radiance)").unwrap());
    for bad in [f32::NAN, f32::INFINITY, -1., 65505.] {
        assert!(
            Domain {
                background: [bad, 0., 0.],
                stable_background: true
            }
            .validate()
            .is_err()
        );
    }
}

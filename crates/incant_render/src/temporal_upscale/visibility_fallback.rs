//! GPU-only missing-sample classification; original witness thresholds unchanged.
use super::visibility_reconstruction;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mode {
    Missing,
    AllCurrent,
    /// Same current filtering/fallback as Missing, with no history contribution.
    NoHistory,
    /// Retained negative arm: owner/depth alone do not establish radiance continuity.
    UncheckedHistory,
}
impl Mode {
    pub(crate) fn admits_history(self, previous: bool, stationary: bool) -> bool {
        previous
            && match self {
                Self::Missing => stationary,
                Self::UncheckedHistory => true,
                Self::AllCurrent | Self::NoHistory => false,
            }
    }
}
pub(crate) struct Classification {
    pub pipeline: wgpu::RenderPipeline,
    pub group: wgpu::BindGroup,
    pub mask: wgpu::TextureView,
}
const GUARD: &str = r#"
    let packed=textureLoad(visible_owner,pixel,lane);
    let id=(packed.x|(packed.y<<16u))&2147483647u;
    if id==0u {continue;}
    let position=unpack(textureLoad(visible_position,pixel,lane));
    let depth=textureLoad(visible_depth,pixel,lane);
    if bitcast<u32>(position.x)!=(packed.z|(packed.w<<16u))
        || !finite4(vec4f(position,depth,0)) || !(depth>=0.0 && depth<1.0)
        || any(position<vec2f(pixel)) || any(position>=vec2f(pixel)+vec2f(1)) {continue;}
"#;
pub(crate) fn classify_source(mode: Mode) -> String {
    let base = radiance_source(mode);
    let helpers = base.split_once("struct Resolved").unwrap().0;
    let decision = if mode == Mode::AllCurrent {
        "mask|=1u<<u32(lane);"
    } else {
        r#"let c=current_witness(position,id,depth);let h=history_witness(position,id,m);
        if c.mass>0.0 {mask|=1u<<u32(lane+4);}
        if h.mass>0.0 {mask|=1u<<u32(lane+8);}
        if c.mass==0.0 && h.mass==0.0 {mask|=1u<<u32(lane);}"#
    };
    format!(
        r#"{helpers}
@fragment fn classify(@builtin(position) p:vec4f)->@location(0) u32 {{
    // Low four bits remain the actual shading requests. Spare bits retain
    // current/history witness decisions and the accepted continuity policy;
    // no additional attachment or CPU admission is introduced.
    let pixel=vec2i(p.xy);var mask=select(0u,65536u,frame.output.z>0.5);
    for(var lane=0;lane<4;lane++) {{
        {GUARD}
        let m=vec4f(unpack(textureLoad(visible_xy,pixel,lane)),unpack(textureLoad(visible_zw,pixel,lane)));
        {decision}
    }}
    return mask;
}}
"#
    )
}
fn radiance_source(mode: Mode) -> String {
    let source = visibility_reconstruction::source();
    if mode == Mode::UncheckedHistory {
        return source;
    }
    // A geometric match establishes visibility/correspondence, not unchanged
    // illumination. The existing accepted snapshot includes packed light world
    // transforms, camera and all geometry; material/environment bindings are
    // immutable for this retained route. Conservatively reject all old radiance
    // on any discontinuity. Only accepted submissions publish the snapshot.
    let old = "if frame.depth.w<0.5 || !valid_motion(m)";
    assert_eq!(source.matches(old).count(), 1);
    source.replace(
        old,
        "if frame.output.z<0.5 || frame.depth.w<0.5 || !valid_motion(m)",
    )
}
pub(crate) fn resolve_source(mode: Mode) -> String {
    let mut source = radiance_source(mode);
    source.push_str(
        r#"
@group(0) @binding(13) var missing_mask:texture_2d<u32>;
@group(0) @binding(14) var current_fallback:texture_multisampled_2d<f32>;
@group(0) @binding(15) var fallback_depth:texture_depth_multisampled_2d;
"#,
    );
    let old = "else { complete=false;continue; } // Never-observed radiance stays missing.";
    assert_eq!(source.matches(old).count(), 1);
    source = source.replace(
        old,
        r#"else {
            let fallback=textureLoad(current_fallback,pixel,lane);
            if (textureLoad(missing_mask,pixel,0).x&(1u<<u32(lane)))==0u
                || !finite4(fallback) || fallback.a!=1.0
                || textureLoad(fallback_depth,pixel,lane)!=depth {
                complete=false;continue;
            }
            color=fallback.rgb;
        }"#,
    );
    if mode == Mode::AllCurrent {
        // Independent current-frame reference: no low or history radiance read.
        assert_eq!(
            source
                .matches(
                    "let c=current_witness(position,id,depth);let h=history_witness(position,id,m);"
                )
                .count(),
            1
        );
        source = source.replace(
            "let c=current_witness(position,id,depth);let h=history_witness(position,id,m);",
            "let c=Witness(vec3f(0),0.0,vec3f(0),8.0,vec3f(0));let h=c;",
        );
    }
    source
}
impl Classification {
    pub(crate) fn encode(&self, e: &mut wgpu::CommandEncoder) {
        let mut p = e.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Missing current/history radiance mask"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.mask,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        p.set_pipeline(&self.pipeline);
        p.set_bind_group(0, &self.group, &[]);
        p.draw(0..3, 0..1);
    }
}

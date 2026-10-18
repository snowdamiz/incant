//! GPU-only missing-sample classification; original witness thresholds unchanged.
use super::visibility_reconstruction;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mode {
    Missing,
    AllCurrent,
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
    let base = visibility_reconstruction::source();
    let helpers = base.split_once("struct Resolved").unwrap().0;
    let predicate = if mode == Mode::AllCurrent {
        "true"
    } else {
        "current_witness(position,id,depth).mass==0.0 && history_witness(position,id,m).mass==0.0"
    };
    format!(
        r#"{helpers}
@fragment fn classify(@builtin(position) p:vec4f)->@location(0) u32 {{
    let pixel=vec2i(p.xy);var mask=0u;
    for(var lane=0;lane<4;lane++) {{
        {GUARD}
        let m=vec4f(unpack(textureLoad(visible_xy,pixel,lane)),unpack(textureLoad(visible_zw,pixel,lane)));
        if {predicate} {{mask|=1u<<u32(lane);}}
    }}
    return mask;
}}
"#
    )
}
pub(crate) fn resolve_source(mode: Mode) -> String {
    let mut source = visibility_reconstruction::source();
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

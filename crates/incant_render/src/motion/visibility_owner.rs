//! Four-arm owner/depth diagnosis. Original shader and geometry oracle stay fixed.
use super::{MotionDraw, msl};
use serde_json::{Value, json};
use std::{
    io::Write,
    path::{Path, PathBuf},
    sync::Mutex,
};

pub(super) struct Arm {
    pub root: PathBuf,
    pub explicit: bool,
    pub reverse: bool,
    sequence: Mutex<u32>,
}
pub(super) fn write(root: &Path, name: &str, value: &Value) {
    let bytes = serde_json::to_vec_pretty(value).unwrap();
    assert!(bytes.len() <= 1024 * 1024);
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join(name))
        .unwrap()
        .write_all(&bytes)
        .unwrap();
}
impl Arm {
    pub fn new(root: &Path, explicit: bool, reverse: bool) -> Self {
        Self {
            root: root.into(),
            explicit,
            reverse,
            sequence: Mutex::new(0),
        }
    }
    pub fn record(&self, draw: &MotionDraw, uniform: &[u8]) {
        let mut sequence = self.sequence.lock().unwrap();
        assert!(*sequence < 11, "bounded eleven accepted frames per arm");
        let order: Vec<_> = (0..draw.draws.len()).map(|ordinal| {
            let index = if self.reverse { draw.draws.len()-1-ordinal } else { ordinal };
            let item = &draw.draws[index];
            let b = &item.batch;
            let primitive = &b.model.primitives[b.primitive];
            json!({"ordinal":ordinal,"original_draw_index":index,"primitive":b.primitive,
                "nodes":b.placements.iter().map(|p|p.node).collect::<Vec<_>>(),
                "instances":b.count,"index_count":primitive.index_count,"mirrored":b.mirrored,
                "current_instances_handle":format!("{:?}",b.instances),
                "previous_instances_handle":format!("{:?}",item.previous_instances),
                "current_vertices_handle":format!("{:?}",b.vertices.as_ref().unwrap_or(&primitive.vertices)),
                "previous_vertices_handle":format!("{:?}",item.previous_vertices),
                "indices_handle":format!("{:?}",primitive.indices)})
        }).collect();
        write(
            &self.root,
            &format!("draw-{}.json", *sequence),
            &json!({
            "accepted_frame_index":*sequence,"uniform_actual_bytes":uniform,"draw_order":order,
            "reverse":self.reverse,"explicit_sample_index":self.explicit,
            "recorded_before_encode":true,"depth_clear":1,"depth_compare":"Less","depth_write":true,
            "color_load":"clear zero","color_store":"store","resolve":false,"viewport":[0,0,64,64]}),
        );
        *sequence += 1;
    }
}
pub(super) fn source(explicit: bool) -> String {
    let mut text = super::source();
    if explicit {
        for (from, to) in [
            (
                "fn fragment(in:Output)",
                "fn fragment(in:Output,@builtin(sample_index) actual_sample:u32)",
            ),
            ("pack(m.zw)", "(pack(m.zw)|vec4u(0u,0u,actual_sample,0u))"),
        ] {
            assert_eq!(
                text.matches(from).count(),
                1,
                "explicit sample anchor changed"
            );
            text = text.replace(from, to);
        }
    }
    text
}
pub(super) fn copy(explicit: bool) -> String {
    let mut text = super::readback::COPY.to_string();
    for (from, to) in [
        (
            "let o=textureLoad(owner,p,s).xy;",
            "let o=textureLoad(owner,p,s).xy;let raw_zw=textureLoad(motion_zw,p,s);",
        ),
        (
            "unpack(textureLoad(motion_zw,p,s))",
            if explicit {
                "unpack(raw_zw & vec4u(65535u,65535u,65532u,65535u))"
            } else {
                "unpack(raw_zw)"
            },
        ),
        (
            "vec4u(o.x|(o.y<<16u),0u,0u,0u)",
            "vec4u(o.x|(o.y<<16u),o.x,o.y,raw_zw.z)",
        ),
    ] {
        assert_eq!(
            text.matches(from).count(),
            1,
            "diagnostic readback anchor changed"
        );
        text = text.replace(from, to);
    }
    text
}
pub(super) fn pipeline(
    d: &wgpu::Device,
    desc: &wgpu::RenderPipelineDescriptor<'_>,
    arm: Option<&Arm>,
    index: usize,
    kind: &str,
    scene: &str,
) -> wgpu::RenderPipeline {
    let (pipeline, msl) = msl::capture(
        arm.is_some(),
        &[("vertex", "Vertex"), ("fragment", "Fragment")],
        || d.create_render_pipeline(desc),
    );
    if let Some(arm) = arm {
        let source = if kind == "calibration" {
            format!("{}\n{}", super::CALIBRATION, super::PACK)
        } else {
            scene.to_string()
        };
        write(
            &arm.root,
            &format!("pipeline-{kind}-{index}.json"),
            &json!({"WGSL":source,"MSL":msl,
            "vertex_buffers":format!("{:?}",desc.vertex.buffers),"primitive":format!("{:?}",desc.primitive),
            "targets":format!("{:?}",desc.fragment.as_ref().unwrap().targets),
            "depth":format!("{:?}",desc.depth_stencil),"multisample":format!("{:?}",desc.multisample),
                "layout_handle":format!("{:?}",desc.layout),
                "layout_groups":if kind=="scene" {vec!["frame uniform vertex+fragment","existing material"]} else {vec![]},
                "cache":desc.cache.is_some()}),
        );
        assert_eq!(
            msl["complete"], true,
            "actual HAL MSL required before drawing"
        );
    }
    pipeline
}
#[test]
fn visibility_owner_variants_validate_without_narrowing_original_motion() {
    assert_eq!(source(false), super::source());
    for explicit in [false, true] {
        for text in [source(explicit), copy(explicit)] {
            let module = naga::front::wgsl::parse_str(&text).unwrap();
            naga::valid::Validator::new(
                naga::valid::ValidationFlags::all(),
                naga::valid::Capabilities::all(),
            )
            .validate(&module)
            .unwrap();
        }
        for valid in [0f32, 1f32] {
            assert_eq!(valid.to_bits() & 65535, 0);
            for lane in 0..4u32 {
                let packed = valid.to_bits() | lane;
                assert_eq!(f32::from_bits(packed & !3), valid);
                assert_eq!(packed & 3, lane);
            }
        }
    }
}

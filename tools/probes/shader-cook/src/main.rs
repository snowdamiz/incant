use naga::{
    AddressSpace, ArraySize, ShaderStage, TypeInner,
    back::{msl, spv},
    valid::{Capabilities, ValidationFlags, Validator},
};
use serde_json::json;
use std::{fs, path::Path};
mod hlsl;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::args().nth(1).expect("repository source path");
    let out = std::env::args().nth(2).expect("new output path");
    fs::create_dir(&out)?;
    let root = Path::new(&root).join("crates/incant_render/src");
    let mut rows = Vec::new();
    for (family, files) in [
        ("diagnostic", vec!["fixture.wgsl"]),
        (
            "material",
            vec![
                "shadows/shade.wgsl",
                "lighting/shade.wgsl",
                "model_material.wgsl",
            ],
        ),
        ("shadow", vec!["shadows/caster.wgsl"]),
        ("sprite", vec!["sprites/sprite.wgsl"]),
        ("grid", vec!["lighting/grid.wgsl"]),
        ("output", vec!["output.wgsl", "tone_map.wgsl"]),
    ] {
        let source = files
            .iter()
            .map(|p| fs::read_to_string(root.join(p)))
            .collect::<Result<Vec<_>, _>>()?
            .join("\n");
        let module = naga::front::wgsl::parse_str(&source)?;
        let info = Validator::new(ValidationFlags::all(), Capabilities::all()).validate(&module)?;
        for ep in &module.entry_points {
            let (m, i) = naga::back::pipeline_constants::process_overrides(
                &module,
                &info,
                Some((ep.stage, ep.name.as_str())),
                &Default::default(),
            )?;
            let mut resources = msl::EntryPointResources::default();
            let (mut buffers, mut textures, mut samplers) = (0, 0, 0);
            let mut bound: Vec<_> = m
                .global_variables
                .iter()
                .filter_map(|(_, v)| v.binding.map(|b| (b, v)))
                .collect();
            bound.sort_by_key(|(b, _)| *b);
            for (binding, v) in bound {
                let target = match v.space {
                    AddressSpace::Uniform | AddressSpace::Storage { .. } => {
                        let slot = buffers;
                        buffers += 1;
                        msl::BindTarget {
                            buffer: Some(slot),
                            mutable: matches!(v.space, AddressSpace::Storage { access } if access.contains(naga::StorageAccess::STORE)),
                            ..Default::default()
                        }
                    }
                    AddressSpace::Handle => match m.types[v.ty].inner {
                        TypeInner::Image { .. } => {
                            let slot = textures;
                            textures += 1;
                            msl::BindTarget {
                                texture: Some(slot),
                                ..Default::default()
                            }
                        }
                        TypeInner::Sampler { .. } => {
                            let slot = samplers;
                            samplers += 1;
                            msl::BindTarget {
                                sampler: Some(msl::BindSamplerTarget::Resource(slot)),
                                ..Default::default()
                            }
                        }
                        _ => panic!("unhandled resource"),
                    },
                    _ => panic!("unhandled resource address space"),
                };
                resources.resources.insert(binding, target);
            }
            resources.sizes_buffer = Some(buffers);
            let mut dynamic = Vec::new();
            let mut workgroup = Vec::new();
            for (_, v) in m.global_variables.iter() {
                let ty = match &m.types[v.ty].inner {
                    TypeInner::Struct { members, .. } => members.last().unwrap().ty,
                    _ => v.ty,
                };
                if matches!(
                    m.types[ty].inner,
                    TypeInner::Array {
                        size: ArraySize::Dynamic,
                        ..
                    }
                ) {
                    let b = v.binding.unwrap();
                    dynamic.push(json!({"name":v.name,"group":b.group,"binding":b.binding}));
                }
                if v.space == AddressSpace::WorkGroup {
                    workgroup
                        .push(json!({"name":v.name,"bytes":m.types[v.ty].inner.size(m.to_ctx())}));
                }
            }
            let opts = msl::Options {
                lang_version: (2, 4),
                fake_missing_bindings: false,
                per_entry_point_map: [(ep.name.clone(), resources)].into(),
                bounds_check_policies: naga::proc::BoundsCheckPolicies {
                    index: naga::proc::BoundsCheckPolicy::Restrict,
                    buffer: naga::proc::BoundsCheckPolicy::Restrict,
                    image_load: naga::proc::BoundsCheckPolicy::Restrict,
                    binding_array: naga::proc::BoundsCheckPolicy::Unchecked,
                },
                ..Default::default()
            };
            let pipeline = msl::PipelineOptions {
                entry_point: Some((ep.stage, ep.name.clone())),
                vertex_pulling_transform: false,
                ..Default::default()
            };
            let (msl, translation) = msl::write_string(&m, &i, &opts, &pipeline)?;
            for r in translation.entry_point_names {
                r?;
            }
            let spv = spv::write_vec(
                &m,
                &i,
                &spv::Options {
                    // Keep this probe's output independent of Cargo's profile.
                    // Naga's default conditionally adds DEBUG in debug builds.
                    flags: spv::WriterFlags::ADJUST_COORDINATE_SPACE
                        | spv::WriterFlags::LABEL_VARYINGS
                        | spv::WriterFlags::CLAMP_FRAG_DEPTH
                        | spv::WriterFlags::DEBUG,
                    ..Default::default()
                },
                Some(&spv::PipelineOptions {
                    shader_stage: ep.stage,
                    entry_point: ep.name.clone(),
                }),
            )?;
            let stem = format!("{family}-{}", ep.name);
            let (hlsl, hlsl_entry) = hlsl::translate(&m, &i, &module, ep.stage, &ep.name)?;
            fs::write(Path::new(&out).join(format!("{stem}.hlsl")), &hlsl)?;
            fs::write(Path::new(&out).join(format!("{stem}.metal")), &msl)?;
            fs::write(
                Path::new(&out).join(format!("{stem}.spv")),
                spv.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<_>>(),
            )?;
            rows.push(json!({"family":family,"entry":ep.name,"stage":format!("{:?}",ep.stage),"workgroup_size":ep.workgroup_size,"runtime_buffers":dynamic,"workgroup_memory":workgroup,"msl_bytes":msl.len(),"spv_bytes":spv.len()*4,"hlsl_bytes":hlsl.len(),"hlsl_entry":hlsl_entry,"vertex_stage":ep.stage==ShaderStage::Vertex}));
        }
    }
    let report = json!({"naga":"29.0.4","scope":"CPU translation and metadata reflection only; compact probe bindings are not the engine pipeline ABI; vertex pulling omitted; no runtime, device, appearance or shipping claim","entry_points":rows});
    fs::write(
        Path::new(&out).join("report.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

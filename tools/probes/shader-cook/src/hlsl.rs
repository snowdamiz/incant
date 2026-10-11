//! Compiler feasibility only. These explicit probe registers do not describe
//! wgpu's root signature or sampler heaps and must not be loaded by the engine.
use naga::{Module, ShaderStage, TypeInner, back::hlsl, valid::ModuleInfo};

pub fn translate(
    module: &Module,
    info: &ModuleInfo,
    full_module: &Module,
    stage: ShaderStage,
    entry: &str,
) -> Result<(String, String), Box<dyn std::error::Error>> {
    let mut options = hlsl::Options {
        shader_model: hlsl::ShaderModel::V6_0,
        fake_missing_bindings: false,
        special_constants_binding: Some(hlsl::BindTarget {
            space: 254,
            register: 0,
            ..Default::default()
        }),
        ..Default::default()
    };
    for (_, global) in module.global_variables.iter() {
        let Some(binding) = global.binding else {
            continue;
        };
        let space = u8::try_from(binding.group)?;
        if space >= 254 {
            return Err("probe reserves register spaces 254 and 255".into());
        }
        options.binding_map.insert(
            binding,
            hlsl::BindTarget {
                space,
                register: binding.binding,
                restrict_indexing: true,
                ..Default::default()
            },
        );
        if matches!(module.types[global.ty].inner, TypeInner::Sampler { .. }) {
            options.sampler_buffer_binding_map.insert(
                hlsl::SamplerIndexBufferKey {
                    group: binding.group,
                },
                hlsl::BindTarget {
                    space: 254,
                    register: binding.group + 1,
                    ..Default::default()
                },
            );
        }
    }
    let pipeline = hlsl::PipelineOptions {
        entry_point: Some((stage, entry.to_owned())),
    };
    // Link a vertex stage's output interface to its paired fragment inputs.
    let fragment = full_module
        .entry_points
        .iter()
        .find(|ep| ep.stage == ShaderStage::Fragment)
        .and_then(|ep| hlsl::FragmentEntryPoint::new(full_module, &ep.name));
    let mut text = String::new();
    let reflection =
        hlsl::Writer::new(&mut text, &options, &pipeline).write(module, info, fragment.as_ref())?;
    let names: Vec<_> = reflection
        .entry_point_names
        .into_iter()
        .collect::<Result<_, _>>()?;
    if names.len() != 1 {
        return Err("expected one compacted HLSL entry point".into());
    }
    Ok((text, names.into_iter().next().unwrap()))
}

use incant_shader_cook::{Backend, compute};
const SOURCE: &str = include_str!("../../incant_render/src/lighting/grid.wgsl");

#[test]
fn fixed_cluster_artifacts_are_deterministic_and_keep_exact_resource_interfaces() {
    let a = compute::cook(SOURCE).unwrap();
    assert_eq!(a, compute::cook(SOURCE).unwrap());
    assert_eq!(a.artifacts.len(), 3);
    for a in &a.artifacts {
        assert_eq!(a.entry, "build");
        if a.backend == Backend::Vulkan {
            let m = naga::front::spv::parse_u8_slice(
                &a.bytes,
                &naga::front::spv::Options {
                    adjust_coordinate_space: false,
                    strict_capabilities: true,
                    block_ctx_dump_prefix: None,
                },
            )
            .unwrap();
            // Structural reader only: Naga29 upgrades the membership array to
            // atomic elements but not its aggregate null store. That reader IR
            // limitation is pinned below; emitted Vulkan words are independently
            // checked by spirv-val in the offline validation receipt.
            assert_eq!(m.entry_points.len(), 1);
            assert_eq!(m.entry_points[0].workgroup_size, [64, 1, 1]);
            let mut bindings: Vec<_> = m
                .global_variables
                .iter()
                .filter_map(|(_, g)| g.binding)
                .map(|b| (b.group, b.binding))
                .collect();
            bindings.sort_unstable();
            assert_eq!(bindings, [(0, 0), (0, 1), (0, 2), (0, 3)]);
        } else {
            let text = std::str::from_utf8(&a.bytes).unwrap();
            if a.backend == Backend::Metal {
                for token in [
                    "[[buffer(4)]]",
                    "threadgroup metal::atomic_uint hits;",
                    "threadgroup type_7 membership;",
                    "metal::atomic_uint inner[128];",
                    "(_buffer_sizes.size1 - 0 - 64) / 64",
                    "(_buffer_sizes.size2 - 0 - 4) / 4",
                    "(_buffer_sizes.size3 - 0 - 4) / 4",
                    "metal::atomic_fetch_or_explicit",
                    "loop_bound",
                ] {
                    assert!(text.contains(token), "missing {token}")
                }
                assert!(!text.contains("threadgroup metal::atomic_uint&"));
                assert!(!text.contains("[[threadgroup("));
                assert_eq!(text.matches("metal::threadgroup_barrier").count(), 3);
                assert!(
                    text.find("threadgroup type_7 membership;").unwrap()
                        < text.find("metal::atomic_store_explicit").unwrap()
                );
                let lengths = text
                    .split("struct _mslBufferSizes {")
                    .nth(1)
                    .unwrap()
                    .split("};")
                    .next()
                    .unwrap();
                assert_eq!(
                    lengths
                        .lines()
                        .filter(|line| line.trim().starts_with("uint "))
                        .count(),
                    3
                );
            } else {
                for token in [
                    "register(b0)",
                    "register(t0)",
                    "register(u0)",
                    "register(u1)",
                    "[numthreads(64, 1, 1)]",
                    "groupshared",
                    "GroupMemoryBarrierWithGroupSync",
                    "lights.Load",
                    "counts.Store",
                    "masks.Store",
                ] {
                    assert!(text.contains(token), "missing {token}\n{text}")
                }
            }
        }
    }
    if let Some(path) = std::env::var_os("INCANT_CLUSTER_COOK_EVIDENCE") {
        compute::write_bundle(&a, std::path::Path::new(&path)).unwrap()
    }
}

#[test]
fn cluster_refuses_unknown_workgroup_storage_resource_and_invocation_contracts() {
    for changed in [
        SOURCE.replace("@workgroup_size(64)", "@workgroup_size(32)"),
        SOURCE.replace("atomic<u32>,128", "atomic<u32>,129"),
        SOURCE.replace("@binding(3)", "@binding(4)"),
        SOURCE.replace("var<storage,read> lights", "var<storage,read_write> lights"),
        SOURCE.replace("position_range:vec4f", "@size(32) position_range:vec4f"),
        SOURCE.replace("view:mat4x4f", "@size(80) view:mat4x4f"),
        SOURCE.replace("@builtin(workgroup_id)", "@builtin(global_invocation_id)"),
        SOURCE.replace("fn build(", "fn other("),
        format!("{SOURCE}\noverride additional:u32=2u;\n"),
    ] {
        assert_ne!(changed, SOURCE);
        assert!(compute::cook(&changed).is_err(), "accepted {changed}")
    }
}

#[test]
fn naga_reader_atomic_aggregate_limitation_is_explicit() {
    let b = compute::cook(SOURCE).unwrap();
    let a = b
        .artifacts
        .iter()
        .find(|a| a.backend == Backend::Vulkan)
        .unwrap();
    let m = naga::front::spv::parse_u8_slice(
        &a.bytes,
        &naga::front::spv::Options {
            adjust_coordinate_space: false,
            strict_capabilities: true,
            block_ctx_dump_prefix: None,
        },
    )
    .unwrap();
    let e = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&m)
    .unwrap_err();
    let naga::valid::ValidationError::Function {
        handle,
        source: naga::valid::FunctionError::InvalidStoreTypes { pointer, value },
        ..
    } = e.as_inner()
    else {
        panic!("unexpected reader failure: {e:?}")
    };
    let f = &m.functions[*handle];
    let naga::Expression::GlobalVariable(global) = f.expressions[*pointer] else {
        panic!("not the known global store")
    };
    assert!(
        m.global_variables[global]
            .name
            .as_deref()
            .is_none_or(|name| name == "membership")
    );
    assert_eq!(
        m.global_variables[global].space,
        naga::AddressSpace::WorkGroup
    );
    assert_eq!(m.global_variables.iter().filter(|(_,g)| g.space==naga::AddressSpace::WorkGroup && matches!(m.types[g.ty].inner,naga::TypeInner::Array{size:naga::ArraySize::Constant(n),..} if n.get()==128)).count(),1);
    let naga::Expression::Constant(constant) = f.expressions[*value] else {
        panic!("not the aggregate null value")
    };
    let naga::TypeInner::Array {
        base: destination,
        size: dn,
        stride: 4,
    } = m.types[m.global_variables[global].ty].inner
    else {
        panic!("destination array")
    };
    let naga::TypeInner::Array {
        base: source,
        size: sn,
        stride: 4,
    } = m.types[m.constants[constant].ty].inner
    else {
        panic!("source array")
    };
    assert_eq!(dn, sn);
    assert!(matches!(dn,naga::ArraySize::Constant(n) if n.get()==128));
    let scalar = naga::Scalar {
        kind: naga::ScalarKind::Uint,
        width: 4,
    };
    assert_eq!(m.types[destination].inner, naga::TypeInner::Atomic(scalar));
    assert_eq!(m.types[source].inner, naga::TypeInner::Scalar(scalar));
}

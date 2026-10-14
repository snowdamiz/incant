//! Guarded, opt-in test assembly only. Production shaders and history ABI stay unchanged.
use super::*;

#[derive(Clone, Copy, Debug)]
pub(super) enum Coverage {
    Alpha,
    Provenance,
}
impl Coverage {
    pub(super) fn selected() -> Self {
        match std::env::var("INCANT_OWNER_CAMERA_COVERAGE").as_deref() {
            Err(std::env::VarError::NotPresent) | Ok("alpha") => Self::Alpha,
            Ok("provenance") => Self::Provenance,
            Ok(other) => panic!("unknown diagnostic coverage: {other}"),
            Err(error) => panic!("invalid diagnostic coverage: {error}"),
        }
    }
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Alpha => "legacy-alpha",
            Self::Provenance => "explicit-partial-bit17",
        }
    }
    pub(super) fn assemble(self, mut shader: String) -> String {
        if matches!(self, Self::Alpha) {
            return shader;
        }
        for (before, after, count) in [
            (
                "struct Stored { a:u32, b:u32, m:u32, over:bool, ok:bool }",
                "struct Stored { a:u32, b:u32, m:u32, over:bool, ok:bool, partial:bool }",
                1,
            ),
            (
                "Stored(v.x&ID_BITS,0u,0u,false,false)",
                "Stored(v.x&ID_BITS,0u,0u,false,false,(v.x&PARTIAL_COVERAGE)!=0u)",
                1,
            ),
            (
                "(v.x&~(HIGH_BIT|ID_BITS))",
                "(v.x&~(HIGH_BIT|ID_BITS|PARTIAL_COVERAGE))",
                1,
            ),
            ("(e.surface || c.a<1.0)", "(e.surface || r.partial)", 1),
            (
                "let persist=admitted(c) && c.a>0.0 && r.ok",
                "let persist=admitted(c) && c.a>0.0 && r.ok && (r.partial || c.a==1.0)",
                1,
            ),
            (
                "if e.opaque && c.a==1.0 {result.color.a=1.0;}",
                "if e.opaque && (w==0.0 || !r.partial) && c.a==1.0 {result.color.a=1.0;}",
                1,
            ),
            (
                "result.record=encode(carried(r,e.owners));",
                "result.record=coverage_encode(carried(r,e.owners),(w>0.0 && r.partial) || (e.mass>0.0 && !e.opaque));",
                1,
            ),
            (
                "result.record=encode(e.owners);",
                "result.record=coverage_encode(e.owners,e.mass>0.0 && !e.opaque);",
                2,
            ),
            (
                "if !admitted(c) || c.a<1.0 || !compatible(z,expected)",
                "if !admitted(c) || c.a<1.0 || r.partial || !compatible(z,expected)",
                1,
            ),
            (
                "if !admitted(c) || !(c.a>0.0) || c.a>=1.0 || !(evidence.x>0.0)",
                "if !admitted(c) || !(c.a>0.0) || !r.partial || !(evidence.x>0.0)",
                1,
            ),
            (
                "result.record=encode(owner_set(selected));",
                "result.record=coverage_encode(owner_set(selected),true);",
                1,
            ),
        ] {
            assert_eq!(
                shader.matches(before).count(),
                count,
                "coverage splice: {before}"
            );
            shader = shader.replace(before, after);
        }
        let cubic = "|| changed_id(record.a) || !(color.a>0.0 && color.a<1.0)";
        if shader.contains("fn cubic_history(") {
            assert_eq!(shader.matches(cubic).count(), 1);
            shader = shader.replace(
                cubic,
                "|| changed_id(record.a) || !(color.a>0.0) || !record.partial",
            );
        } else {
            assert!(!shader.contains(cubic));
        }
        shader.push('\n');
        shader.push_str(include_str!("upscaling/owner_coverage_experiment.wgsl"));
        shader
    }
}

pub(super) fn labeled_nodes(stage: &Stage, input: &SceneInput) -> Vec<serde_json::Value> {
    let rows: Vec<_> = input
        .nodes
        .iter()
        .map(|n| {
            let entity = stage
                .f
                .project
                .scenes
                .values()
                .flat_map(|s| s.entities.values())
                .find(|e| e.id == n.id)
                .expect("authored diagnostic node");
            json!({"role":entity.name,"world":n.world.to_cols_array()})
        })
        .collect();
    let names: std::collections::BTreeSet<_> =
        rows.iter().map(|r| r["role"].as_str().unwrap()).collect();
    assert_eq!(
        names.len(),
        rows.len(),
        "semantic fixture roles must be unique"
    );
    rows
}

#[test]
fn explicit_coverage_shader_contracts_and_semantic_roles() {
    for radius in [2, 3] {
        for kernel in [Kernel::Point, Kernel::Footprint] {
            for history in [History::Tent, History::Cubic] {
                let source =
                    Coverage::Provenance.assemble(shader_with_options(radius, kernel, history));
                let module = naga::front::wgsl::parse_str(&source).unwrap();
                naga::valid::Validator::new(
                    naga::valid::ValidationFlags::all(),
                    naga::valid::Capabilities::all(),
                )
                .validate(&module)
                .unwrap();
            }
        }
    }
    let a = Stage::new(true);
    let b = Stage::new(true);
    let names = |stage: &Stage| {
        let mut names: Vec<_> = labeled_nodes(stage, &stage.input())
            .iter()
            .map(|r| r["role"].as_str().unwrap().to_owned())
            .collect();
        names.sort();
        names
    };
    assert_eq!(names(&a), names(&b));
}

#[test]
fn coverage_word_contract_and_disabled_shader_identity() {
    // Independent integer wire-layout check, not an opacity threshold. The bit
    // occupies the first unused primary-word position above the 17-bit ID.
    let admit = |word: u32| {
        let owner = word % (1 << 17);
        let reserved = (word / (1 << 18)) % (1 << 13);
        word >= (1 << 31) && reserved == 0 && (1..=100000).contains(&owner)
    };
    for owner in [1, 65535, 65536, 100000] {
        for partial in [false, true] {
            let word = (1 << 31) | owner | if partial { 1 << 17 } else { 0 };
            assert!(admit(word));
            assert_eq!(word & 0x1ffff, owner);
            assert_eq!((word & (1 << 17)) != 0, partial);
            assert!(!admit(word & !(1 << 31)));
            for bit in 18..31 {
                assert!(!admit(word | (1 << bit)));
            }
        }
    }
    for owner in [0, 100001, 131071] {
        assert!(!admit((1 << 31) | (1 << 17) | owner));
    }
    for (history, expected) in [
        (
            History::Tent,
            "e7cedfa967ed931785ca3c3a91d644364991219ef2a2f49c67a85e35eaf97803",
        ),
        (
            History::Cubic,
            "020fad07981d4d0be5c400c6ff9b8f15f5330bac72a6cef40fbfc58acab2df69",
        ),
    ] {
        assert_eq!(
            hex(Coverage::Alpha
                .assemble(shader_with_options(3, Kernel::Point, history))
                .as_bytes()),
            expected
        );
    }
}

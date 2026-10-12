#![cfg(feature = "static-conformance")]
#[path = "../../incant_wasm_runtime/tests/support/animation_graph.rs"]
mod fixture;
use incant_runtime::*;
use incant_wasm_dev::DevModuleHost;
use incant_wasm_runtime::{StaticModule, embedded};
const DT: f64 = 1. / 60.;
fn id(n: u8) -> StableId {
    StableId([n; 16])
}
fn guest(name: &str) -> Vec<u8> {
    std::fs::read(std::path::Path::new(env!("INCANT_RUST_SDK_GUESTS")).join(format!("{name}.wasm")))
        .unwrap()
}
fn oracle(world: &mut NativeWorld, turn: u32) {
    let graph = world.animation_graph_state(id(1)).unwrap();
    let mut poses = Vec::new();
    world.snapshot_animations(&mut poses).unwrap();
    let pose = poses.iter().find(|p| p.id == id(1)).unwrap();
    let first = [
        f64::from(turn + 1),
        pose.blend.map_or(-1., |b| b.sample.seconds),
        pose.state.sample.seconds,
    ];
    let second = [
        graph.parameters[0],
        graph.seconds,
        pose.blend.map_or(-1., |b| b.weight),
    ];
    world.with_numeric_columns(|view| {
        view.for_each_chunk(|chunk| {
            for (entity, transform) in chunk
                .ids
                .iter()
                .zip(chunk.transforms.as_chunks_mut::<10>().0.iter_mut())
            {
                transform[..3].copy_from_slice(if *entity == id(1) { &first } else { &second });
            }
        });
    });
    let control = match turn {
        0 | 4 | 5 | 6 => Some(StructuralCommand::ControlAnimationGraph {
            entity: id(1),
            control: AnimationGraphControl::SetParameter {
                parameter: id(10),
                value: match turn {
                    0 => 0.75,
                    4 => 0.,
                    5 => 1.,
                    _ => 0.5,
                },
            },
        }),
        1 | 3 => Some(StructuralCommand::ControlAnimation {
            entity: id(1),
            control: AnimationControl::SetPlaying(turn == 3),
        }),
        2 | 9 => Some(StructuralCommand::ControlAnimationGraph {
            entity: id(1),
            control: AnimationGraphControl::SelectState(id(20)),
        }),
        _ => None,
    };
    if let Some(c) = control {
        let mut commands = FrameCommands::default();
        commands.push(c).unwrap();
        world.apply(&mut commands).unwrap();
    }
}
#[test]
fn actual_rust_graph_snapshots_controls_pause_and_endpoints_match_independent_native_oracle() {
    let name = "rust-animation-graph";
    let mut native = StaticModule::new(embedded::linked(name).unwrap()).unwrap();
    let mut dev = DevModuleHost::new(&guest(name)).unwrap();
    let mut a = NativeWorld::new(fixture::scene(0.25));
    let mut b = NativeWorld::new(fixture::scene(0.25));
    let mut expected = NativeWorld::new(fixture::scene(0.25));
    for turn in 0..12 {
        a.step().unwrap();
        b.step().unwrap();
        expected.step().unwrap();
        oracle(&mut expected, turn);
        native.tick(&mut a, DT).unwrap();
        dev.tick(&mut b, DT).unwrap();
        assert_eq!(
            a.save_bytes().unwrap(),
            expected.save_bytes().unwrap(),
            "static turn {turn}"
        );
        assert_eq!(
            b.save_bytes().unwrap(),
            expected.save_bytes().unwrap(),
            "development turn {turn}"
        );
        assert_eq!(
            native.logs().unwrap().entries(),
            dev.logs().unwrap().entries()
        );
        assert_eq!(native.logs().unwrap().entries().len(), turn as usize + 1);
        if turn == 2 {
            assert!(a.animation_graph_state(id(1)).unwrap().paused);
        }
        if turn == 7 {
            dev.reload(&guest(name)).unwrap();
        }
    }
}
#[test]
fn actual_sdk_caught_and_native_errors_leave_no_partial_graph_numeric_entity_or_log_work() {
    for mode in [
        "capacity",
        "pose_capacity",
        "number",
        "range",
        "authored_range",
        "parameter",
        "state",
        "entity",
        "quota",
        "callback",
        "trap",
    ] {
        let name = format!("rust-animation-graph-{}", mode.replace('_', "-"));
        let mut native = StaticModule::new(embedded::linked(&name).unwrap()).unwrap();
        let mut dev = DevModuleHost::new(&guest(&name)).unwrap();
        let mut a = NativeWorld::new(fixture::scene(0.25));
        let mut b = NativeWorld::new(fixture::scene(0.25));
        let before = a.save_bytes().unwrap();
        assert!(native.tick(&mut a, DT).is_err(), "static {mode}");
        assert!(dev.tick(&mut b, DT).is_err(), "development {mode}");
        assert_eq!(a.save_bytes().unwrap(), before);
        assert_eq!(b.save_bytes().unwrap(), before);
        assert!(native.is_failed() && dev.is_failed());
        assert!(native.logs().is_err() && dev.logs().is_err());
        assert!(native.tick(&mut a, DT).is_err() && dev.tick(&mut b, DT).is_err());
    }
}
#[test]
fn graph_versions_signatures_and_nonexistent_v3_control_import_fail_closed() {
    for import in [
        r#"(import "incant_animation_graph_v2" "snapshot_len" (func (param i64)(result i32)))"#,
        r#"(import "incant_animation_graph_v1" "read_snapshot" (func (param i64 i32)(result i32)))"#,
        r#"(import "incant_animation_graph_v1" "control" (func (param i64 i32 i32)(result i32)))"#,
        r#"(import "incant_animation_v3" "control" (func (param i64 i32 i32)))"#,
    ] {
        let bytes = wat::parse_str(format!("(module {import} (memory (export \"memory\") 1) (func (export \"incant_abi\") (result i32) i32.const 1) (func (export \"update\") (param i64 f64)))")).unwrap();
        assert!(DevModuleHost::new(&bytes).is_err());
        assert!(incant_wasm_cook::prepare(&bytes).is_err());
    }
}

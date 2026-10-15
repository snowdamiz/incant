#[path = "common/allocations.rs"]
mod allocations;
mod common;
use common::*;
use incant_render_data::{LightValues, SceneInput};
use incant_render_native::{SyncError, UpdateKind};
use incant_runtime::{LightControl, StructuralCommand};
use std::panic::{AssertUnwindSafe, catch_unwind};
fn values(scene: &SceneInput) -> LightValues {
    let mut matches = scene
        .nodes
        .iter()
        .filter(|node| node.id == "00000000000000000000000000000003");
    let value = matches
        .next()
        .expect("light owner 3")
        .light
        .as_ref()
        .expect("punctual light")
        .values();
    assert!(matches.next().is_none(), "unique light owner 3");
    value
}
#[test]
fn scalar_light_updates_reuse_numeric_storage_and_refusal_or_unwind_restores_last_accepted_values()
{
    let mut world = world();
    let mut input = load(&world);
    let ptr = input.input().nodes.as_ptr();
    let label = input.camera_label().as_ptr();
    let original = values(input.input());
    world
        .apply(&mut queue([
            StructuralCommand::ControlLight {
                entity: id(3),
                control: LightControl::Color([0.25, 0.5, 0.75]),
            },
            StructuralCommand::ControlLight {
                entity: id(3),
                control: LightControl::Intensity(0.),
            },
        ]))
        .unwrap();
    let (result, count) = allocations::record(|| {
        input.try_sync(&world.spatial_view().unwrap(), |scene, camera, kind| {
            assert_eq!(kind, UpdateKind::Motion);
            assert_eq!(camera.as_ptr(), label);
            assert_eq!(values(scene).intensity, 0.);
            Err::<(), _>("late consumer refusal")
        })
    });
    assert_eq!(count, 0);
    assert!(matches!(result, Err(SyncError::Consumer(_))));
    assert_eq!(values(input.input()), original);
    assert!(
        catch_unwind(AssertUnwindSafe(|| input.try_sync(
            &world.spatial_view().unwrap(),
            |_, _, _| -> Result<(), ()> { panic!("consumer unwind") }
        )))
        .is_err()
    );
    assert_eq!(values(input.input()), original);
    let (result, count) = allocations::record(|| {
        input.try_sync(&world.spatial_view().unwrap(), |_, _, kind| {
            assert_eq!(kind, UpdateKind::Motion);
            Ok::<_, ()>(())
        })
    });
    result.unwrap();
    assert_eq!(count, 0);
    assert_eq!(input.input().nodes.as_ptr(), ptr);
    assert_eq!(input.binding_generation(), 0);
    assert_eq!(values(input.input()).color, [0.25, 0.5, 0.75]);
}

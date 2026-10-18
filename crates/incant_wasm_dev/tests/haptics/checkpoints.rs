use super::*;
use incant_runtime::{
    CheckpointAllocationBudget, CheckpointPayloadBudget, PlanarCheckpointWorkBudget,
    SpatialCheckpointWorkBudget,
};
fn payload() -> CheckpointPayloadBudget {
    CheckpointPayloadBudget::new(64 * 1024 * 1024).unwrap()
}
fn planar() -> PlanarCheckpointWorkBudget {
    PlanarCheckpointWorkBudget::new(100_000_000)
}
fn spatial() -> SpatialCheckpointWorkBudget {
    SpatialCheckpointWorkBudget::new(100_000_000)
}
fn allocations() -> CheckpointAllocationBudget {
    CheckpointAllocationBudget::new(64 * 1024 * 1024, 100_000)
}
#[test]
fn ordinary_and_persisted_capture_and_restore_refuse_pending_without_consuming_it() {
    for development in [false, true] {
        let code = program(development, "rust-haptics");
        let initial = scene();
        let mut live =
            NativeSession::new(initial.clone(), code.clone(), Default::default()).unwrap();
        let ordinary = live.checkpoint_bytes().unwrap();
        let checkpoint = live
            .checkpoint(&mut payload(), &mut planar(), &mut spatial())
            .unwrap();
        let persisted = checkpoint
            .persisted_bytes(&mut payload(), &mut planar(), &mut spatial())
            .unwrap();
        live.set_haptic_capabilities(caps()).unwrap();
        live.tick(&[]).unwrap();
        let before = live.world().unwrap().save_bytes().unwrap();
        assert!(live.checkpoint_bytes().is_err());
        assert!(
            live.checkpoint(&mut payload(), &mut planar(), &mut spatial())
                .is_err()
        );
        assert!(live.restore_checkpoint_bytes(&ordinary).is_err());
        assert!(
            live.restore_checkpoint(
                &checkpoint,
                &mut payload(),
                &mut planar(),
                &mut spatial(),
                &mut allocations()
            )
            .is_err()
        );
        assert!(
            live.restore_persisted_bytes(
                &persisted,
                &mut payload(),
                &mut planar(),
                &mut spatial(),
                &mut allocations()
            )
            .is_err()
        );
        assert_eq!(live.world().unwrap().save_bytes().unwrap(), before);
        assert!(!live.is_failed());
        assert_eq!(
            live.take_haptics().unwrap().iter().collect::<Vec<_>>(),
            expected()
        );
        // A damaged file also preserves current authority and accepted state.
        let mut corrupt = ordinary.clone();
        corrupt[0] ^= 1;
        assert!(live.restore_checkpoint_bytes(&corrupt).is_err());
        assert_eq!(live.world().unwrap().save_bytes().unwrap(), before);
        live.tick(&[]).unwrap();
        assert_eq!(live.take_haptics().unwrap().len(), 4);
        for route in 0..3 {
            live.set_haptic_capabilities(caps()).unwrap();
            match route {
                0 => live.restore_checkpoint_bytes(&ordinary).unwrap(),
                1 => live
                    .restore_checkpoint(
                        &checkpoint,
                        &mut payload(),
                        &mut planar(),
                        &mut spatial(),
                        &mut allocations(),
                    )
                    .unwrap(),
                _ => live
                    .restore_persisted_bytes(
                        &persisted,
                        &mut payload(),
                        &mut planar(),
                        &mut spatial(),
                        &mut allocations(),
                    )
                    .unwrap(),
            }
            assert_eq!(live.world().unwrap().tick(), 0);
            assert!(live.take_haptics().unwrap().is_empty());
            live.tick(&[]).unwrap();
            assert_eq!(
                live.world().unwrap().snapshot()[0].transform.translation[0],
                1.
            );
            assert!(live.take_haptics().unwrap().is_empty());
        }
        let mut fresh =
            NativeSession::from_checkpoint_bytes(&ordinary, &initial, code, Default::default())
                .unwrap();
        fresh.tick(&[]).unwrap();
        assert!(fresh.take_haptics().unwrap().is_empty());
    }
}

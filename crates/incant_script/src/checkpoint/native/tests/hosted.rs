use super::*;
use incant_runtime::CookedPhysics;
use incant_types::{Collider, ColliderShape, RigidBody};

#[cfg(feature = "audio")]
mod audio;
#[cfg(feature = "ui")]
mod ui;

fn physical_scene() -> CookedScene {
    CookedScene::new(
        id(99),
        144,
        (1..=3)
            .map(|n| CookedEntity {
                id: id(n),
                parent: (n == 2).then(|| id(1)),
                transform: Default::default(),
                velocity: None,
            })
            .collect(),
    )
    .unwrap()
    .with_physics(vec![CookedPhysics {
        entity: id(3),
        body: Some(RigidBody::default()),
        collider: Collider {
            shape: ColliderShape::Box {
                half_extents: [0.5; 3],
            },
            ..Default::default()
        },
        angular_velocity: None,
    }])
    .unwrap()
}
fn live(initial: CookedScene) -> NativeSession {
    NativeSession::new(
        initial,
        program("t.state.set(Number(t.state.get()||0)+1);"),
        Default::default(),
    )
    .unwrap()
}
fn prepare(checkpoint: &NativeSessionCheckpoint) -> Result<PendingSessionCheckpoint> {
    checkpoint.prepare(
        &mut payload(),
        &mut planar(),
        &mut spatial(),
        &mut allocations(),
    )
}
#[cfg(feature = "ui")]
fn prepare_for(
    checkpoint: &NativeSessionCheckpoint,
    live: &NativeSession,
) -> Result<PendingSessionCheckpoint> {
    checkpoint.prepare_for(
        live,
        &mut payload(),
        &mut planar(),
        &mut spatial(),
        &mut allocations(),
    )
}

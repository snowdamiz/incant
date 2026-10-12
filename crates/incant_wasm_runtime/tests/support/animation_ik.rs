#[path = "animation_poses.rs"]
mod poses;
use incant_runtime::*;
pub use poses::id;
use std::sync::Arc;
pub fn scene() -> CookedScene {
    let base = poses::scene();
    let mut nodes = base.spatial().to_vec();
    for n in &mut nodes {
        if n.entity == id(1) || n.entity == id(2) {
            n.ik = Some(Arc::new(NativeIk {
                mesh_fingerprint: [4; 32],
                initial: Arc::from([
                    AnimationIkSample {
                        id: id(9),
                        chain: [0, 1, 2],
                        target: [1., 1., 0.],
                        pole: [0., 1., 0.],
                        weight: 1.,
                    },
                    AnimationIkSample {
                        id: id(8),
                        chain: [1, 2, 3],
                        target: [2., 1., 0.],
                        pole: [0., 1., 0.],
                        weight: 0.5,
                    },
                ]),
            }));
        }
    }
    let mut entities = base.entities().to_vec();
    entities.extend((1000..1012).map(|n| CookedEntity {
        id: id(n),
        parent: None,
        transform: incant_types::Transform::default(),
        velocity: None,
    }));
    CookedScene::new(id(99), 60, entities)
        .unwrap()
        .with_spatial(base.visual_assets().to_vec(), nodes)
        .unwrap()
}

/// Drive all existing native tree/pair/scalar clocks through their real controls.
pub fn advance_pose_oracle(world: &mut NativeWorld, turn: u32) {
    poses::oracle(world, turn);
}

/// Native independent complete-pose and ordered-constraint observation oracle.
pub fn oracle(world: &mut NativeWorld, turn: u32) {
    let mut rows = Vec::new();
    world.snapshot_ik(&mut rows).unwrap();
    assert_eq!(rows.len(), 4);
    world.with_numeric_columns(|view| {
        view.for_each_chunk(|chunk| {
            for (id, transform) in chunk
                .ids
                .iter()
                .zip(chunk.transforms.as_chunks_mut::<10>().0)
            {
                let n = u128::from_be_bytes(id.0);
                if (1000..1012).contains(&n) {
                    let at = (n - 1000) as usize;
                    let row = rows[at / 3];
                    let value = match at % 3 {
                        0 => row.sample.target,
                        1 => row.sample.pole,
                        _ => [
                            row.sample.weight,
                            u128::from_be_bytes(row.sample.id.0) as f64,
                            f64::from(row.sample.chain[1]),
                        ],
                    };
                    transform[..3].copy_from_slice(&value);
                }
            }
        })
    });
    advance_pose_oracle(world, turn);
    let mut commands = FrameCommands::default();
    for (index, row) in rows.iter().enumerate() {
        commands
            .push(StructuralCommand::ControlIk {
                entity: row.entity,
                control: IkControl {
                    constraint: row.sample.id,
                    target: [(f64::from(turn) + 1.) / 8., index as f64, 0.],
                    pole: [0., 1., (f64::from(turn) + 1.) / 64.],
                    weight: f64::from(turn % 3) / 2.,
                },
            })
            .unwrap();
    }
    world.apply(&mut commands).unwrap();
}

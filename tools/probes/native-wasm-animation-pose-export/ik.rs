//! Public package guest: full pose and IK records plus independent numeric observations.
#![no_std]
use incant_module::{
    AnimationControl as A, AnimationGraphControl as G, AnimationGraphKey as Key,
    AnimationPoseKind as Kind, EntityId, Error, IDENTITY, LogLevel, Result, Tick,
};
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    core::arch::wasm32::unreachable()
}
incant_module::module!(simulate);
use core::fmt::Write;
struct Text {
    bytes: [u8; 1024],
    len: usize,
}
impl core::fmt::Write for Text {
    fn write_str(&mut self, text: &str) -> core::fmt::Result {
        let end = self.len.checked_add(text.len()).ok_or(core::fmt::Error)?;
        self.bytes
            .get_mut(self.len..end)
            .ok_or(core::fmt::Error)?
            .copy_from_slice(text.as_bytes());
        self.len = end;
        Ok(())
    }
}
static mut INPUT: [u8; incant_module::INPUT_MAX_BYTES] = [0; incant_module::INPUT_MAX_BYTES];
fn id(n: u128) -> EntityId {
    EntityId::from_u128(n)
}
fn simulate(tick: &mut Tick<'_>, _: f64) -> Result<()> {
    // Scratch only, never game state. The persistent turn counter is native entity 4.
    let input = tick.input(unsafe { &mut *(&raw mut INPUT) })?;
    let clock = input.tick;
    let failed = [
        "old_v1",
        "old_v2",
        "old_v3",
        "old_v4",
        "short_scratch",
        "late_command",
        "bad_constraint",
    ]
    .iter()
    .position(|name| input.action(name).is_some_and(|a| a.active));
    let mut bytes = [0; 4096];
    let snapshot = tick.animation_ik(&mut bytes)?;
    let poses = snapshot.poses();
    let mut ik = [[0.; 3]; 12];
    if snapshot.constraints().len() != 4 {
        return Err(Error::Malformed);
    }
    for (index, row) in snapshot.constraints().enumerate() {
        if row.entity != id(index as u128 / 2 + 1)
            || row.constraint != id(if index % 2 == 0 { 9 } else { 8 })
            || row.chain != if index % 2 == 0 { [0, 1, 2] } else { [1, 2, 3] }
        {
            return Err(Error::Malformed);
        }
        ik[index * 3] = row.target;
        ik[index * 3 + 1] = row.pole;
        ik[index * 3 + 2] = [
            row.weight,
            u128::from_be_bytes(row.constraint.0) as f64,
            f64::from(row.chain[1]),
        ];
    }
    if poses.scene() != id(99) || poses.len() != 3 {
        return Err(Error::Malformed);
    }
    let mut observations = [[-2.0, 0.0, 0.0]; 48];
    let mut metadata = [[0.0; 3]; 9];
    for (index, pose) in poses.iter().enumerate() {
        if pose.entity != id(index as u128 + 1)
            || (index == 0 && pose.kind != Kind::WeightedQuaternionMean)
        {
            return Err(Error::Malformed);
        }
        for (slot, sample) in pose.samples().enumerate() {
            observations[index * 16 + slot] = [
                sample.clip.map_or(-1.0, f64::from),
                sample.seconds,
                sample.weight,
            ];
        }
        metadata[index * 3] = [
            pose.samples().len() as f64,
            pose.kind as u8 as f64,
            pose.seconds,
        ];
        metadata[index * 3 + 1] = [
            pose.speed,
            f64::from(u8::from(pose.playing)),
            f64::from(u8::from(pose.completed)),
        ];
        metadata[index * 3 + 2] = [
            f64::from(u8::from(pose.looping)),
            pose.transition.map_or(-1.0, |t| t.elapsed),
            pose.transition.map_or(-1.0, |t| t.duration),
        ];
    }
    let mut turn = None;
    for index in 0..tick.chunk_count()? {
        let chunk = tick.chunk(index)?;
        let mut ids = [id(0); 128];
        let ids = tick.read_ids(chunk, &mut ids)?;
        let mut transforms = [IDENTITY; 128];
        let transforms = tick.read_transforms(chunk, &mut transforms)?;
        for (entity, transform) in ids.iter().zip(transforms.iter_mut()) {
            let n = u128::from_be_bytes(entity.0);
            if n == 4 {
                turn = Some(transform[0] as u32);
                transform[0] += 1.0;
            } else if (100..148).contains(&n) {
                transform[..3].copy_from_slice(&observations[(n - 100) as usize]);
            } else if (1000..1012).contains(&n) {
                transform[..3].copy_from_slice(&ik[(n - 1000) as usize]);
            } else if (200..209).contains(&n) {
                transform[..3].copy_from_slice(&metadata[(n - 200) as usize]);
            }
        }
        tick.write_transforms(chunk, transforms)?;
    }
    let turn = turn.ok_or(Error::Malformed)?;
    match turn {
        0 | 4 | 5 | 6 | 16 => tick.control_animation_graph(
            &id(1),
            G::SetParameter {
                parameter: Key::from_u128(10),
                value: match turn {
                    0 => 0.75,
                    4 => 0.0,
                    5 => 1.0,
                    16 => 0.25,
                    _ => 0.5,
                },
            },
        )?,
        1 | 12 => tick.control_animation(&id(1), A::Playing(false))?,
        2 | 14 => tick.control_animation_graph(&id(1), G::SelectState(Key::from_u128(20)))?,
        3 | 13 => tick.control_animation(&id(1), A::Playing(true))?,
        7 => tick.control_animation(&id(1), A::Speed(-1.0))?,
        8 => tick.control_animation(&id(1), A::Speed(1.0))?,
        9 => tick.control_animation_graph(&id(1), G::SelectState(Key::from_u128(21)))?,
        10 => tick.control_animation(&id(1), A::Speed(1.0))?,
        15 => tick.control_animation_graph(
            &id(1),
            G::SetParameter {
                parameter: Key::from_u128(12),
                value: 0.25,
            },
        )?,
        _ => {}
    }
    match turn {
        0 => tick.control_animation(
            &id(2),
            A::Crossfade {
                clip: Some(1),
                seconds: 0.5,
                duration: 0.2,
            },
        )?,
        1 | 10 => tick.control_animation(&id(2), A::Playing(false))?,
        3 | 12 => tick.control_animation(&id(2), A::Playing(true))?,
        4 => tick.control_animation(&id(2), A::Speed(-1.0))?,
        7 => tick.control_animation(&id(2), A::Seek(0.7))?,
        8 => tick.control_animation(
            &id(2),
            A::Crossfade {
                clip: None,
                seconds: 0.0,
                duration: 0.1,
            },
        )?,
        15 => tick.control_animation(
            &id(2),
            A::Select {
                clip: Some(0),
                seconds: 0.0,
            },
        )?,
        _ => {}
    }
    for (index, row) in snapshot.constraints().enumerate() {
        tick.set_animation_ik(
            row.entity,
            row.constraint,
            [(f64::from(turn) + 1.) / 8., index as f64, 0.],
            [0., 1., (f64::from(turn) + 1.) / 64.],
            f64::from(turn % 3) / 2.,
        )?;
    }
    let mut after = [0; 4096];
    tick.animation_ik(&mut after)?;
    if after != bytes {
        return Err(Error::Malformed);
    }
    if u64::from(turn) + 1 != clock {
        return Err(Error::Malformed);
    }
    for index in 0..3 {
        let mut record = Text {
            bytes: [0; 1024],
            len: 0,
        };
        write!(
            &mut record,
            "{{\"tick\":{},\"entity\":{},\"samples\":{:?},\"metadata\":{:?}}}",
            clock,
            index + 1,
            &observations[index * 16..(index + 1) * 16],
            &metadata[index * 3..(index + 1) * 3]
        )
        .map_err(|_| Error::Malformed)?;
        tick.log(
            core::str::from_utf8(&record.bytes[..record.len]).map_err(|_| Error::Malformed)?,
            LogLevel::Info,
        )?;
    }
    for index in 0..2 {
        let mut record = Text {
            bytes: [0; 1024],
            len: 0,
        };
        write!(
            &mut record,
            "{{\"tick\":{},\"entity\":{},\"constraints\":{:?}}}",
            clock,
            index + 1,
            &ik[index * 6..(index + 1) * 6]
        )
        .map_err(|_| Error::Malformed)?;
        tick.log(
            core::str::from_utf8(&record.bytes[..record.len]).map_err(|_| Error::Malformed)?,
            LogLevel::Info,
        )?;
    }
    if let Some(mode) = failed {
        tick.spawn(&id(77), None, &IDENTITY, None)?;
        tick.log("failure prefix must not publish", LogLevel::Warn)?;
        match mode {
            0 => {
                let _ = tick.legacy_animations(&mut [0; 2048]);
            }
            1 => {
                let _ = tick.transition_animations(&mut [0; 2048]);
            }
            2 => {
                let _ = tick.animations(&mut [0; 2048]);
            }
            3 => {
                let _ = tick.animation_poses(&mut [0; 4096]);
            }
            4 => {
                let _ = tick.animation_ik(&mut []);
            }
            5 => tick.set_velocity(&id(90), Some(&[1., 0., 0.]))?,
            _ => tick.set_animation_ik(id(1), id(99), [1., 1., 0.], [0., 1., 0.], 1.)?,
        }
    }
    Ok(())
}

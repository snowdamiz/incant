//! Complete root motion, compound poses and ordered IK through real gameplay.
#![no_std]
use incant_module::{
    AnimationControl as A, AnimationGraphControl as G, AnimationGraphKey as Key, EntityId, Error,
    IDENTITY, LogLevel, Result, Tick,
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
    let input = tick.input(unsafe { &mut *(&raw mut INPUT) })?;
    let clock = input.tick;
    let failed = [
        "old_v5",
        "old_ik_v2",
        "short_root",
        "late_command",
        "bad_root",
    ]
    .iter()
    .position(|name| input.action(name).is_some_and(|a| a.active));
    let mut body_values = [0.; 60];
    let mut bytes = [0; 16384];
    let snapshot = tick.root_motion(&mut bytes)?;
    let poses = snapshot.poses();
    let root_values = root_values(snapshot);
    let mut values = [0.; 384];
    let mut index = 0;
    let mut push = |v| {
        values[index] = v;
        index += 1;
    };
    push(poses.len() as f64);
    for r in poses.iter() {
        for v in [
            u128::from_be_bytes(r.entity.0) as f64,
            r.clip.map_or(-1., f64::from),
            r.seconds,
            r.speed,
            f64::from(r.playing),
            f64::from(r.looping),
            f64::from(r.completed),
            r.kind as u8 as f64,
            r.outer_weight.unwrap_or(-1.),
        ] {
            push(v);
        }
        let t = r.transition;
        for v in [
            t.map_or(-1., |v| v.clip.map_or(-1., f64::from)),
            t.map_or(-1., |v| v.seconds),
            t.map_or(-1., |v| v.speed),
            t.map_or(-1., |v| f64::from(v.playing)),
            t.map_or(-1., |v| f64::from(v.looping)),
            t.map_or(-1., |v| v.elapsed),
            t.map_or(-1., |v| v.duration),
        ] {
            push(v);
        }
        for i in 0..2 {
            if let Some(g) = r.group(i) {
                push(g.interpolation as u8 as f64);
                push(g.samples().len() as f64);
                for s in g.samples() {
                    push(s.clip.map_or(-1., f64::from));
                    push(s.seconds);
                    push(s.weight);
                }
            } else {
                push(-1.);
                push(0.);
            }
        }
    }
    let mut graph_bytes = [0; 4096];
    let graphs = tick.animation_graphs_v2(&mut graph_bytes)?;
    push(graphs.graphs().len() as f64);
    for g in graphs.graphs().iter() {
        for v in [
            u128::from_be_bytes(g.entity.0) as f64,
            u128::from_be_bytes(g.state.0) as f64,
            f64::from(g.paused),
            g.seconds,
            g.elapsed,
        ] {
            push(v);
        }
        let f = graphs.fades().find(|f| f.entity == g.entity);
        for v in [
            f.map_or(-1., |v| u128::from_be_bytes(v.state.0) as f64),
            f.map_or(-1., |v| v.seconds),
            f.map_or(-1., |v| v.speed),
            f.map_or(-1., |v| f64::from(v.playing)),
            f.map_or(-1., |v| v.elapsed),
            f.map_or(-1., |v| v.duration),
        ] {
            push(v);
        }
        push(g.parameters().len() as f64);
        for p in g.parameters() {
            push(u128::from_be_bytes(p.id.0) as f64);
            push(p.value);
        }
    }
    push(snapshot.constraints().len() as f64);
    for r in snapshot.constraints() {
        push(u128::from_be_bytes(r.entity.0) as f64);
        push(u128::from_be_bytes(r.constraint.0) as f64);
        for v in r.chain {
            push(f64::from(v));
        }
        for v in r.target.into_iter().chain(r.pole) {
            push(v);
        }
        push(r.weight);
    }
    let mut turn = None;
    for n in 0..tick.chunk_count()? {
        let chunk = tick.chunk(n)?;
        let mut ids = [id(0); 256];
        let ids = tick.read_ids(chunk, &mut ids)?;
        let mut transforms = [IDENTITY; 256];
        let transforms = tick.read_transforms(chunk, &mut transforms)?;
        let mut velocities = [[0.; 3]; 256];
        if chunk.has_velocity() {
            tick.read_velocities(chunk, &mut velocities)?;
        }
        for (i, (id, t)) in ids.iter().zip(transforms.iter_mut()).enumerate() {
            let n = u128::from_be_bytes(id.0);
            if let Some(row) = [1, 2, 3, 5].iter().position(|value| *value == n) {
                let at = row * 15;
                body_values[at] = n as f64;
                body_values[at + 1] = f64::from(chunk.has_velocity());
                body_values[at + 2..at + 12].copy_from_slice(t);
                body_values[at + 12..at + 15].copy_from_slice(&velocities[i]);
            }
            if n == 4 {
                turn = Some(t[0] as u32);
                t[0] += 1.;
            } else if (100..228).contains(&n) {
                let offset = (n - 100) as usize * 3;
                t[..3].copy_from_slice(&values[offset..offset + 3]);
            } else if (400..486).contains(&n) {
                let offset = (n - 400) as usize * 3;
                t[..3].copy_from_slice(&root_values[offset..offset + 3]);
            }
        }
        tick.write_transforms(chunk, transforms)?;
    }
    for n in 0..tick.chunk_count()? {
        let chunk = tick.chunk(n)?;
        let mut ids = [id(0); 256];
        let ids = tick.read_ids(chunk, &mut ids)?;
        let mut transforms = [IDENTITY; 256];
        let transforms = tick.read_transforms(chunk, &mut transforms)?;
        for (id, t) in ids.iter().zip(transforms.iter_mut()) {
            let n = u128::from_be_bytes(id.0);
            if (500..520).contains(&n) {
                let at = (n - 500) as usize * 3;
                t[..3].copy_from_slice(&body_values[at..at + 3]);
            }
        }
        tick.write_transforms(chunk, transforms)?;
    }
    let turn = turn.ok_or(Error::Malformed)?;
    let animation = match turn {
        4 | 9 => Some(A::Playing(false)),
        7 | 12 => Some(A::Playing(true)),
        10 | 18 => Some(A::Speed(-2.)),
        16 => Some(A::Speed(1.)),
        _ => None,
    };
    if let Some(control) = animation {
        tick.control_animation(
            &id(if turn == 9 || turn == 12 || turn == 18 {
                2
            } else {
                1
            }),
            control,
        )?;
    }
    if turn == 2 {
        tick.control_animation(
            &id(2),
            A::Crossfade {
                clip: Some(1),
                seconds: 0.3,
                duration: 0.2,
            },
        )?;
    }
    if turn == 24 || turn == 28 {
        tick.control_animation_graph(
            &id(1),
            G::SelectState(Key::from_u128(if turn == 24 { 23 } else { 20 })),
        )?;
    }
    if turn == 1 || turn == 20 {
        tick.control_animation_graph(
            &id(1),
            G::SetParameter {
                parameter: Key::from_u128(10),
                value: if turn == 1 { 0.75 } else { 0.25 },
            },
        )?;
    }
    for (n, r) in snapshot.constraints().enumerate() {
        tick.set_animation_ik(
            r.entity,
            r.constraint,
            [f64::from(turn) / 8., n as f64, 0.],
            [0., 1., f64::from(turn) / 64.],
            f64::from(turn % 3) / 2.,
        )?;
    }
    if [3, 8, 11, 15].contains(&turn) {
        tick.control_root_motion(
            id(1),
            incant_module::RootMotionControl::SetApplying(turn == 8 || turn == 15),
        )?;
    }
    if turn == 17 {
        tick.control_root_motion(id(1), incant_module::RootMotionControl::ResetAccumulated)?;
    }
    let mut after = [0; 16384];
    tick.root_motion(&mut after)?;
    if after != bytes {
        return Err(Error::Malformed);
    }
    let mut after_graph = [0; 4096];
    tick.animation_graphs_v2(&mut after_graph)?;
    if after_graph != graph_bytes {
        return Err(Error::Malformed);
    }
    if u64::from(turn) + 1 != clock {
        return Err(Error::Malformed);
    }
    let mut complete = [0.; 702];
    complete[..384].copy_from_slice(&values);
    complete[384..642].copy_from_slice(&root_values);
    complete[642..].copy_from_slice(&body_values);
    for (part, values) in complete.chunks_exact(27).enumerate() {
        let mut record = Text {
            bytes: [0; 1024],
            len: 0,
        };
        write!(
            &mut record,
            "{{\"tick\":{},\"part\":{},\"values\":{:?}}}",
            clock, part, values
        )
        .map_err(|_| Error::Malformed)?;
        tick.log(
            core::str::from_utf8(&record.bytes[..record.len]).map_err(|_| Error::Malformed)?,
            LogLevel::Info,
        )?;
    }
    match failed {
        Some(0) => {
            let _ = tick.animation_poses_v5(&mut [0; 8192]);
        }
        Some(1) => {
            let _ = tick.animation_ik_v2(&mut [0; 8192]);
        }
        Some(2) => {
            let _ = tick.root_motion(&mut []);
        }
        Some(3) => tick.set_velocity(&id(90), Some(&[1., 0., 0.]))?,
        Some(4) => {
            tick.control_root_motion(id(4), incant_module::RootMotionControl::SetApplying(false))?
        }
        _ => {}
    }
    Ok(())
}

fn root_values(s: incant_module::RootMotionSnapshot<'_>) -> [f64; 258] {
    use incant_module::{RootMotionClock, RootMotionClockOwner as Owner};
    let mut values = [0.; 258];
    let mut at = 0;
    let mut push = |v| {
        values[at] = v;
        at += 1;
    };
    push(s.len() as f64);
    for r in s.iter() {
        for bytes in r.entity.0.as_chunks::<4>().0 {
            push(u32::from_be_bytes(*bytes) as f64);
        }
        for bytes in r.fingerprint.as_chunks::<4>().0 {
            push(u32::from_be_bytes(*bytes) as f64);
        }
        push(f64::from(r.root));
        push(r.policy as u8 as f64);
        push(f64::from(r.applying));
        push((r.tick >> 32) as f64);
        push((r.tick as u32) as f64);
        for (i, c) in [Some(r.current), r.outgoing].into_iter().enumerate() {
            if i == 1 {
                push(f64::from(c.is_some()));
            }
            let c = c.unwrap_or(RootMotionClock {
                owner: Owner::Rest,
                cycle: 0,
                seconds: 0.,
            });
            let (kind, state, clip) = match c.owner {
                Owner::Rest => (0., id(0), -1.),
                Owner::Clip(n) => (1., id(0), f64::from(n)),
                Owner::State(s) => (2., s, -1.),
            };
            push(kind);
            for bytes in state.0.as_chunks::<4>().0 {
                push(u32::from_be_bytes(*bytes) as f64);
            }
            push(clip);
            push(((c.cycle as u64) >> 32) as f64);
            push((c.cycle as u32) as f64);
            push(c.seconds);
        }
        for d in [
            r.requested,
            r.applied,
            r.accumulated_requested,
            r.accumulated_applied,
        ] {
            for v in d.translation.into_iter().chain(d.rotation) {
                push(v);
            }
        }
    }
    values
}

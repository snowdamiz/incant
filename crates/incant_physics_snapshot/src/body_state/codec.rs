use super::validate::{HEADER, STRIDE, context};
use super::*;
use crate::{
    Budget, Error, MAX_BYTES, Result,
    arena_codec::{frame_length, read_identity, write_identity},
    codec::Reader,
    vector,
};
const MAGIC: &[u8; 8] = b"ICBODY01";
fn words<const N: usize>(r: &mut Reader<'_>) -> Result<[u32; N]> {
    let mut out = [0; N];
    for v in &mut out {
        *v = r.word()?;
    }
    Ok(out)
}
fn boolean(r: &mut Reader<'_>) -> Result<bool> {
    match r.word()? {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(Error::Header),
    }
}
fn pose(r: &mut Reader<'_>) -> Result<LocalPose> {
    Ok(LocalPose {
        translation: words(r)?,
        rotation: words(r)?,
    })
}
fn velocity(r: &mut Reader<'_>) -> Result<BodyVelocity> {
    Ok(BodyVelocity {
        linear: words(r)?,
        angular: words(r)?,
    })
}
fn local_mass(r: &mut Reader<'_>) -> Result<LocalMass> {
    Ok(LocalMass {
        center: words(r)?,
        inverse_mass: r.word()?,
        inverse_principal_inertia: words(r)?,
        principal_frame: words(r)?,
    })
}
fn mass(r: &mut Reader<'_>) -> Result<BodyMass> {
    let world_center = words(r)?;
    let effective_inverse_mass = words(r)?;
    let effective_inverse_inertia = words(r)?;
    let local = local_mass(r)?;
    let locked_axes = r.word()?;
    let tag = r.word()?;
    let data: [u32; 11] = words(r)?;
    let additional = match tag {
        0 if data == [0; 11] => None,
        1 if data[1..] == [0; 10] => Some(AdditionalMass::Mass(data[0])),
        2 => Some(AdditionalMass::Properties(LocalMass {
            center: [data[0], data[1], data[2]],
            inverse_mass: data[3],
            inverse_principal_inertia: [data[4], data[5], data[6]],
            principal_frame: [data[7], data[8], data[9], data[10]],
        })),
        _ => return Err(Error::Header),
    };
    Ok(BodyMass {
        world_center,
        effective_inverse_mass,
        effective_inverse_inertia,
        local,
        locked_axes,
        additional,
        max_extent: r.word()?,
    })
}
fn row(r: &mut Reader<'_>) -> Result<RigidBodyState> {
    let position = pose(r)?;
    let next_position = pose(r)?;
    let retained_velocity = velocity(r)?;
    let ccd_velocity = velocity(r)?;
    let forces = BodyForces {
        force: words(r)?,
        torque: words(r)?,
        gravity_scale: r.word()?,
        user_force: words(r)?,
        user_torque: words(r)?,
        gyroscopic: boolean(r)?,
    };
    let damping = words(r)?;
    let mass = mass(r)?;
    let activation = BodyActivation {
        linear_threshold: r.word()?,
        angular_threshold: r.word()?,
        time_until_sleep: r.word()?,
        time_since_can_sleep: r.word()?,
        sleeping: boolean(r)?,
        previous_pose: pose(r)?,
    };
    let ccd = BodyCcd {
        thickness: r.word()?,
        active: boolean(r)?,
        enabled: boolean(r)?,
        soft_prediction: r.word()?,
        allow_fast_rotation: boolean(r)?,
    };
    let ids = words(r)?;
    let changes = r.word()?;
    let kind = match r.word()? {
        0 => BodyKind::Dynamic,
        1 => BodyKind::Fixed,
        2 => BodyKind::KinematicPosition,
        3 => BodyKind::KinematicVelocity,
        _ => return Err(Error::UnsupportedBody),
    };
    let dominance = r.word()? as i32;
    let enabled = boolean(r)?;
    let a: u64 = u64::from(r.word()?) | (u64::from(r.word()?) << 32);
    let b: u64 = u64::from(r.word()?) | (u64::from(r.word()?) << 32);
    Ok(RigidBodyState {
        position,
        next_position,
        velocity: retained_velocity,
        ccd_velocity,
        forces,
        damping,
        mass,
        activation,
        ccd,
        ids,
        changes,
        kind,
        dominance,
        enabled,
        additional_iterations: [a, b],
        soft_body: Handle {
            slot: r.word()?,
            generation: r.word()?,
        },
        soft_cluster: r.word()?,
        soft_motion_margin: r.word()?,
        user_data: words(r)?,
    })
}
pub fn decode_body_state<'a>(
    bytes: &[u8],
    c: BodyContext<'a>,
    budget: &mut Budget,
) -> Result<ValidatedBodyState<'a>> {
    if !(HEADER..=MAX_BYTES).contains(&bytes.len()) {
        return Err(Error::Length);
    }
    let mut r = Reader { bytes, position: 0 };
    let identity = c.attachments.record().identity;
    read_identity(&mut r, MAGIC, identity)?;
    let n = r.word()? as usize;
    if r.word()? != 0 {
        return Err(Error::Header);
    }
    context(identity, n, c)?;
    if bytes.len() != frame_length(HEADER, STRIDE, n)? {
        return Err(Error::Length);
    }
    budget.charge(
        n.checked_mul(size_of::<RigidBodyState>())
            .ok_or(Error::Limit)?,
    )?;
    let mut bodies = vector(n)?;
    for _ in 0..n {
        bodies.push(row(&mut r)?);
    }
    validate_body_state(BodyStateTable { identity, bodies }, c)
}
fn put_pose(p: LocalPose, put: &mut impl FnMut(u32)) {
    for v in p.translation.into_iter().chain(p.rotation) {
        put(v);
    }
}
fn put_velocity(v: BodyVelocity, put: &mut impl FnMut(u32)) {
    for b in v.linear.into_iter().chain(v.angular) {
        put(b);
    }
}
fn put_local(m: LocalMass, put: &mut impl FnMut(u32)) {
    for v in m
        .center
        .into_iter()
        .chain([m.inverse_mass])
        .chain(m.inverse_principal_inertia)
        .chain(m.principal_frame)
    {
        put(v);
    }
}
fn put_mass(m: BodyMass, put: &mut impl FnMut(u32)) {
    for v in m
        .world_center
        .into_iter()
        .chain(m.effective_inverse_mass)
        .chain(m.effective_inverse_inertia)
    {
        put(v);
    }
    put_local(m.local, put);
    put(m.locked_axes);
    match m.additional {
        None => {
            put(0);
            for _ in 0..11 {
                put(0);
            }
        }
        Some(AdditionalMass::Mass(v)) => {
            put(1);
            put(v);
            for _ in 0..10 {
                put(0);
            }
        }
        Some(AdditionalMass::Properties(v)) => {
            put(2);
            put_local(v, put);
        }
    }
    put(m.max_extent);
}
pub fn encode_body_state(v: &ValidatedBodyState<'_>, budget: &mut Budget) -> Result<Vec<u8>> {
    let r = v.record();
    let n = frame_length(HEADER, STRIDE, r.bodies.len())?;
    budget.charge(n)?;
    let mut out = vector(n)?;
    write_identity(&mut out, MAGIC, r.identity);
    let mut put = |v: u32| out.extend_from_slice(&v.to_le_bytes());
    put(r.bodies.len() as u32);
    put(0);
    for b in &r.bodies {
        put_pose(b.position, &mut put);
        put_pose(b.next_position, &mut put);
        put_velocity(b.velocity, &mut put);
        put_velocity(b.ccd_velocity, &mut put);
        for v in b
            .forces
            .force
            .into_iter()
            .chain(b.forces.torque)
            .chain([b.forces.gravity_scale])
            .chain(b.forces.user_force)
            .chain(b.forces.user_torque)
        {
            put(v);
        }
        put(u32::from(b.forces.gyroscopic));
        for v in b.damping {
            put(v);
        }
        put_mass(b.mass, &mut put);
        for v in [
            b.activation.linear_threshold,
            b.activation.angular_threshold,
            b.activation.time_until_sleep,
            b.activation.time_since_can_sleep,
            u32::from(b.activation.sleeping),
        ] {
            put(v);
        }
        put_pose(b.activation.previous_pose, &mut put);
        for v in [
            b.ccd.thickness,
            u32::from(b.ccd.active),
            u32::from(b.ccd.enabled),
            b.ccd.soft_prediction,
            u32::from(b.ccd.allow_fast_rotation),
        ] {
            put(v);
        }
        for v in b.ids {
            put(v);
        }
        put(b.changes);
        put(b.kind as u32);
        put(b.dominance as u32);
        put(u32::from(b.enabled));
        for v in b.additional_iterations {
            put(v as u32);
            put((v >> 32) as u32);
        }
        put(b.soft_body.slot);
        put(b.soft_body.generation);
        put(b.soft_cluster);
        put(b.soft_motion_margin);
        for v in b.user_data {
            put(v);
        }
    }
    Ok(out)
}

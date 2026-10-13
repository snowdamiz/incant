use super::{
    validate::{HEADER, STRIDE, context},
    *,
};
use crate::{
    Budget, Error, MAX_BYTES, Result,
    arena_codec::{frame_length, read_identity, write_identity},
    codec::Reader,
    vector,
};
const MAGIC: &[u8; 8] = b"ICCOLL01";
fn words<const N: usize>(r: &mut Reader<'_>) -> Result<[u32; N]> {
    let mut v = [0; N];
    for w in &mut v {
        *w = r.word()?;
    }
    Ok(v)
}
fn rule(r: &mut Reader<'_>) -> Result<CombineRule> {
    match r.word()? {
        0 => Ok(CombineRule::Average),
        1 => Ok(CombineRule::Min),
        2 => Ok(CombineRule::Multiply),
        3 => Ok(CombineRule::Max),
        _ => Err(Error::Collider),
    }
}
fn groups(r: &mut Reader<'_>) -> Result<ColliderGroups> {
    Ok(ColliderGroups {
        memberships: r.word()?,
        filter: r.word()?,
        test: match r.word()? {
            0 => GroupTest::And,
            1 => GroupTest::Or,
            _ => return Err(Error::Collider),
        },
    })
}
fn row(r: &mut Reader<'_>) -> Result<ColliderState> {
    let kind = match r.word()? {
        0 => ColliderKind::Solid,
        1 => ColliderKind::Sensor,
        _ => return Err(Error::Collider),
    };
    let position = LocalPose {
        translation: words(r)?,
        rotation: words(r)?,
    };
    let tag = r.word()?;
    let m: [u32; 11] = words(r)?;
    let mass = match tag {
        0 if m[1..] == [0; 10] => ColliderMass::Density(m[0]),
        1 if m[1..] == [0; 10] => ColliderMass::Mass(m[0]),
        2 => ColliderMass::Properties(LocalMass {
            center: [m[0], m[1], m[2]],
            inverse_mass: m[3],
            inverse_principal_inertia: [m[4], m[5], m[6]],
            principal_frame: [m[7], m[8], m[9], m[10]],
        }),
        _ => return Err(Error::Collider),
    };
    let changes = r.word()?;
    let material = ColliderMaterial {
        friction: r.word()?,
        restitution: r.word()?,
        friction_rule: rule(r)?,
        restitution_rule: rule(r)?,
    };
    let active_collision_types = r.word()?;
    let collision_groups = groups(r)?;
    let solver_groups = groups(r)?;
    let active_hooks = r.word()?;
    let active_events = r.word()?;
    let enabled = match r.word()? {
        0 => ColliderEnabled::Enabled,
        1 => ColliderEnabled::DisabledByParent,
        2 => ColliderEnabled::Disabled,
        _ => return Err(Error::Collider),
    };
    let contact_skin = r.word()?;
    let force_event_threshold = r.word()?;
    let deformable = match r.word()? {
        0 => false,
        1 => true,
        _ => return Err(Error::Header),
    };
    Ok(ColliderState {
        kind,
        position,
        mass,
        changes,
        material,
        active_collision_types,
        collision_groups,
        solver_groups,
        active_hooks,
        active_events,
        enabled,
        contact_skin,
        force_event_threshold,
        deformable,
        user_data: words(r)?,
    })
}
pub fn decode_collider_state<'a>(
    bytes: &[u8],
    c: ColliderContext<'a>,
    budget: &mut Budget,
) -> Result<ValidatedColliderState<'a>> {
    if !(HEADER..=MAX_BYTES).contains(&bytes.len()) {
        return Err(Error::Length);
    }
    let mut r = Reader { bytes, position: 0 };
    let identity = c.bodies.record().identity;
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
        n.checked_mul(size_of::<ColliderState>())
            .ok_or(Error::Limit)?,
    )?;
    let mut colliders = vector(n)?;
    for _ in 0..n {
        colliders.push(row(&mut r)?);
    }
    debug_assert_eq!(r.position, bytes.len());
    validate_collider_state(
        ColliderStateTable {
            identity,
            colliders,
        },
        c,
    )
}
pub fn encode_collider_state(
    v: &ValidatedColliderState<'_>,
    budget: &mut Budget,
) -> Result<Vec<u8>> {
    let r = v.record();
    let n = frame_length(HEADER, STRIDE, r.colliders.len())?;
    budget.charge(n)?;
    let mut out = vector(n)?;
    write_identity(&mut out, MAGIC, r.identity);
    let mut put = |v: u32| out.extend_from_slice(&v.to_le_bytes());
    put(r.colliders.len() as u32);
    put(0);
    for c in &r.colliders {
        put(c.kind as u32);
        for v in c
            .position
            .translation
            .into_iter()
            .chain(c.position.rotation)
        {
            put(v);
        }
        match c.mass {
            ColliderMass::Density(v) | ColliderMass::Mass(v) => {
                put(if matches!(c.mass, ColliderMass::Density(_)) {
                    0
                } else {
                    1
                });
                put(v);
                for _ in 0..10 {
                    put(0);
                }
            }
            ColliderMass::Properties(m) => {
                put(2);
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
        }
        for v in [
            c.changes,
            c.material.friction,
            c.material.restitution,
            c.material.friction_rule as u32,
            c.material.restitution_rule as u32,
            c.active_collision_types,
        ] {
            put(v);
        }
        for g in [c.collision_groups, c.solver_groups] {
            for v in [g.memberships, g.filter, g.test as u32] {
                put(v);
            }
        }
        for v in [
            c.active_hooks,
            c.active_events,
            c.enabled as u32,
            c.contact_skin,
            c.force_event_threshold,
            c.deformable as u32,
        ] {
            put(v);
        }
        for v in c.user_data {
            put(v);
        }
    }
    debug_assert_eq!(out.len(), n);
    Ok(out)
}

use super::*;
use glam::DMat3;

fn length(v: DVec3) -> IkResult<f64> {
    let largest = v.abs().max_element();
    let value = if largest == 0. {
        0.
    } else {
        (v / largest).length() * largest
    };
    if !value.is_finite() {
        return Err(TwoBoneIkError::Numeric);
    }
    Ok(value)
}
fn unit(v: DVec3) -> IkResult<DVec3> {
    let length = length(v)?;
    if length == 0. {
        return Err(TwoBoneIkError::Degenerate);
    }
    Ok(v / length)
}
fn rotation(m: DMat4) -> IkResult<DQuat> {
    if !m.is_finite() || m.row(3) != glam::DVec4::W {
        return Err(TwoBoneIkError::UnsupportedTransform);
    }
    let axes = [
        m.x_axis.truncate(),
        m.y_axis.truncate(),
        m.z_axis.truncate(),
    ];
    let lengths = [length(axes[0])?, length(axes[1])?, length(axes[2])?];
    let max = lengths.into_iter().fold(0., f64::max);
    if max == 0.
        || lengths
            .iter()
            .any(|l| *l == 0. || (l / max - 1.).abs() > 1e-6)
    {
        return Err(TwoBoneIkError::UnsupportedTransform);
    }
    let [x, y, z] = std::array::from_fn(|i| axes[i] / lengths[i]);
    if x.dot(y).abs() > 1e-6
        || x.dot(z).abs() > 1e-6
        || y.dot(z).abs() > 1e-6
        || x.cross(y).dot(z) <= 0.
    {
        return Err(TwoBoneIkError::UnsupportedTransform);
    }
    let q = DQuat::from_mat3(&DMat3::from_cols(x, y, z)).normalize();
    if !q.is_finite() {
        return Err(TwoBoneIkError::Numeric);
    }
    Ok(q)
}
fn bend(direction: DVec3, pole: DVec3, first: DVec3) -> IkResult<DVec3> {
    for candidate in [pole, first] {
        let length = length(candidate)?;
        if length == 0. {
            continue;
        }
        let candidate = candidate / length;
        let projected = candidate - direction * candidate.dot(direction);
        if projected.length() > 1e-10 {
            return unit(projected);
        }
    }
    // Least-aligned axis ensures a well-conditioned perpendicular and fixed ties.
    let axis = if direction.x.abs() <= direction.y.abs() && direction.x.abs() <= direction.z.abs() {
        DVec3::X
    } else if direction.y.abs() <= direction.z.abs() {
        DVec3::Y
    } else {
        DVec3::Z
    };
    unit(axis - direction * axis.dot(direction))
}
fn swing(from: DVec3, to: DVec3, normal: DVec3) -> IkResult<DQuat> {
    let cross = from.cross(to);
    let dot = from.dot(to).clamp(-1., 1.);
    // Resolve the ambiguous exact half-turn in the selected bend plane rather
    // than introducing an unrelated twist axis. atan2 preserves near-half turns.
    let axis = if length(cross)? > 1e-14 {
        unit(cross)?
    } else if dot >= 0. {
        return Ok(DQuat::IDENTITY);
    } else {
        unit(normal - from * normal.dot(from))?
    };
    Ok(DQuat::from_axis_angle(axis, length(cross)?.atan2(dot)))
}
fn weighted_delta(from: DQuat, to: DQuat, weight: f64) -> DQuat {
    let mut delta = (to * from.conjugate()).normalize();
    // At a half-turn, quaternion products can leave either sign of tiny w.
    // Do not let that roundoff arbitrarily reverse an intermediate weighted arc.
    // Canonical XYZ sign breaks this geometrically ambiguous tie explicitly.
    if delta.w.abs() <= 1e-12 {
        delta.w = 0.;
        if [delta.x, delta.y, delta.z]
            .into_iter()
            .find(|v| v.abs() > 1e-12)
            .is_some_and(|v| v < 0.)
        {
            delta = -delta;
        }
        delta = delta.normalize();
    } else if delta.w < 0. {
        delta = -delta;
    }
    DQuat::IDENTITY.slerp(delta, weight).normalize()
}

pub(super) fn solve(
    c: &TwoBoneIk,
    parent: DMat4,
    root_local: DMat4,
    mid_local: DMat4,
    root: DMat4,
    mid: DMat4,
    tip: DVec3,
) -> IkResult<[DQuat; 2]> {
    let parent_q = rotation(parent)?;
    let root_local_q = rotation(root_local)?;
    let mid_local_q = rotation(mid_local)?;
    let root_q = rotation(root)?;
    let mid_q = rotation(mid)?;
    let a = root.w_axis.truncate();
    let b = mid.w_axis.truncate();
    let first = b - a;
    let second = tip - b;
    let l1 = length(first)?;
    let l2 = length(second)?;
    let total = l1 + l2;
    if l1 == 0. || l2 == 0. {
        return Err(TwoBoneIkError::Degenerate);
    }
    if !total.is_finite() {
        return Err(TwoBoneIkError::Numeric);
    }
    let target = DVec3::from_array(c.target) - a;
    let distance = length(target)?;
    let direction = if distance > 0. {
        target / distance
    } else if length(tip - a)? > 0. {
        unit(tip - a)?
    } else {
        unit(first)?
    };
    let side = bend(direction, DVec3::from_array(c.pole) - a, first)?;
    // Scale first so squaring a valid long bone never overflows. At min reach
    // an unequal chain folds back; at zero/equal reach the knee lies on side.
    let radius = distance.clamp((l1 - l2).abs(), total);
    let (u, v, d) = (l1 / total, l2 / total, radius / total);
    let x = if d == 0. {
        0.
    } else {
        (((u - v) * (u + v) + d * d) / (2. * d)).clamp(-u, u)
    };
    let h = ((u - x) * (u + x)).max(0.).sqrt();
    let desired_first = (direction * x + side * h) * total;
    let desired_second = direction * radius - desired_first;
    let normal = unit(direction.cross(side))?;
    let root_delta = swing(unit(first)?, unit(desired_first)?, normal)?;
    let mid_delta = swing(unit(root_delta * second)?, unit(desired_second)?, normal)?;
    let root_final = (parent_q.conjugate() * root_delta * root_q).normalize();
    let mid_final =
        ((root_delta * root_q).conjugate() * mid_delta * root_delta * mid_q).normalize();
    let out = [
        weighted_delta(root_local_q, root_final, c.weight),
        weighted_delta(mid_local_q, mid_final, c.weight),
    ];
    if out.iter().any(|q| !q.is_finite()) {
        return Err(TwoBoneIkError::Numeric);
    }
    Ok(out)
}

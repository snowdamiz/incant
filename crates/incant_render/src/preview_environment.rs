//! Default procedural studio environment for the material preview (handoff 0016).
//!
//! Direction-only and infinitely distant: it describes what a surface sees, not
//! scene geometry. Input is a world-space direction with +Y up; output is linear
//! Rec.709 radiance, finite, nonnegative and at most `MAX_RADIANCE` per channel.
//! Prefiltering, BRDF terms, GPU resources and tone mapping live elsewhere.
//!
//! Intent: a neutral photographic cove. A dim, nearly flat upper hemisphere
//! (the zenith a hair cooler than the horizon) meets a darker floor through a
//! soft horizon band, so metals show a clear sky/ground split and diffuse
//! shapes read grounded. Tints stay under 3% relative chroma, so neutral
//! materials do not pick up a cast. Three soft-edged panels give metals and
//! glossy dielectrics something to reflect:
//!
//! - a key softbox around the directional key ([1,2,3]) so reflected and direct
//!   highlights agree; from the fixed camera it sits near the view direction;
//! - a broad, dim fill card on the camera's right, opposite the key;
//! - a long horizontal rim strip behind and above the subject for edge reads.
//!
//! Rectangular panels show roughness as edge blur. Every edge ramp spans about
//! 10 degrees or more (about 14 px on a 128 px, 90-degree cubemap face), and the
//! narrowest panel, the rim strip, is about 25 degrees tall including its ramps.
//! The largest step between neighboring texels of a 128 px face is about 0.24,
//! so no feature is a hard one-texel edge and each blurs progressively through
//! the rougher prefiltered levels.
//!
//! Panel hierarchy (tuned on the first GPU captures): on an 0.8 metal the key
//! panel reflects at about 1.35 linear, onto the tone-map shoulder (sRGB ~249),
//! while the fill reflects at about 0.6, below the shoulder (sRGB ~202). A
//! brighter fill landed within a few sRGB levels of the key after shoulder
//! compression and competed with it.
//!
//! Diffuse budget: the old constant fill was 0.3 and is replaced, not added to.
//! The cosine-weighted mean radiance of this environment, which is the value
//! that replaces that fill, averages about 0.30 over all normals: about 0.42
//! facing up, 0.25 facing away from the key and 0.15 facing down. The key
//! panel slightly reinforces the directional key's diffuse; that is deliberate.
//!
//! The floor is part of this distant environment only. There is no floor
//! geometry, shadowing or visibility; reflected floor is never an occluder.

/// Upper bound of every returned channel.
pub(crate) const MAX_RADIANCE: f32 = 16.0;

const ZENITH: [f32; 3] = [0.27, 0.273, 0.276];
const HORIZON: [f32; 3] = [0.29, 0.29, 0.29];
const FLOOR: [f32; 3] = [0.15, 0.148, 0.146];

struct Panel {
    /// Direction to the panel center, not necessarily normalized.
    center: [f32; 3],
    /// Half extents in the panel plane at unit distance (tangent units).
    half_width: f32,
    half_height: f32,
    /// Half width of the smooth edge transition, in the same units.
    softness: f32,
    radiance: [f32; 3],
}

const PANELS: [Panel; 3] = [
    // Key softbox, aligned with the directional key light.
    Panel {
        center: [1.0, 2.0, 3.0],
        half_width: 0.34,
        half_height: 0.24,
        softness: 0.09,
        radiance: [1.4, 1.4, 1.4],
    },
    // Fill card on the camera's right, low and broad.
    Panel {
        center: [0.95, 0.3, -0.35],
        half_width: 0.55,
        half_height: 0.45,
        softness: 0.2,
        radiance: [0.45, 0.45, 0.45],
    },
    // Rim strip behind and above the subject, wide and short.
    Panel {
        center: [-0.55, 0.45, -0.75],
        half_width: 0.75,
        half_height: 0.13,
        softness: 0.09,
        radiance: [1.6, 1.6, 1.6],
    },
];

/// Studio radiance seen along `direction` (world space, +Y up).
///
/// Non-normalized input is normalized; zero-length or non-finite input returns
/// the horizon value so callers always receive finite, bounded radiance.
pub(crate) fn radiance(direction: [f32; 3]) -> [f32; 3] {
    let Some(d) = normalize(direction) else {
        return HORIZON;
    };
    let mut color = backdrop(d[1]);
    for panel in &PANELS {
        let weight = panel_weight(panel, d);
        if weight > 0.0 {
            for (channel, emitted) in color.iter_mut().zip(panel.radiance) {
                *channel += weight * emitted;
            }
        }
    }
    color.map(|channel| {
        if channel.is_finite() {
            channel.clamp(0.0, MAX_RADIANCE)
        } else {
            0.0
        }
    })
}

/// Ceiling-to-floor gradient with a soft horizon band about 14 degrees wide.
fn backdrop(up: f32) -> [f32; 3] {
    let sky = up.max(0.0).sqrt();
    let upper = lerp3(HORIZON, ZENITH, sky);
    let ground = smoothstep(-0.12, 0.12, up);
    lerp3(FLOOR, upper, ground)
}

/// Smooth rectangular mask of a planar panel, with a gentle center hotspot.
fn panel_weight(panel: &Panel, d: [f32; 3]) -> f32 {
    let Some(normal) = normalize(panel.center) else {
        return 0.0;
    };
    let facing = dot(d, normal);
    if facing <= 0.05 {
        return 0.0;
    }
    // Panel axes: horizontal tangent first; panels are never placed at a pole.
    let Some(right) = normalize(cross([0.0, 1.0, 0.0], normal)) else {
        return 0.0;
    };
    let up = cross(normal, right);
    // Gnomonic projection onto the panel plane at unit distance.
    let x = dot(d, right) / facing;
    let y = dot(d, up) / facing;
    let mask_x = 1.0
        - smoothstep(
            panel.half_width - panel.softness,
            panel.half_width + panel.softness,
            x.abs(),
        );
    let mask_y = 1.0
        - smoothstep(
            panel.half_height - panel.softness,
            panel.half_height + panel.softness,
            y.abs(),
        );
    let r2 = (x / panel.half_width).powi(2) + (y / panel.half_height).powi(2);
    let hotspot = 1.0 - 0.2 * r2.min(1.0);
    mask_x * mask_y * hotspot
}

fn normalize(v: [f32; 3]) -> Option<[f32; 3]> {
    // Prescale by the largest component so huge or tiny inputs neither
    // overflow nor underflow; NaN and infinity are rejected.
    let largest = v[0].abs().max(v[1].abs()).max(v[2].abs());
    if !largest.is_finite() || largest <= 0.0 || v.iter().any(|c| c.is_nan()) {
        return None;
    }
    let scaled = v.map(|c| c / largest);
    let length = dot(scaled, scaled).sqrt();
    Some(scaled.map(|c| c / length))
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn lerp3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

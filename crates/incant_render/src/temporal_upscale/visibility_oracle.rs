//! Geometry and measured-sample oracle. No resolver, lock, or history color math.
use super::raster_geometry::{self as geometry, EDGE, Point, STRIP};
use crate::motion::visibility::Sample;
use serde_json::{Value, json};
pub(super) const OCCLUDER: [Point; 4] = [[4., 42.], [57., 42.], [57., 48.], [4., 48.]];
const CHANGED: u32 = 1 << 31;
const POSITION_EPSILON: f64 = 2e-5;
const DEPTH_MOTION_EPSILON: f64 = 2e-6;

pub(super) fn calibration(samples: &[Sample]) -> Result<[Point; 4], &'static str> {
    if samples.len() != 64 * 64 * 4 {
        return Err("calibration length");
    }
    let pattern = std::array::from_fn(|s| {
        [
            f64::from(samples[s].position_depth[0]),
            f64::from(samples[s].position_depth[1]),
        ]
    });
    for (i, sample) in samples.iter().enumerate() {
        let pixel = i / 4;
        let lane = i % 4;
        let xy = [
            f64::from(sample.position_depth[0]),
            f64::from(sample.position_depth[1]),
        ];
        let local = [xy[0] - (pixel % 64) as f64, xy[1] - (pixel / 64) as f64];
        if sample.owner != [lane as u32 + 1, 0, 0, 0]
            || sample.motion != [0.; 4]
            || sample.position_depth[2] != 0.5
            || sample.position_depth[3] != lane as f32
            || local
                .iter()
                .any(|v| !v.is_finite() || !(0. ..1.).contains(v))
            || local
                .iter()
                .zip(pattern[lane])
                .any(|(a, b)| (a - b).abs() > POSITION_EPSILON)
        {
            return Err("sample index/position calibration");
        }
    }
    for (i, a) in pattern.iter().enumerate() {
        if pattern[..i]
            .iter()
            .any(|b| (a[0] - b[0]).hypot(a[1] - b[1]) < 0.01)
        {
            return Err("distinct sample positions");
        }
    }
    Ok(pattern)
}

#[derive(Clone, Copy, Default)]
pub(super) struct Pose {
    pub mesh_y: f64,
    pub camera_y: f64,
    pub occluder_y: f64,
    pub occluder_z: f64,
    pub removed: bool,
}
impl Pose {
    pub fn initial() -> Self {
        Self {
            occluder_y: 20.,
            occluder_z: 1.,
            ..Default::default()
        }
    }
    fn shift(self, occluder: bool) -> f64 {
        -8. * (if occluder {
            self.occluder_y
        } else {
            self.mesh_y
        } - self.camera_y)
    }
}
pub(super) struct Scene {
    pub current: Pose,
    pub previous: Pose,
    pub words: [u32; 2],
    pub has_previous: bool,
}
pub(super) fn word(id: u32, changed: bool) -> u32 {
    assert!(id > 0 && id < CHANGED);
    id | if changed { CHANGED } else { 0 }
}
fn expected(scene: &Scene, point: Point) -> Option<usize> {
    let p = [point[0], point[1] - scene.current.shift(true)];
    match geometry::contains(p, &OCCLUDER) {
        Some(true) => Some(1),
        None => None,
        Some(false) if scene.current.removed => Some(2),
        Some(false) => {
            let p = [point[0], point[1] - scene.current.shift(false)];
            match (geometry::contains(p, &EDGE), geometry::contains(p, &STRIP)) {
                (Some(true), _) | (_, Some(true)) => Some(0),
                (Some(false), Some(false)) => Some(2),
                _ => None,
            }
        }
    }
}
/// Returns mismatches instead of asserting so all raw inputs and diagnostics
/// survive a native failure. Boundary ties are listed, never silently widened.
pub(super) fn inspect(samples: &[Sample], pattern: [Point; 4], scene: &Scene) -> Value {
    assert_eq!(samples.len(), 64 * 64 * 4);
    let mut errors = Vec::new();
    let mut ties = Vec::new();
    let mut covered = [0usize; 2];
    for (i, s) in samples.iter().enumerate() {
        let pixel = i / 4;
        let lane = i % 4;
        let xy = [
            (pixel % 64) as f64 + pattern[lane][0],
            (pixel / 64) as f64 + pattern[lane][1],
        ];
        if s.motion
            .iter()
            .chain(s.position_depth.iter())
            .any(|v| !v.is_finite())
            || s.position_depth[3] != lane as f32
            || s.owner[1..] != [0; 3]
        {
            errors.push(json!([pixel % 64, pixel / 64, lane, "invalid record"]));
            continue;
        }
        let Some(owner) = expected(scene, xy) else {
            ties.push([pixel % 64, pixel / 64, lane]);
            continue;
        };
        if owner == 2 {
            if s.owner[0] != 0 || s.motion != [0.; 4] || s.position_depth[..3] != [0., 0., 1.] {
                errors.push(json!([
                    pixel % 64,
                    pixel / 64,
                    lane,
                    "invented visible sample"
                ]));
            }
            continue;
        }
        covered[owner] += 1;
        let z = if owner == 1 {
            scene.current.occluder_z
        } else {
            0.
        };
        let previous_z = if owner == 1 {
            scene.previous.occluder_z
        } else {
            0.
        };
        let depth = (10. - z - 0.1) / (50. - 0.1);
        let previous_xy = [
            xy[0],
            xy[1] + scene.previous.shift(owner == 1) - scene.current.shift(owner == 1),
        ];
        let valid = scene.has_previous && previous_xy.iter().all(|v| (0. ..64.).contains(v));
        let motion = if valid {
            [
                0.,
                (previous_xy[1] - xy[1]) / 64.,
                (10. - previous_z - 0.1) / (50. - 0.1),
                1.,
            ]
        } else {
            [0.; 4]
        };
        let valid_sample = s.owner[0] == scene.words[owner]
            && (f64::from(s.position_depth[2]) - depth).abs() < DEPTH_MOTION_EPSILON
            && s.position_depth[..2]
                .iter()
                .zip(xy)
                .all(|(a, b)| (f64::from(*a) - b).abs() < POSITION_EPSILON)
            && s.motion
                .iter()
                .zip(motion)
                .all(|(a, b)| (f64::from(*a) - b).abs() < DEPTH_MOTION_EPSILON);
        if !valid_sample {
            errors.push(json!([
                pixel % 64,
                pixel / 64,
                lane,
                "owner/depth/previous correspondence"
            ]));
        }
    }
    json!({"covered_samples":covered,"tie_samples":ties,"mismatches":errors})
}
pub(super) fn assert_inspection(report: &Value) {
    assert_eq!(
        report["mismatches"].as_array().unwrap().len(),
        0,
        "{report}"
    );
}

pub(super) fn static_metrics(samples: &[Sample], mesh: u32) -> Value {
    let halos: Value = serde_json::from_str(include_str!("visibility_halos.json")).unwrap();
    let rows = halos["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 148);
    let mut invented = Vec::new();
    for row in rows {
        let x = row["x"].as_u64().unwrap() as usize;
        let y = row["y"].as_u64().unwrap() as usize;
        assert_eq!(geometry::area(&EDGE, x, y), 0.);
        if samples[(y * 64 + x) * 4..(y * 64 + x + 1) * 4]
            .iter()
            .any(|s| s.owner[0] != 0)
        {
            invented.push(row.clone());
        }
    }
    let mut missing = Vec::new();
    let mut area_error = 0.;
    let mut area_mass = 0.;
    let mut rows = Vec::new();
    for x in 6..55 {
        let area = geometry::area(&STRIP, x, 44);
        assert!(area > 0.);
        let n = samples[(44 * 64 + x) * 4..(44 * 64 + x + 1) * 4]
            .iter()
            .filter(|s| s.owner[0] & !CHANGED == mesh)
            .count();
        let coverage = n as f64 / 4.;
        area_error += (coverage - area).abs();
        area_mass += coverage;
        if n == 0 {
            missing.push([x, 44]);
        }
        rows.push(json!({"x":x,"y":44,"analytic_area":area,"samples":n}));
    }
    json!({"halo_rows_checked":148,"invented_halo_rows":invented,"strip_49_pixels":rows,
        "strip_missed_analytic_slivers":missing,"strip_absolute_area_error":area_error,"strip_sampled_area_mass":area_mass,
        "analytic_area":12.,"visibility_is_not_radiance":true})
}

#[test]
fn visibility_calibration_rejects_pixel_center_broadcast_and_wrong_indices() {
    let make = |pattern: [Point; 4]| {
        (0..64 * 64 * 4)
            .map(|i| Sample {
                motion: [0.; 4],
                owner: [i as u32 % 4 + 1, 0, 0, 0],
                position_depth: [
                    (i / 4 % 64) as f32 + pattern[i % 4][0] as f32,
                    (i / 4 / 64) as f32 + pattern[i % 4][1] as f32,
                    0.5,
                    (i % 4) as f32,
                ],
            })
            .collect::<Vec<_>>()
    };
    let pattern = [[0.125, 0.25], [0.375, 0.5], [0.625, 0.75], [0.875, 0.125]];
    assert_eq!(calibration(&make(pattern)).unwrap(), pattern);
    assert!(calibration(&make([[0.5, 0.5]; 4])).is_err());
    let mut bad = make(pattern);
    bad[17].owner[0] = 1;
    assert!(calibration(&bad).is_err());
    let mut bad = make(pattern);
    bad[500].position_depth[0] += 0.25;
    assert!(calibration(&bad).is_err());
}
#[test]
fn visibility_geometry_distinguishes_current_owner_from_previous_validity() {
    let scene = Scene {
        current: Pose::initial(),
        previous: Pose::initial(),
        words: [word(1, true), word(2, false)],
        has_previous: false,
    };
    assert_eq!(expected(&scene, [24., 44.875]), Some(0));
    assert_eq!(expected(&scene, [24., 44.5]), Some(2));
    let moved = Scene {
        current: Pose {
            mesh_y: 1.,
            ..Pose::initial()
        },
        ..scene
    };
    assert_eq!(expected(&moved, [24., 44.875]), Some(2));
    let occluded = Scene {
        current: Pose {
            occluder_y: 0.,
            ..Pose::initial()
        },
        ..moved
    };
    assert_eq!(expected(&occluded, [24., 44.875]), Some(1));
    assert_eq!(word(1, true) & !CHANGED, word(1, false));
}
#[test]
fn visibility_halo_fixture_retains_every_zero_area_sample() {
    let rows: Value = serde_json::from_str(include_str!("visibility_halos.json")).unwrap();
    let rows = rows["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 148);
    let mut keys = std::collections::BTreeSet::new();
    for row in rows {
        let frame = row["frame"].as_u64().unwrap();
        let x = row["x"].as_u64().unwrap() as usize;
        let y = row["y"].as_u64().unwrap() as usize;
        assert!((9..=16).contains(&frame));
        assert!(keys.insert((frame, x, y)));
        assert_eq!(geometry::area(&EDGE, x, y), 0.);
    }
}

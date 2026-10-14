//! N8's reviewed geometry fixes the ROI before observing the output. These C2
//! model checks cover the authored row and six neighbouring rows; they do not
//! establish a GPU or appearance result. Earlier R1/R3 failures remain archived.
use super::{quality_geometry::*, quality_oracle::*};
use glam::DVec2;

#[test]
fn exact_stripe_roi_and_adjacent_rows_meet_c2_model_bound() {
    for orthographic in [false, true] {
        let grid = fixture("stripes_gate", orthographic);
        let receiver_point = |x: usize, y: usize| {
            let (origin, direction) = grid.camera.ray(DVec2::new(x as f64 + 0.5, y as f64 + 0.5));
            origin + direction * ((-4. - origin.z) / direction.z)
        };
        let xs: Vec<_> = (0..grid.size[0])
            .filter(|&x| (-0.15..0.15).contains(&receiver_point(x, 0).x))
            .collect();
        assert!(!xs.is_empty());
        let row = (0..grid.size[1])
            .find(|&y| {
                xs.iter().all(|&x| {
                    receiver_point(x, y).y < -0.5
                        && grid.interior(x, y, 2)
                        && grid.at(x as isize, y as isize).unwrap().1.owner == 0
                })
            })
            .unwrap();
        assert_eq!(
            (row, xs[0], *xs.last().unwrap()),
            if orthographic {
                (110, 150, 156)
            } else {
                (124, 148, 158)
            }
        );
        let old = raw(&grid, OLD);
        let candidate = filtered(&grid, &raw(&grid, REPAIRED), FILTER);
        let peak = |values: &[f64], y: usize| {
            let samples = xs.iter().map(|&x| values[y * grid.size[0] + x]);
            samples.clone().reduce(f64::max).unwrap() - samples.reduce(f64::min).unwrap()
        };
        assert_eq!(peak(&old, row), if orthographic { 0.3125 } else { 0.125 });
        for y in row..row + 7 {
            let before = peak(&old, y);
            let after = peak(&candidate, y);
            let passes = before > 0. && after <= before * 0.25;
            eprintln!(
                "SSAO_N8_EXACT {}",
                serde_json::json!({
                    "orthographic":orthographic,"row":y,"x_first":xs[0],"x_last":xs.last(),
                    "old_peak":before,"candidate_peak":after,"ratio":after/before,
                    "required_ratio":0.25,"model_pass":passes,"production_admission":false
                })
            );
            assert!(
                passes,
                "projection={orthographic} row={y}: {after}/{before}"
            );
        }
    }
}

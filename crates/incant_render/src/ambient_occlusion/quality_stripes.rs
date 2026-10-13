//! N8's reviewed geometry fixes the ROI before observing the output. A passing
//! regression here retains the known perspective quality failure; it does not
//! accept that failure as a quality pass.
use super::{quality_geometry::*, quality_oracle::*};
use glam::DVec2;

#[test]
fn exact_stripe_roi_retains_perspective_quality_failure() {
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
        let peak = |values: &[f64]| {
            let samples = xs.iter().map(|&x| values[row * grid.size[0] + x]);
            samples.clone().reduce(f64::max).unwrap() - samples.reduce(f64::min).unwrap()
        };
        let before = peak(&old);
        let after = peak(&candidate);
        assert_eq!(before, if orthographic { 0.3125 } else { 0.125 });
        assert!((after * 255. - if orthographic { 14. } else { 12. }).abs() < 1e-10);
        let passes = after <= before * 0.25;
        assert_eq!(passes, orthographic);
        eprintln!(
            "SSAO_N8_EXACT {}",
            serde_json::json!({
                "orthographic":orthographic,"row":row,"x_first":xs[0],"x_last":xs.last(),
                "old_peak":before,"candidate_peak":after,"ratio":after/before,
                "required_ratio":0.25,"quality_pass":passes,"production_admission":false
            })
        );
    }
}

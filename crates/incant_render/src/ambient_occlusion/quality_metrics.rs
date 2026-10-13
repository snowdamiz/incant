//! Measurements of the unapproved draft gates. A reported false gate stays false;
//! these tests verify bounds/control sensitivity, not adoption of the proposal.
use super::{quality_geometry::*, quality_oracle::*};

fn steps(grid: &Grid, values: &[f64]) -> usize {
    let mut count = 0;
    for y in 0..grid.size[1] - 1 {
        for x in 0..grid.size[0] - 1 {
            if grid.interior(x, y, 2) && grid.interior(x + 1, y, 2) && grid.interior(x, y + 1, 2) {
                let i = y * grid.size[0] + x;
                count += usize::from(
                    (values[i] - values[i + 1])
                        .abs()
                        .max((values[i] - values[i + grid.size[0]]).abs())
                        >= 1. / 16.,
                );
            }
        }
    }
    count
}
fn row_step(grid: &Grid, values: &[f64]) -> f64 {
    (100..171)
        .filter(|&x| grid.interior(x, 97, 2) && grid.interior(x + 1, 97, 2))
        .map(|x| (values[97 * grid.size[0] + x] - values[97 * grid.size[0] + x + 1]).abs())
        .fold(0., f64::max)
}

#[test]
fn quality_reports_original_proposed_gates_and_discriminating_mutations() {
    for (name, ortho, claimed_baseline) in [
        ("white_slab", false, 1397),
        ("white_slab", true, 1782),
        ("edge_slab", false, 2046),
    ] {
        let grid = fixture(name, ortho);
        let old = raw(&grid, OLD);
        let input = raw(&grid, REPAIRED);
        let result = filtered(&grid, &input, FILTER);
        let no_rotation = filtered(
            &grid,
            &raw(
                &grid,
                Config {
                    rotate: false,
                    ..REPAIRED
                },
            ),
            FILTER,
        );
        let no_jitter = filtered(
            &grid,
            &raw(
                &grid,
                Config {
                    jitter: false,
                    ..REPAIRED
                },
            ),
            FILTER,
        );
        let asymmetric = filtered(
            &grid,
            &input,
            Filter {
                symmetric: false,
                ..FILTER
            },
        );
        assert_ne!(result, no_rotation);
        assert_ne!(result, no_jitter);
        assert_ne!(result, asymmetric);
        assert!(
            result
                .iter()
                .all(|a| a.is_finite() && (0.0..=1.0).contains(a))
        );
        let baseline = steps(&grid, &old);
        let count = steps(&grid, &result);
        println!(
            "SSAO_METRIC {}",
            serde_json::json!({
                "fixture":name,"orthographic":ortho,"size":SIZE,"rect":RECT,
                "N7_mask":"both edge endpoints and both forward neighbours own the same surface in Chebyshev radius 2",
                "old_raw_step_pixels":baseline,"draft_baseline":claimed_baseline,
                "candidate_step_pixels":count,"N7_at_most_one_tenth_actual":count as f64 <= baseline as f64 * 0.1,
                "N7_at_most_one_tenth_draft":count as f64 <= claimed_baseline as f64 * 0.1,
                "no_rotation_step_pixels":steps(&grid,&no_rotation),
                "filter_off_step_pixels":steps(&grid,&input.iter().copied().map(r8).collect::<Vec<_>>()),
                "old_raw_row_max":row_step(&grid,&old),"candidate_row_max":row_step(&grid,&result),
                "N6_at_most_one_over_64":row_step(&grid,&result)<=1./64.,
                "no_jitter_row_max":row_step(&grid,&no_jitter),
                "asymmetric_changed_pixels":result.iter().zip(&asymmetric).filter(|(a,b)|a!=b).count(),
            })
        );
        if name == "white_slab" && !ortho {
            let start = 97 * grid.size[0] + 100;
            let end = 97 * grid.size[0] + 172;
            println!(
                "SSAO_N6_RECORD {}",
                serde_json::json!({
                    "y":97,"x_start":100,"x_end_inclusive":171,
                    "old":&old[start..end],"candidate":&result[start..end],"jitter_off":&no_jitter[start..end],
                    "max_step_left_x":(100..171).filter(|&x|grid.interior(x,97,2)&&grid.interior(x+1,97,2)).filter(|&x|((result[97*grid.size[0]+x]-result[97*grid.size[0]+x+1]).abs()-row_step(&grid,&result)).abs()<1e-15).collect::<Vec<_>>()
                })
            );
        }
    }
}

#[test]
fn quality_reports_stripe_period_without_inventing_a_pass_from_unspecified_roi() {
    let grid = fixture("stripes", false);
    let old = raw(&grid, OLD);
    let result = filtered(&grid, &raw(&grid, REPAIRED), FILTER);
    let mutation = filtered(
        &grid,
        &raw(
            &grid,
            Config {
                rotate: false,
                ..REPAIRED
            },
        ),
        FILTER,
    );
    // One exact projected 0.3-unit period at the slab plane, starting x=0.4.
    // This line crosses occluders; draft's "behind" receiver-row placement has
    // no defined y/origin. Report coverage; do not silently discard slab samples.
    let start = grid
        .camera
        .project(glam::DVec3::new(0.4, 0., -3.))
        .unwrap()
        .x
        .ceil() as usize;
    let end = grid
        .camera
        .project(glam::DVec3::new(0.7, 0., -3.))
        .unwrap()
        .x
        .ceil() as usize;
    for y in [97, 130, 150, 170] {
        let range = y * grid.size[0] + start..y * grid.size[0] + end;
        let peak = |a: &[f64]| {
            a[range.clone()]
                .iter()
                .copied()
                .fold(f64::NEG_INFINITY, f64::max)
                - a[range.clone()]
                    .iter()
                    .copied()
                    .fold(f64::INFINITY, f64::min)
        };
        let before = peak(&old);
        let after = peak(&result);
        println!(
            "SSAO_STRIPES {}",
            serde_json::json!({"row":y,"start":start,"end":end,"old_peak":before,"candidate_peak":after,"no_rotation_peak":peak(&mutation),"N8_at_most_quarter":before>0.&&after<=before*0.25,"receiver_samples":grid.hits[range.clone()].iter().filter(|h|h.is_some_and(|q|q.owner==0)).count(),"sample_count":range.len()})
        );
    }
}

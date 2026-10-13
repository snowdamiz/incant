use super::*;
use crate::resolution::{FramePlan, ResolutionScale};

#[test]
fn physical_pyramid_and_budget_do_not_shrink_with_drs() {
    let output = [321, 193];
    let plan = Plan::new(output, 6, 8192).unwrap();
    assert_eq!(
        plan.sizes,
        [[161, 97], [81, 49], [41, 25], [21, 13], [11, 7], [6, 4]]
    );
    for numerator in 4..=16 {
        let scale = ResolutionScale::new(numerator).unwrap();
        let frame = FramePlan::new(output, [3.25, 5.5, 310.5, 180.25], None, scale).unwrap();
        let span = plan.source_span(frame.size, frame.rect, scale).unwrap();
        assert_eq!(span, output.map(|v| v as f32 * scale.fraction()));
        assert!(span[0] / plan.sizes[0][0] as f32 <= 2.);
        assert!(span[1] / plan.sizes[0][1] as f32 <= 2.);
        // The final physical edge maps to the fractional source extent, not
        // its ceil-rounded allocation; every level retains the same origin.
        assert_eq!(span.map(|v| v.ceil() as u32), frame.size);
        assert_eq!(Plan::new(output, 6, 8192).unwrap(), plan);
    }
    // A small internal attachment never admits an oversized physical pyramid.
    assert!(Plan::new([4096, 4096], 6, 8192).is_err());
    assert_eq!(
        plan.source_span(
            [80, 48],
            [0., 0., 80., 48.],
            ResolutionScale::new(4).unwrap()
        ),
        Err(BloomError::TargetSize)
    );
}

// Analytic physical rectangle: integrate its overlap with each source pixel
// cell. This keeps the light's covered area fixed across source raster grids.
fn emitter(output: [u32; 2], scale: f64, center: [f64; 2]) -> Field {
    let size = output.map(|v| (f64::from(v) * scale).ceil() as u32);
    let mut data = Vec::new();
    for y in 0..size[1] {
        for x in 0..size[0] {
            let overlap: [f64; 2] = std::array::from_fn(|axis| {
                let p = f64::from([x, y][axis]);
                let lo = (center[axis] - 6.) * scale;
                let hi = (center[axis] + 6.) * scale;
                ((p + 1.).min(hi) - p.max(lo)).max(0.)
            });
            data.push([3. * overlap[0] * overlap[1]; 3]);
        }
    }
    Field { size, data }
}

fn moments(field: &Field, output: [u32; 2]) -> [f64; 4] {
    let mut mass = 0.;
    let mut first = [0.; 2];
    let mut second = 0.;
    for y in 0..output[1] {
        for x in 0..output[0] {
            let p = [f64::from(x) + 0.5, f64::from(y) + 0.5];
            let value = field.sample([p[0] / f64::from(output[0]), p[1] / f64::from(output[1])])[0];
            mass += value;
            for axis in 0..2 {
                first[axis] += p[axis] * value;
            }
            second += (p[0] * p[0] + p[1] * p[1]) * value;
        }
    }
    let c = first.map(|v| v / mass);
    [
        mass,
        c[0],
        c[1],
        (second / mass - c[0] * c[0] - c[1] * c[1]).sqrt(),
    ]
}

#[test]
fn odd_output_fixed_light_preserves_energy_centroid_and_footprint_across_drs() {
    let output = [321, 193];
    let center = [186.55, 83.06];
    let full = emitter(output, 1., center);
    let baseline = moments(&anchored_pyramid(&full, output, 1., 6), output);
    // At full resolution the existing filter is byte-for-math unchanged.
    for (a, b) in anchored_pyramid(&full, output, 1., 6)
        .data
        .iter()
        .zip(pyramid(full, 6).data)
    {
        near(*a, b);
    }
    let mut old_max_radius_change: f64 = 0.;
    for numerator in [16, 12, 8, 4] {
        let scale = f64::from(numerator) / 16.;
        let input = emitter(output, scale, center);
        let actual = moments(&anchored_pyramid(&input, output, scale, 6), output);
        let old = moments(&pyramid(input, 6), output);
        eprintln!("DRS {numerator}/16 fixed={actual:?} old={old:?}");
        assert!(
            (actual[0] / baseline[0] - 1.).abs() < 0.01,
            "energy: {actual:?}"
        );
        assert!(
            (actual[1] - baseline[1]).hypot(actual[2] - baseline[2]) < 0.6,
            "centroid: {actual:?}"
        );
        assert!(
            (actual[3] / baseline[3] - 1.).abs() < 0.025,
            "radius: {actual:?}"
        );
        old_max_radius_change = old_max_radius_change.max((old[3] / baseline[3] - 1.).abs());
    }
    // This controlled light must expose the original changing footprint.
    assert!(old_max_radius_change > 0.25);
}

#[test]
fn fractional_last_source_pixel_contributes_only_its_physical_overlap() {
    let output = [9, 7];
    let scale = ResolutionScale::new(8).unwrap();
    let frame = FramePlan::new(output, [1.25, 0.5, 7.5, 6.25], None, scale).unwrap();
    let plan = Plan::new(output, 3, 8192).unwrap();
    let span = plan.source_span(frame.size, frame.rect, scale).unwrap();
    assert_eq!(frame.rect, [0.625, 0.25, 3.75, 3.125]);
    assert_eq!(span, [4.5, 3.5]);
    let input = Field {
        size: frame.size,
        data: (0..frame.size[0] * frame.size[1])
            .map(|i| {
                if i % frame.size[0] == 4 {
                    [2.; 3]
                } else {
                    [0.; 3]
                }
            })
            .collect(),
    };
    let first = input.resample(plan.sizes[0], span.map(f64::from));
    let mean = first.data.iter().map(|v| v[0]).sum::<f64>() / first.data.len() as f64;
    assert!((mean - 2. / 9.).abs() < 1e-12);
    // Using ceil allocation as a span would incorrectly weigh a complete texel.
    let wrong = input.resample(plan.sizes[0], frame.size.map(f64::from));
    let wrong_mean = wrong.data.iter().map(|v| v[0]).sum::<f64>() / wrong.data.len() as f64;
    assert!((wrong_mean - mean).abs() > 0.1);
}

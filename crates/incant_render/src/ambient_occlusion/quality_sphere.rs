//! Isolated 24x48 triangulated sphere model and continuous convex rise bound.
use super::quality_geometry::{Camera, Grid, Hit};
use glam::{DMat4, DVec3};

struct Triangle {
    p: [DVec3; 3],
    n: [DVec3; 3],
}
impl Triangle {
    fn hit(&self, o: DVec3, d: DVec3) -> Option<(f64, DVec3, DVec3)> {
        let e1 = self.p[1] - self.p[0];
        let e2 = self.p[2] - self.p[0];
        let h = d.cross(e2);
        let det = e1.dot(h);
        if det.abs() < 1e-14 {
            return None;
        }
        let s = o - self.p[0];
        let u = s.dot(h) / det;
        let q = s.cross(e1);
        let v = d.dot(q) / det;
        let t = e2.dot(q) / det;
        if !(0.0..=1.0).contains(&u) || v < 0. || u + v > 1. || t <= 0. {
            return None;
        }
        let mut g = e1.cross(e2).normalize();
        if g.dot(-d) < 0. {
            g = -g;
        }
        Some((
            t,
            g,
            (self.n[0] * (1. - u - v) + self.n[1] * u + self.n[2] * v).normalize(),
        ))
    }
}

fn triangles() -> Vec<Triangle> {
    let vertex = |ring: usize, segment: usize| {
        let theta = ring as f64 / 24. * std::f64::consts::PI;
        let phi = segment as f64 / 48. * std::f64::consts::TAU;
        let n = DVec3::new(
            theta.sin() * phi.cos(),
            theta.cos(),
            theta.sin() * phi.sin(),
        );
        (n * 0.6, n)
    };
    let mut result = Vec::new();
    for ring in 0..24 {
        for segment in 0..48 {
            let a = vertex(ring, segment);
            let b = vertex(ring + 1, segment);
            let c = vertex(ring + 1, segment + 1);
            let d = vertex(ring, segment + 1);
            // Same diagonal/order as the authored helper's gltf::sphere(24,48).
            for v in [[a, d, b], [d, c, b]] {
                if (v[1].0 - v[0].0).cross(v[2].0 - v[0].0).length_squared() > 1e-12 {
                    result.push(Triangle {
                        p: v.map(|x| x.0),
                        n: v.map(|x| x.1),
                    });
                }
            }
        }
    }
    result
}

fn sphere(orthographic: bool) -> Grid {
    let view = DMat4::look_at_rh(DVec3::new(0., 1.3, 3.), DVec3::ZERO, DVec3::Y);
    let tris: Vec<_> = triangles()
        .into_iter()
        .map(|t| Triangle {
            p: t.p.map(|p| view.transform_point3(p)),
            n: t.n.map(|n| view.transform_vector3(n)),
        })
        .collect();
    let camera = Camera {
        orthographic,
        rect: [0., 0., 160., 160.],
    };
    Grid::new([160, 160], camera, |pixel| {
        let (o, d) = camera.ray(pixel);
        tris.iter()
            .filter_map(|t| t.hit(o, d))
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(t, geometric, shading)| Hit {
                p: o + d * t,
                geometric,
                shading,
                owner: 0,
            })
    })
}

#[test]
fn quality_sphere_continuous_bound_and_lower_bias_witness() {
    let tris = triangles();
    assert_eq!(tris.len(), 2208);
    // For this ideal analytic-vertex mesh, interpolated n is parallel to p.
    // Any surface S lies in the radius-R ball. Therefore dot(S-p,n)<=R-|p|.
    // Each triangle lies on a plane at distance d, so |p|>=d for EVERY barycentric
    // point, not just a sampled grid. R-min(d) is conservative (not the exact
    // maximum). This proof does not absorb GPU/f32 tessellation errors.
    let mut upper = 0_f64;
    let mut witness = 0_f64;
    for t in &tris {
        let face = (t.p[1] - t.p[0]).cross(t.p[2] - t.p[0]).normalize();
        upper = upper.max(0.6 - t.p[0].dot(face).abs());
        for w in [[0.5, 0.5, 0.], [0., 0.5, 0.5], [0.5, 0., 0.5], [1. / 3.; 3]] {
            let p = t.p[0] * w[0] + t.p[1] * w[1] + t.p[2] * w[2];
            let n = (t.n[0] * w[0] + t.n[1] * w[1] + t.n[2] * w[2]).normalize();
            assert!((p.normalize() - n).length() < 1e-14);
            for s in t.p {
                witness = witness.max((s - p).dot(n));
            }
        }
    }
    assert!(upper < 0.01);
    assert!(witness > 0.001 && witness <= upper);
    println!(
        "SSAO_SPHERE_BOUND {}",
        serde_json::json!({"triangles":tris.len(),"radius":0.6,"continuous_conservative_upper":upper,"sampled_lower_witness":witness,"bias_001_admitted":upper<0.01,"bias_0001_refused":witness>0.001})
    );
}

#[test]
fn quality_isolated_smooth_shaded_convex_sphere_stays_open_in_both_projections() {
    use super::quality_oracle::*;
    for ortho in [false, true] {
        let grid = sphere(ortho);
        assert!(grid.hits.iter().filter(|h| h.is_some()).count() > 100);
        for cfg in [OLD, REPAIRED] {
            let input = raw(&grid, cfg);
            assert!(input.iter().all(|&a| a == 1.));
            assert!(filtered(&grid, &input, FILTER).iter().all(|&a| a == 1.));
        }
    }
}

//! CPU-only desktop measurements, never substituted for the mobile tier gate.
use incant_nav::{NavigationGeometry, NavigationMesh, NavigationSettings, PathRequest};
use std::{collections::BTreeMap, time::Instant};
fn wall(x: f32) -> NavigationGeometry {
    NavigationGeometry {
        vertices: vec![
            [x - 0.5, 0., -1.],
            [x + 0.5, 0., -1.],
            [x + 0.5, 0., 1.],
            [x - 0.5, 0., 1.],
            [x - 0.5, 3., -1.],
            [x + 0.5, 3., -1.],
            [x + 0.5, 3., 1.],
            [x - 0.5, 3., 1.],
        ],
        triangles: vec![
            [4, 6, 5],
            [4, 7, 6],
            [0, 1, 2],
            [0, 2, 3],
            [0, 4, 5],
            [0, 5, 1],
            [1, 5, 6],
            [1, 6, 2],
            [2, 6, 7],
            [2, 7, 3],
            [3, 7, 4],
            [3, 4, 0],
        ],
    }
}
fn stats(mut samples: Vec<f64>) -> serde_json::Value {
    samples.sort_by(f64::total_cmp);
    serde_json::json!({"samples":samples.len(),"p50_ms":samples[samples.len()/2],"p95_ms":samples[samples.len()*95/100],"max_ms":samples.last().unwrap()})
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut cases = vec![];
    for (name, extent, tile_cells) in [("one_tile", 4., 32), ("sixteen_tiles", 16., 32)] {
        let s = NavigationSettings {
            min: [-extent, -2., -extent],
            max: [extent, 5., extent],
            cell_size: 0.25,
            tile_cells,
            ..Default::default()
        };
        let floor = NavigationGeometry {
            vertices: vec![
                [-extent, 0., -extent],
                [extent, 0., -extent],
                [extent, 0., extent],
                [-extent, 0., extent],
            ],
            triangles: vec![[0, 2, 1], [0, 3, 2]],
        };
        let mut sources = BTreeMap::from([("floor".into(), floor), ("wall".into(), wall(0.))]);
        let mut nav = NavigationMesh::default();
        let full = Instant::now();
        let first = nav.rebuild(&s, &sources)?;
        let full_ms = full.elapsed().as_secs_f64() * 1000.;
        let q = PathRequest {
            start: [-extent + 1.5, 0., 0.],
            end: [extent - 1.5, 0., 0.],
            snap_distance: 1.,
            max_visited: 1000,
        };
        let (mut rebuilds, mut unchanged, mut queries, mut tiles) =
            (vec![], vec![], vec![], vec![]);
        for i in 0..120 {
            sources.insert("wall".into(), wall(if i % 2 == 0 { 0.2 } else { -0.2 }));
            let begin = Instant::now();
            let report = nav.rebuild(&s, &sources)?;
            let rebuild = begin.elapsed().as_secs_f64() * 1000.;
            let begin = Instant::now();
            let same = nav.rebuild(&s, &sources)?;
            let reuse = begin.elapsed().as_secs_f64() * 1000.;
            assert!(same.rebuilt.is_empty());
            let begin = Instant::now();
            let path = nav.find_path(&q)?.ok_or("path absent")?;
            std::hint::black_box(path);
            let query = begin.elapsed().as_secs_f64() * 1000.;
            if i >= 20 {
                rebuilds.push(rebuild);
                unchanged.push(reuse);
                queries.push(query);
                tiles.push(report.rebuilt.len());
            }
        }
        if name == "one_tile" {
            assert!(tiles.iter().all(|n| *n == 1));
        }
        cases.push(serde_json::json!({"name":name,"tiles":first.rebuilt.len(),"initial_build_ms":full_ms,"changed_tiles":tiles.iter().copied().collect::<std::collections::BTreeSet<_>>(),"rebuild":stats(rebuilds),"unchanged":stats(unchanged),"path":stats(queries)}));
    }
    println!(
        "{}",
        serde_json::json!({"os":std::env::consts::OS,"arch":std::env::consts::ARCH,"cases":cases,"mobile_gate":false})
    );
    Ok(())
}

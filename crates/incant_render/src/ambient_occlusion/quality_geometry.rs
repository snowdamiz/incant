//! Test-only analytic depth/normal inputs. No renderer or GPU readback is used.
use glam::{DVec2, DVec3};

pub const SIZE: [usize; 2] = [321, 193];
pub const RECT: [f64; 4] = [13.25, 17.5, 280.5, 160.5];

#[derive(Clone, Copy)]
pub struct Camera {
    pub orthographic: bool,
    pub rect: [f64; 4],
}
impl Camera {
    pub fn inside(self, p: DVec2) -> bool {
        let [x, y, w, h] = self.rect;
        p.x >= x && p.y >= y && p.x < x + w && p.y < y + h
    }
    pub fn ray(self, pixel: DVec2) -> (DVec3, DVec3) {
        let [x, y, w, h] = self.rect;
        let ndc = (pixel - DVec2::new(x, y)) / DVec2::new(w, h) * 2. - DVec2::ONE;
        if self.orthographic {
            (DVec3::new(ndc.x * 4. * w / h, -ndc.y * 4., 0.), -DVec3::Z)
        } else {
            (
                DVec3::ZERO,
                DVec3::new(ndc.x * 0.5_f64.tan() * w / h, -ndc.y * 0.5_f64.tan(), -1.),
            )
        }
    }
    pub fn project(self, p: DVec3) -> Option<DVec2> {
        if !p.is_finite() || -p.z < 0.1 || -p.z >= 100. {
            return None;
        }
        let [x, y, w, h] = self.rect;
        let ndc = if self.orthographic {
            DVec2::new(p.x / (4. * w / h), p.y / 4.)
        } else {
            DVec2::new(
                p.x / (-p.z * 0.5_f64.tan() * w / h),
                p.y / (-p.z * 0.5_f64.tan()),
            )
        };
        let at =
            DVec2::new(x, y) + (ndc * DVec2::new(0.5, -0.5) + DVec2::splat(0.5)) * DVec2::new(w, h);
        self.inside(at).then_some(at)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Hit {
    pub p: DVec3,
    pub geometric: DVec3,
    pub shading: DVec3,
    pub owner: usize,
}
pub struct Grid {
    pub size: [usize; 2],
    pub camera: Camera,
    pub hits: Vec<Option<Hit>>,
}
impl Grid {
    pub fn new(
        size: [usize; 2],
        camera: Camera,
        mut hit: impl FnMut(DVec2) -> Option<Hit>,
    ) -> Self {
        assert!(size[0] > 0 && size[1] > 0 && size[0] * size[1] <= 4 * 1024 * 1024);
        let hits = (0..size[1])
            .flat_map(|y| (0..size[0]).map(move |x| DVec2::new(x as f64 + 0.5, y as f64 + 0.5)))
            .map(|p| if camera.inside(p) { hit(p) } else { None })
            .collect();
        Self { size, camera, hits }
    }
    pub fn at(&self, x: isize, y: isize) -> Option<(usize, Hit)> {
        if x < 0 || y < 0 || x as usize >= self.size[0] || y as usize >= self.size[1] {
            return None;
        }
        let i = y as usize * self.size[0] + x as usize;
        self.hits[i].map(|h| (i, h))
    }
    /// Explicit Chebyshev silhouette exclusion; no unstated image mask.
    pub fn interior(&self, x: usize, y: usize, band: isize) -> bool {
        let Some((_, h)) = self.at(x as isize, y as isize) else {
            return false;
        };
        (-band..=band).all(|dy| {
            (-band..=band).all(|dx| {
                self.at(x as isize + dx, y as isize + dy)
                    .is_some_and(|(_, q)| q.owner == h.owner)
            })
        })
    }
}

#[derive(Clone, Copy)]
pub struct Rect {
    pub x: [f64; 2],
    pub y: [f64; 2],
    pub z: f64,
    pub shading: DVec3,
}
pub fn rectangles(camera: Camera, rects: &[Rect]) -> Grid {
    Grid::new(SIZE, camera, |pixel| {
        let (o, d) = camera.ray(pixel);
        rects
            .iter()
            .enumerate()
            .filter_map(|(owner, r)| {
                let p = o + d * ((r.z - o.z) / d.z);
                (p.z <= -0.1
                    && p.z > -100.
                    && p.x >= r.x[0]
                    && p.x < r.x[1]
                    && p.y >= r.y[0]
                    && p.y < r.y[1])
                    .then_some(Hit {
                        p,
                        geometric: DVec3::Z,
                        shading: r.shading,
                        owner,
                    })
            })
            .max_by(|a, b| a.p.z.total_cmp(&b.p.z))
    })
}
pub fn fixture(name: &str, orthographic: bool) -> Grid {
    let camera = Camera {
        orthographic,
        rect: RECT,
    };
    let receiver = Rect {
        x: [-8., 8.],
        y: [-6., 6.],
        z: -4.,
        shading: DVec3::Z,
    };
    let slab = Rect {
        x: [0.4, 3.],
        y: [-1.5, 1.5],
        z: -3.,
        shading: DVec3::Z,
    };
    let rects = match name {
        "flat" => vec![receiver],
        "white_slab" => vec![receiver, slab],
        "reversed_normal" => vec![
            Rect {
                shading: -DVec3::Z,
                ..receiver
            },
            slab,
        ],
        "far_slab" => vec![receiver, Rect { z: -1., ..slab }],
        "edge_slab" => vec![
            receiver,
            Rect {
                x: [2.2, 6.],
                ..slab
            },
        ],
        "mask_cutout" => vec![
            receiver,
            Rect {
                x: [1.7, 3.],
                ..slab
            },
        ],
        // Retained original diagnostic placement. The reviewed addendum's
        // explicitly defined N8 fixture is separate, below.
        "stripes" => std::iter::once(receiver)
            .chain((0..4).map(|i| Rect {
                x: [0.4 + i as f64 * 0.3, 0.5 + i as f64 * 0.3],
                ..slab
            }))
            .collect(),
        "stripes_gate" => std::iter::once(receiver)
            .chain((0..4).map(|i| Rect {
                x: [-0.5 + i as f64 * 0.3, -0.4 + i as f64 * 0.3],
                y: [-0.5, 1.5],
                ..slab
            }))
            .collect(),
        _ => panic!("unknown analytic fixture"),
    };
    rectangles(camera, &rects)
}

//! CPU cubemap convention matches WebGPU: +X, -X, +Y, -Y, +Z, -Z.
//! Bilinear taps crossing an edge are projected onto the adjacent face.
use glam::Vec3;

pub(super) struct Cube {
    pub size: u32,
    pub pixels: Vec<Vec3>,
}
pub(super) fn direction(face: usize, u: f32, v: f32) -> Vec3 {
    match face {
        0 => Vec3::new(1., -v, -u),
        1 => Vec3::new(-1., -v, u),
        2 => Vec3::new(u, 1., v),
        3 => Vec3::new(u, -1., -v),
        4 => Vec3::new(u, -v, 1.),
        _ => Vec3::new(-u, -v, -1.),
    }
    .normalize()
}
fn coordinates(d: Vec3) -> (usize, f32, f32) {
    let a = d.abs();
    let (face, u, v, scale) = if a.x >= a.y && a.x >= a.z {
        if d.x >= 0. {
            (0, -d.z, -d.y, a.x)
        } else {
            (1, d.z, -d.y, a.x)
        }
    } else if a.y >= a.z {
        if d.y >= 0. {
            (2, d.x, d.z, a.y)
        } else {
            (3, d.x, -d.z, a.y)
        }
    } else if d.z >= 0. {
        (4, d.x, -d.y, a.z)
    } else {
        (5, -d.x, -d.y, a.z)
    };
    (face, u / scale, v / scale)
}
impl Cube {
    pub fn from_fn(size: u32, sample: impl Fn(Vec3) -> Vec3) -> Self {
        let mut pixels = Vec::with_capacity((size * size * 6) as usize);
        for face in 0..6 {
            for y in 0..size {
                for x in 0..size {
                    pixels.push(sample(direction(
                        face,
                        2. * (x as f32 + 0.5) / size as f32 - 1.,
                        2. * (y as f32 + 0.5) / size as f32 - 1.,
                    )));
                }
            }
        }
        Self { size, pixels }
    }
    fn pixel(&self, face: usize, x: u32, y: u32) -> Vec3 {
        self.pixels[(face as u32 * self.size * self.size + y * self.size + x) as usize]
    }
    fn tap(&self, face: usize, x: i32, y: i32) -> Vec3 {
        if (0..self.size as i32).contains(&x) && (0..self.size as i32).contains(&y) {
            return self.pixel(face, x as u32, y as u32);
        }
        let d = direction(
            face,
            2. * (x as f32 + 0.5) / self.size as f32 - 1.,
            2. * (y as f32 + 0.5) / self.size as f32 - 1.,
        );
        let (face, u, v) = coordinates(d);
        let texel = |c: f32| (((c + 1.) * 0.5 * self.size as f32) as u32).min(self.size - 1);
        self.pixel(face, texel(u), texel(v))
    }
    pub fn sample(&self, d: Vec3) -> Vec3 {
        let (face, u, v) = coordinates(d);
        let x = (u + 1.) * 0.5 * self.size as f32 - 0.5;
        let y = (v + 1.) * 0.5 * self.size as f32 - 0.5;
        let (ix, iy) = (x.floor() as i32, y.floor() as i32);
        self.tap(face, ix, iy)
            .lerp(self.tap(face, ix + 1, iy), x - x.floor())
            .lerp(
                self.tap(face, ix, iy + 1)
                    .lerp(self.tap(face, ix + 1, iy + 1), x - x.floor()),
                y - y.floor(),
            )
    }
    pub fn downsample(&self) -> Self {
        let size = self.size / 2;
        let mut pixels = Vec::with_capacity((size * size * 6) as usize);
        for face in 0..6 {
            for y in 0..size {
                for x in 0..size {
                    pixels.push(
                        (self.pixel(face, x * 2, y * 2)
                            + self.pixel(face, x * 2 + 1, y * 2)
                            + self.pixel(face, x * 2, y * 2 + 1)
                            + self.pixel(face, x * 2 + 1, y * 2 + 1))
                            * 0.25,
                    );
                }
            }
        }
        Self { size, pixels }
    }
}
pub(super) fn sample_lod(mips: &[Cube], d: Vec3, lod: f32) -> Vec3 {
    let lod = lod.clamp(0., (mips.len() - 1) as f32);
    let low = lod as usize;
    mips[low]
        .sample(d)
        .lerp(mips[(low + 1).min(mips.len() - 1)].sample(d), lod.fract())
}

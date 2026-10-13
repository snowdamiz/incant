//! Independent f64 reference. Does not execute a GPU shader or establish pixels.
pub fn near(a: [f64; 3], b: [f64; 3]) {
    for (a, b) in a.into_iter().zip(b) {
        assert!((a - b).abs() < 1e-10, "{a} != {b}");
    }
}
pub fn mix(a: [f64; 3], b: [f64; 3], w: f64) -> [f64; 3] {
    std::array::from_fn(|i| a[i] * (1. - w) + b[i] * w)
}
pub fn extract(h: [f64; 4], threshold: f64) -> [f64; 3] {
    let a = h[3].clamp(0., 1.);
    if a == 0. {
        return [0.; 3];
    }
    let r: [f64; 3] = std::array::from_fn(|i| (h[i] / a.max(1e-8)).clamp(0., 65504.));
    let y = r[0] * 0.2126 + r[1] * 0.7152 + r[2] * 0.0722;
    std::array::from_fn(|i| h[i].clamp(0., 65504.) * (y - threshold).max(0.) / y.max(1e-8))
}
pub fn tone(r: [f64; 3]) -> [f64; 3] {
    let peak = r.into_iter().fold(0., f64::max);
    if peak < 0.8 {
        return r;
    }
    let new_peak = 1. - 0.04 / (peak - 0.6);
    let ratio = new_peak / peak;
    let scaled = r.map(|v| v * ratio);
    let g = 1. - 1. / (0.15 * (peak - new_peak) + 1.);
    mix(scaled, [new_peak; 3], g)
}
pub fn compose(r: [f64; 3], l: [f64; 3], a: f64, b: [f64; 3]) -> [f64; 3] {
    let l = l.map(|v| v.clamp(0., 65504.));
    let halo = tone(l);
    mix(
        std::array::from_fn(|i| b[i] + halo[i]),
        tone(std::array::from_fn(|i| (r[i] + l[i]).clamp(0., 65504.))),
        a,
    )
    .map(|v| v.clamp(0., 1.))
}
#[derive(Clone)]
pub struct Field {
    pub size: [u32; 2],
    pub data: Vec<[f64; 3]>,
}
impl Field {
    pub fn reduce(&self) -> Self {
        self.resample(self.size.map(|n| n.div_ceil(2)), self.size.map(f64::from))
    }
    // Integrate source pixel cells over a caller-specified continuous extent.
    // Loops cover every overlap, independently of the shader's fixed 3x3 bound.
    pub fn resample(&self, size: [u32; 2], span: [f64; 2]) -> Self {
        let mut data = Vec::new();
        for y in 0..size[1] {
            for x in 0..size[0] {
                let lo = [
                    f64::from(x) * span[0] / f64::from(size[0]),
                    f64::from(y) * span[1] / f64::from(size[1]),
                ];
                let hi = [
                    f64::from(x + 1) * span[0] / f64::from(size[0]),
                    f64::from(y + 1) * span[1] / f64::from(size[1]),
                ];
                let area = (hi[0] - lo[0]) * (hi[1] - lo[1]);
                let mut value = [0.; 3];
                for sy in lo[1].floor() as u32..hi[1].ceil() as u32 {
                    for sx in lo[0].floor() as u32..hi[0].ceil() as u32 {
                        let w = (hi[0].min(f64::from(sx + 1)) - lo[0].max(f64::from(sx)))
                            * (hi[1].min(f64::from(sy + 1)) - lo[1].max(f64::from(sy)))
                            / area;
                        for (i, c) in value.iter_mut().enumerate() {
                            *c += self.data[(sy * self.size[0] + sx) as usize][i] * w;
                        }
                    }
                }
                data.push(value);
            }
        }
        Self { size, data }
    }
    pub fn sample(&self, uv: [f64; 2]) -> [f64; 3] {
        let p = [
            uv[0] * f64::from(self.size[0]) - 0.5,
            uv[1] * f64::from(self.size[1]) - 0.5,
        ];
        let base = p.map(|v| v.floor() as i32);
        let f = [p[0] - p[0].floor(), p[1] - p[1].floor()];
        let at = |x: i32, y: i32| {
            self.data[(y.clamp(0, self.size[1] as i32 - 1) as u32 * self.size[0]
                + x.clamp(0, self.size[0] as i32 - 1) as u32) as usize]
        };
        mix(
            mix(at(base[0], base[1]), at(base[0] + 1, base[1]), f[0]),
            mix(at(base[0], base[1] + 1), at(base[0] + 1, base[1] + 1), f[0]),
            f[1],
        )
    }
}
pub fn pyramid(mut input: Field, levels: usize) -> Field {
    let mut down = Vec::new();
    for _ in 0..levels {
        input = input.reduce();
        down.push(input.clone());
        if input.size == [1, 1] {
            break;
        }
    }
    reconstruct(down)
}
pub fn anchored_pyramid(input: &Field, output: [u32; 2], scale: f64, levels: usize) -> Field {
    let mut current = input.resample(
        output.map(|v| v.div_ceil(2)),
        output.map(|v| f64::from(v) * scale),
    );
    let mut down = vec![current.clone()];
    for _ in 1..levels {
        if current.size == [1, 1] {
            break;
        }
        current = current.reduce();
        down.push(current.clone());
    }
    reconstruct(down)
}
fn reconstruct(mut down: Vec<Field>) -> Field {
    let count = down.len();
    let mut result = down.pop().unwrap();
    while let Some(mut fine) = down.pop() {
        let n = (count - down.len()) as f64;
        for y in 0..fine.size[1] {
            for x in 0..fine.size[0] {
                let i = (y * fine.size[0] + x) as usize;
                fine.data[i] = mix(
                    fine.data[i],
                    result.sample([
                        (f64::from(x) + 0.5) / f64::from(fine.size[0]),
                        (f64::from(y) + 0.5) / f64::from(fine.size[1]),
                    ]),
                    (n - 1.) / n,
                );
            }
        }
        result = fine;
    }
    result
}

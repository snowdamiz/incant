#[cfg(test)]
use glam::{Mat4, Vec3};

pub(crate) const DIRECTIONS: usize = 16;
pub(crate) const STEPS: usize = 4;
/// Four extra full-resolution attachments: depth32 + normal RGBA16 + two AO R8.
/// At most 56 MiB logical texel payload in the one-size idle cache (driver padding
/// is not measured here). In-flight commands retain replaced attachments; this
/// is not a bound on caller-owned queue depth or aggregate GPU memory.
pub(crate) const MAX_PIXELS: u64 = 4 * 1024 * 1024;
pub(crate) const BYTES_PER_PIXEL: u64 = 4 + 8 + 1 + 1;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AmbientOcclusion {
    /// View/world units, independent of perspective distance or render scale.
    pub radius: f32,
    /// View/world units. Rejects near-coplanar self intersections.
    pub bias: f32,
    /// Linear diffuse accessibility mix, 0 disables darkening and 1 applies it.
    pub strength: f32,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum AmbientOcclusionError {
    #[error(
        "SSAO requires finite radius in [0.0001, 1000], bias in [0, radius), strength in [0, 1]"
    )]
    Parameters,
    #[error("SSAO attachment dimensions exceed the four-megapixel / 56 MiB budget")]
    TargetSize,
    #[error("SSAO projection or viewport exceeds supported finite precision")]
    Projection,
    #[error("SSAO material shader contract changed")]
    ShaderContract,
    #[error("SSAO is not yet available with native cooked-materials")]
    CookedMaterials,
}
impl AmbientOcclusion {
    pub fn validate(self) -> Result<(), AmbientOcclusionError> {
        if !(0.0001..=1000.).contains(&self.radius)
            || !self.bias.is_finite()
            || self.bias < 0.
            || self.bias >= self.radius
            || !(0.0..=1.0).contains(&self.strength)
        {
            return Err(AmbientOcclusionError::Parameters);
        }
        Ok(())
    }
}

pub(crate) fn target_bytes(size: [u32; 2]) -> Result<u64, AmbientOcclusionError> {
    let pixels = u64::from(size[0]) * u64::from(size[1]);
    if size.contains(&0) || size.iter().any(|&v| v > 8192) || pixels > MAX_PIXELS {
        return Err(AmbientOcclusionError::TargetSize);
    }
    Ok(pixels * BYTES_PER_PIXEL)
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct Uniform {
    pub projection: [[f32; 4]; 4],
    pub inverse: [[f32; 4]; 4],
    pub view: [[f32; 4]; 4],
    pub viewport: [f32; 4],
    pub parameters: [f32; 4],
    pub directions: [[f32; 4]; DIRECTIONS],
    pub rotations: [[f32; 4]; 16],
}
const _: () = {
    assert!(size_of::<Uniform>() == 736);
    assert!(std::mem::offset_of!(Uniform, viewport) == 192);
    assert!(std::mem::offset_of!(Uniform, directions) == 224);
    assert!(std::mem::offset_of!(Uniform, rotations) == 480);
};
impl Uniform {
    pub fn new(
        camera: crate::camera::CameraView,
        rect: [f32; 4],
        size: [u32; 2],
        settings: AmbientOcclusion,
    ) -> Result<Self, AmbientOcclusionError> {
        settings.validate()?;
        target_bytes(size)?;
        if rect.iter().any(|v| !v.is_finite())
            || rect[0] < 0.
            || rect[1] < 0.
            || rect[2] <= 0.
            || rect[3] <= 0.
            || rect[0] + rect[2] > size[0] as f32
            || rect[1] + rect[3] > size[1] as f32
        {
            return Err(AmbientOcclusionError::Projection);
        }
        let projection = camera
            .projection(rect[2] / rect[3])
            .map_err(|_| AmbientOcclusionError::Projection)?;
        let inverse = projection.inverse();
        if !inverse.is_finite() || !camera.view.is_finite() {
            return Err(AmbientOcclusionError::Projection);
        }
        // Reject inversion overflow/ill-conditioning at the actual frustum corners.
        for z in [0., 1.] {
            for x in [-1., 1.] {
                for y in [-1., 1.] {
                    let h = inverse * glam::Vec4::new(x, y, z, 1.);
                    let p = h.truncate() / h.w;
                    if h.w <= 0. || !p.is_finite() || p.abs().max_element() > 1e12 {
                        return Err(AmbientOcclusionError::Projection);
                    }
                }
            }
        }
        Ok(Self {
            projection: projection.to_cols_array_2d(),
            inverse: inverse.to_cols_array_2d(),
            view: camera.view.to_cols_array_2d(),
            viewport: rect,
            parameters: [
                settings.radius,
                settings.bias,
                settings.strength,
                STEPS as f32,
            ],
            directions: directions(),
            rotations: rotations(),
        })
    }
}

/// One interleaved azimuth stratum per internal-target Bayer phase. Uniform
/// arrays require a 16-byte stride; round the f64 trigonometry only once.
pub(crate) fn rotations() -> [[f32; 4]; 16] {
    std::array::from_fn(|i| {
        let angle = std::f64::consts::TAU / 16. * (i as f64 + 0.5) / 16.;
        [angle.cos() as f32, angle.sin() as f32, 0., 0.]
    })
}

/// Deterministic equal-probability strata of a cosine-weighted hemisphere:
/// disk radius sqrt(u), height sqrt(1-u). Equal ray weights estimate the diffuse
/// projected-solid-angle visibility integral, not specular visibility.
pub(crate) fn directions() -> [[f32; 4]; DIRECTIONS] {
    std::array::from_fn(|i| {
        let u = (i as f32 + 0.5) / DIRECTIONS as f32;
        let v = (i as u32).reverse_bits() as f64 / 4294967296.;
        let phi = std::f64::consts::TAU * v;
        [
            (u.sqrt() as f64 * phi.cos()) as f32,
            (u.sqrt() as f64 * phi.sin()) as f32,
            (1. - u).sqrt(),
            0.,
        ]
    })
}

#[cfg(test)]
pub(crate) fn reconstruct(
    inverse: Mat4,
    rect: [f32; 4],
    pixel: glam::Vec2,
    depth: f32,
) -> Option<Vec3> {
    if !(0.0..1.0).contains(&depth) || !pixel.is_finite() {
        return None;
    }
    let uv = (pixel - glam::Vec2::new(rect[0], rect[1])) / glam::Vec2::new(rect[2], rect[3]);
    if uv.min_element() < 0. || uv.max_element() >= 1. {
        return None;
    }
    let p = inverse * glam::Vec4::new(2. * uv.x - 1., 1. - 2. * uv.y, depth, 1.);
    let p = p.truncate() / p.w;
    p.is_finite().then_some(p)
}

//! Test-only accepted-frame readback ownership. No raw pointer or public API.
use super::targets::Targets;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct Snapshot {
    // Preserve both safe texture handles and their logical allocation lease.
    targets: Arc<Targets>,
    original: [wgpu::Texture; 3],
    _motion_owner: crate::motion::DiagnosticOwner,
    pub frame: [f32; 24],
    pub motion_frame: [f32; 36],
    pub sequence: u64,
    pub reset: bool,
    /// Actual native getters after encoding: pre-exposure, motion scales XY,
    /// jitter XY, content WH, reset, reversed depth. Not GPU completion data.
    pub native_settings: Option<[f32; 9]>,
    /// Supplied reset, getter immediately before encode, getter immediately after.
    /// Post-encode mutation is observed, never assumed to prove a reset or completion.
    pub native_reset: Option<[bool; 3]>,
}
pub(crate) struct Plane {
    pub name: &'static str,
    pub texture: wgpu::Texture,
    pub origin: [u32; 2],
    pub size: [u32; 2],
    pub bytes_per_pixel: u32,
    pub aspect: wgpu::TextureAspect,
}
impl Plane {
    // wgpu requires depth copies to cover the full subresource. Crop only the
    // mapped bytes; the raw evidence remains the exact active source rectangle.
    pub fn copy_rect(&self) -> ([u32; 2], [u32; 2]) {
        if self.aspect == wgpu::TextureAspect::DepthOnly {
            ([0; 2], [self.texture.width(), self.texture.height()])
        } else {
            (self.origin, self.size)
        }
    }
    pub fn new(
        name: &'static str,
        texture: &wgpu::Texture,
        origin: [u32; 2],
        size: [u32; 2],
    ) -> Self {
        let (bytes_per_pixel, aspect) = match texture.format() {
            wgpu::TextureFormat::Rgba16Float => (8, wgpu::TextureAspect::All),
            wgpu::TextureFormat::Rgba32Float => (16, wgpu::TextureAspect::All),
            wgpu::TextureFormat::Depth32Float => (4, wgpu::TextureAspect::DepthOnly),
            wgpu::TextureFormat::R32Float | wgpu::TextureFormat::Rg16Float => {
                (4, wgpu::TextureAspect::All)
            }
            wgpu::TextureFormat::R8Unorm | wgpu::TextureFormat::R8Uint => {
                (1, wgpu::TextureAspect::All)
            }
            wgpu::TextureFormat::R16Float => (2, wgpu::TextureAspect::All),
            wgpu::TextureFormat::Rgba8UnormSrgb => (4, wgpu::TextureAspect::All),
            other => panic!("undeclared diagnostic copy format: {other:?}"),
        };
        assert!(texture.usage().contains(wgpu::TextureUsages::COPY_SRC));
        assert_eq!(texture.sample_count(), 1);
        assert!(!size.contains(&0));
        assert!(
            origin[0]
                .checked_add(size[0])
                .is_some_and(|x| x <= texture.width())
        );
        assert!(
            origin[1]
                .checked_add(size[1])
                .is_some_and(|y| y <= texture.height())
        );
        Self {
            name,
            texture: texture.clone(),
            origin,
            size,
            bytes_per_pixel,
            aspect,
        }
    }
}
impl Snapshot {
    pub(super) fn new(
        targets: Arc<Targets>,
        frame: [f32; 24],
        motion_frame: [f32; 36],
        original: [wgpu::Texture; 3],
        motion_owner: crate::motion::DiagnosticOwner,
        identity: (u64, bool),
    ) -> Self {
        Self {
            targets,
            original,
            _motion_owner: motion_owner,
            frame,
            motion_frame,
            sequence: identity.0,
            reset: identity.1,
            native_settings: None,
            native_reset: None,
        }
    }
    pub fn content(&self) -> [u32; 2] {
        [self.frame[2] as u32, self.frame[3] as u32]
    }
    pub fn output(&self) -> [u32; 2] {
        self.targets.size
    }
    pub fn mask_views(&self) -> [wgpu::TextureView; 3] {
        [
            self.original[0].create_view(&Default::default()),
            self.targets.views[4].clone(),
            self.original[2].create_view(&Default::default()),
        ]
    }
    pub fn planes(&self) -> [Plane; 10] {
        let origin = [self.frame[0] as u32, self.frame[1] as u32];
        let size = self.content();
        [
            Plane::new("current", &self.original[0], origin, size),
            Plane::new("original-motion", &self.original[1], origin, size),
            Plane::new("original-depth", &self.original[2], origin, size),
            Plane::new("packed-color", &self.targets.textures[0], [0; 2], size),
            Plane::new("packed-depth", &self.targets.textures[1], [0; 2], size),
            Plane::new("packed-motion", &self.targets.textures[2], [0; 2], size),
            Plane::new("reactive", &self.targets.textures[3], [0; 2], size),
            Plane::new("vendor", &self.targets.textures[4], [0; 2], self.output()),
            Plane::new(
                "normalized",
                self.targets.normalized.texture(),
                [0; 2],
                self.output(),
            ),
            Plane::new("exposure", &self.targets.textures[5], [0; 2], [1; 2]),
        ]
    }
}

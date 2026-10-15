//! Explicit cold world-only sampling policy. No automatic quality fallback.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum SceneMsaa {
    One,
    Four,
}
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SceneMsaaError {
    #[error("scene MSAA supports only sample counts 1 and 4, requested {0}")]
    SampleCount(u32),
    #[error("scene 4x MSAA requires render/resolve support for {0:?}")]
    UnsupportedFormat(wgpu::TextureFormat),
    #[error(
        "scene {samples}x attachments exceed dimension, pixel or {bytes} byte logical cache budget"
    )]
    TargetSize { samples: u32, bytes: u64 },
    #[error("SSAO with scene 4x MSAA is unsupported; no approximate depth resolve is available")]
    AmbientOcclusion,
}
impl TryFrom<u32> for SceneMsaa {
    type Error = SceneMsaaError;
    fn try_from(count: u32) -> Result<Self, Self::Error> {
        match count {
            1 => Ok(Self::One),
            4 => Ok(Self::Four),
            _ => Err(SceneMsaaError::SampleCount(count)),
        }
    }
}
impl SceneMsaa {
    pub fn samples(self) -> u32 {
        match self {
            Self::One => 1,
            Self::Four => 4,
        }
    }
    pub(crate) fn index(self) -> usize {
        match self {
            Self::One => 0,
            Self::Four => 1,
        }
    }
    pub(crate) fn catalog(self) -> &'static [Self] {
        match self {
            Self::One => &[Self::One],
            Self::Four => &[Self::One, Self::Four],
        }
    }
    pub(crate) fn select(self, pixel_grid: bool) -> Self {
        if pixel_grid { Self::One } else { self }
    }
    /// Logical texel payload in the one-size cache, excluding queued old owners,
    /// driver padding, other renderer resources and caller-owned output textures.
    pub fn target_bytes(self, size: [u32; 2]) -> Result<u64, SceneMsaaError> {
        let (bytes_per_pixel, maximum) = match self {
            Self::One => (12, 192 * 1024 * 1024),
            Self::Four => (56, 512 * 1024 * 1024),
        };
        let refused = || SceneMsaaError::TargetSize {
            samples: self.samples(),
            bytes: maximum,
        };
        let pixels = u64::from(size[0])
            .checked_mul(u64::from(size[1]))
            .ok_or_else(refused)?;
        let bytes = pixels.checked_mul(bytes_per_pixel).ok_or_else(refused)?;
        if size.contains(&0)
            || size.iter().any(|&n| n > 8192)
            || pixels > 16 * 1024 * 1024
            || bytes > maximum
        {
            return Err(refused());
        }
        Ok(bytes)
    }
    pub(crate) fn validate_formats(
        self,
        color: wgpu::TextureFormatFeatures,
        depth: wgpu::TextureFormatFeatures,
    ) -> Result<(), SceneMsaaError> {
        if self == Self::One {
            return Ok(());
        }
        let render = wgpu::TextureUsages::RENDER_ATTACHMENT;
        let four = wgpu::TextureFormatFeatureFlags::MULTISAMPLE_X4;
        if !color
            .allowed_usages
            .contains(render | wgpu::TextureUsages::TEXTURE_BINDING)
            || !color
                .flags
                .contains(four | wgpu::TextureFormatFeatureFlags::MULTISAMPLE_RESOLVE)
        {
            return Err(SceneMsaaError::UnsupportedFormat(
                wgpu::TextureFormat::Rgba16Float,
            ));
        }
        if !depth.allowed_usages.contains(render) || !depth.flags.contains(four) {
            return Err(SceneMsaaError::UnsupportedFormat(
                wgpu::TextureFormat::Depth32Float,
            ));
        }
        Ok(())
    }
    pub(crate) fn validate_ao(self, enabled: bool) -> Result<(), SceneMsaaError> {
        if self == Self::Four && enabled {
            Err(SceneMsaaError::AmbientOcclusion)
        } else {
            Ok(())
        }
    }
    pub(crate) fn state(self) -> wgpu::MultisampleState {
        wgpu::MultisampleState {
            count: self.samples(),
            mask: !0,
            alpha_to_coverage_enabled: false,
        }
    }
}
impl crate::Renderer {
    /// Cold explicit world-MSAA mode. Prepares the finite 1x and 4x world
    /// catalogs when 4x is requested; no pipelines are created during a draw.
    /// Pixel-grid and retained/temporal submits still select 1x deliberately.
    pub async fn new_with_scene_msaa(
        instance: &wgpu::Instance,
        surface: Option<&wgpu::Surface<'_>>,
        samples: u32,
    ) -> crate::Result<Self> {
        Self::new_configured(
            instance,
            surface,
            false,
            SceneMsaa::try_from(samples)?,
            None,
            #[cfg(all(feature = "native-compilation", target_os = "macos"))]
            None,
            #[cfg(all(
                feature = "metal-binary-archives",
                feature = "native-compilation",
                target_os = "macos"
            ))]
            wgpu_hal::metal::binary_archive::Mode::Capture,
        )
        .await
    }
    /// Explicit instrumented counterpart; timestamps include the real color
    /// resolve pass, while query/readback overhead remains outside intervals.
    pub async fn new_with_scene_msaa_and_gpu_timing(
        instance: &wgpu::Instance,
        surface: Option<&wgpu::Surface<'_>>,
        samples: u32,
    ) -> crate::Result<Self> {
        Self::new_configured(
            instance,
            surface,
            true,
            SceneMsaa::try_from(samples)?,
            None,
            #[cfg(all(feature = "native-compilation", target_os = "macos"))]
            None,
            #[cfg(all(
                feature = "metal-binary-archives",
                feature = "native-compilation",
                target_os = "macos"
            ))]
            wgpu_hal::metal::binary_archive::Mode::Capture,
        )
        .await
    }
    pub fn scene_sample_count(&self) -> u32 {
        self.scene_msaa.samples()
    }
}
#[cfg(test)]
mod device_tests;
#[cfg(test)]
mod fixture;
#[cfg(test)]
mod tests;

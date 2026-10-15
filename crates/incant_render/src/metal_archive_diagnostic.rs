//! Host-only diagnostic construction; no environment/project policy switch.
impl crate::Renderer {
    /// Require archive membership for the actual initial and later cold catalogs.
    /// Available only with both explicit archive and native diagnostic features.
    /// Missing/invalid archives and native misses are errors; this mode never
    /// captures, falls back, or publishes. It does not submit a frame.
    pub async fn new_requiring_metal_archive(
        instance: &crate::wgpu::Instance,
    ) -> crate::Result<Self> {
        Self::new_configured(
            instance,
            None,
            false,
            crate::SceneMsaa::One,
            None,
            wgpu_hal::metal::binary_archive::Mode::RequireHit,
        )
        .await
    }
}

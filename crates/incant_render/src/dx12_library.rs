//! Trusted host diagnostics; no project path, script option or environment mode.
impl crate::Renderer {
    /// Require actual native library hits for every initial and later cold PSO.
    /// Missing/unsafe/stale storage, unsupported drivers and native misses fail;
    /// this route never creates a replacement PSO, stores or publishes bytes.
    /// The only storage source is the current user's private LocalAppData cache.
    pub async fn new_requiring_dx12_library(
        instance: &crate::wgpu::Instance,
    ) -> crate::Result<Self> {
        Self::new_configured(
            instance,
            None,
            false,
            crate::SceneMsaa::One,
            crate::pipeline_cache::dx12::Options {
                config: wgpu_hal::dx12::pipeline_library::Config {
                    mode: wgpu_hal::dx12::pipeline_library::Mode::RequireHits,
                    ..Default::default()
                },
                #[cfg(test)]
                directory: None,
                #[cfg(test)]
                observed: None,
            },
        )
        .await
    }
}

#[cfg(test)]
mod tests;

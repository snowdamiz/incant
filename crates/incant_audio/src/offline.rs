use kira::backend::{Backend, Renderer};

/// A pull backend for file export and numeric audio verification. This calls
/// Kira's real sample renderer; it makes no claim about device latency or output.
pub struct OfflineBackend {
    renderer: Option<Renderer>,
}
impl Backend for OfflineBackend {
    type Settings = u32;
    type Error = &'static str;
    fn setup(rate: u32, _: usize) -> Result<(Self, u32), Self::Error> {
        if !(8000..=192000).contains(&rate) {
            return Err("sample rate outside 8–192 kHz");
        }
        Ok((Self { renderer: None }, rate))
    }
    fn start(&mut self, renderer: Renderer) -> Result<(), Self::Error> {
        self.renderer = Some(renderer);
        Ok(())
    }
}
impl OfflineBackend {
    /// Fill interleaved stereo frames. Offline stream sources may read bounded
    /// cache chunks synchronously; this is not a real-time callback API.
    pub fn render(&mut self, output: &mut [f32]) -> Result<(), &'static str> {
        if !output.len().is_multiple_of(2) {
            return Err("stereo output requires pairs of samples");
        }
        let renderer = self
            .renderer
            .as_mut()
            .ok_or("audio backend is not initialized")?;
        renderer.on_start_processing();
        renderer.process(output, 2);
        Ok(())
    }
}

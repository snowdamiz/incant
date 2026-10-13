//! Postprocess coverage uses target coordinates, independently of geometry's
//! fractional viewport. The shaders still enforce the original rectangle.
pub(super) struct PostprocessRaster {
    size: [u32; 2],
    scissor: [u32; 4],
}
impl PostprocessRaster {
    /// `rect` and `size` have passed Uniform::new. Round outward so every pixel
    /// center accepted by the shader is rasterized, including fractional edges.
    pub(super) fn new(rect: [f32; 4], size: [u32; 2]) -> Self {
        let x = (rect[0].floor() as u32).min(size[0]);
        let y = (rect[1].floor() as u32).min(size[1]);
        let right = ((rect[0] + rect[2]).ceil() as u32).min(size[0]);
        let bottom = ((rect[1] + rect[3]).ceil() as u32).min(size[1]);
        Self {
            size,
            scissor: [x, y, right - x, bottom - y],
        }
    }

    pub(super) fn apply(&self, pass: &mut wgpu::RenderPass<'_>) -> bool {
        let [x, y, width, height] = self.scissor;
        if width == 0 || height == 0 {
            return false;
        }
        pass.set_viewport(0., 0., self.size[0] as f32, self.size[1] as f32, 0., 1.);
        pass.set_scissor_rect(x, y, width, height);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fractional_edges_keep_every_shader_admitted_pixel() {
        let size = [19, 15];
        for rect in [
            [1.25, 1.5, 16.5, 12.25],
            [0., 0., 19., 15.],
            [18.5, 14.5, 0.5, 0.5],
            [0.5, 0.5, 0.01, 0.01],
            [1., 1., f32::EPSILON / 4., f32::EPSILON / 4.],
        ] {
            let [x, y, w, h] = PostprocessRaster::new(rect, size).scissor;
            assert!(x + w <= size[0] && y + h <= size[1]);
            for py in 0..size[1] {
                for px in 0..size[0] {
                    let center = [px as f32 + 0.5, py as f32 + 0.5];
                    let inside = center[0] >= rect[0]
                        && center[1] >= rect[1]
                        && center[0] < rect[0] + rect[2]
                        && center[1] < rect[1] + rect[3];
                    if inside {
                        assert!(px >= x && py >= y && px < x + w && py < y + h);
                    }
                }
            }
        }
        // The failing Dx12 fixture omitted its right column and bottom row.
        assert_eq!(
            PostprocessRaster::new([1.25, 1.5, 16.5, 12.25], size).scissor,
            [1, 1, 17, 13]
        );
    }
}

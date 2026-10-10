use super::*;
use incant_assets::{Texture, TextureFormat};

fn decode_srgb(v: f32) -> f32 {
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}
/// Convert only level zero to linear premultiplied half floats before filtering.
/// Mipmapped sprite atlases require separately extruded levels; not inferred here.
fn premultiplied(source: &Texture) -> Result<Vec<u8>> {
    source.validate()?;
    let mut out = Vec::with_capacity(source.width as usize * source.height as usize * 8);
    for pixel in source.levels[0].chunks_exact(source.format.bytes_per_pixel()) {
        let mut color: [f32; 4] = if source.format == TextureFormat::Rgba32Float {
            std::array::from_fn(|i| f32::from_le_bytes(pixel[i * 4..i * 4 + 4].try_into().unwrap()))
        } else {
            std::array::from_fn(|i| f32::from(pixel[i]) / 255.)
        };
        if color.iter().any(|v| !v.is_finite() || *v < 0.) || color[3] > 1. {
            return Err(ResourceError::TextureRange.into());
        }
        if source.format == TextureFormat::Rgba8Srgb {
            for v in &mut color[..3] {
                *v = decode_srgb(*v);
            }
        }
        for i in 0..3 {
            color[i] *= color[3];
        }
        for v in color {
            let half = half::f16::from_f32(v);
            if !half.is_finite() {
                return Err(ResourceError::TextureRange.into());
            }
            out.extend_from_slice(&half.to_bits().to_le_bytes());
        }
    }
    Ok(out)
}
pub(super) fn upload(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    asset: Arc<RuntimeAsset>,
) -> Result<GpuTexture> {
    let RuntimeAssetData::Texture(source) = asset.data() else {
        return Err(ResourceError::TextureSize.into());
    };
    let source = &source.texture;
    if source.width > device.limits().max_texture_dimension_2d
        || source.height > device.limits().max_texture_dimension_2d
    {
        return Err(ResourceError::TextureSize.into());
    }
    let bytes = premultiplied(source)?;
    let image = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Premultiplied sprite texture"),
        size: wgpu::Extent3d {
            width: source.width,
            height: source.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba16Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &image,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &bytes,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(source.width * 8),
            rows_per_image: Some(source.height),
        },
        image.size(),
    );
    let view = image.create_view(&Default::default());
    let groups = [wgpu::FilterMode::Nearest, wgpu::FilterMode::Linear].map(|mode| {
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Sprite sampling"),
            mag_filter: mode,
            min_filter: mode,
            ..Default::default()
        });
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Sprite image and sampling"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        })
    });
    Ok(GpuTexture {
        _source: asset,
        _image: image,
        groups,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invisible_rgb_is_removed_before_linear_filtering_and_alpha_remains_linear() {
        let texture = Texture {
            width: 2,
            height: 1,
            format: TextureFormat::Rgba8Srgb,
            levels: vec![vec![255, 0, 0, 0, 128, 128, 128, 128], vec![0; 4]],
        };
        let out = premultiplied(&texture).unwrap();
        let values: Vec<_> = out
            .as_chunks::<2>()
            .0
            .iter()
            .map(|v| half::f16::from_bits(u16::from_le_bytes(*v)).to_f32())
            .collect();
        assert_eq!(&values[..4], &[0.; 4]);
        let alpha = 128. / 255.;
        let radiance = decode_srgb(alpha) * alpha;
        assert!((values[4] - radiance).abs() < 0.0001);
        assert!((values[7] - alpha).abs() < 0.0003);
    }
}

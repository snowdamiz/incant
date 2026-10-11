use crate::{Result, Texture, TextureFormat, invalid};
use ktx2::{
    Format, Header,
    dfd::{Basic, Block},
};
fn vk(format: TextureFormat) -> Format {
    match format {
        TextureFormat::Rgba8Srgb => Format::R8G8B8A8_SRGB,
        TextureFormat::Rgba8Linear => Format::R8G8B8A8_UNORM,
        TextureFormat::Rgba32Float => Format::R32G32B32A32_SFLOAT,
    }
}
pub fn decode_ktx2(bytes: &[u8]) -> Result<Texture> {
    if bytes.len() > crate::MAX_SOURCE_BYTES {
        return Err(crate::AssetError::Limit("KTX2 bytes"));
    }
    let raw = bytes
        .get(..Header::LENGTH)
        .ok_or_else(|| invalid("truncated KTX2 header"))?;
    let header = Header::from_bytes(raw.try_into().unwrap()).map_err(|e| invalid(e.to_string()))?;
    crate::texture::dimensions(header.pixel_width, header.pixel_height)?;
    let format = match header.format {
        Some(Format::R8G8B8A8_SRGB) => TextureFormat::Rgba8Srgb,
        Some(Format::R8G8B8A8_UNORM) => TextureFormat::Rgba8Linear,
        Some(Format::R32G32B32A32_SFLOAT) => TextureFormat::Rgba32Float,
        _ => return Err(invalid("unsupported cooked KTX2 format")),
    };
    if header.pixel_depth != 0
        || header.layer_count != 0
        || header.face_count != 1
        || header.supercompression_scheme.is_some()
        || header.level_count as usize
            != crate::texture::mip_count(header.pixel_width, header.pixel_height)
    {
        return Err(invalid("unsupported cooked KTX2 layout"));
    }
    let reader = ktx2::Reader::new(bytes).map_err(|e| invalid(e.to_string()))?;
    let (basic, type_size) = Basic::from_format(vk(format)).map_err(|e| invalid(e.to_string()))?;
    let blocks = reader.dfd_blocks();
    if header.type_size != type_size || blocks != [Block::Basic(basic)] {
        return Err(invalid("KTX2 format descriptor mismatch"));
    }
    if reader
        .key_value_data()
        .find(|(k, _)| *k == "KTXorientation")
        .map(|(_, v)| v)
        != Some(b"rd\0".as_slice())
    {
        return Err(invalid("KTX2 requires top-left orientation"));
    }
    let mut levels = Vec::new();
    for (i, level) in reader.levels().enumerate() {
        let expected = (header.pixel_width >> i).max(1) as usize
            * (header.pixel_height >> i).max(1) as usize
            * format.bytes_per_pixel();
        if level.data.len() != expected || level.uncompressed_byte_length != expected as u64 {
            return Err(invalid("KTX2 level length mismatch"));
        }
        levels.push(std::sync::Arc::from(level.data));
    }
    let texture = Texture {
        width: header.pixel_width,
        height: header.pixel_height,
        format,
        levels,
    };
    texture.validate()?;
    Ok(texture)
}

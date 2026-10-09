use crate::{Result, Texture, TextureFormat, invalid};
use ktx2::{
    Format, Header, Index, LevelIndex,
    dfd::{Basic, Block},
};
fn vk(format: TextureFormat) -> Format {
    match format {
        TextureFormat::Rgba8Srgb => Format::R8G8B8A8_SRGB,
        TextureFormat::Rgba8Linear => Format::R8G8B8A8_UNORM,
        TextureFormat::Rgba32Float => Format::R32G32B32A32_SFLOAT,
    }
}
/// Portable KTX2 with a full mip chain and Khronos DFD. These lossless formats
/// are GPU-uploadable; target-specific block compression is a separate cook tier.
pub fn encode_ktx2(texture: &Texture) -> Result<Vec<u8>> {
    texture.validate()?;
    let format = vk(texture.format);
    let (basic, type_size) = Basic::from_format(format).map_err(|e| invalid(e.to_string()))?;
    let block = Block::Basic(basic).to_vec();
    let mut dfd = ((block.len() + 4) as u32).to_le_bytes().to_vec();
    dfd.extend(block);
    let mut kvd = Vec::new();
    for (key, value) in [("KTXorientation", "rd"), ("KTXwriter", "Incant 0.0.1")] {
        let entry = format!("{key}\0{value}\0");
        kvd.extend((entry.len() as u32).to_le_bytes());
        kvd.extend(entry.as_bytes());
        while !kvd.len().is_multiple_of(4) {
            kvd.push(0);
        }
    }
    let dfd_start = Header::LENGTH + LevelIndex::LENGTH * texture.levels.len();
    let kvd_start = dfd_start + dfd.len();
    let header = Header {
        format: Some(format),
        type_size,
        pixel_width: texture.width,
        pixel_height: texture.height,
        pixel_depth: 0,
        layer_count: 0,
        face_count: 1,
        level_count: texture.levels.len() as u32,
        supercompression_scheme: None,
        index: Index {
            dfd_byte_offset: dfd_start as u32,
            dfd_byte_length: dfd.len() as u32,
            kvd_byte_offset: kvd_start as u32,
            kvd_byte_length: kvd.len() as u32,
            sgd_byte_offset: 0,
            sgd_byte_length: 0,
        },
    };
    let mut bytes = header.as_bytes().to_vec();
    bytes.resize(dfd_start, 0);
    bytes.extend(dfd);
    bytes.extend(kvd);
    // KTX level indices are largest-first; payloads are smallest-first for streaming.
    for (i, level) in texture.levels.iter().enumerate().rev() {
        let alignment = texture.format.bytes_per_pixel();
        while !bytes.len().is_multiple_of(alignment) {
            bytes.push(0);
        }
        let index = LevelIndex {
            byte_offset: bytes.len() as u64,
            byte_length: level.len() as u64,
            uncompressed_byte_length: level.len() as u64,
        };
        let start = Header::LENGTH + i * LevelIndex::LENGTH;
        bytes[start..start + LevelIndex::LENGTH].copy_from_slice(&index.as_bytes());
        bytes.extend(level);
    }
    Ok(bytes)
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
        levels.push(level.data.to_vec());
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

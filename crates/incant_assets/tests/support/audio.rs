pub fn wav(channels: u16, rate: u32, frames: &[[i16; 2]]) -> Vec<u8> {
    let len = (frames.len() * channels as usize * 2) as u32;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + len).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16_u32.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&channels.to_le_bytes());
    bytes.extend_from_slice(&rate.to_le_bytes());
    bytes.extend_from_slice(&(rate * u32::from(channels) * 2).to_le_bytes());
    bytes.extend_from_slice(&(channels * 2).to_le_bytes());
    bytes.extend_from_slice(&16_u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&len.to_le_bytes());
    for frame in frames {
        for sample in frame.iter().take(channels as usize) {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
    }
    bytes
}

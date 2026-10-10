//! Decode WAV/OGG packets into bounded, checksummed stereo-f32 chunks.
use crate::audio::{AUDIO_CHUNK_FRAMES, FRAME_BYTES, MAX_AUDIO_PCM_BYTES, key, path};
use crate::{AssetError, AudioMetadata, CookedAudio, Result, SourceSet, invalid, load_audio};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Cursor, Write},
    path::Path,
};
use symphonia::core::{
    codecs::CodecParameters,
    formats::{TrackType, probe::Hint},
    io::MediaSourceStream,
};

pub fn cook_audio(root: &Path, source: &Path, cache: &Path) -> Result<CookedAudio> {
    let mut sources = SourceSet::new(root)?;
    let bytes = sources.read(source)?;
    let dependency = sources
        .dependencies()
        .into_iter()
        .next()
        .ok_or_else(|| invalid("missing audio source"))?;
    let fingerprint = key(&dependency)?;
    if let Ok(cached) = load_audio(cache, &fingerprint) {
        return Ok(cached);
    }
    let extension = source
        .extension()
        .and_then(|v| v.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !matches!(extension.as_str(), "wav" | "ogg") {
        return Err(AssetError::Unsupported("expected WAV or OGG audio".into()));
    }
    // Do not accept another enabled container under a misleading extension.
    let signature = match extension.as_str() {
        "wav" => bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WAVE"),
        "ogg" => bytes.starts_with(b"OggS"),
        _ => false,
    };
    if !signature {
        return Err(invalid("audio container does not match its extension"));
    }
    let mut hint = Hint::new();
    hint.with_extension(&extension);
    let mss = MediaSourceStream::new(Box::new(Cursor::new(bytes)), Default::default());
    let mut reader = symphonia::default::get_probe()
        .probe(&hint, mss, Default::default(), Default::default())
        .map_err(decode_error)?;
    let track = reader
        .default_track(TrackType::Audio)
        .ok_or_else(|| invalid("audio track missing"))?;
    let track_id = track.id;
    let params = match track.codec_params.as_ref() {
        Some(CodecParameters::Audio(params)) => params,
        _ => return Err(invalid("audio codec parameters missing")),
    };
    let rate = params
        .sample_rate
        .ok_or_else(|| invalid("audio sample rate missing"))?;
    let channels = params
        .channels
        .as_ref()
        .map(|c| c.count())
        .ok_or_else(|| invalid("audio channels missing"))?;
    if !(8000..=192000).contains(&rate) || !(1..=2).contains(&channels) {
        return Err(AssetError::Unsupported(
            "audio requires mono/stereo at 8–192 kHz".into(),
        ));
    }
    if track
        .num_frames
        .is_some_and(|n| n > (MAX_AUDIO_PCM_BYTES / FRAME_BYTES) as u64)
        || params.max_frames_per_packet.is_some_and(|n| n > 65536)
    {
        return Err(AssetError::Limit("audio frames"));
    }
    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(params, &Default::default())
        .map_err(decode_error)?;
    fs::create_dir_all(cache)?;
    let mut file = tempfile::NamedTempFile::new_in(cache)?;
    let mut pending = Vec::with_capacity(AUDIO_CHUNK_FRAMES * FRAME_BYTES);
    let mut hashes = Vec::new();
    let mut content = Sha256::new();
    let mut frames = 0_u64;
    let mut packets = 0_usize;
    while let Some(packet) = reader.next_packet().map_err(decode_error)? {
        packets += 1;
        if packets > 1_000_000 {
            return Err(AssetError::Limit("audio packets"));
        }
        if packet.track_id != track_id {
            continue;
        }
        let buffer = decoder.decode(&packet).map_err(decode_error)?;
        if buffer.spec().rate() != rate || buffer.num_planes() != channels as usize {
            return Err(AssetError::Unsupported(
                "audio format changes within a clip".into(),
            ));
        }
        frames += buffer.frames() as u64;
        if frames > (MAX_AUDIO_PCM_BYTES / FRAME_BYTES) as u64 || buffer.frames() > 65536 {
            return Err(AssetError::Limit("decoded audio frames"));
        }
        let mut samples = vec![0_f32; buffer.samples_interleaved()];
        buffer.copy_to_slice_interleaved(&mut samples);
        for frame in samples.chunks_exact(channels as usize) {
            for sample in [frame[0], frame[frame.len() - 1]] {
                if !sample.is_finite() {
                    return Err(invalid("decoded audio contains a nonfinite sample"));
                }
                pending.extend_from_slice(&sample.to_le_bytes());
            }
            if pending.len() == AUDIO_CHUNK_FRAMES * FRAME_BYTES {
                flush(&mut file, &mut pending, &mut hashes, &mut content)?;
            }
        }
    }
    if frames == 0 {
        return Err(invalid("audio clip has no frames"));
    }
    flush(&mut file, &mut pending, &mut hashes, &mut content)?;
    let content_sha256 = format!("{:x}", content.finalize());
    let metadata = AudioMetadata {
        version: 1,
        fingerprint: fingerprint.clone(),
        dependency,
        sample_rate: rate,
        channels: channels as u8,
        frames,
        content_sha256,
        chunk_sha256: hashes,
    };
    file.as_file().sync_all()?;
    file.persist(path(cache, &metadata.content_sha256, "ipcm")?)
        .map_err(|e| e.error)?;
    let mut manifest = tempfile::NamedTempFile::new_in(cache)?;
    manifest.write_all(&serde_json::to_vec(&metadata)?)?;
    manifest.as_file().sync_all()?;
    manifest
        .persist(path(cache, &fingerprint, "json")?)
        .map_err(|e| e.error)?;
    let mut result = load_audio(cache, &fingerprint)?;
    result.cache_hit = false;
    Ok(result)
}
fn flush(
    file: &mut impl Write,
    bytes: &mut Vec<u8>,
    hashes: &mut Vec<String>,
    content: &mut Sha256,
) -> Result<()> {
    if !bytes.is_empty() {
        file.write_all(bytes)?;
        content.update(&*bytes);
        hashes.push(crate::sha256(bytes));
        bytes.clear();
    }
    Ok(())
}
fn decode_error(error: symphonia::core::errors::Error) -> AssetError {
    invalid(format!("audio decode: {error}"))
}

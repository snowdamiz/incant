use super::*;
use incant_assets_runtime::sha256;

#[test]
fn arbitrary_tick_chunks_preserve_pcm_bits_and_hash() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("game.pcm");
    let samples: Vec<_> = (0..10_000)
        .map(|i| match i % 4 {
            0 => -0.0,
            1 => f32::from_bits(1),
            _ => (i as f32 - 5000.) / 8192.,
        })
        .collect();
    let expected: Vec<_> = samples.iter().flat_map(|v| v.to_le_bytes()).collect();
    let mut output = PcmOutput::new(&path, 5000).unwrap();
    for chunk in samples.chunks(614) {
        output.append(chunk).unwrap();
    }
    assert_eq!(output.frames(), 5000);
    assert_eq!(output.sha256(), sha256(&expected));
    assert!(!path.exists());
    output.publish(&path).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), expected);
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[derive(Default)]
struct CountWrites {
    bytes: u64,
    largest: usize,
    writes: usize,
}
impl Write for CountWrites {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        self.bytes += data.len() as u64;
        self.largest = self.largest.max(data.len());
        self.writes += 1;
        Ok(data.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
#[test]
fn long_capture_has_constant_chunk_bound_and_incremental_digest() {
    let tick = [0.25_f32; 1600];
    let bytes: Vec<_> = tick.iter().flat_map(|v| v.to_le_bytes()).collect();
    let mut expected = Sha256::new();
    let mut stream = Stream::new(CountWrites::default(), 2_400_000);
    for _ in 0..3000 {
        stream.append(&tick).unwrap();
        expected.update(&bytes);
    }
    assert_eq!(stream.frames, 2_400_000);
    assert_eq!(stream.writer.bytes, 19_200_000);
    assert_eq!(stream.writer.writes, 3000);
    assert!(stream.writer.largest <= ENCODE_SAMPLES * 4);
    assert_eq!(stream.hash.finalize(), expected.finalize());
}

struct FailAfter {
    remaining: usize,
    written: usize,
}
impl Write for FailAfter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.remaining == 0 {
            return Err(io::Error::other("injected disk failure"));
        }
        let count = self.remaining.min(bytes.len());
        self.remaining -= count;
        self.written += count;
        Ok(count)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
#[test]
fn partial_io_failure_is_terminal_and_never_counts_a_partial_tick() {
    let mut stream = Stream::new(
        FailAfter {
            remaining: 9000,
            written: 0,
        },
        10000,
    );
    stream.append(&[1.; 2]).unwrap();
    assert!(
        stream
            .append(&[2.; 4096])
            .unwrap_err()
            .to_string()
            .contains("disk failure")
    );
    assert_eq!(stream.frames, 1);
    assert_eq!(stream.writer.written, 9000);
    assert!(stream.append(&[3.; 2]).is_err());
    assert_eq!(stream.writer.written, 9000);
}

#[test]
fn rejected_or_abandoned_stream_cleans_staging_and_never_clobbers() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("game.pcm");
    {
        let mut output = PcmOutput::new(&path, 3).unwrap();
        output.append(&[1.; 2]).unwrap();
        assert!(output.append(&[1.; 6]).is_err());
        assert!(output.publish(&path).is_err());
    }
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    {
        let mut output = PcmOutput::new(&path, 2).unwrap();
        output.append(&[1.; 2]).unwrap();
        // A later script/observation/save admission error drops the owner.
    }
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    std::fs::write(&path, b"existing accepted data").unwrap();
    let mut output = PcmOutput::new(&path, 1).unwrap();
    output.append(&[0.; 2]).unwrap();
    assert!(output.publish(&path).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"existing accepted data");
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    assert!(PcmOutput::new(&dir.path().join("absent/file"), 1).is_err());
    let mut invalid = Stream::new(CountWrites::default(), u64::MAX);
    assert!(invalid.append(&[0.]).is_err());
    assert_eq!(invalid.writer.bytes, 0);
}

mod runner;

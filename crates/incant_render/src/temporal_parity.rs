//! Test-only, bounded retained numeric data. No framebuffer/window capture API.
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write as _,
    path::{Path, PathBuf},
};

const MAX_RECORDS: usize = 64;
const MAX_BYTES: usize = 2 * 1024 * 1024;
pub(crate) struct Output {
    directory: PathBuf,
    test: &'static str,
    records: Vec<serde_json::Value>,
    bytes: usize,
}
impl Output {
    pub(crate) fn optional(test: &'static str) -> Option<Self> {
        std::env::var_os("INCANT_TEMPORAL_PARITY_OUTPUT")
            .map(|p| Self::new(Path::new(&p), test).unwrap())
    }
    fn new(directory: &Path, test: &'static str) -> std::io::Result<Self> {
        if !directory.is_absolute() {
            return Err(std::io::Error::other(
                "absolute numeric output path required",
            ));
        }
        fs::create_dir(directory)?; // Never reuse or overwrite a prior attempt.
        let result = Self {
            directory: directory.into(),
            test,
            records: Vec::new(),
            bytes: 0,
        };
        result.save(false)?;
        Ok(result)
    }
    fn save(&self, complete: bool) -> std::io::Result<()> {
        let bytes = serde_json::to_vec_pretty(&serde_json::json!({
            "format":"incant-fixed-temporal-parity-v1", "test":self.test,
            "cooked_temporal":cfg!(feature="cooked-temporal"), "complete":complete,
            "records":self.records, "bytes":self.bytes,
            "comparison":"exact raw cross-mode equality; existing analytical assertions unchanged",
        }))
        .map_err(std::io::Error::other)?;
        fs::write(self.directory.join("summary.json"), bytes)
    }
    pub(crate) fn record(
        &mut self,
        name: &str,
        width: u32,
        height: u32,
        format: &str,
        data: &[u8],
    ) -> std::io::Result<()> {
        let bpp = match format {
            "r32uint" | "depth32float" | "rgba8srgb" => 4,
            "rgba16float" => 8,
            "rgba32float" => 16,
            _ => return Err(std::io::Error::other("unknown numeric output format")),
        };
        let expected = width.checked_mul(height).and_then(|n| n.checked_mul(bpp));
        let next_bytes = self.bytes.checked_add(data.len());
        if name.is_empty()
            || name.len() > 80
            || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
            || width == 0
            || height == 0
            || width > 128
            || height > 128
            || expected.map(|n| n as usize) != Some(data.len())
            || self.records.len() >= MAX_RECORDS
            || next_bytes.is_none_or(|n| n > MAX_BYTES)
        {
            return Err(std::io::Error::other("numeric output contract/capacity"));
        }
        let file = format!("{name}.raw");
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(self.directory.join(&file))?
            .write_all(data)?;
        self.records.push(
            serde_json::json!({"name":name,"file":file,"width":width,"height":height,
            "format":format,"bytes":data.len(),"sha256":format!("{:x}",Sha256::digest(data))}),
        );
        self.bytes = next_bytes.unwrap();
        self.save(false)
    }
    pub(crate) fn complete(self) {
        self.save(true).unwrap();
    }
}

#[test]
fn create_only_numeric_output_preserves_bytes_and_records_unfinished_attempts() {
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join("output");
    let mut output = Output::new(&directory, "fixture").unwrap();
    output
        .record("one", 1, 1, "r32uint", &7_u32.to_le_bytes())
        .unwrap();
    assert!(Output::new(&directory, "duplicate").is_err());
    assert!(output.record("one", 1, 1, "r32uint", &[0; 4]).is_err());
    assert_eq!(
        fs::read(directory.join("one.raw")).unwrap(),
        7_u32.to_le_bytes()
    );
    let summary: serde_json::Value =
        serde_json::from_slice(&fs::read(directory.join("summary.json")).unwrap()).unwrap();
    assert_eq!(summary["complete"], false);
    assert_eq!(summary["records"].as_array().unwrap().len(), 1);
    output.complete();
    let summary: serde_json::Value =
        serde_json::from_slice(&fs::read(directory.join("summary.json")).unwrap()).unwrap();
    assert_eq!(summary["complete"], true);
}

#[test]
fn numeric_output_rejects_shape_names_format_and_capacity_before_creation() {
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join("output");
    let mut output = Output::new(&directory, "fixture").unwrap();
    for (name, w, h, format, bytes) in [
        ("../escape", 1, 1, "r32uint", 4),
        ("bad", 0, 1, "r32uint", 0),
        ("bad", u32::MAX, 2, "rgba32float", 4),
        ("bad", 1, 1, "r32uint", 3),
        ("bad", 1, 1, "unknown", 4),
    ] {
        assert!(output.record(name, w, h, format, &vec![0; bytes]).is_err());
    }
    for i in 0..MAX_RECORDS {
        output
            .record(&format!("n{i}"), 1, 1, "r32uint", &[0; 4])
            .unwrap();
    }
    assert!(output.record("late", 1, 1, "r32uint", &[0; 4]).is_err());
    assert!(!directory.join("late.raw").exists());
    let mut output = Output::new(&root.path().join("bytes"), "fixture").unwrap();
    for i in 0..8 {
        output
            .record(
                &format!("n{i}"),
                128,
                128,
                "rgba32float",
                &vec![0; 128 * 128 * 16],
            )
            .unwrap();
    }
    assert!(output.record("late", 1, 1, "r32uint", &[0; 4]).is_err());
    assert_eq!(output.bytes, MAX_BYTES);
}

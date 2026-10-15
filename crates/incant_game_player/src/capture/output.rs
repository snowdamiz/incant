//! Create-only per-frame publication. An interrupted/failed run keeps partial
//! evidence; only finish writes manifest.json. PCM is a separate transaction.
use super::{Options, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{self, Read, Write},
    path::Path,
};
const MAX_OUTPUT: u64 = 320 * 1024 * 1024;
const MAX_JSON: u64 = 1024 * 1024;

pub(super) struct Output {
    root: std::path::PathBuf,
    written: u64,
    failed: bool,
}
pub(super) struct Limited<'a> {
    pub writer: &'a mut fs::File,
    pub remaining: u64,
}
impl Write for Limited<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() as u64 > self.remaining {
            return Err(io::Error::other("capture publication byte limit"));
        }
        let n = self.writer.write(bytes)?;
        self.remaining -= n as u64;
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}
impl Output {
    pub fn create(options: &Options) -> Result<Self> {
        // Never adopt or clean a preexisting directory, file or symlink.
        fs::create_dir(&options.directory)?;
        let mut output = Self {
            root: options.directory.clone(),
            written: 0,
            failed: false,
        };
        output.json(
            "partial.json",
            &serde_json::json!({"format":"incant-player-frame-capture","version":1,
            "complete":false,"physical_size":options.size,"selected_host_steps":options.steps,
            "scope":"sampled offscreen world and screen UI; no window or temporal continuity"}),
        )?;
        Ok(output)
    }
    pub fn publish(
        &mut self,
        name: &str,
        limit: u64,
        write: impl FnOnce(&mut Limited<'_>) -> Result<()>,
    ) -> Result<(u64, String)> {
        if self.failed {
            return Err("capture publication is terminal after failure".into());
        }
        let result = self.publish_inner(name, limit, write);
        self.failed |= result.is_err();
        result
    }
    fn publish_inner(
        &mut self,
        name: &str,
        limit: u64,
        write: impl FnOnce(&mut Limited<'_>) -> Result<()>,
    ) -> Result<(u64, String)> {
        if Path::new(name).file_name().and_then(|v| v.to_str()) != Some(name)
            || limit > MAX_OUTPUT - self.written
        {
            return Err("capture output reservation".into());
        }
        let mut file = tempfile::Builder::new()
            .prefix(".incant-frame-")
            .tempfile_in(&self.root)?;
        write(&mut Limited {
            writer: file.as_file_mut(),
            remaining: limit,
        })?;
        let size = file.as_file().metadata()?.len();
        if size > limit {
            return Err("capture output byte limit".into());
        }
        file.as_file().sync_all()?;
        let mut input = file.reopen()?;
        let mut hash = Sha256::new();
        let mut scratch = [0; 8192];
        loop {
            let n = input.read(&mut scratch)?;
            if n == 0 {
                break;
            }
            hash.update(&scratch[..n]);
        }
        file.persist_noclobber(self.root.join(name))
            .map_err(|e| e.error)?;
        self.written += size;
        Ok((size, format!("{:x}", hash.finalize())))
    }
    pub fn json(&mut self, name: &str, value: &impl Serialize) -> Result<()> {
        self.publish(name, MAX_JSON, |file| {
            serde_json::to_writer(file, value)?;
            Ok(())
        })?;
        Ok(())
    }
}

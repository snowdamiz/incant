//! Exact ordered child recipes. No worker-supplied arbitrary argv authority.
use crate::{Error, Result, SourcePackage, backend::Config};
use std::path::Path;
fn text(path: &Path) -> Result<String> {
    path.to_str()
        .map(str::to_owned)
        .ok_or(Error::Invalid("UTF-8 tool path"))
}
pub(crate) fn admit(
    argv: &[String],
    config: &Config,
    job: &Path,
    source: Option<&SourcePackage>,
) -> Result<()> {
    let compiler = text(&config.rustc)?;
    if argv == [compiler.clone(), "-vV".into()] {
        return Ok(());
    }
    let repo = config
        .worker
        .parent()
        .and_then(Path::parent)
        .ok_or(Error::Invalid("engine worker location"))?;
    let sdk = job.join("work/sdk");
    let flags = [
        "--edition=2024",
        "--target=wasm32-unknown-unknown",
        "-C",
        "panic=abort",
        "-C",
        "opt-level=2",
    ];
    for (name, path) in [
        ("incant_wasm_abi", "crates/incant_wasm_abi/src/lib.rs"),
        ("incant_module", "sdk/rust/src/lib.rs"),
    ] {
        let mut expected = vec![
            compiler.clone(),
            text(&repo.join(path))?,
            "--crate-name".into(),
            name.into(),
            "--crate-type=rlib".into(),
        ];
        expected.extend(flags.iter().map(|s| (*s).into()));
        expected.extend(["-o".into(), text(&sdk.join(format!("lib{name}.rlib")))?]);
        if name == "incant_module" {
            expected.extend([
                "--extern".into(),
                format!(
                    "incant_wasm_abi={}",
                    sdk.join("libincant_wasm_abi.rlib").display()
                ),
            ]);
        }
        if argv == expected {
            return Ok(());
        }
    }
    let prefix = vec![
        "/usr/bin/sandbox-exec".into(),
        "-f".into(),
        text(&job.join("work/policy.sb"))?,
    ];
    let mut cfg = prefix.clone();
    cfg.extend([
        compiler.clone(),
        "--print".into(),
        "cfg".into(),
        "--target=wasm32-unknown-unknown".into(),
    ]);
    if argv == cfg {
        return Ok(());
    }
    let mut validation = prefix.clone();
    validation.extend([
        text(&config.validator)?,
        text(&job.join("work/run/module.wasm"))?,
        text(&job.join("work/run/validated.wasm"))?,
    ]);
    if argv == validation {
        return Ok(());
    }
    if let Some(source) = source {
        source.digest()?;
        let toolchain = config
            .rustc
            .parent()
            .and_then(Path::parent)
            .ok_or(Error::Invalid("toolchain location"))?;
        for host in ["aarch64-apple-darwin", "x86_64-apple-darwin"] {
            let linker = toolchain.join(format!("lib/rustlib/{host}/bin/rust-lld"));
            let mut expected = prefix.clone();
            expected.extend([
                compiler.clone(),
                text(&job.join("work/source").join(&source.entry))?,
                "--crate-name".into(),
                "incant_untrusted_module".into(),
                "--crate-type=cdylib".into(),
            ]);
            expected.extend(flags.iter().map(|s| (*s).into()));
            expected.extend([
                "--extern".into(),
                format!(
                    "incant_module={}",
                    sdk.join("libincant_module.rlib").display()
                ),
                "-L".into(),
                format!("dependency={}", sdk.display()),
                "-C".into(),
                "codegen-units=1".into(),
                "-C".into(),
                format!("linker={}", linker.display()),
                "-o".into(),
                text(&job.join("work/run/module.wasm"))?,
            ]);
            if argv == expected {
                return Ok(());
            }
        }
    }
    Err(Error::Invalid(
        "worker child recipe differs from fixed SDK/Wasm/validator recipe",
    ))
}

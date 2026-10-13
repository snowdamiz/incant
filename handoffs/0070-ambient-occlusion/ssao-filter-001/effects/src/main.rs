//! `effects_gpu case <name> <new-output-dir>` runs one frozen numerical case;
//! `effects_gpu appearance <new-output-dir>` renders the frozen appearance set.
//! Only the guarded runner (ssao-filter-001/runner.py) launches it, under an
//! explicit GPU grant. Exit 0: every invariant held; 1: a verdict failed (all
//! receipts, frames and R8 stage planes kept); 2: usage error or a refusal
//! before completion (including a missing or malformed stage capture).
use incant_0070_ssao_filter_effects::{appearance, numeric};
use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.as_slice() {
        [command, name, out] if command == "case" => numeric::run_case(name, Path::new(out)),
        [command, out] if command == "appearance" => appearance::run(Path::new(out)),
        _ => Err("usage: effects_gpu case <name> <new-output-dir> | effects_gpu appearance <new-output-dir>".into()),
    };
    match result {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => {
            eprintln!("verdict failed; every receipt and frame is retained");
            ExitCode::from(1)
        }
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::from(2)
        }
    }
}

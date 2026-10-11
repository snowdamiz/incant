use std::{env, fs, path::PathBuf};
fn main() {
    println!("cargo:rerun-if-env-changed=INCANT_WASM_BUNDLE");
    println!("cargo:rerun-if-env-changed=INCANT_WASM_MSVC_OPT_LEVEL");
    if env::var_os("CARGO_FEATURE_EMBEDDED_FIXTURE").is_none() {
        return;
    }
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let bundle = PathBuf::from(
        env::var_os("INCANT_WASM_BUNDLE")
            .expect("embedded-fixture requires a trusted tools/wasm2c.py output directory"),
    );
    let vendor = root.join("../../third_party/wabt");
    let mut build = cc::Build::new();
    let compiler = build.get_compiler();
    if env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        // Keep the initial Windows route specifically MSVC, not GNU or clang-cl.
        assert_eq!(env::var("TARGET").unwrap(), "x86_64-pc-windows-msvc");
        assert!(compiler.is_like_msvc() && !compiler.is_like_clang());
        assert_eq!(compiler.path().file_stem().unwrap(), "cl");
        let level = env::var("INCANT_WASM_MSVC_OPT_LEVEL").unwrap_or_else(|_| "0".into());
        assert!(
            matches!(level.as_str(), "0" | "2"),
            "MSVC conformance level must be 0 or 2"
        );
        // /Od is the initial conservative baseline. /O2 is an explicit candidate
        // exercised by the same hosted numerical/recursion/fuel controls.
        // Bounds and TLS call-depth checks stay enabled in either configuration.
        build
            .opt_level_str(&level)
            .flag("/TC")
            .flag("/std:c11")
            .flag("/experimental:c11atomics")
            .flag("/fp:strict");
        if level == "0" {
            build.flag("/Od");
        }
        // Record the actual discovered cl.exe, not a different compiler found
        // later on PATH. /? reports its banner without compiling a probe file.
        let info = std::process::Command::new(compiler.path())
            .envs(compiler.env().iter().cloned())
            .args(["/Bv", "/?"])
            .output()
            .expect("read actual MSVC compiler identity");
        let identity = format!(
            "path: {}\noptimization: {level}\n{}\n{}",
            compiler.path().display(),
            String::from_utf8_lossy(&info.stdout),
            String::from_utf8_lossy(&info.stderr)
        );
        fs::write(
            PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("msvc-compiler.txt"),
            identity,
        )
        .unwrap();
    } else {
        // glibc hides sigsetjmp/siglongjmp under strict C11 unless POSIX
        // declarations are requested before every system header. Preserve
        // WABT's signal-mask-aware C trap path rather than replacing it.
        if env::var("CARGO_CFG_TARGET_OS").unwrap() == "linux" {
            build.define("_POSIX_C_SOURCE", "200809L");
        }
        build
            .flag("-std=c11")
            .flag("-Werror=implicit-function-declaration")
            .flag("-fno-optimize-sibling-calls")
            .flag("-frounding-math")
            .flag("-ffp-contract=off");
        if compiler.is_like_gnu() && !compiler.is_like_clang() {
            build.flag("-fsignaling-nans");
        }
    }
    build
        .include(&vendor)
        .include(root.join("c"))
        .include(&bundle)
        .define("WASM_RT_USE_MMAP", "0")
        .define("WASM_RT_MEMCHECK_BOUNDS_CHECK", "1")
        .define("WASM_RT_STACK_DEPTH_COUNT", "1")
        .define("WASM_RT_MAX_CALL_STACK_DEPTH", "64")
        .define("WASM_RT_USE_SEGUE", "0")
        .warnings(false);
    for source in [
        vendor.join("wasm-rt-impl.c"),
        vendor.join("wasm-rt-mem-impl.c"),
        root.join("c/imports.c"),
    ] {
        build.file(source);
    }
    let mut files: Vec<_> = fs::read_dir(&bundle)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    files.sort();
    for file in files {
        if file.extension().is_some_and(|ext| ext == "c") {
            build.file(file);
        }
    }
    println!("cargo:rerun-if-changed={}", bundle.display());
    println!("cargo:rerun-if-changed={}", vendor.display());
    println!("cargo:rerun-if-changed=c");
    build.compile("incant_static_wasm");
    fs::copy(
        bundle.join("linked.rs"),
        PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("linked.rs"),
    )
    .unwrap();
}

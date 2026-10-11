use std::{env, fs, path::PathBuf};
fn main() {
    println!("cargo:rerun-if-env-changed=INCANT_WASM_BUNDLE");
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
    // Initial executed route is macOS Clang. Linux GCC/Clang uses the same
    // upstream-prescribed semantics; MSVC flags have not been verified yet.
    let compiler = build.get_compiler();
    assert!(
        !compiler.is_like_msvc(),
        "static Wasm MSVC conformance is not established"
    );
    // glibc hides sigsetjmp/siglongjmp under strict C11 unless POSIX
    // declarations are requested before every system header. Preserve
    // WABT's signal-mask-aware C trap path rather than replacing it.
    if env::var("CARGO_CFG_TARGET_OS").unwrap() == "linux" {
        build.define("_POSIX_C_SOURCE", "200809L");
    }
    build
        .include(&vendor)
        .include(root.join("c"))
        .include(&bundle)
        .flag("-std=c11")
        .flag("-Werror=implicit-function-declaration")
        .flag("-fno-optimize-sibling-calls")
        .flag("-frounding-math")
        .flag("-ffp-contract=off")
        .define("WASM_RT_USE_MMAP", "0")
        .define("WASM_RT_MEMCHECK_BOUNDS_CHECK", "1")
        .define("WASM_RT_STACK_DEPTH_COUNT", "1")
        .define("WASM_RT_MAX_CALL_STACK_DEPTH", "64")
        .define("WASM_RT_USE_SEGUE", "0")
        .warnings(false);
    if compiler.is_like_gnu() && !compiler.is_like_clang() {
        build.flag("-fsignaling-nans");
    }
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

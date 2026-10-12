fn main() {
    println!("cargo:rerun-if-changed=src/fixture.wgsl");
    println!("cargo:rerun-if-changed=src/shadows/caster.wgsl");
    println!("cargo:rerun-if-changed=src/output.wgsl");
    println!("cargo:rerun-if-changed=src/tone_map.wgsl");
    #[cfg(feature = "cooked-output")]
    if std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() != Ok("wasm32") {
        let source = ["src/output.wgsl", "src/tone_map.wgsl"]
            .map(|path| std::fs::read_to_string(path).expect("engine-owned shader source"))
            .join("\n");
        let bundle =
            incant_shader_cook::cook_output(&source).expect("fixed output shader contract");
        let output =
            std::path::PathBuf::from(std::env::var_os("OUT_DIR").expect("Cargo output directory"))
                .join("output-shaders");
        incant_shader_cook::write_bundle(&bundle, &output).expect("write cooked output shaders");
    }
    #[cfg(feature = "cooked-geometry")]
    if std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() != Ok("wasm32") {
        use incant_shader_cook::geometry::{self, Kind};
        for (kind, path, name) in [
            (Kind::Diagnostic, "src/fixture.wgsl", "diagnostic-shaders"),
            (Kind::Shadow, "src/shadows/caster.wgsl", "shadow-shaders"),
        ] {
            let source = std::fs::read_to_string(path).expect("engine-owned geometry shader");
            let bundle = geometry::cook(kind, &source).expect("fixed geometry shader contract");
            let output = std::path::PathBuf::from(
                std::env::var_os("OUT_DIR").expect("Cargo output directory"),
            )
            .join(name);
            geometry::write_bundle(kind, &bundle, &output).expect("write cooked geometry shaders");
        }
    }
}

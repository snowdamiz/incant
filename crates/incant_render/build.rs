fn main() {
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
}

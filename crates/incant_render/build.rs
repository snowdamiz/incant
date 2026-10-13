fn main() {
    #[cfg(feature = "cooked-materials")]
    if std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() != Ok("wasm32") {
        use incant_shader_cook::geometry::{self, Kind};
        let paths = [
            "src/shadows/shade.wgsl",
            "src/lighting/shade.wgsl",
            "src/model_material.wgsl",
        ];
        for path in paths {
            println!("cargo:rerun-if-changed={path}");
        }
        let source = paths
            .map(|path| std::fs::read_to_string(path).expect("engine-owned material source"))
            .join("\n");
        let bundle =
            geometry::cook(Kind::Material, &source).expect("fixed material shader contract");
        let output =
            std::path::PathBuf::from(std::env::var_os("OUT_DIR").expect("Cargo output directory"))
                .join("material-shaders");
        geometry::write_bundle(Kind::Material, &bundle, &output)
            .expect("write cooked material shaders");
    }
    println!("cargo:rerun-if-changed=src/fixture.wgsl");
    println!("cargo:rerun-if-changed=src/ui/ui.wgsl");
    println!("cargo:rerun-if-changed=src/text/text.wgsl");
    println!("cargo:rerun-if-changed=src/sprites/sprite.wgsl");
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
    #[cfg(feature = "cooked-sprites")]
    if std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() != Ok("wasm32") {
        use incant_shader_cook::geometry::{self, Kind};
        let source =
            std::fs::read_to_string("src/sprites/sprite.wgsl").expect("engine-owned sprite shader");
        let bundle = geometry::cook(Kind::Sprite, &source).expect("fixed sprite shader contract");
        let output =
            std::path::PathBuf::from(std::env::var_os("OUT_DIR").expect("Cargo output directory"))
                .join("sprite-shaders");
        geometry::write_bundle(Kind::Sprite, &bundle, &output)
            .expect("write cooked sprite shaders");
    }
    #[cfg(feature = "cooked-ui")]
    if std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() != Ok("wasm32") {
        use incant_shader_cook::geometry::{self, Kind};
        let source = std::fs::read_to_string("src/ui/ui.wgsl").expect("engine-owned UI shader");
        let bundle = geometry::cook(Kind::Ui, &source).expect("fixed UI shader contract");
        let output =
            std::path::PathBuf::from(std::env::var_os("OUT_DIR").expect("Cargo output directory"))
                .join("ui-shaders");
        geometry::write_bundle(Kind::Ui, &bundle, &output).expect("write cooked UI shaders");
    }
    #[cfg(feature = "cooked-text")]
    if std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() != Ok("wasm32") {
        use incant_shader_cook::geometry::{self, Kind};
        let source =
            std::fs::read_to_string("src/text/text.wgsl").expect("engine-owned text shader");
        let bundle = geometry::cook(Kind::Text, &source).expect("fixed text shader contract");
        let output =
            std::path::PathBuf::from(std::env::var_os("OUT_DIR").expect("Cargo output directory"))
                .join("text-shaders");
        geometry::write_bundle(Kind::Text, &bundle, &output).expect("write cooked text shaders");
    }
}

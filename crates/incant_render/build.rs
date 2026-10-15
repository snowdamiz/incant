#[cfg(feature = "cooked-ssao")]
#[path = "src/ambient_occlusion/source.rs"]
mod ambient_occlusion_source;
#[cfg(feature = "cooked-bloom")]
#[path = "src/bloom/source.rs"]
mod bloom_source;
#[cfg(feature = "cooked-metalfx-boundary")]
#[path = "src/metalfx/source.rs"]
mod metalfx_source;
#[cfg(feature = "cooked-temporal")]
#[path = "src/lighting/temporal_source.rs"]
mod temporal_source;
fn main() {
    #[cfg(feature = "cooked-metalfx-boundary")]
    if std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() != Ok("wasm32") {
        use incant_shader_cook::metalfx_boundary::{self, Kind};
        for path in [
            "src/metalfx/source.rs",
            "src/metalfx/common.wgsl",
            "src/metalfx/alpha.wgsl",
            "src/metalfx/prepare.wgsl",
            "src/metalfx/normalize.wgsl",
        ] {
            println!("cargo:rerun-if-changed={path}");
        }
        let output =
            std::path::PathBuf::from(std::env::var_os("OUT_DIR").expect("Cargo output directory"));
        for (kind, name) in [(Kind::Pack, "pack"), (Kind::Normalize, "normalize")] {
            let source = metalfx_source::source(kind == Kind::Normalize);
            let bundle = metalfx_boundary::cook(kind, &source)
                .expect("fixed MetalFX boundary source and ABI");
            metalfx_boundary::write_bundle(
                kind,
                &bundle,
                &output.join(format!("metalfx-{name}-shaders")),
            )
            .expect("write complete MetalFX boundary");
        }
    }

    #[cfg(feature = "cooked-ssao")]
    if std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() != Ok("wasm32") {
        use incant_shader_cook::ambient_occlusion::{self, Kind};
        for path in [
            "src/ambient_occlusion/source.rs",
            "src/ambient_occlusion/prepass.wgsl",
            "src/ambient_occlusion/ao.wgsl",
            "src/ambient_occlusion/filter.wgsl",
            "src/model_material.wgsl",
            "src/shadows/shade.wgsl",
            "src/lighting/shade.wgsl",
        ] {
            println!("cargo:rerun-if-changed={path}");
        }
        let output =
            std::path::PathBuf::from(std::env::var_os("OUT_DIR").expect("Cargo output directory"));
        for (kind, name) in [
            (Kind::Prepass, "prepass"),
            (Kind::Forward, "forward"),
            (Kind::Sample, "sample"),
            (Kind::Horizontal, "horizontal"),
            (Kind::Vertical, "vertical"),
        ] {
            let source = match kind {
                Kind::Prepass => {
                    ambient_occlusion_source::material(true).expect("guarded SSAO prepass")
                }
                Kind::Forward => ambient_occlusion_source::material(false)
                    .expect("guarded SSAO diffuse material"),
                Kind::Sample => std::fs::read_to_string("src/ambient_occlusion/ao.wgsl")
                    .expect("fixed SSAO sampling"),
                Kind::Horizontal | Kind::Vertical => {
                    std::fs::read_to_string("src/ambient_occlusion/filter.wgsl")
                        .expect("fixed SSAO filter")
                }
            };
            let bundle = ambient_occlusion::cook(kind, &source)
                .expect("complete fixed SSAO kind/source/ABI");
            ambient_occlusion::write_bundle(
                kind,
                &bundle,
                &output.join(format!("ssao-{name}-shaders")),
            )
            .expect("write SSAO family");
        }
    }

    #[cfg(feature = "cooked-temporal")]
    if std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() != Ok("wasm32") {
        use incant_shader_cook::temporal::{self, Kind};
        let output =
            std::path::PathBuf::from(std::env::var_os("OUT_DIR").expect("Cargo output directory"));
        for (kind, paths, name) in [
            (Kind::Motion, vec!["src/motion/shader.wgsl"], "motion"),
            (
                Kind::MotionOwner,
                vec!["src/motion/owner.wgsl"],
                "motion-owner",
            ),
            (Kind::Depth, vec!["src/motion/shader.wgsl"], "depth"),
            (Kind::Resolve, vec!["src/temporal/resolve.wgsl"], "resolve"),
            (Kind::Copy, vec!["src/temporal/copy.wgsl"], "copy"),
        ] {
            let source = paths
                .iter()
                .map(|path| {
                    println!("cargo:rerun-if-changed={path}");
                    std::fs::read_to_string(path).expect("engine-owned temporal program")
                })
                .collect::<Vec<_>>()
                .join("\n");
            let bundle = temporal::cook(kind, &source).expect("fixed temporal source and ABI");
            temporal::write_bundle(kind, &bundle, &output.join(format!("temporal-{name}")))
                .expect("write temporal program");
        }
        println!("cargo:rerun-if-changed=src/lighting/temporal_source.rs");
        println!("cargo:rerun-if-changed=src/lighting/grid.wgsl");
        let source =
            temporal_source::temporal_grid_source().expect("guarded temporal cluster source");
        let bundle =
            incant_shader_cook::compute::cook(&source).expect("fixed temporal compute ABI");
        incant_shader_cook::compute::write_bundle(
            &bundle,
            &output.join("temporal-cluster-shaders"),
        )
        .expect("write temporal clusters");
    }

    #[cfg(feature = "cooked-bloom")]
    if std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() != Ok("wasm32") {
        use incant_shader_cook::bloom::{self, Kind};
        for path in [
            "src/bloom/source.rs",
            "src/bloom/filter.wgsl",
            "src/bloom/output.wgsl",
            "src/output.wgsl",
            "src/tone_map.wgsl",
        ] {
            println!("cargo:rerun-if-changed={path}");
        }
        let filter =
            std::fs::read_to_string("src/bloom/filter.wgsl").expect("engine-owned bloom source");
        let display = bloom_source::display();
        let output =
            std::path::PathBuf::from(std::env::var_os("OUT_DIR").expect("Cargo output directory"));
        for (kind, name) in [
            (Kind::Prefilter, "prefilter"),
            (Kind::Downsample, "downsample"),
            (Kind::Upsample, "upsample"),
            (Kind::Display, "display"),
        ] {
            let bundle = bloom::cook(
                kind,
                if kind == Kind::Display {
                    &display
                } else {
                    &filter
                },
            )
            .expect("fixed bloom source and ABI");
            bloom::write_bundle(kind, &bundle, &output.join(format!("bloom-{name}-shaders")))
                .expect("write complete cooked bloom family");
        }
    }

    #[cfg(feature = "cooked-clusters")]
    if std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() != Ok("wasm32") {
        println!("cargo:rerun-if-changed=src/lighting/grid.wgsl");
        let source = std::fs::read_to_string("src/lighting/grid.wgsl")
            .expect("engine-owned cluster assignment source");
        let bundle =
            incant_shader_cook::compute::cook(&source).expect("fixed cluster assignment contract");
        let output =
            std::path::PathBuf::from(std::env::var_os("OUT_DIR").expect("Cargo output directory"))
                .join("cluster-shaders");
        incant_shader_cook::compute::write_bundle(&bundle, &output)
            .expect("write cooked cluster assignment");
    }
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

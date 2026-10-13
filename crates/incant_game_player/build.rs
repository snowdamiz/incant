fn main() -> Result<(), Box<dyn std::error::Error>> {
    for key in [
        "INCANT_NATIVE_PROJECT",
        "INCANT_NATIVE_PROJECT_SCENE",
        "INCANT_MOBILE_PROJECT",
        "INCANT_MOBILE_PROJECT_SCENE",
        "INCANT_NATIVE_WASM_FINGERPRINT",
        "INCANT_NATIVE_WASM_INPUT",
        "INCANT_NATIVE_WASM_PREPARED",
        "INCANT_NATIVE_WASM_REGISTRY",
    ] {
        println!("cargo:rerun-if-env-changed={key}");
    }
    if let Some(path) = std::env::var_os("INCANT_MOBILE_PROJECT") {
        if std::env::var_os("INCANT_NATIVE_PROJECT").is_some()
            || std::env::var_os("INCANT_NATIVE_PROJECT_SCENE").is_some()
        {
            return Err("mobile and desktop project selectors cannot be combined".into());
        }
        #[cfg(all(feature = "project", feature = "mobile-ffi"))]
        return incant_cook::embed_mobile_project(
            std::path::Path::new(&path),
            std::env::var("INCANT_MOBILE_PROJECT_SCENE").ok().as_deref(),
        );
        #[cfg(not(all(feature = "project", feature = "mobile-ffi")))]
        {
            let _ = path;
            return Err("authored mobile archive requires project and mobile-ffi features".into());
        }
    }
    if std::env::var_os("INCANT_MOBILE_PROJECT_SCENE").is_some() {
        return Err("mobile project scene selection requires a mobile project".into());
    }
    if let Some(path) = std::env::var_os("INCANT_NATIVE_PROJECT") {
        #[cfg(feature = "project")]
        return incant_cook::embed_project(
            std::path::Path::new(&path),
            std::env::var("INCANT_NATIVE_PROJECT_SCENE").ok().as_deref(),
        );
        #[cfg(not(feature = "project"))]
        {
            let _ = path;
            return Err("authored project build requires the project feature".into());
        }
    }
    if std::env::var_os("INCANT_NATIVE_PROJECT_SCENE").is_some() {
        return Err("project scene selection requires a project".into());
    }
    println!("cargo:rustc-env=INCANT_AUTHORED_WASM=");
    incant_cook::embed_script("fixtures/game.js")
}

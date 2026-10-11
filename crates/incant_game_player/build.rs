fn main() -> Result<(), Box<dyn std::error::Error>> {
    for key in ["INCANT_NATIVE_PROJECT", "INCANT_NATIVE_PROJECT_SCENE"] {
        println!("cargo:rerun-if-env-changed={key}");
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
    incant_cook::embed_script("fixtures/game.js")
}

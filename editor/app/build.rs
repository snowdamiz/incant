fn main() {
    // Included icons/assets must exist before rustc records its input timestamps.
    let attributes = tauri_build::Attributes::new().codegen(tauri_build::CodegenContext::new());
    tauri_build::try_build(attributes).expect("generate Tauri build context");
}

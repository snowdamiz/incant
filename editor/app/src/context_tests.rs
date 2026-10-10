#[test]
fn build_context_preserves_configuration_assets_and_icon() {
    let context = super::app_context();
    let mut configured: tauri::Config =
        serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
    // Tauri omits this authoring-only schema URL from its generated runtime config.
    configured.schema = None;
    assert_eq!(
        serde_json::to_value(context.config()).unwrap(),
        serde_json::to_value(&configured).unwrap()
    );
    assert_eq!(
        context.package_info().name,
        configured.product_name.as_deref().unwrap()
    );
    assert_eq!(
        context.package_info().version.to_string(),
        configured.version.as_deref().unwrap()
    );
    let assets: Vec<_> = context.assets().iter().collect();
    #[cfg(feature = "custom-protocol")]
    assert!(
        assets
            .iter()
            .any(|(path, bytes)| { path.ends_with("/index.html") && !bytes.is_empty() })
    );
    #[cfg(not(feature = "custom-protocol"))]
    assert!(assets.is_empty());
    let icon = context
        .default_window_icon()
        .expect("configured window icon");
    assert!(icon.width() > 0 && icon.height() > 0);
    assert_eq!(
        icon.rgba().len(),
        icon.width() as usize * icon.height() as usize * 4
    );
}

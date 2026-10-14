use super::*;
#[test]
fn finite_modes_routes_and_checked_attachment_budgets_are_explicit() {
    for count in [0, 2, 3, 8, 16, u32::MAX] {
        assert_eq!(
            SceneMsaa::try_from(count),
            Err(SceneMsaaError::SampleCount(count))
        );
    }
    assert_eq!(SceneMsaa::Four.select(true), SceneMsaa::One);
    assert_eq!(SceneMsaa::Four.select(false), SceneMsaa::Four);
    assert_eq!(SceneMsaa::One.catalog(), &[SceneMsaa::One]);
    assert_eq!(
        SceneMsaa::Four.catalog(),
        &[SceneMsaa::One, SceneMsaa::Four]
    );
    assert_eq!(
        SceneMsaa::One.target_bytes([4096, 4096]).unwrap(),
        192 * 1024 * 1024
    );
    assert_eq!(
        SceneMsaa::Four.target_bytes([3840, 2160]).unwrap(),
        464_486_400
    );
    assert!(SceneMsaa::Four.target_bytes([4096, 4096]).is_err());
    assert!(SceneMsaa::Four.target_bytes([3072, 3072]).is_ok());
    for size in [
        [0, 1],
        [1, 0],
        [8193, 1],
        [u32::MAX, u32::MAX],
        [4096, 2400],
    ] {
        assert!(SceneMsaa::Four.target_bytes(size).is_err());
    }
    for mode in [SceneMsaa::One, SceneMsaa::Four] {
        assert!(!mode.state().alpha_to_coverage_enabled);
    }
}
#[test]
fn missing_multisample_resolve_or_depth_capability_and_ao_refuse() {
    let color =
        wgpu::TextureFormat::Rgba16Float.guaranteed_format_features(wgpu::Features::empty());
    let depth =
        wgpu::TextureFormat::Depth32Float.guaranteed_format_features(wgpu::Features::empty());
    SceneMsaa::Four.validate_formats(color, depth).unwrap();
    for flag in [
        wgpu::TextureFormatFeatureFlags::MULTISAMPLE_X4,
        wgpu::TextureFormatFeatureFlags::MULTISAMPLE_RESOLVE,
    ] {
        let mut bad = color;
        bad.flags.remove(flag);
        assert_eq!(
            SceneMsaa::Four.validate_formats(bad, depth),
            Err(SceneMsaaError::UnsupportedFormat(
                wgpu::TextureFormat::Rgba16Float
            ))
        );
        SceneMsaa::One.validate_formats(bad, depth).unwrap();
    }
    let mut bad = depth;
    bad.flags
        .remove(wgpu::TextureFormatFeatureFlags::MULTISAMPLE_X4);
    assert_eq!(
        SceneMsaa::Four.validate_formats(color, bad),
        Err(SceneMsaaError::UnsupportedFormat(
            wgpu::TextureFormat::Depth32Float
        ))
    );
    let mut bad = color;
    bad.allowed_usages
        .remove(wgpu::TextureUsages::TEXTURE_BINDING);
    assert!(SceneMsaa::Four.validate_formats(bad, depth).is_err());
    let mut bad = depth;
    bad.allowed_usages
        .remove(wgpu::TextureUsages::RENDER_ATTACHMENT);
    assert!(SceneMsaa::Four.validate_formats(color, bad).is_err());
    assert_eq!(
        SceneMsaa::Four.validate_ao(true),
        Err(SceneMsaaError::AmbientOcclusion)
    );
    SceneMsaa::One.validate_ao(true).unwrap();
    SceneMsaa::Four.validate_ao(false).unwrap();
}

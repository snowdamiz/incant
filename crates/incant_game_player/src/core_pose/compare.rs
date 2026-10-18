use incant_render_data::UiInput;
use incant_render_native::NativeRenderInput;

pub(super) fn canonical(a: &NativeRenderInput, b: &NativeRenderInput) {
    assert_eq!((a.tick(), a.camera_label()), (b.tick(), b.camera_label()));
    assert_eq!(a.input().nodes.len(), b.input().nodes.len());
    for (a, b) in a.input().nodes.iter().zip(&b.input().nodes) {
        assert_eq!(a.id, b.id);
        assert_eq!(a.world, b.world);
        assert_eq!(a.camera, b.camera);
        assert!(a.light == b.light);
        assert_eq!(a.mesh.is_some(), b.mesh.is_some());
        if let (Some(a), Some(b)) = (&a.mesh, &b.mesh) {
            assert_eq!(a.animation, b.animation);
            assert!(a.animation_blend.is_none() && b.animation_blend.is_none());
        }
    }
}

pub(super) fn ui(a: &UiInput, b: &UiInput) {
    assert_eq!(a.viewport, b.viewport);
    assert_eq!(a.atlas.pixels(), b.atlas.pixels());
    assert_eq!(a.widgets.len(), b.widgets.len());
    for (x, y) in a.widgets.iter().zip(&b.widgets) {
        assert_eq!(
            (
                x.id, x.role, x.disabled, x.focused, x.pressed, x.bounds, x.visible, x.origin,
                x.clip
            ),
            (
                y.id, y.role, y.disabled, y.focused, y.pressed, y.bounds, y.visible, y.origin,
                y.clip
            )
        );
        assert_eq!(x.layout.source(), y.layout.source());
        assert_eq!(x.layout.glyphs(), y.layout.glyphs());
        assert_eq!(x.layout.lines(), y.layout.lines());
        assert_eq!(x.layout.missing_clusters(), y.layout.missing_clusters());
        assert_eq!(x.layout.size(), y.layout.size());
        assert_eq!(x.glyphs.len(), y.glyphs.len());
        // Handles belong to distinct atlas owners. Compare the actual accepted
        // entries and all atlas bytes, never opaque owner identities.
        for (x, y) in x.glyphs.iter().zip(y.glyphs.iter()) {
            assert_eq!(
                a.atlas.get(x).expect("ordinary glyph"),
                b.atlas.get(y).expect("pose glyph")
            );
        }
        assert_eq!(x.halos.is_some(), y.halos.is_some());
        if let (Some(x), Some(y)) = (&x.halos, &y.halos) {
            assert_eq!(x.len(), y.len());
            for (x, y) in x.iter().zip(y.iter()) {
                assert_eq!(
                    a.atlas.get(x).expect("ordinary halo"),
                    b.atlas.get(y).expect("pose halo")
                );
            }
        }
        assert_eq!(x.edit, y.edit);
    }
}

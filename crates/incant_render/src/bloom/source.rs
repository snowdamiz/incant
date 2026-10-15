//! One guarded engine-owned source assembly shared by runtime and build host.
pub(crate) fn display() -> String {
    const ORIGINAL: &str = "var color = mix(display.backdrop.rgb,tone_map(radiance),coverage);";
    let original = include_str!("../output.wgsl");
    assert_eq!(
        original.matches(ORIGINAL).count(),
        1,
        "fixed output seam changed"
    );
    format!(
        "{}\n{}\n{}",
        original.replace(
            ORIGINAL,
            "var color = display.backdrop.rgb; if inside { color = bloom_color(radiance,coverage,position.xy); }"
        ),
        include_str!("../tone_map.wgsl"),
        include_str!("output.wgsl")
    )
}

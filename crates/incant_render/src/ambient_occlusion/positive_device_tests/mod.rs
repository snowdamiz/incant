//! Positive-strength native/default comparison. Raw output precedes assertions;
//! an external bounded runner compares the two separately compiled routes.
mod fixture;
use crate::AmbientOcclusion;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

const W: u32 = 128;
const H: u32 = 96;
const BACKGROUND: [u8; 3] = [20, 21, 25];
// A fixture observability floor, not an appearance metric: at least16 pixels
// must darken by8 UNORM8 codes, well beyond a single output rounding step.
const MIN_EFFECT_PIXELS: usize = 16;
const MIN_EFFECT_CODES: u8 = 8;

#[derive(Debug, Serialize)]
struct Metrics {
    effect_pixels: usize,
    maximum_darkening: u8,
    brighter_pixels: usize,
    coverage_mismatches: usize,
    alpha_mismatches: usize,
}
impl Metrics {
    fn accepted(&self) -> bool {
        self.effect_pixels >= MIN_EFFECT_PIXELS
            && self.brighter_pixels == 0
            && self.coverage_mismatches == 0
            && self.alpha_mismatches == 0
    }
}
fn metrics(zero: &[u8], positive: &[u8]) -> Metrics {
    assert_eq!(zero.len(), positive.len());
    assert!(!zero.is_empty() && zero.len().is_multiple_of(4));
    let mut result = Metrics {
        effect_pixels: 0,
        maximum_darkening: 0,
        brighter_pixels: 0,
        coverage_mismatches: 0,
        alpha_mismatches: 0,
    };
    for (a, b) in zero
        .as_chunks::<4>()
        .0
        .iter()
        .zip(positive.as_chunks::<4>().0.iter())
    {
        let difference = (0..3).map(|c| a[c].saturating_sub(b[c])).max().unwrap();
        result.maximum_darkening = result.maximum_darkening.max(difference);
        result.effect_pixels += usize::from(difference >= MIN_EFFECT_CODES);
        result.brighter_pixels += usize::from((0..3).any(|c| b[c] > a[c]));
        result.coverage_mismatches += usize::from((a[..3] == BACKGROUND) != (b[..3] == BACKGROUND));
        result.alpha_mismatches += usize::from(a[3] != b[3]);
    }
    result
}

#[test]
fn imported_fixture_has_occluding_depth_textured_mask_and_unchanged_native_inputs() {
    let (input, fingerprint) = fixture::scene();
    assert_eq!(fingerprint.len(), 64);
    let resolved = crate::scene::resolve(&input).unwrap();
    assert_eq!(resolved.stats.model_triangles, 4);
    assert_eq!(resolved.stats.primitive_instances, 2);
    assert!(resolved.cameras.contains_key("camera"));
}

#[test]
fn effect_control_rejects_no_effect_increased_color_coverage_and_alpha_changes() {
    let zero = [120, 130, 140, 255].repeat(MIN_EFFECT_PIXELS);
    let positive = [110, 120, 130, 255].repeat(MIN_EFFECT_PIXELS);
    assert!(metrics(&zero, &positive).accepted());
    assert!(!metrics(&zero, &zero).accepted());
    for pixel in [
        [121, 120, 130, 255],
        [110, 120, 130, 254],
        [20, 21, 25, 255],
    ] {
        let mut bad = positive.clone();
        bad[..4].copy_from_slice(&pixel);
        assert!(!metrics(&zero, &bad).accepted());
    }
}

#[test]
#[ignore = "requires delegated Metal: disabled/zero/positive SSAO128x96, six submissions; raw numerical bytes only"]
fn positive_strength_textured_mask_whole_route_exports_comparable_pixels() {
    if cfg!(all(
        feature = "cooked-output",
        not(feature = "cooked-ssao"),
        not(target_arch = "wasm32")
    )) {
        assert!(super::super::shader::admit().is_err());
        return;
    }
    let temporary = tempfile::tempdir().unwrap();
    let output = std::env::var_os("INCANT_SSAO_POSITIVE_OUTPUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| temporary.path().join("pixels"));
    fs::create_dir(&output).unwrap(); // Explicit paths must be fresh, never overwrite.
    let (input, fingerprint) = fixture::scene();
    let mut renderer = crate::test_support::checked(crate::Renderer::headless().unwrap());
    let scene = renderer
        .prepare(&input)
        .unwrap()
        .with_camera("camera")
        .unwrap();
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let mut frames = Vec::new();
    let mut rows = Vec::new();
    for (name, strength) in [
        ("disabled", None),
        ("zero", Some(0.)),
        ("positive", Some(1.)),
    ] {
        if let Some(strength) = strength {
            pollster::block_on(renderer.enable_ambient_occlusion(AmbientOcclusion {
                radius: 1.,
                bias: 0.01,
                strength,
            }))
            .unwrap();
        }
        let target = crate::hdr_output::texture(&renderer, format, W, H);
        renderer.queue.submit([renderer
            .draw_scene(
                &scene,
                &target.create_view(&Default::default()),
                format,
                W,
                H,
                None,
            )
            .unwrap()]);
        let pixels = crate::hdr_output::read(&renderer, &target);
        fs::write(output.join(format!("{name}.rgba")), &pixels).unwrap();
        rows.push(serde_json::json!({"name":name,"bytes":pixels.len(),"sha256":format!("{:x}",Sha256::digest(&pixels))}));
        frames.push(pixels);
    }
    let effect = metrics(&frames[1], &frames[2]);
    let summary = serde_json::json!({"format":"incant-ssao-positive-v1","width":W,"height":H,
        "fixture_fingerprint":fingerprint,"cooked_ssao":cfg!(feature="cooked-ssao"),"frames":rows,
        "zero_equals_disabled":frames[0]==frames[1],"effect":effect,"effect_accepted":effect.accepted(),
        "minimum_effect_pixels":MIN_EFFECT_PIXELS,"minimum_darkening_codes":MIN_EFFECT_CODES,
        "cross_route_acceptance":"exact RGBA byte equality; no tolerance or appearance gate"});
    fs::write(
        output.join("summary.json"),
        serde_json::to_vec_pretty(&summary).unwrap(),
    )
    .unwrap();
    eprintln!("SSAO_POSITIVE {summary}");
    assert_eq!(
        frames[0], frames[1],
        "zero strength must preserve textured/MASK ordinary output"
    );
    assert!(
        effect.accepted(),
        "nonzero occlusion must affect visible receiver without coverage/alpha changes: {effect:?}"
    );
}

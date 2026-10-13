use incant_physics_snapshot::*;
#[path = "support/integration_fixture.rs"]
mod fixture;
fn budget() -> Budget {
    Budget::new(MAX_ALLOCATION).unwrap()
}
#[test]
fn bit_exact_sentinels_switches_and_full_usize_values() {
    for d in [Dimension::Planar, Dimension::Spatial] {
        let mut r = fixture::config(d);
        r.dt = (-0f32).to_bits();
        r.min_ccd_dt = (-0f32).to_bits();
        r.num_solver_iterations = 0;
        r.normalized_max_corrective_velocity = f32::MAX.to_bits();
        r.normalized_max_linear_velocity = f32::MAX.to_bits();
        r.length_unit = 2f32.to_bits();
        r.normalized_prediction_distance = (-0f32).to_bits();
        r.num_internal_pgs_iterations = u64::MAX;
        r.num_internal_stabilization_iterations = 0;
        r.max_ccd_substeps = 0;
        let v = validate_integration_config(r).unwrap();
        assert_eq!(v.prediction_distance(), (-0f32).to_bits());
        let bytes = encode_integration_config(&v, &mut budget()).unwrap();
        assert_eq!(bytes.len(), 144);
        let decoded = decode_integration_config(&bytes, r.identity).unwrap();
        assert_eq!(decoded.record(), &r);
        assert_eq!(
            encode_integration_config(&decoded, &mut budget()).unwrap(),
            bytes
        );
    }
}
#[test]
fn hostile_numeric_domains_and_explicit_soft_refusal() {
    let original = fixture::config(Dimension::Spatial);
    for bits in [
        f32::NAN.to_bits(),
        f32::INFINITY.to_bits(),
        f32::NEG_INFINITY.to_bits(),
    ] {
        let mut r = original;
        r.dt = bits;
        assert!(matches!(
            validate_integration_config(r),
            Err(Error::Integration)
        ));
    }
    for mutate in [
        |r: &mut IntegrationConfig| r.length_unit = (-0f32).to_bits(),
        |r: &mut IntegrationConfig| r.min_ccd_dt = (-1f32).to_bits(),
        |r: &mut IntegrationConfig| r.warmstart_coefficient = 1.01f32.to_bits(),
        |r: &mut IntegrationConfig| r.contact_softness.damping_ratio = (-1f32).to_bits(),
        |r: &mut IntegrationConfig| {
            r.contact_softness.natural_frequency = 0;
            r.contact_softness.damping_ratio = 0;
        },
        |r: &mut IntegrationConfig| r.num_solver_iterations = 0,
        |r: &mut IntegrationConfig| r.dt = 1,
        |r: &mut IntegrationConfig| {
            r.length_unit = 2f32.to_bits();
            r.normalized_prediction_distance = f32::MAX.to_bits();
        },
    ] {
        let mut r = original;
        mutate(&mut r);
        assert!(matches!(
            validate_integration_config(r),
            Err(Error::Integration)
        ));
    }
    let mut r = original;
    r.soft_bodies = IntegrationSoftBodies::Unsupported;
    assert!(matches!(
        validate_integration_config(r),
        Err(Error::UnsupportedIntegration)
    ));
    r = original;
    r.friction_model = IntegrationFriction::Planar;
    assert!(matches!(
        validate_integration_config(r),
        Err(Error::Integration)
    ));
}
#[test]
fn fixed_frame_header_enums_trailing_and_aggregate_budget() {
    let v = validate_integration_config(fixture::config(Dimension::Spatial)).unwrap();
    let bytes = encode_integration_config(&v, &mut budget()).unwrap();
    let id = v.record().identity;
    for i in 0..bytes.len() {
        assert!(decode_integration_config(&bytes[..i], id).is_err());
    }
    let mut extra = bytes.clone();
    extra.push(0);
    assert!(matches!(
        decode_integration_config(&extra, id),
        Err(Error::Length)
    ));
    for (offset, word) in [(128, 16), (132, 3), (136, 2), (140, 1)] {
        let mut bad = bytes.clone();
        bad[offset..offset + 4].copy_from_slice(&u32::to_le_bytes(word));
        assert!(decode_integration_config(&bad, id).is_err());
    }
    for offset in [0, 8, 10, 11, 12] {
        let mut bad = bytes.clone();
        bad[offset] ^= 0xff;
        assert!(decode_integration_config(&bad, id).is_err());
    }
    let mut b = Budget::new(287).unwrap();
    encode_integration_config(&v, &mut b).unwrap();
    assert!(matches!(
        encode_integration_config(&v, &mut b),
        Err(Error::Limit)
    ));
}

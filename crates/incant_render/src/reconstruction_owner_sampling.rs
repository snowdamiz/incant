//! Test-only controls that isolate stationary sampling from camera reprojection.
use super::*;

#[derive(Clone, Copy)]
pub(super) enum Motion {
    Pan,
    Stationary,
}
impl Motion {
    pub(super) fn selected() -> Self {
        match std::env::var("INCANT_OWNER_CAMERA_MOTION").as_deref() {
            Err(std::env::VarError::NotPresent) | Ok("pan") => Self::Pan,
            Ok("stationary") => Self::Stationary,
            Ok(other) => panic!("unknown diagnostic motion: {other}"),
            Err(error) => panic!("invalid diagnostic motion: {error}"),
        }
    }
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Pan => "pan-after-frame14",
            Self::Stationary => "stationary",
        }
    }
    pub(super) fn pose(self, stage: &Stage, input: &mut SceneInput, frame: u32) {
        super::super::recurrence_fixture::pose(
            stage,
            input,
            match self {
                Self::Pan => frame,
                Self::Stationary => 1,
            },
            true,
        );
    }
}

#[derive(Clone, Copy)]
pub(super) enum Sampling {
    Incant8,
    Halton32,
}
impl Sampling {
    pub(super) fn selected() -> Self {
        match std::env::var("INCANT_OWNER_CAMERA_JITTER").as_deref() {
            Err(std::env::VarError::NotPresent) | Ok("incant8") => Self::Incant8,
            Ok("halton32") => Self::Halton32,
            Ok(other) => panic!("unknown diagnostic jitter: {other}"),
            Err(error) => panic!("invalid diagnostic jitter: {error}"),
        }
    }
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Incant8 => "incant8",
            Self::Halton32 => "halton32",
        }
    }
    pub(super) fn begin(self) -> Option<crate::temporal::jitter::DiagnosticHalton32> {
        match self {
            Self::Incant8 => None,
            Self::Halton32 => Some(crate::temporal::jitter::DiagnosticHalton32::begin()),
        }
    }
}

#[test]
fn stationary_sampling_preserves_poses_and_restores_jitter() {
    use crate::temporal::jitter;
    let stage = Stage::new(true);
    let mut input = stage.input();
    Motion::Stationary.pose(&stage, &mut input, 1);
    let worlds: Vec<_> = input.nodes.iter().map(|n| n.world).collect();
    let lights: Vec<_> = input.nodes.iter().map(|n| n.light.clone()).collect();
    for frame in 1..=64 {
        Motion::Stationary.pose(&stage, &mut input, frame);
        assert!(input.nodes.iter().zip(&worlds).all(|(n, w)| n.world == *w));
        assert!(input.nodes.iter().zip(&lights).all(|(n, l)| n.light == *l));
    }
    assert_eq!(jitter::phase_count(true), 8);
    let ordinary = jitter::offset(0, true);
    {
        let _guard = Sampling::Halton32.begin();
        assert_eq!(jitter::phase_count(true), 32);
        assert_ne!(jitter::offset(0, true), ordinary);
        let phases: Vec<_> = (0..32).map(|i| jitter::offset(i, true)).collect();
        for (i, sample) in phases.iter().enumerate() {
            assert!(sample.iter().all(|v| v.abs() < 0.5));
            assert!(!phases[..i].contains(sample));
            assert_eq!(*sample, jitter::offset(i as u32 + 32, true));
        }
        assert_eq!(jitter::phase_count(false), 8);
    }
    assert_eq!(jitter::phase_count(true), 8);
    assert_eq!(jitter::offset(0, true), ordinary);
    Motion::Pan.pose(&stage, &mut input, 64);
    assert_ne!(
        input.nodes[node(&input, &stage.camera)].world,
        camera_world(1)
    );
}

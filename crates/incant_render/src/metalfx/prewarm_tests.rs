//! Real native encoding abandoned before submission; no GPU execution/readback.
use super::*;
use incant_render_authoring::RendererAuthoringExt as _;

#[test]
#[ignore = "requires coordinated Mac MetalFX device; one raw encode, zero submissions/maps"]
fn prepared_extent_survives_abandon_and_completion_retains_targets() {
    let r = crate::test_support::checked(crate::Renderer::headless().unwrap());
    let range = super::super::native::range(&r.device).unwrap();
    println!(
        "actual adapter {:?}; native range {range:?}",
        r.adapter.get_info()
    );
    let scene = r
        .diagnostic_scene(&incant_doc::Project::empty("prepared-abandon"))
        .unwrap();
    let mut owner = Owner::new(
        Preference::RequireMetalFx,
        Config {
            relative_depth: 0.01,
            absolute_depth: 0.001,
        },
        Ok(range),
    )
    .unwrap();
    owner.prepare_extent(&r, [64, 64]).unwrap();
    let targets = Arc::downgrade(&owner.prepared.as_ref().unwrap().targets);
    let scaler = Rc::downgrade(&owner.prepared.as_ref().unwrap().scaler);
    let output = [0., 0., 64., 64.];
    let frame = crate::resolution::FramePlan::new(
        [64, 64],
        output,
        None,
        crate::resolution::ResolutionScale::new(8).unwrap(),
    )
    .unwrap();
    // Admission does not consume the explicitly prepared owner.
    let discarded = owner.prepare(&r, &scene, false, &frame, output).unwrap();
    assert!(discarded.native());
    drop(discarded);
    assert!(targets.upgrade().is_some());
    assert_eq!(owner.selection(), None);
    let mut pending = owner.prepare(&r, &scene, false, &frame, output).unwrap();
    let raw = owner.encode(&r.device, &mut pending).unwrap();
    assert!(
        owner
            .history
            .reset(1, false, pending.plan.unwrap().dimensions())
    );
    let mut post = r.device.create_command_encoder(&Default::default());
    // This is the graph's actual callback registration, not a manually held Arc.
    pending.completion(&mut post);
    let post = post.finish();
    drop(pending);
    drop(owner);
    assert!(
        scaler.upgrade().is_none(),
        "Rust scaler wrapper was retired"
    );
    assert!(
        targets.upgrade().is_some(),
        "unsubmitted completion retains targets"
    );
    // The raw Metal buffer retains its native scaler/referenced textures itself.
    drop(raw);
    assert!(
        targets.upgrade().is_some(),
        "post callback is the remaining Rust owner"
    );
    drop(post);
    assert!(
        targets.upgrade().is_none(),
        "abandoned callback releases targets"
    );
    r.check_device().unwrap();
    println!("prepared owner abandoned: raw_encodes=1 submissions=0 maps=0");
}

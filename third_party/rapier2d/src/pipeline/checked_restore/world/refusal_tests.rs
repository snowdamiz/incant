#[test]
fn checked_world_empty_joint_history_is_not_pristine() {
    use world::OmittedOwnerError as E;
    let mut w = PhysicsWorld::default();
    let a = w
        .insert(RigidBodyBuilder::dynamic(), world_cuboid(0.1, 0.1))
        .0;
    let b = w
        .insert(
            RigidBodyBuilder::dynamic().translation(world_vector(3., 0.)),
            world_cuboid(0.1, 0.1),
        )
        .0;
    let j = w
        .impulse_joints
        .insert(a, b, FixedJointBuilder::new(), false);
    w.impulse_joints.remove(j, false).unwrap();
    w.step();
    assert!(w.impulse_joints.is_empty());
    assert_eq!(
        w.impulse_joints.checkpoint_empty_gate(),
        Err(E::ImpulseArena)
    );
    let mut mb = MultibodyJointSet::new();
    let j = mb.insert(a, b, FixedJointBuilder::new(), false).unwrap();
    mb.remove(j, false);
    assert!(mb.multibodies().next().is_none());
    assert_eq!(mb.checkpoint_empty_gate(), Err(E::MultibodyArena));
}

#[test]
fn checked_world_empty_soft_removed_history_and_epoch_wrap() {
    use world::OmittedOwnerError as E;
    let mut soft = PhysicsWorld::default();
    let h = soft.insert_soft_body(SoftBodyBuilder::rope(
        world_vector(0., 0.),
        world_vector(1., 0.),
        3,
    ));
    soft.remove_soft_body(h).unwrap();
    soft.step();
    assert!(soft.soft_bodies.is_empty());
    assert_eq!(soft.soft_bodies.checkpoint_empty_gate(), Err(E::SoftArena));
    let mut w = PhysicsWorld::default();
    let (b, _) = w.insert(RigidBodyBuilder::dynamic(), world_cuboid(0.1, 0.1));
    w.step();
    w.impulse_joints.assembly_epoch = u32::MAX;
    let mut restored = CompletedCapture::step(&mut w).restore();
    assert_eq!(w.impulse_joints.assembly_epoch, u32::MAX);
    for world in [&mut w, &mut restored] {
        world.bodies[b].set_enabled(false);
        world.step();
    }
    assert_eq!(w.impulse_joints.assembly_epoch, 0);
    assert!(whole_equal(&mut w, &mut restored));
    // Actual native removal history still refuses even at a wrapped-zero epoch.
    let other = w.bodies.insert(RigidBodyBuilder::dynamic().build());
    let joint = w
        .impulse_joints
        .insert(b, other, FixedJointBuilder::new(), false);
    w.impulse_joints.remove(joint, false);
    w.impulse_joints.assembly_epoch = 0;
    assert_eq!(
        w.impulse_joints.checkpoint_empty_gate(),
        Err(E::ImpulseArena)
    );
}
#[test]
fn checked_world_private_envelope_header_and_budget_refuse() {
    let mut w = PhysicsWorld::default();
    w.insert_collider(world_cuboid(0.1, 0.1), None);
    let capture = CompletedCapture::step(&mut w);
    let bytes = &capture.bytes;
    for n in [0, 7, 15, bytes.len() - 1] {
        assert!(world::AssemblyFrame::decode(&bytes[..n]).is_err());
    }
    let mut bad = bytes.clone();
    bad[7] ^= 1;
    assert!(world::AssemblyFrame::decode(&bad).is_err());
    let mut bad = bytes.clone();
    bad[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(world::AssemblyFrame::decode(&bad).is_err());
    let mut bad = bytes.clone();
    bad.push(0);
    assert!(world::AssemblyFrame::decode(&bad).is_err());
    let a = world::AssemblyFrame::decode(bytes).unwrap();
    assert!(
        world::AssemblyFrame::encode(
            a.inner(),
            a.impulse_epoch(),
            &mut AllocationBudget::new(bytes.len() - 1, 1)
        )
        .is_err()
    );
}

#[test]
fn checked_world_identity_mismatch_refuses_before_construction() {
    let (mut w, _) = broad_three();
    let capture = CompletedCapture::step(&mut w);
    let assembly = world::AssemblyFrame::decode(&capture.bytes).unwrap();
    let mut wrong = identity();
    wrong.build[0] ^= 1;
    assert_eq!(
        with_decoded_world_boundary(
            assembly.inner(),
            wrong,
            &mut Budget::new(MAX_ALLOCATION).unwrap(),
            |_| Ok(())
        )
        .err(),
        Some(Error::Identity)
    );
    with_decoded_world_boundary(
        assembly.inner(),
        identity(),
        &mut Budget::new(MAX_ALLOCATION).unwrap(),
        |frame| {
            assert_eq!(
                admit_numeric_frame(frame, wrong, &mut WorkBudget::new(u64::MAX)).err(),
                Some(crate::pipeline::checked_restore::AdmissionError::Identity)
            );
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(assembly.inner(), world_bytes(capture.world));
}

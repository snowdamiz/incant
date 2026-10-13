//! Captures actual trusted native owners; no serde input ever constructs Rapier.
macro_rules! world_capture {
    ($rapier:ident,$bits:path,$posebits:path) => {
        fn world_config(p: &IntegrationParameters) -> IntegrationConfig {
            let value = serde_json::to_value(p).unwrap();
            assert_eq!(
                serde_json::to_vec(&p.soft_bodies).unwrap(),
                serde_json::to_vec(&$rapier::dynamics::SoftBodiesSettings::default()).unwrap()
            );
            IntegrationConfig {
                identity: identity(),
                dt: p.dt.to_bits(),
                min_ccd_dt: p.min_ccd_dt.to_bits(),
                contact_softness: ContactSoftness {
                    natural_frequency: p.contact_softness.natural_frequency.to_bits(),
                    damping_ratio: p.contact_softness.damping_ratio.to_bits(),
                },
                static_contact_softness: ContactSoftness {
                    natural_frequency: p.static_contact_softness.natural_frequency.to_bits(),
                    damping_ratio: p.static_contact_softness.damping_ratio.to_bits(),
                },
                warmstart_coefficient: p.warmstart_coefficient.to_bits(),
                length_unit: p.length_unit.to_bits(),
                normalized_allowed_linear_error: p.normalized_allowed_linear_error.to_bits(),
                normalized_max_corrective_velocity: p.normalized_max_corrective_velocity.to_bits(),
                normalized_prediction_distance: p.normalized_prediction_distance.to_bits(),
                normalized_max_linear_velocity: p.normalized_max_linear_velocity.to_bits(),
                normalized_contact_recycle_distance: p
                    .normalized_contact_recycle_distance
                    .to_bits(),
                num_solver_iterations: p.num_solver_iterations as u64,
                num_internal_pgs_iterations: p.num_internal_pgs_iterations as u64,
                num_internal_stabilization_iterations: p.num_internal_stabilization_iterations
                    as u64,
                max_ccd_substeps: p.max_ccd_substeps as u64,
                contact_clustering: p.contact_clustering,
                contact_recycling: p.contact_recycling,
                friction_in_bias_pass: p.friction_in_bias_pass,
                warmstart_joints: p.warmstart_joints,
                friction_model: match value.get("friction_model").and_then(Value::as_str) {
                    None => IntegrationFriction::Planar,
                    Some("Simplified") => IntegrationFriction::Simplified,
                    Some("Coulomb") => IntegrationFriction::Coulomb,
                    Some(v) => panic!("unsupported friction {v}"),
                },
                soft_bodies: IntegrationSoftBodies::PinnedDefaults,
            }
        }
        fn world_cache(w: &PhysicsWorld) -> CcdState {
            let cache = match w.ccd_solver.checkpoint_fixed_targets() {
                None => CcdCache::Uninitialized,
                Some((p, None)) => CcdCache::FullBvh {
                    prediction: p.to_bits(),
                },
                Some((p, Some(rows))) => CcdCache::Targets {
                    prediction: p.to_bits(),
                    targets: rows
                        .iter()
                        .map(|(h, a)| CcdTarget {
                            collider: handle(h.into_raw_parts()),
                            mins: $bits(a.mins),
                            maxs: $bits(a.maxs),
                        })
                        .collect(),
                },
            };
            CcdState {
                identity: identity(),
                cache,
            }
        }
        fn with_world<T>(w: &mut PhysicsWorld, f: impl FnOnce(WorldBoundary<'_>) -> T) -> T {
            // Only tests call this immediately after their own completed positive step.
            // The library never authenticates that phase from empty counts or a label.
            assert!(w.integration_parameters.dt > 0.);
            assert!(
                w.impulse_joints.is_empty()
                    && w.multibody_joints.multibodies().next().is_none()
                    && w.soft_bodies.is_empty()
            );
            let o = w.physics_pipeline.checkpoint_observation().unwrap();
            let bv = ordered_capture::capture(&w.bodies);
            let cv = ordered_capture::capture(&w.colliders);
            let (ba, bh) = layout(&bv["bodies"], identity());
            let (ca, ch) = layout(&cv["colliders"], identity());
            let journals = PendingJournals {
                modified_bodies: bv["modified_bodies"][0].as_array().unwrap().len() as u32,
                modified_colliders: cv["modified_colliders"][0].as_array().unwrap().len() as u32,
                removed_colliders: cv["removed_colliders"].as_array().unwrap().len() as u32,
            };
            let mut a = AttachmentRecord {
                identity: identity(),
                bodies: vec![],
                colliders: vec![],
                attached: vec![],
            };
            let mut bodies = vec![];
            let mut colliders = vec![];
            let mut shapes = vec![];
            for h in bh {
                let body = &w.bodies[RigidBodyHandle::from_raw_parts(h.slot, h.generation)];
                a.bodies.push(AttachmentSpan {
                    start: a.attached.len() as u32,
                    count: body.colliders().len() as u32,
                });
                a.attached
                    .extend(body.colliders().iter().map(|h| handle(h.into_raw_parts())));
                bodies.push(body_capture::body(
                    &ordered_capture::capture(body),
                    identity().dimension,
                ));
            }
            for h in ch {
                let co = &w.colliders[ColliderHandle::from_raw_parts(h.slot, h.generation)];
                a.colliders.push(co.parent().map(|h| ParentAttachment {
                    body: handle(h.into_raw_parts()),
                    local_pose: $posebits(co.position_wrt_parent().unwrap()),
                }));
                colliders.push(collider_capture::collider(
                    &ordered_capture::capture(co),
                    identity().dimension,
                ));
                shapes.push(geometry(co));
            }
            let a = validate_attachments(a, &ba, &ca, &mut budget()).unwrap();
            let bodies = validate_body_state(
                BodyStateTable {
                    identity: identity(),
                    bodies,
                },
                BodyContext {
                    attachments: &a,
                    journals,
                },
            )
            .unwrap();
            let colliders = validate_collider_state(
                ColliderStateTable {
                    identity: identity(),
                    colliders,
                },
                ColliderContext {
                    bodies: &bodies,
                    geometries: &shapes,
                    journals,
                },
            )
            .unwrap();
            let raw = w.narrow_phase.contact_graph().raw_graph();
            let g = graph(&ordered_capture::capture(raw), identity());
            let mut t = ManifoldTable {
                identity: identity(),
                nodes: raw
                    .raw_nodes()
                    .iter()
                    .map(|n| handle(n.weight.into_raw_parts()))
                    .collect(),
                pairs: vec![],
                manifolds: vec![],
                points: vec![],
                solver: vec![],
            };
            for e in raw.raw_edges() {
                let p = &e.weight;
                let rigid = p.rigid().unwrap();
                let plain = RecordSpan {
                    start: t.manifolds.len() as u32,
                    count: rigid.manifolds.len() as u32,
                };
                for m in &rigid.manifolds {
                    push(&mut t, m);
                }
                let clusters = RecordSpan {
                    start: t.manifolds.len() as u32,
                    count: rigid.solver_clusters.len() as u32,
                };
                for m in &rigid.solver_clusters {
                    push(&mut t, m);
                }
                t.pairs.push(ContactPairManifolds {
                    colliders: [
                        handle(p.collider1.into_raw_parts()),
                        handle(p.collider2.into_raw_parts()),
                    ],
                    plain,
                    clusters,
                });
            }
            let m = validate_manifolds(
                t,
                ManifoldContext {
                    graph: &g,
                    attachments: &a,
                    geometries: &shapes,
                    journals,
                },
                &mut budget(),
            )
            .unwrap();
            let pairs = validate_pair_state(pair_states(w), &m, &mut budget()).unwrap();
            let islands = validate_islands(
                island_capture::capture(&ordered_capture::capture(&w.islands), identity()),
                IslandContext {
                    bodies: &bodies,
                    manifolds: &m,
                    domain: IslandDomain::default(),
                },
                &mut budget(),
            )
            .unwrap();
            let scan = w.islands.checkpoint_sleep_scan().unwrap();
            let multibody_epoch =
                word(&ordered_capture::capture(&w.multibody_joints)["topology_epoch"]);
            let rigid = validate_rigid_owner(
                rigid_owner_capture::capture(
                    &ordered_capture::capture(&w.narrow_phase),
                    identity(),
                    scan.stamp,
                    &scan.slots,
                ),
                RigidOwnerContext {
                    islands: &islands,
                    pairs: &pairs,
                    colliders: &colliders,
                    multibody_epoch,
                },
                &mut budget(),
            )
            .unwrap();
            let (indices, n) = narrow_record(w);
            let indices = validate_coarena(indices, &ca, &mut budget()).unwrap();
            let intersections = graph(
                &ordered_capture::capture(w.narrow_phase.intersection_graph().raw_graph()),
                identity(),
            );
            let narrow = validate_narrow_owner(
                n,
                NarrowOwnerContext {
                    rigid: &rigid,
                    indices: &indices,
                    intersections: &intersections,
                },
                &mut budget(),
            )
            .unwrap();
            let (tree, adj, broad) =
                broad_capture::capture(&ordered_capture::capture(&w.broad_phase), identity());
            let tree = validate_pending_bvh(
                tree,
                PendingBvhContext {
                    colliders: &colliders,
                    pending_aabbs: &broad.pending_aabbs,
                },
                &mut budget(),
            )
            .unwrap();
            let adj = validate_coarena(adj, &ca, &mut budget()).unwrap();
            let broad = validate_broad_owner(
                broad.clone(),
                BroadOwnerContext {
                    tree: &tree,
                    narrow: &narrow,
                    adjacency: &adj,
                },
                &mut budget(),
            )
            .unwrap();
            let integration =
                validate_integration_config(world_config(&w.integration_parameters)).unwrap();
            let ccd = validate_ccd_state(
                world_cache(w),
                CcdContext {
                    colliders: &colliders,
                    integration: &integration,
                },
            )
            .unwrap();
            let record = PipelineState {
                identity: identity(),
                gravity: $bits(w.gravity),
                post_solve_ccd_active: o.post_solve_ccd_active,
                phase: PipelinePhase::CompletedPositive,
                deferred_bvh_receiver: o.deferred_bvh_receiver,
                deferred_bvh_inline: o.deferred_bvh_inline,
                quarantine: [
                    o.quarantine_bodies as u64,
                    o.quarantine_colliders as u64,
                    o.quarantine_soft_bodies as u64,
                    o.quarantine_body_workspace as u64,
                    o.quarantine_collider_workspace as u64,
                ],
            };
            let pipeline = validate_pipeline_state(
                record,
                PipelineContext {
                    integration: &integration,
                    ccd: &ccd,
                },
            )
            .unwrap();
            f(join_world_boundary(&broad, &pipeline).unwrap())
        }
        fn world_bytes(w: &mut PhysicsWorld) -> Vec<u8> {
            with_world(w, |v| {
                let bytes = encode_world_boundary(v, &mut budget()).unwrap();
                let mut shared = budget();
                let second =
                    with_decoded_world_boundary(&bytes, identity(), &mut shared, |decoded| {
                        assert_eq!(decoded.broad().record(), v.broad().record());
                        assert_eq!(decoded.pipeline().record(), v.pipeline().record());
                        encode_world_boundary(decoded, &mut budget())
                    })
                    .unwrap();
                assert_eq!(bytes, second);
                bytes
            })
        }
    };
}

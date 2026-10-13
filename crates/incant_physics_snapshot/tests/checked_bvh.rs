//! Actual public native mutations around typed BVH storage reconstruction.
//! A tree is not a complete restored broad/world owner. No native deserialization.
#![cfg(feature = "native-admission")]
#[path = "support/bvh_record.rs"]
mod bvh_record;
use incant_physics_snapshot::construction::*;
use incant_physics_snapshot::*;
fn budget() -> Budget {
    Budget::new(MAX_ALLOCATION).unwrap()
}
fn permit(plan: AllocationPlan) -> AllocationPermit {
    AllocationBudget::new(plan.bytes(), plan.allocations())
        .admit(plan)
        .unwrap()
}
macro_rules! cases {
    ($module:ident,$parry:ident,$dim:ident,$vector:expr,$fixture:expr) => {
        mod $module {
            use super::*;
            use $parry::{bounding_volume::Aabb,math::Vector,partitioning::{Bvh,BvhWorkspace}};
            fn identity() -> Identity { Identity { dimension: Dimension::$dim, build: [92;32] } }
            fn aabb(i:u32) -> Aabb {
                let v=$vector;let x=i as f32*0.125;
                Aabb::new(v(x,-0.0),v(x+1.,1.))
            }
            fn record(tree:&Bvh)->BvhRecord { bvh_record::capture(serde_json::to_value(tree).unwrap(),identity()) }
            fn restore(tree:&Bvh)->Bvh {
                let r=record(tree);let checked=validate(r.clone(),&mut budget()).unwrap();
                let raw=encode(&checked,&mut budget()).unwrap();
                let checked=decode(&raw,identity(),&mut budget()).unwrap();
                let plan=Bvh::checkpoint_plan(&checked,identity()).unwrap();let mut p=permit(plan);
                let out=Bvh::from_checkpoint(&checked,identity(),&mut p).unwrap();p.finish().unwrap();
                assert_eq!(record(&out),r);out
            }
            #[test]
            fn checked_bvh_exact_storage_and_native_mutation_order_continue() {
                let mut original=Bvh::new();let mut scratch=BvhWorkspace::default();
                let mut restored=restore(&original);let mut fresh=BvhWorkspace::default();let mut steps=0;
                for tick in 0..128 {
                    let id=tick*3+5;original.insert(aabb(tick),id);restored.insert(aabb(tick),id);
                    assert_eq!(record(&original),record(&restored));steps+=1;
                    if [0,1,15,63,127].contains(&tick) {
                        original.refit(&mut scratch);restored.refit(&mut fresh);
                        eprintln!("STRICT_CAPTURE insert_tick={tick}");
                        if tick==0 {assert_eq!(record(&original).nodes[0].left.mins[1],(-0.0f32).to_bits());}
                        restored=restore(&original);fresh=BvhWorkspace::default();
                    }
                }
                let last=127*3+5;original.remove(last);restored.remove(last);
                assert_eq!(record(&original),record(&restored));
                let tail=record(&original);assert_eq!(tail.leaves.slots.last(),Some(&None));
                restored=restore(&original);fresh=BvhWorkspace::default();
                original.insert(aabb(127),last);restored.insert(aabb(127),last);
                assert_eq!(record(&original),record(&restored));steps+=2;
                original.refit(&mut scratch);restored.refit(&mut fresh);
                for tick in 0..96 {
                    let id=(tick%32)*3+5;
                    original.remove(id);restored.remove(id);
                    assert_eq!(record(&original),record(&restored));
                    if tick%7==0 {eprintln!("STRICT_CAPTURE remove_tick={tick}");restored=restore(&original);fresh=BvhWorkspace::default();}
                    original.insert(aabb(tick+250),id);restored.insert(aabb(tick+250),id);
                    if tick%2==0 {original.refit(&mut scratch);restored.refit(&mut fresh);}
                    else {original.optimize_incremental(&mut scratch);restored.optimize_incremental(&mut fresh);original.refit_without_resolve(&mut scratch);restored.refit_without_resolve(&mut fresh);}
                    assert_eq!(record(&original),record(&restored));steps+=1;
                }
                assert_eq!(steps,226);
                eprintln!("CHECKED_BVH dimension={} paired_mutation_steps={steps} exact_order=true fresh_workspace=true",identity().dimension as u8);
            }
            #[test]
            fn checked_bvh_identity_budget_and_hostile_input_refuse() {
                let mut tree=Bvh::new();for i in 0..8 {tree.insert(aabb(i),i*2);}
                let r=record(&tree);let checked=validate(r.clone(),&mut budget()).unwrap();
                let plan=Bvh::checkpoint_plan(&checked,identity()).unwrap();
                assert!(AllocationBudget::new(plan.bytes()-1,plan.allocations()).admit(plan).is_err());
                assert!(AllocationBudget::new(plan.bytes(),plan.allocations()-1).admit(plan).is_err());
                let mut p=permit(plan);let mut wrong=identity();wrong.build[0]^=1;
                assert!(Bvh::checkpoint_plan(&checked,wrong).is_err());
                assert!(Bvh::from_checkpoint(&checked,wrong,&mut p).is_err());
                wrong=identity();wrong.dimension=if wrong.dimension==Dimension::Planar {Dimension::Spatial}else{Dimension::Planar};
                assert!(Bvh::from_checkpoint(&checked,wrong,&mut p).is_err());
                let out=Bvh::from_checkpoint(&checked,identity(),&mut p).unwrap();assert_eq!(record(&out),r);
                assert!(Bvh::from_checkpoint(&checked,identity(),&mut p).is_err());p.finish().unwrap();
                let mut partial=AllocationPlan::array::<$parry::partitioning::BvhNodeWide>(r.nodes.len()).unwrap();
                partial.append(AllocationPlan::array::<$parry::partitioning::BvhNodeIndex>(r.parents.len()).unwrap()).unwrap();
                let mut partial=permit(partial);
                assert!(Bvh::from_checkpoint(&checked,identity(),&mut partial).is_err());partial.finish().unwrap();
                let mut short=permit(AllocationPlan::array::<u8>(1).unwrap());
                assert!(Bvh::from_checkpoint(&checked,identity(),&mut short).is_err());
                let raw=encode(&checked,&mut budget()).unwrap();
                for end in 0..raw.len() {assert!(decode(&raw[..end],identity(),&mut budget()).is_err());}
                let mut bad=r.clone();bad.parents[0]=u32::MAX;assert!(validate(bad,&mut budget()).is_err());
                let mut bad=r;bad.nodes[0].left.children=u32::MAX;assert!(validate(bad,&mut budget()).is_err());
            }
            #[test]
            fn checked_bvh_pending_bounds_are_exact_until_native_processing() {
                with_decoded_world_boundary($fixture,identity(),&mut budget(),|frame| {
                    let pending=frame.broad().context().tree;
                    let plan=Bvh::checkpoint_pending_plan(pending,identity()).unwrap();let mut p=permit(plan);
                    let exact=Bvh::from_pending_checkpoint(pending,identity(),&mut p).unwrap();p.finish().unwrap();
                    assert_eq!(&record(&exact),pending.record());
                    // Independently mutate a live native tree against actual valid
                    // collider handles. AABB/shape coherence is not claimed here.
                    let colliders=pending.context().colliders;
                    let mut handles=Vec::new();
                    for pair in &frame.broad().record().pairs { for h in pair.colliders { if !handles.contains(&h) { handles.push(h); } } }
                    assert!(handles.len()>=3);
                    let mut tree=Bvh::new();let mut scratch=BvhWorkspace::default();
                    for (i,h) in handles.iter().enumerate() {tree.insert(aabb(i as u32),h.slot);}
                    tree.refit(&mut scratch);
                    let before=record(&tree);
                    let moved=*handles.iter().find(|h| before.leaves.slots[h.slot as usize].unwrap()/2!=0).unwrap();
                    tree.insert_or_update_partially(aabb(500),moved.slot,0.125);
                    let r=record(&tree);assert!(validate(r.clone(),&mut budget()).is_err());
                    let queue=[moved];
                    let c=PendingBvhContext {colliders,pending_aabbs:&queue};
                    let witness=validate_pending_bvh(r.clone(),c,&mut budget()).unwrap();
                    let raw=encode_pending_bvh(&witness,&mut budget()).unwrap();
                    let loaded=decode_pending_bvh(&raw,c,&mut budget()).unwrap();
                    let plan=Bvh::checkpoint_pending_plan(&loaded,identity()).unwrap();let mut p=permit(plan);
                    let mut out=Bvh::from_pending_checkpoint(&loaded,identity(),&mut p).unwrap();p.finish().unwrap();
                    assert_eq!(record(&out),r);
                    assert!(validate_pending_bvh(r,PendingBvhContext {pending_aabbs:&[],..c},&mut budget()).is_err());
                    let mut fresh=BvhWorkspace::default();
                    tree.refit(&mut scratch);out.refit(&mut fresh);assert_eq!(record(&tree),record(&out));
                    for i in 0..32 {tree.optimize_incremental(&mut scratch);out.optimize_incremental(&mut fresh);assert_eq!(record(&tree),record(&out));if i%4==0 {tree.refit(&mut scratch);out.refit(&mut fresh);}}
                    eprintln!("CHECKED_BVH_PENDING dimension={} refit=1 optimize=32 unnormalized_bits=true",identity().dimension as u8);
                    Ok(())
                }).unwrap();
            }
        }
    };
}
cases!(
    planar,
    parry2d,
    Planar,
    |x, y| Vector::new(x, y),
    include_bytes!(
        "../../../third_party/rapier2d/src/pipeline/checked_restore/island_fixtures/0.bin"
    )
);
cases!(
    spatial,
    parry3d,
    Spatial,
    |x, y| Vector::new(x, y, 0.),
    include_bytes!(
        "../../../third_party/rapier3d/src/pipeline/checked_restore/island_fixtures/0.bin"
    )
);

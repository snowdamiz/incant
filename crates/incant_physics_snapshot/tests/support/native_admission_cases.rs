//! Actual native worlds and hostile inert bytes; no input constructs live state.
macro_rules! native_admission_cases {
 ($rapier:ident,$vector:path,$cuboid:path)=>{
    fn native_check(w:&mut PhysicsWorld)->$rapier::pipeline::checked_restore::WorkReport {
        use $rapier::pipeline::checked_restore::*;
        let before=serde_json::to_vec(w).unwrap();
        let report=with_world(w,|f|admit_numeric_frame(f,identity(),&mut WorkBudget::new(10_000_000)).unwrap().report());
        assert_eq!(serde_json::to_vec(w).unwrap(),before);report
    }
    #[test] fn native_admission_actual_primitives_and_derived_mass_are_bit_exact(){
        for shape in 0..3 {
            let(mut w,body,co)=fixture(false);
            match shape {
                1=>w.colliders[co].set_shape(SharedShape::ball(0.45)),
                2=>w.colliders[co].set_shape(SharedShape::capsule_y(0.3,0.35)),_=>{}
            }
            for tick in 0..24 {
                match tick {
                    3=>w.bodies[body].set_additional_mass(2.,true),
                    6=>w.colliders[co].set_density(0.),
                    9=>w.colliders[co].set_mass(3.),
                    12=>w.bodies[body].lock_rotations(true,true),
                    15=>w.bodies[body].set_body_type(RigidBodyType::KinematicVelocityBased,true),
                    18=>w.bodies[body].set_body_type(RigidBodyType::Dynamic,true),_=>{}
                }
                w.step();let report=native_check(&mut w);assert!(report.validation_units>0);
            }
        }
        eprintln!("NATIVE_ADMISSION dimension={} primitive_steps=72 exact_derived_mass=true saved_bits_unchanged=true",identity().dimension as u8);
    }
    #[test] fn native_admission_work_identity_and_compound_refusal(){
        use $rapier::pipeline::checked_restore::*;
        let(mut w,_,_)=fixture(false);w.step();
        with_world(&mut w,|f|{
            let report=admit_numeric_frame(f,identity(),&mut WorkBudget::new(u64::MAX)).unwrap().report();
            let units=report.validation_units+report.conservative_solver_units;
            let mut exact=WorkBudget::new(units);
            admit_numeric_frame(f,identity(),&mut exact).unwrap();assert_eq!(exact.remaining(),0);
            assert_eq!(admit_numeric_frame(f,identity(),&mut exact).err(),Some(AdmissionError::Work));
            assert_eq!(admit_numeric_frame(f,identity(),&mut WorkBudget::new(units-1)).err(),Some(AdmissionError::Work));
            let mut b=WorkBudget::new(100);
            assert_eq!(admit_numeric_frame(f,Identity{build:[8;32],..identity()},&mut b).err(),Some(AdmissionError::Identity));assert_eq!(b.remaining(),100);
        });
        let(mut compound,_,_)=fixture(true);compound.step();with_world(&mut compound,|f|assert_eq!(admit_numeric_frame(f,identity(),&mut WorkBudget::new(u64::MAX)).err(),Some(AdmissionError::UnsupportedGeometry)));
    }
    #[test] fn native_admission_rejects_inert_valid_derived_and_work_forgery(){
        use $rapier::pipeline::checked_restore::*;
        let(mut w,_,_)=fixture(false);w.step();let original=world_bytes(&mut w);
        let offset=|tag:usize|224+(0..tag).map(|i|u32::from_le_bytes(original[52+i*8..56+i*8].try_into().unwrap())as usize).sum::<usize>();
        let check=|bytes:&[u8]|with_decoded_world_boundary(bytes,identity(),&mut budget(),|f|Ok(admit_numeric_frame(f,identity(),&mut WorkBudget::new(10_000_000)).err())).unwrap();
        for (field,error) in [(168,AdmissionError::WorldMass),(228,AdmissionError::LocalMass),(312,AdmissionError::Extent)]{
            let mut bytes=original.clone();let start=offset(5)+52+field;let value=f32::from_bits(u32::from_le_bytes(bytes[start..start+4].try_into().unwrap()));bytes[start..start+4].copy_from_slice(&(value+0.25).to_bits().to_le_bytes());
            assert_eq!(check(&bytes),Some(error),"field={field}");
        }
        for field in [96,104,112,120] {let mut bytes=original.clone();let start=offset(19)+field;bytes[start..start+8].copy_from_slice(&u64::MAX.to_le_bytes());assert_eq!(check(&bytes),Some(AdmissionError::Work));}
        assert_eq!(world_bytes(&mut w),original);
    }
    #[test] fn native_admission_exact_historical_ccd_aabb_refuses_forgery(){
        use $rapier::pipeline::checked_restore::*;
        let mut w=PhysicsWorld::default();w.gravity=$vector(0.,0.);w.insert_collider($cuboid(0.25,0.25),None);
        w.insert(RigidBodyBuilder::dynamic().translation($vector(-20.,10.)).linvel($vector(60.,0.)),$cuboid(0.1,0.1));w.step();native_check(&mut w);
        assert!(matches!(world_cache(&w).cache,CcdCache::Targets{..}));
        let mut bytes=world_bytes(&mut w);let start=224+(0..20).map(|i|u32::from_le_bytes(bytes[52+i*8..56+i*8].try_into().unwrap())as usize).sum::<usize>()+60+8;
        let value=f32::from_bits(u32::from_le_bytes(bytes[start..start+4].try_into().unwrap()));bytes[start..start+4].copy_from_slice(&(value-1.).to_bits().to_le_bytes());
        let error=with_decoded_world_boundary(&bytes,identity(),&mut budget(),|f|Ok(admit_numeric_frame(f,identity(),&mut WorkBudget::new(10_000_000)).err())).unwrap();assert_eq!(error,Some(AdmissionError::CcdBounds));
    }
    #[test] fn native_admission_native_forces_and_scalar_overflow(){
        use $rapier::pipeline::checked_restore::*;
        let(mut w,body,_)=fixture(false);
        w.gravity=$vector(-0.0,-9.81);
        w.bodies[body].set_gravity_scale(-0.0,true);
        w.bodies[body].add_force($vector(2.,-3.),true);
        w.bodies[body].lock_translations(true,true);
        w.step();native_check(&mut w);
        assert_eq!(w.gravity.x.to_bits(), (-0.0f32).to_bits());
        assert_eq!(w.bodies[body].gravity_scale().to_bits(), (-0.0f32).to_bits());
        w.bodies[body].lock_translations(false,true);
        w.bodies[body].set_gravity_scale(2.,true);
        for _ in 0..8 {w.step();native_check(&mut w);}
        let original=world_bytes(&mut w);
        let mut bytes=original.clone();
        let start=224+(0..5).map(|i|u32::from_le_bytes(bytes[52+i*8..56+i*8].try_into().unwrap())as usize).sum::<usize>()+52;
        // Row zero is the dynamic fixture body. Every scalar remains finite,
        // but the actual next native force construction overflows.
        bytes[start+128..start+132].copy_from_slice(&f32::MAX.to_bits().to_le_bytes());
        let error=with_decoded_world_boundary(&bytes,identity(),&mut budget(),|f|Ok(admit_numeric_frame(f,identity(),&mut WorkBudget::new(10_000_000)).err())).unwrap();
        assert_eq!(error,Some(AdmissionError::Arithmetic));
        assert_eq!(world_bytes(&mut w),original);
    }
    #[test] fn native_admission_body_extra_native_key_order(){
        use $rapier::pipeline::checked_restore::*;
        // Small actual native completed steps establish the ordinary extra-count
        // path. Huge values below are inert arithmetic controls, never stepped.
        for extra in [0usize,1,2,4] {
            let(mut w,body,_)=fixture(false);
            w.bodies[body].set_additional_solver_iterations(extra);
            w.bodies[body].set_additional_pgs_iterations(extra);
            w.step();let report=native_check(&mut w);
            assert_eq!(report.max_substeps,w.integration_parameters.num_solver_iterations+extra);
            assert_eq!(report.max_pgs,w.integration_parameters.num_internal_pgs_iterations+extra);
        }
        let(mut w,_,_)=fixture(false);w.step();
        let original=world_bytes(&mut w);let base=native_check(&mut w);
        let start=224+(0..5).map(|i|u32::from_le_bytes(original[52+i*8..56+i*8].try_into().unwrap())as usize).sum::<usize>()+52+416;
        for raw in [0usize,1,0xffff,0x1_0000,usize::MAX] {
            let mut bytes=original.clone();
            for off in [0,8] {bytes[start+off..start+off+8].copy_from_slice(&(raw as u64).to_le_bytes());}
            with_decoded_world_boundary(&bytes,identity(),&mut budget(),|f|{
                let report=admit_numeric_frame(f,identity(),&mut WorkBudget::new(u64::MAX)).unwrap().report();
                let extra=(raw as u32).min(0xffff)as usize;
                assert_eq!(report.max_substeps,base.max_substeps+extra);
                assert_eq!(report.max_pgs,base.max_pgs+extra);Ok(())
            }).unwrap();
        }
        #[cfg(target_pointer_width="64")]
        {
            let mut bytes=original.clone();
            for off in [0,8] {bytes[start+off..start+off+8].copy_from_slice(&(1u64<<32).to_le_bytes());}
            with_decoded_world_boundary(&bytes,identity(),&mut budget(),|f|{
                let report=admit_numeric_frame(f,identity(),&mut WorkBudget::new(u64::MAX)).unwrap().report();
                assert_eq!(report.max_substeps,base.max_substeps);assert_eq!(report.max_pgs,base.max_pgs);Ok(())
            }).unwrap();
        }
        assert_eq!(world_bytes(&mut w),original);
    }
 };
}

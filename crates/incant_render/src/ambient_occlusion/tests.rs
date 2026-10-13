use super::{math::*, pipelines::material_source};
use crate::camera::CameraView;
use glam::{Mat4, Vec2, Vec3, Vec4};

fn settings() -> AmbientOcclusion {
    AmbientOcclusion {
        radius: 2.,
        bias: 0.01,
        strength: 1.,
    }
}
fn camera(orthographic: bool) -> CameraView {
    CameraView {
        view: Mat4::IDENTITY,
        eye: Vec3::ZERO,
        forward: -Vec3::Z,
        near: 0.1,
        far: 100.,
        fov: 1.,
        orthographic_half_height: orthographic.then_some(4.),
        pixel_resolution: None,
        jitter: [0.; 2],
        jitter_guard: [0.; 2],
    }
}
#[test]
fn typed_parameters_and_attachment_budget_reject_before_preparation() {
    settings().validate().unwrap();
    for radius in [0., -1., f32::NAN, f32::INFINITY, 1001., f32::MIN_POSITIVE] {
        assert_eq!(
            AmbientOcclusion {
                radius,
                ..settings()
            }
            .validate(),
            Err(AmbientOcclusionError::Parameters)
        );
    }
    for bias in [-1., 2., f32::NAN, f32::INFINITY] {
        assert!(AmbientOcclusion { bias, ..settings() }.validate().is_err());
    }
    for strength in [-0.01, 1.01, f32::NAN, f32::INFINITY] {
        assert!(
            AmbientOcclusion {
                strength,
                ..settings()
            }
            .validate()
            .is_err()
        );
    }
    assert_eq!(target_bytes([2048, 2048]).unwrap(), 56 * 1024 * 1024);
    assert_eq!(target_bytes([8192, 512]).unwrap(), 56 * 1024 * 1024);
    for size in [
        [0, 1],
        [1, 0],
        [2048, 2049],
        [u32::MAX, u32::MAX],
        [8193, 1],
    ] {
        assert!(target_bytes(size).is_err());
    }
}
#[test]
fn hemisphere_samples_are_unit_cosine_strata_not_uniform_solid_angle() {
    let kernel = directions();
    for (i, k) in kernel.iter().enumerate() {
        assert!((Vec3::from_slice(k).length_squared() - 1.).abs() < 2e-7);
        assert!(k[2] > 0.);
        assert!((k[2] * k[2] - (1. - (i as f32 + 0.5) / 16.)).abs() < 1e-7);
    }
    assert_eq!(kernel, directions());
    assert!((kernel.iter().map(|k| k[2] * k[2]).sum::<f32>() / 16. - 0.5).abs() < 1e-7);
}
#[test]
fn inverse_projection_recovers_perspective_and_orthographic_view_units_at_offset_viewports() {
    for orthographic in [false, true] {
        for rect in [
            [0., 0., 640., 480.],
            [13.25, 29.75, 320.5, 180.5],
            [0., 0., 161., 91.],
        ] {
            let c = camera(orthographic);
            let u = Uniform::new(c, rect, [800, 600], settings()).unwrap();
            let projection = Mat4::from_cols_array_2d(&u.projection);
            let inverse = Mat4::from_cols_array_2d(&u.inverse);
            for p in [
                Vec3::new(0., 0., -0.1),
                Vec3::new(0.04, -0.02, -1.),
                Vec3::new(0.2, 0.3, -20.),
            ] {
                let ndc = projection.project_point3(p);
                let pixel = Vec2::new(rect[0], rect[1])
                    + (ndc.truncate() * Vec2::new(0.5, -0.5) + Vec2::splat(0.5))
                        * Vec2::new(rect[2], rect[3]);
                let restored = reconstruct(inverse, rect, pixel, ndc.z.max(0.)).unwrap();
                assert!(
                    (restored - p).length() < 0.0003,
                    "{orthographic} {p:?} {restored:?}"
                );
            }
        }
    }
}
#[test]
fn invalid_projection_and_outside_background_samples_never_generate_coordinates() {
    let mut c = camera(false);
    c.near = 0.;
    assert!(Uniform::new(c, [0., 0., 100., 100.], [100, 100], settings()).is_err());
    for rect in [
        [0., 0., 0., 10.],
        [-1., 0., 10., 10.],
        [0., 0., 101., 10.],
        [0., f32::NAN, 10., 10.],
    ] {
        assert!(Uniform::new(camera(false), rect, [100, 100], settings()).is_err());
    }
    let rect = [10., 20., 100., 100.];
    for (pixel, depth) in [
        (Vec2::new(9., 50.), 0.5),
        (Vec2::new(110., 50.), 0.5),
        (Vec2::new(30., 50.), 1.),
        (Vec2::new(30., 50.), f32::NAN),
    ] {
        assert!(reconstruct(Mat4::IDENTITY, rect, pixel, depth).is_none());
    }
}
#[test]
fn all_actual_wgsl_sources_validate_without_a_device() {
    for (source, entries) in [
        (include_str!("ao.wgsl").to_owned(), 2),
        (include_str!("filter.wgsl").to_owned(), 3),
        (material_source(true).unwrap(), 2),
        (material_source(false).unwrap(), 2),
    ] {
        let module = naga::front::wgsl::parse_str(&source)
            .unwrap_or_else(|e| panic!("{}", e.emit_to_string(&source)));
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        )
        .validate(&module)
        .unwrap();
        assert_eq!(module.entry_points.len(), entries);
    }
}

#[test]
fn sampling_and_filter_share_exact_host_uniform_and_typed_texture_bindings() {
    for (source, binding_count) in [
        (include_str!("ao.wgsl"), 3),
        (include_str!("filter.wgsl"), 4),
    ] {
        let module = naga::front::wgsl::parse_str(source).unwrap();
        let globals: Vec<_> = module.global_variables.iter().map(|(_, v)| v).collect();
        assert_eq!(globals.len(), binding_count);
        let uniform = globals
            .iter()
            .find(|v| v.binding.as_ref().unwrap().binding == 0)
            .unwrap();
        assert_eq!(uniform.space, naga::AddressSpace::Uniform);
        let naga::TypeInner::Struct { members, span } = &module.types[uniform.ty].inner else {
            panic!("expected uniform struct")
        };
        assert_eq!(*span as usize, size_of::<Uniform>());
        assert_eq!(
            members.iter().map(|m| m.offset).collect::<Vec<_>>(),
            [0, 64, 128, 192, 208, 224, 480]
        );
        for member in &members[5..] {
            assert!(matches!(module.types[member.ty].inner,
                naga::TypeInner::Array { stride: 16, size: naga::ArraySize::Constant(n), .. } if n.get() == 16));
        }
        for global in globals.iter().skip(1) {
            let binding = global.binding.as_ref().unwrap();
            assert_eq!(binding.group, 0);
            let naga::TypeInner::Image {
                dim,
                arrayed,
                class,
            } = module.types[global.ty].inner
            else {
                panic!("expected texture")
            };
            assert_eq!(dim, naga::ImageDimension::D2);
            assert!(!arrayed);
            assert_eq!(
                class,
                if binding.binding == 1 {
                    naga::ImageClass::Depth { multi: false }
                } else {
                    naga::ImageClass::Sampled {
                        kind: naga::ScalarKind::Float,
                        multi: false,
                    }
                }
            );
        }
    }
}
#[test]
fn forward_variant_changes_only_diffuse_irradiance_and_prepass_reuses_actual_vertex_and_mask() {
    let original = include_str!("../model_material.wgsl");
    let forward = material_source(false).unwrap();
    let needle = "(1.0-reflectance)*(1.0-metallic)*base.rgb*irradiance";
    let rewritten =
        format!("({needle})*textureLoad(diffuse_accessibility,vec2i(in.position.xy),0).r");
    assert_eq!(forward.matches(&rewritten).count(), 1);
    assert!(forward.replace(&rewritten, needle).ends_with(original));
    let prepass = material_source(true).unwrap();
    assert!(prepass.starts_with(original.split_once("@fragment fn fragment").unwrap().0));
    assert!(prepass.contains("material.flags.x==1u && base.a<material.factors.w"));
    assert!(prepass.contains("material.flags.z!=0u && !front"));
}

// Historical unrotated first-layer control. The independent quality oracle tests
// the repaired kernel and staged filter. Surface depth here comes from an
// independent analytic plane/foreground half-plane, never a fabricated AO image.
fn accessibility(orthographic: bool, obstacle: bool, normal: Vec3, strength: f32) -> f32 {
    let c = camera(orthographic);
    let rect = [0., 0., 512., 512.];
    let projection = c.projection(1.).unwrap();
    let inverse = projection.inverse();
    let p = Vec3::new(0., 0., -4.);
    let axis = if normal.z.abs() > 0.9 {
        Vec3::Y
    } else {
        Vec3::Z
    };
    let tangent = axis.cross(normal).normalize();
    let bitangent = normal.cross(tangent);
    let mut blocked = 0.;
    for k in directions() {
        let direction = tangent * k[0] + bitangent * k[1] + normal * k[2];
        for step in 1..=STEPS {
            let q = p + direction * settings().radius * step as f32 / STEPS as f32;
            let clip = projection * q.extend(1.);
            if clip.w <= 0. || clip.z < 0. || clip.z >= clip.w {
                continue;
            }
            let uv = clip.truncate().truncate() / clip.w * Vec2::new(0.5, -0.5) + Vec2::splat(0.5);
            if uv.min_element() < 0. || uv.max_element() >= 1. {
                continue;
            }
            let pixel = (uv * 512.).floor() + Vec2::splat(0.5);
            let ray_at = |z: f32| {
                let depth = projection.project_point3(Vec3::new(0., 0., z)).z;
                reconstruct(inverse, rect, pixel, depth).unwrap()
            };
            let foreground = ray_at(-3.);
            let surface = if obstacle && foreground.x >= 0.4 {
                foreground
            } else {
                ray_at(-4.)
            };
            let delta = surface - p;
            if surface.z >= q.z + settings().bias
                && delta.dot(normal) > settings().bias
                && delta.length_squared() <= settings().radius.powi(2)
            {
                blocked += 1.;
                break;
            }
        }
    }
    1. - strength * blocked / DIRECTIONS as f32
}
#[test]
fn analytic_flat_surface_stays_open_and_nearby_foreground_only_occludes_facing_hemisphere() {
    for ortho in [false, true] {
        assert_eq!(accessibility(ortho, false, Vec3::Z, 1.), 1.);
        let blocked = accessibility(ortho, true, Vec3::Z, 1.);
        assert!(blocked > 0. && blocked < 1., "{ortho}: {blocked}");
        assert_eq!(accessibility(ortho, true, -Vec3::Z, 1.), 1.);
        assert_eq!(accessibility(ortho, true, Vec3::Z, 0.), 1.);
        assert!((accessibility(ortho, true, Vec3::Z, 0.5) - (1. + blocked) * 0.5).abs() < 1e-7);
    }
}
#[test]
fn camera_projection_extraction_preserves_geometry_matrix_and_world_normal_rotation() {
    let mut c = camera(false);
    c.view = Mat4::look_at_rh(Vec3::new(20., 30., 40.), Vec3::ZERO, Vec3::Y);
    assert_eq!(
        c.matrix(1.75).unwrap(),
        c.projection(1.75).unwrap() * c.view
    );
    let world = Mat4::from_rotation_y(0.4) * Mat4::from_scale(Vec3::new(-2., 3., 0.5));
    let n = world
        .inverse()
        .transpose()
        .transform_vector3(Vec3::new(1., 2., 3.).normalize())
        .normalize();
    let instance = crate::scene::Instance::new(world).unwrap();
    let columns = instance.normal.map(|x| Vec4::from_array(x).truncate());
    let actual = glam::Mat3::from_cols(columns[0], columns[1], columns[2])
        * Vec3::new(1., 2., 3.).normalize();
    assert!((actual.normalize() - n).length() < 1e-6);
    assert!(((c.view * n.extend(0.)).truncate().length() - 1.).abs() < 1e-6);
}

#[test]
fn jittered_reconstruction_matches_actual_geometry_at_fractional_viewports() {
    let rect = [13.25, 29.75, 320.5, 180.5];
    for ortho in [false, true] {
        let mut c = camera(ortho);
        c.view = Mat4::look_at_rh(Vec3::new(2., 3., 4.), Vec3::ZERO, Vec3::Y);
        for jitter in [[0.; 2], [0.375 / rect[2], -0.75 / rect[3]]] {
            c.jitter = jitter;
            let u = Uniform::new(c, rect, [512, 256], settings()).unwrap();
            let inverse = Mat4::from_cols_array_2d(&u.inverse);
            for view in [Vec3::new(0.04, -0.02, -1.), Vec3::new(0.2, 0.3, -20.)] {
                let world = c.view.inverse().transform_point3(view);
                // Observe the existing geometry route, not the new projection helper.
                let ndc = c.matrix(rect[2] / rect[3]).unwrap().project_point3(world);
                let pixel = Vec2::new(rect[0], rect[1])
                    + (ndc.truncate() * Vec2::new(0.5, -0.5) + Vec2::splat(0.5))
                        * Vec2::new(rect[2], rect[3]);
                let actual = reconstruct(inverse, rect, pixel, ndc.z).unwrap();
                assert!(
                    (actual - view).length() < 0.0005,
                    "ortho={ortho} jitter={jitter:?} expected={view:?} actual={actual:?}"
                );
            }
        }
    }
}

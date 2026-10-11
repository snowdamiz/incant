//! Real backend allocations across new material variants and display targets.
use super::materials::Fixture;
use incant_render::{Renderer, wgpu};
use serde_json::json;

#[test]
#[ignore = "requires a native GPU; run by all desktop workflows"]
fn all_current_variants_exist_before_first_draw_and_reimport_creates_no_pipelines() {
    let instance = wgpu::Instance::default();
    let start = std::time::Instant::now();
    let renderer = pollster::block_on(Renderer::new(&instance, None)).unwrap();
    let initial = instance.generate_report().expect("native backend report");
    assert!(initial.hub.render_pipelines.num_allocated >= 26);
    assert!(initial.hub.compute_pipelines.num_allocated >= 1);
    eprintln!(
        "local renderer preparation: {:?}; adapter {}; render pipelines {}",
        start.elapsed(),
        renderer.adapter_name,
        initial.hub.render_pipelines.num_allocated
    );
    let mut fixture = Fixture::new(json!({"pbrMetallicRoughness":{"metallicFactor":0}}));
    let mut light = incant_doc::Entity::new("Warmup shadow sun");
    light
        .components
        .insert("Transform".into(), json!(incant_doc::Transform::default()));
    light.components.insert(
        "DirectionalLight".into(),
        json!({"color":[1,1,1],"intensity":2,"shadows":{"distance":40}}),
    );
    fixture
        .project
        .scenes
        .values_mut()
        .next()
        .unwrap()
        .entities
        .insert(light.id.clone(), light);
    let scope = renderer
        .device
        .push_error_scope(wgpu::ErrorFilter::Validation);
    for alpha in ["OPAQUE", "MASK", "BLEND"] {
        for double_sided in [false, true] {
            for mirrored in [false, true] {
                fixture.edit(|g| {
                    g["materials"][0]["alphaMode"] = json!(alpha);
                    g["materials"][0]["doubleSided"] = json!(double_sided);
                    g["nodes"][0]["scale"] = json!([if mirrored { -1. } else { 1. }, 1., 1.]);
                });
                for entity in fixture
                    .project
                    .scenes
                    .values_mut()
                    .next()
                    .unwrap()
                    .entities
                    .values_mut()
                {
                    if let Some(mesh) = entity.components.get_mut("MeshRenderer") {
                        mesh["cast_shadows"] = json!(alpha != "BLEND");
                    }
                }
                let scene = fixture.scene(&renderer);
                for format in [
                    wgpu::TextureFormat::Rgba8Unorm,
                    wgpu::TextureFormat::Rgba8UnormSrgb,
                    wgpu::TextureFormat::Bgra8Unorm,
                    wgpu::TextureFormat::Bgra8UnormSrgb,
                ] {
                    let target = renderer.device.create_texture(&wgpu::TextureDescriptor {
                        label: Some("Prewarmed format"),
                        size: wgpu::Extent3d {
                            width: 64,
                            height: 48,
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format,
                        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                        view_formats: &[],
                    });
                    let view = target.create_view(&Default::default());
                    let commands = renderer
                        .draw_scene(&scene, &view, format, 64, 48, None)
                        .unwrap();
                    let submission = renderer.queue.submit([commands]);
                    renderer
                        .device
                        .poll(wgpu::PollType::Wait {
                            submission_index: Some(submission),
                            timeout: Some(std::time::Duration::from_secs(30)),
                        })
                        .unwrap();
                    let current = instance.generate_report().unwrap();
                    assert_eq!(
                        current.hub.render_pipelines, initial.hub.render_pipelines,
                        "new pipeline while drawing {alpha}/{double_sided}/{mirrored}/{format:?}"
                    );
                    assert_eq!(current.hub.compute_pipelines, initial.hub.compute_pipelines);
                    assert_eq!(current.hub.shader_modules, initial.hub.shader_modules);
                }
            }
        }
    }
    assert!(pollster::block_on(scope.pop()).is_none());
}

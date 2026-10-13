//! Bounded numerical backend diagnosis. No render targets, images or presentation.
use std::{sync::mpsc, time::Duration};
use wgpu::util::DeviceExt as _;

const WORDS: u32 = 32_768;
const ROUNDS: u32 = 64;
const SHADER: &str = r#"
@group(0) @binding(0) var<storage, read_write> values: array<u32>;
@compute @workgroup_size(64) fn compute(@builtin(global_invocation_id) id: vec3u) {
    var value = id.x;
    for (var i=0u; i<64u; i++) { value = value*1664525u+1013904223u; }
    values[id.x] = value;
}
"#;

#[test]
#[ignore = "bounded native numeric diagnosis; requires exclusive GPU ownership"]
fn timestamp_write_and_resolve_paths() {
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    let features =
        wgpu::Features::TIMESTAMP_QUERY | wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS;
    assert!(adapter.features().contains(features));
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        required_features: features,
        ..Default::default()
    }))
    .unwrap();
    eprintln!(
        "TIMESTAMP_PROBE_ADAPTER {:?}; period={}",
        adapter.get_info(),
        queue.get_timestamp_period()
    );
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Timestamp integer work"),
        source: wgpu::ShaderSource::Wgsl(SHADER.into()),
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Timestamp integer work"),
        layout: None,
        module: &shader,
        entry_point: Some("compute"),
        compilation_options: Default::default(),
        cache: None,
    });
    let work = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Timestamp independently checked integer work"),
        size: u64::from(WORDS) * 4,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: work.as_entire_binding(),
        }],
    });
    for descriptor in [false, true] {
        for later_resolve in [false, true] {
            let queries = device.create_query_set(&wgpu::QuerySetDescriptor {
                label: None,
                ty: wgpu::QueryType::Timestamp,
                count: 2,
            });
            // A no-op resolve leaves MAX rather than looking like zero samples.
            let resolve = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: &[255; 16],
                usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            });
            let readback = device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: 24,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let mut encoder = device.create_command_encoder(&Default::default());
            if !descriptor {
                encoder.write_timestamp(&queries, 0);
            }
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: None,
                    timestamp_writes: descriptor.then_some(wgpu::ComputePassTimestampWrites {
                        query_set: &queries,
                        beginning_of_pass_write_index: Some(0),
                        end_of_pass_write_index: Some(1),
                    }),
                });
                pass.set_pipeline(&pipeline);
                pass.set_bind_group(0, &bindings, &[]);
                pass.dispatch_workgroups(WORDS / 64, 1, 1);
            }
            if !descriptor {
                encoder.write_timestamp(&queries, 1);
            }
            if later_resolve {
                queue.submit([encoder.finish()]);
                device
                    .poll(wgpu::PollType::Wait {
                        submission_index: None,
                        timeout: Some(Duration::from_secs(10)),
                    })
                    .unwrap();
                encoder = device.create_command_encoder(&Default::default());
            }
            encoder.resolve_query_set(&queries, 0..2, &resolve, 0);
            encoder.copy_buffer_to_buffer(&resolve, 0, &readback, 0, 16);
            encoder.copy_buffer_to_buffer(&work, 0, &readback, 16, 4);
            encoder.copy_buffer_to_buffer(&work, u64::from(WORDS - 1) * 4, &readback, 20, 4);
            queue.submit([encoder.finish()]);
            let (tx, rx) = mpsc::channel();
            readback
                .slice(..)
                .map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
            device
                .poll(wgpu::PollType::Wait {
                    submission_index: None,
                    timeout: Some(Duration::from_secs(10)),
                })
                .unwrap();
            rx.recv_timeout(Duration::from_secs(10)).unwrap().unwrap();
            let data = readback.slice(..).get_mapped_range();
            let ticks = [
                u64::from_ne_bytes(data[..8].try_into().unwrap()),
                u64::from_ne_bytes(data[8..16].try_into().unwrap()),
            ];
            let values = [
                u32::from_ne_bytes(data[16..20].try_into().unwrap()),
                u32::from_ne_bytes(data[20..24].try_into().unwrap()),
            ];
            let expected = [0, WORDS - 1].map(|mut value| {
                for _ in 0..ROUNDS {
                    value = value.wrapping_mul(1664525).wrapping_add(1013904223);
                }
                value
            });
            assert_eq!(values, expected, "independently checked compute dispatch");
            eprintln!(
                "TIMESTAMP_PROBE {}",
                serde_json::json!({"descriptor":descriptor,"later_resolve":later_resolve,"ticks":ticks,"work_words":WORDS,"work_rounds":ROUNDS,"values":values,"resolve_changed":ticks!=[u64::MAX;2],"positive_interval":ticks[1]>ticks[0]})
            );
            drop(data);
            readback.unmap();
        }
    }
}

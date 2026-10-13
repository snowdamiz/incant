use super::{
    MotionError, MotionFrame,
    draw::{Draw, MotionDraw},
    owner::{Extent, History, Pending},
    plan,
    provenance::{self, MotionChanges},
    targets::Targets,
};
use crate::{
    RenderScene, Renderer, Result, SceneInput,
    models::{Batch, animation},
};
use glam::DMat4;
use std::sync::Arc;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Previous {
    world: [[f32; 4]; 4],
    valid: f32,
    owner_word: u32,
    reserved: [u32; 2],
}
struct Rows {
    slots: Vec<usize>,
    owner_base: u32,
    packed: Vec<Previous>,
    buffer: wgpu::Buffer,
}
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Frame {
    current: [[f32; 4]; 4],
    previous: [[f32; 4]; 4],
    rect: [f32; 4],
}
const _: () = {
    assert!(size_of::<Previous>() == 80);
    assert!(std::mem::offset_of!(Previous, valid) == 64);
    assert!(std::mem::offset_of!(Previous, owner_word) == 68);
    assert!(std::mem::offset_of!(Previous, reserved) == 72);
    assert!(size_of::<Frame>() == 144);
};

pub(crate) struct MotionState {
    pub(super) history: History,
    nodes: Vec<usize>,
    worlds: Vec<DMat4>,
    rows: Vec<Rows>,
    provenance: provenance::State,
    pub(super) targets: Option<Arc<Targets>>,
    slot: usize,
    bytes: u64,
}
pub(crate) struct Prepared {
    history: Pending,
    targets: Option<Arc<Targets>>,
    slot: usize,
}
pub(crate) struct MotionView<'a> {
    pub output: [u32; 2],
    pub plan: &'a crate::resolution::FramePlan,
    pub temporal: bool,
}
pub(crate) struct TemporalInput {
    pub frame: MotionFrame,
    pub rect: [f32; 4],
    pub surfaces: Option<TemporalSurfaces>,
}
pub(crate) struct TemporalSurfaces {
    pub motion: wgpu::TextureView,
    pub previous_depth: wgpu::TextureView,
    pub slot: usize,
}
impl Prepared {
    pub fn temporal_input(&self, rect: [f32; 4]) -> TemporalInput {
        TemporalInput {
            frame: self.history.frame.clone(),
            rect,
            surfaces: self.targets.as_ref().map(|targets| TemporalSurfaces {
                motion: targets.motion.clone(),
                previous_depth: targets.depth[1 - self.slot].clone(),
                slot: self.slot,
            }),
        }
    }
    pub fn jitter_phase(&self) -> Option<u32> {
        self.history.frame.jitter_phase()
    }
}
fn capacity(batch: &Batch) -> usize {
    if batch.animated.is_some() {
        1
    } else {
        batch.placements.len()
    }
}
fn topology(live: &RenderScene, current: &RenderScene) -> std::result::Result<(), MotionError> {
    if live.batches.len() != current.batches.len() {
        return Err(MotionError::Topology);
    }
    for (a, b) in live.batches.iter().zip(&current.batches) {
        if !Arc::ptr_eq(&a.model, &b.model)
            || a.model._source.version_id() != b.model._source.version_id()
            || a.primitive != b.primitive
            || a.casts_shadows != b.casts_shadows
            || a.mirrored != b.mirrored
            || a.animated != b.animated
            || a.placements.len() != b.placements.len()
            || a.placements
                .iter()
                .zip(b.placements.iter())
                .any(|(x, y)| x.node != y.node || x.local != y.local)
            || a.vertices.as_ref().map(wgpu::Buffer::size)
                != b.vertices.as_ref().map(wgpu::Buffer::size)
        {
            return Err(MotionError::Topology);
        }
    }
    Ok(())
}
impl MotionState {
    pub fn new(
        renderer: &Renderer,
        live: &RenderScene,
        staging: &RenderScene,
        old: Option<&Self>,
    ) -> Result<Self> {
        super::admission()?;
        if renderer.motion.is_none() {
            return Err(MotionError::Disabled.into());
        }
        topology(live, staging)?;
        if live.camera.pixel_resolution.is_some() {
            return Ok(Self {
                history: History::default(),
                nodes: Vec::new(),
                worlds: Vec::new(),
                rows: Vec::new(),
                provenance: provenance::State::new(0),
                targets: None,
                slot: 1,
                bytes: size_of::<Self>() as u64,
            });
        }
        let mut slots = 0usize;
        // Both transition target headers/Arc controls and one prepared packet;
        // opaque driver allocations and allocator padding are not logical payload.
        let mut bytes = (size_of::<Self>()
            + 2 * (size_of::<Targets>() + 2 * size_of::<usize>())
            + size_of::<MotionDraw>()
            + 2 * size_of::<usize>()) as u64;
        plan::charge(
            &mut bytes,
            live.batches.len(),
            size_of::<Rows>() + size_of::<Draw>(),
        )?;
        for batch in &live.batches {
            let n = capacity(batch);
            if n == 0 {
                return Err(MotionError::Topology.into());
            }
            slots = slots
                .checked_add(n)
                .filter(|&n| n <= 2 * incant_render_data::MAX_INSTANCES)
                .ok_or(MotionError::Capacity)?;
            if (n as u64)
                .checked_mul(size_of::<Previous>() as u64)
                .is_none_or(|n| n > renderer.device.limits().max_buffer_size)
            {
                return Err(MotionError::Capacity.into());
            }
            // CPU packed row + GPU row + stable placement-to-node lookup.
            plan::charge(
                &mut bytes,
                n,
                2 * size_of::<Previous>() + size_of::<usize>(),
            )?;
        }
        // Before allocating even the temporary node list, charge its worst case
        // capacity and every previous entity transform (duplicates only reduce it).
        plan::charge(&mut bytes, slots, size_of::<usize>() + size_of::<DMat4>())?;
        plan::charge(&mut bytes, 1, size_of::<Frame>() * 4)?;
        provenance::charge(&mut bytes, live.batches.len(), slots)?;
        plan::overlap(
            old.map_or(0, |o| o.bytes),
            bytes,
            plan::MAX_MOTION_INSTANCE_BYTES,
        )?;
        let mut groups = Vec::with_capacity(live.batches.len());
        for (batch_index, batch) in live.batches.iter().enumerate() {
            let primitive = &batch.model.primitives[batch.primitive];
            if batch.model.materials.materials[primitive.material].alpha
                == incant_assets_runtime::AlphaMode::Blend
            {
                continue;
            }
            let key = if let Some(binding) = batch.animated {
                let [instance, node, geometry] = animation::motion_owner_key(binding);
                provenance::Key([1, instance, node, geometry])
            } else {
                // All allocations remain alive in the retained scenes, so exact
                // shared allocation identity is safe to use only during grouping.
                provenance::Key([
                    0,
                    batch.placements.as_ptr() as usize,
                    batch.placements.len(),
                    0,
                ])
            };
            groups.push(provenance::Group {
                key,
                batch: batch_index,
                count: capacity(batch),
            });
        }
        let (owners, highest) = provenance::assign(&mut groups, live.batches.len())?;
        let provenance = provenance::State::new(highest);
        let mut nodes = Vec::with_capacity(slots);
        for batch in &live.batches {
            if let Some(binding) = batch.animated {
                nodes.push(animation::motion_node(binding, &live.animations));
            } else {
                nodes.extend(batch.placements.iter().map(|p| p.node));
            }
        }
        nodes.sort_unstable();
        nodes.dedup();
        let worlds = vec![DMat4::IDENTITY; nodes.len()];
        let mut rows = Vec::with_capacity(live.batches.len());
        for (batch, owner) in live.batches.iter().zip(owners) {
            let count = capacity(batch);
            let mut slots = Vec::with_capacity(count);
            if let Some(binding) = batch.animated {
                slots.push(
                    nodes
                        .binary_search(&animation::motion_node(binding, &live.animations))
                        .unwrap(),
                );
            } else {
                slots.extend(
                    batch
                        .placements
                        .iter()
                        .map(|p| nodes.binary_search(&p.node).unwrap()),
                );
            }
            rows.push(Rows {
                slots,
                owner_base: owner.first,
                packed: Vec::with_capacity(count),
                buffer: renderer.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Motion previous instance rows"),
                    size: (count.max(1) * size_of::<Previous>()) as u64,
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }),
            });
        }
        Ok(Self {
            history: History::default(),
            nodes,
            worlds,
            rows,
            provenance,
            targets: None,
            slot: 1,
            bytes,
        })
    }
    #[cfg(test)]
    pub(crate) fn readback_view(&self) -> Option<wgpu::TextureView> {
        self.targets.as_ref().map(|t| t.motion.clone())
    }
    pub fn target_bytes(&self) -> u64 {
        self.targets.as_ref().map_or(0, |t| t.bytes)
    }
    pub fn frame(&self) -> Option<MotionFrame> {
        self.history.frame()
    }
    pub fn changes(&self) -> Option<MotionChanges<'_>> {
        self.history
            .frame()
            .map(|frame| self.provenance.view(frame))
    }
    pub fn invalidate(&mut self) {
        self.history.invalidate();
    }
    pub fn prepare(
        &mut self,
        renderer: &Renderer,
        live: &RenderScene,
        current: &RenderScene,
        input: &SceneInput,
        view: MotionView<'_>,
    ) -> Result<(Option<MotionDraw>, Prepared)> {
        let MotionView {
            output,
            plan,
            temporal,
        } = view;
        topology(live, current)?;
        let extent = Extent::new(output, plan.size, plan.rect, current.camera);
        let history = if temporal {
            self.history
                .prepare_temporal(extent, current.camera, plan.rect)?
        } else {
            self.history
                .prepare(extent, current.camera.matrix(plan.rect[2] / plan.rect[3])?)?
        };
        self.provenance.begin();
        if history.frame.bypassed() {
            return Ok((
                None,
                Prepared {
                    history,
                    targets: None,
                    slot: 1,
                },
            ));
        }
        // Preflight all target bytes before creating either a target or a packet.
        let target_bytes =
            plan::target_bytes(plan.size, renderer.device.limits().max_texture_dimension_2d)?;
        plan::overlap(
            self.targets.as_ref().map_or(0, |t| t.bytes),
            target_bytes,
            plan::MAX_MOTION_TARGET_BYTES,
        )?;
        let system = renderer.motion.as_ref().ok_or(MotionError::Disabled)?;
        let valid = f32::from(u8::from(history.frame.has_previous()));
        for (batch, rows) in current.batches.iter().zip(&mut self.rows) {
            rows.packed.clear();
            if let Some(binding) = batch.animated {
                let node = animation::motion_node(binding, &current.animations);
                let previous_entity = self.worlds[rows.slots[0]];
                let current_entity = input.nodes[node].world;
                let changed = valid == 0.
                    || !provenance::same_matrix(previous_entity, current_entity)
                    || !animation::motion_pose_unchanged(
                        binding,
                        &live.animations,
                        &current.animations,
                    );
                let owner_word = self.provenance.mark(rows.owner_base, changed);
                if batch.count > 0 {
                    let world = animation::motion_world(
                        binding,
                        &live.animations,
                        self.worlds[rows.slots[0]],
                    );
                    rows.packed.push(Previous {
                        world: world.to_cols_array_2d(),
                        valid,
                        owner_word,
                        reserved: [0; 2],
                    });
                }
            } else {
                StaticPrevious {
                    placements: &batch.placements,
                    slots: &rows.slots,
                    worlds: &self.worlds,
                    owner_base: rows.owner_base,
                }
                .pack(
                    input,
                    batch.mirrored,
                    valid,
                    &mut self.provenance,
                    &mut rows.packed,
                )?;
            }
            if rows.packed.len() != batch.count as usize {
                return Err(MotionError::Topology.into());
            }
        }
        let targets = Targets::prepare(&renderer.device, plan.size, self.targets.as_ref())?;
        let slot = if self
            .targets
            .as_ref()
            .is_some_and(|t| Arc::ptr_eq(t, &targets))
        {
            1 - self.slot
        } else {
            0
        };
        let uniform = Frame {
            current: history.matrix.to_cols_array_2d(),
            previous: history.previous.to_cols_array_2d(),
            rect: plan.rect,
        };
        let group = |uniform: Frame| {
            let buffer = renderer
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Motion frame"),
                    contents: bytemuck::bytes_of(&uniform),
                    usage: wgpu::BufferUsages::UNIFORM,
                });
            renderer
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Motion frame"),
                    layout: &system.globals,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: buffer.as_entire_binding(),
                    }],
                })
        };
        let frame = group(uniform);
        let overlay_frame = if temporal {
            Some(group(Frame {
                current: current
                    .camera
                    .unjittered()
                    .matrix(plan.rect[2] / plan.rect[3])?
                    .to_cols_array_2d(),
                ..uniform
            }))
        } else {
            None
        };
        let mut draws = Vec::with_capacity(current.batches.len());
        for ((batch, old), rows) in current.batches.iter().zip(&live.batches).zip(&self.rows) {
            let primitive = &batch.model.primitives[batch.primitive];
            let material = &batch.model.materials.materials[primitive.material];
            if batch.count == 0 || material.alpha == incant_assets_runtime::AlphaMode::Blend {
                continue;
            }
            draws.push(Draw {
                batch: Arc::clone(batch),
                previous_vertices: old.vertices.as_ref().unwrap_or(&primitive.vertices).clone(),
                previous_instances: rows.buffer.clone(),
                pipeline: system.pipeline(material.double_sided, batch.mirrored, false),
                depth_pipeline: system.pipeline(material.double_sided, batch.mirrored, true),
            });
        }
        Ok((
            Some(MotionDraw {
                targets: Arc::clone(&targets),
                slot,
                rect: plan.rect,
                frame,
                overlay_frame,
                overlay_depth: None,
                draws,
            }),
            Prepared {
                history,
                targets: Some(targets),
                slot,
            },
        ))
    }
    pub fn write(&self, queue: &wgpu::Queue, pending: &Prepared) {
        if pending.history.frame.bypassed() {
            return;
        }
        for rows in &self.rows {
            if !rows.packed.is_empty() {
                queue.write_buffer(&rows.buffer, 0, bytemuck::cast_slice(&rows.packed));
            }
        }
    }
    pub fn accept(&mut self, pending: Prepared, input: &SceneInput) {
        for (&node, world) in self.nodes.iter().zip(&mut self.worlds) {
            *world = input.nodes[node].world;
        }
        self.provenance.accept();
        self.history.accept(pending.history);
        self.targets = pending.targets;
        self.slot = pending.slot;
    }
}

struct StaticPrevious<'a> {
    placements: &'a [crate::scene::Placement],
    slots: &'a [usize],
    worlds: &'a [DMat4],
    owner_base: u32,
}
impl StaticPrevious<'_> {
    fn pack(
        &self,
        input: &SceneInput,
        mirrored: bool,
        valid: f32,
        provenance: &mut provenance::State,
        out: &mut Vec<Previous>,
    ) -> Result<()> {
        for (index, (placement, &slot)) in self.placements.iter().zip(self.slots).enumerate() {
            let current = input.nodes[placement.node].world * placement.local;
            let old = self.worlds[slot] * placement.local;
            let changed = valid == 0. || !provenance::same_matrix(old, current);
            let owner_word = provenance.mark(
                if self.owner_base == 0 {
                    0
                } else {
                    self.owner_base + index as u32
                },
                changed,
            );
            let current_world = current.as_mat4();
            let transform = crate::scene::Instance::new(current_world)?;
            if (transform.normal[0][3] < 0.) != mirrored {
                continue;
            }
            let previous = (self.worlds[slot] * placement.local).as_mat4();
            out.push(Previous {
                world: previous.to_cols_array_2d(),
                valid,
                owner_word,
                reserved: [0; 2],
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn previous_rows_follow_placement_identity_across_mirrored_packing_without_allocations() {
        let node = |id: &str, world| incant_render_data::SceneNode {
            id: id.into(),
            world,
            camera: None,
            light: None,
            mesh: None,
            sprite: None,
            tilemap: None,
            diagnostic: false,
        };
        let input = SceneInput {
            nodes: vec![
                node("a", DMat4::from_scale(glam::DVec3::new(-1., 1., 1.))),
                node("b", DMat4::IDENTITY),
            ],
            ..Default::default()
        };
        let placements = [
            crate::scene::Placement {
                node: 0,
                local: DMat4::from_translation(glam::DVec3::X),
            },
            crate::scene::Placement {
                node: 1,
                local: DMat4::from_translation(glam::DVec3::Y),
            },
        ];
        let worlds = [
            DMat4::from_translation(glam::DVec3::new(10., 0., 0.)),
            DMat4::from_scale(glam::DVec3::new(-2., 1., 1.)),
        ];
        let mut rows = Vec::with_capacity(2);
        let mut provenance = provenance::State::new(2);
        let allocations = crate::retained_alloc::measured(|| {
            StaticPrevious {
                placements: &placements,
                slots: &[0, 1],
                worlds: &worlds,
                owner_base: 1,
            }
            .pack(&input, false, 1., &mut provenance, &mut rows)
            .unwrap();
            assert_eq!(rows[0].owner_word, 2 | provenance::CHANGED);
            assert_eq!(rows[0].valid, 1.);
            assert_eq!(rows[0].reserved, [0; 2]);
            assert_eq!(rows.len(), 1);
            assert_eq!(
                rows[0].world,
                (worlds[1] * placements[1].local)
                    .as_mat4()
                    .to_cols_array_2d()
            );
            rows.clear();
            StaticPrevious {
                placements: &placements,
                slots: &[0, 1],
                worlds: &worlds,
                owner_base: 1,
            }
            .pack(&input, true, 1., &mut provenance, &mut rows)
            .unwrap();
            assert_eq!(rows[0].owner_word, 1 | provenance::CHANGED);
            assert_eq!(rows[0].valid, 1.);
            assert_eq!(rows[0].reserved, [0; 2]);
            assert_eq!(rows.len(), 1);
            assert_eq!(
                rows[0].world,
                (worlds[0] * placements[0].local)
                    .as_mat4()
                    .to_cols_array_2d()
            );
            rows.clear();
            provenance.begin();
            let accepted = [input.nodes[0].world, input.nodes[1].world];
            StaticPrevious {
                placements: &placements,
                slots: &[0, 1],
                worlds: &accepted,
                owner_base: 1,
            }
            .pack(&input, true, 1., &mut provenance, &mut rows)
            .unwrap();
            assert_eq!(
                rows[0].owner_word, 1,
                "same placement after handedness change"
            );
        });
        assert_eq!(allocations, 0);
    }
}

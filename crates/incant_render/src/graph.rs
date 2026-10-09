//! Bevy-scheduled GPU passes with explicit resource dependencies.
//!
//! Preparation owns all handles; this world never borrows a scene or renderer.
//! Encoding is serial on one command encoder. The caller owns queue submission.
use crate::{
    ResourceError, Result, diagnostic_draw::DiagnosticDraw, models::ModelDraw, output::OutputDraw,
};
use bevy_ecs::{prelude::*, schedule::SingleThreadedExecutor};
use std::sync::Mutex;
#[cfg(test)]
#[path = "../tests/frame_graph/mod.rs"]
mod tests;

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Pass {
    InitializeScene,
    AssignLights,
    Shadows,
    Models,
    Display,
}
#[derive(Resource)]
pub(crate) struct PreparedFrame {
    pub diagnostics: DiagnosticDraw,
    pub models: Option<ModelDraw>,
    pub output: OutputDraw,
}
#[derive(Resource)]
struct Encoder(wgpu::CommandEncoder);
struct GraphState {
    world: World,
    schedule: Schedule,
}
pub(crate) struct FrameGraph(Mutex<GraphState>);
impl Default for FrameGraph {
    fn default() -> Self {
        let mut schedule = Schedule::default();
        schedule.set_executor(SingleThreadedExecutor::new());
        schedule.configure_sets((
            Pass::InitializeScene.before(Pass::Models),
            Pass::AssignLights.before(Pass::Models),
            Pass::Shadows.before(Pass::Models),
            Pass::Models.before(Pass::Display),
        ));
        // Registration order is deliberately unrelated to dependency order.
        schedule.add_systems((
            display.in_set(Pass::Display),
            models.in_set(Pass::Models),
            shadows.in_set(Pass::Shadows),
            assign_lights.in_set(Pass::AssignLights),
            initialize_scene.in_set(Pass::InitializeScene),
        ));
        Self(Mutex::new(GraphState {
            world: World::new(),
            schedule,
        }))
    }
}
impl FrameGraph {
    pub fn encode(
        &self,
        device: &wgpu::Device,
        frame: PreparedFrame,
    ) -> Result<wgpu::CommandBuffer> {
        let mut state = self
            .0
            .lock()
            .map_err(|_| ResourceError::CacheLock("frame graph"))?;
        let GraphState { world, schedule } = &mut *state;
        world.insert_resource(frame);
        world.insert_resource(Encoder(device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor {
                label: Some("Incant frame graph"),
            },
        )));
        schedule.run(world);
        // No frame handles remain in the scheduler after encoding. Wgpu retains
        // the referenced GPU resources in the returned command buffer instead.
        world.remove_resource::<PreparedFrame>();
        let encoder = world
            .remove_resource::<Encoder>()
            .expect("graph owns encoder");
        Ok(encoder.0.finish())
    }
}
fn initialize_scene(frame: Res<PreparedFrame>, mut encoder: ResMut<Encoder>) {
    frame.diagnostics.encode(&mut encoder.0);
}
fn assign_lights(frame: Res<PreparedFrame>, mut encoder: ResMut<Encoder>) {
    if let Some(pass) = frame
        .models
        .as_ref()
        .and_then(|m| m.lighting.compute.as_ref())
    {
        pass.encode(&mut encoder.0);
    }
}
fn models(frame: Res<PreparedFrame>, mut encoder: ResMut<Encoder>) {
    if let Some(pass) = &frame.models {
        pass.encode(&mut encoder.0);
    }
}
fn display(frame: Res<PreparedFrame>, mut encoder: ResMut<Encoder>) {
    frame.output.encode(&mut encoder.0);
}

fn shadows(frame: Res<PreparedFrame>, mut encoder: ResMut<Encoder>) {
    if let Some(pass) = &frame.models {
        pass.shadows.encode(&mut encoder.0);
    }
}

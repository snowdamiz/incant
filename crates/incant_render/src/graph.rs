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
    Sprites,
    Text,
    Display,
}
#[cfg_attr(not(target_arch = "wasm32"), derive(Resource))]
pub(crate) struct PreparedFrame {
    pub diagnostics: DiagnosticDraw,
    pub models: Option<ModelDraw>,
    pub sprites: Option<crate::sprites::SpriteDraw>,
    pub text: Option<crate::text::TextDraw>,
    pub output: OutputDraw,
}
#[cfg_attr(not(target_arch = "wasm32"), derive(Resource))]
struct Encoder(wgpu::CommandEncoder, Option<crate::timing::Capture>);
impl Encoder {
    fn timed(
        &mut self,
        pass: crate::timing::GpuPass,
        encode: impl FnOnce(&mut wgpu::CommandEncoder, Option<&crate::timing::PassQueries>),
    ) {
        let queries = self.1.as_mut().map(|capture| capture.pass(pass));
        encode(&mut self.0, queries.as_ref());
    }
}
#[cfg(not(target_arch = "wasm32"))]
type FrameInput<'w> = Res<'w, PreparedFrame>;
#[cfg(target_arch = "wasm32")]
type FrameInput<'w> = NonSend<'w, PreparedFrame>;
#[cfg(not(target_arch = "wasm32"))]
type EncoderInput<'w> = ResMut<'w, Encoder>;
#[cfg(target_arch = "wasm32")]
type EncoderInput<'w> = NonSendMut<'w, Encoder>;
struct GraphState {
    world: World,
    schedule: Schedule,
    timings: Option<crate::timing::Profiler>,
}
pub(crate) struct FrameGraph(
    Mutex<GraphState>,
    // World can move between threads even when it contains NonSend data. The
    // browser graph itself must stay with the thread that owns its WebGPU handles.
    #[cfg(target_arch = "wasm32")] std::marker::PhantomData<std::rc::Rc<()>>,
);
// These compile-time checks are ambiguous if the browser graph becomes Send or
// Sync. They exercise the actual wasm type even in a library-only cross-check.
#[cfg(target_arch = "wasm32")]
const _: fn() = || {
    trait AmbiguousIfSend<A> {
        fn check() {}
    }
    struct SendMarker;
    impl<T: ?Sized> AmbiguousIfSend<()> for T {}
    impl<T: ?Sized + Send> AmbiguousIfSend<SendMarker> for T {}
    let _ = <FrameGraph as AmbiguousIfSend<_>>::check;
    trait AmbiguousIfSync<A> {
        fn check() {}
    }
    struct SyncMarker;
    impl<T: ?Sized> AmbiguousIfSync<()> for T {}
    impl<T: ?Sized + Sync> AmbiguousIfSync<SyncMarker> for T {}
    let _ = <FrameGraph as AmbiguousIfSync<_>>::check;
};
fn pass_schedule() -> Schedule {
    let mut schedule = Schedule::default();
    schedule.set_executor(SingleThreadedExecutor::new());
    schedule.configure_sets((
        Pass::InitializeScene.before(Pass::Models),
        Pass::AssignLights.before(Pass::Models),
        Pass::Shadows.before(Pass::Models),
        Pass::Models.before(Pass::Sprites),
        Pass::Sprites.before(Pass::Text),
        Pass::Text.before(Pass::Display),
    ));
    schedule
}
impl Default for FrameGraph {
    fn default() -> Self {
        let mut schedule = pass_schedule();
        // Registration order is deliberately unrelated to dependency order.
        schedule.add_systems((
            display.in_set(Pass::Display),
            models.in_set(Pass::Models),
            sprites.in_set(Pass::Sprites),
            text.in_set(Pass::Text),
            shadows.in_set(Pass::Shadows),
            assign_lights.in_set(Pass::AssignLights),
            initialize_scene.in_set(Pass::InitializeScene),
        ));
        Self(
            Mutex::new(GraphState {
                world: World::new(),
                schedule,
                timings: None,
            }),
            #[cfg(target_arch = "wasm32")]
            std::marker::PhantomData,
        )
    }
}
#[cfg(not(target_arch = "wasm32"))]
fn insert_frame(world: &mut World, frame: PreparedFrame, encoder: Encoder) {
    world.insert_resource(frame);
    world.insert_resource(encoder);
}
#[cfg(target_arch = "wasm32")]
fn insert_frame(world: &mut World, frame: PreparedFrame, encoder: Encoder) {
    world.insert_non_send(frame);
    world.insert_non_send(encoder);
}
#[cfg(not(target_arch = "wasm32"))]
fn take_encoder(world: &mut World) -> Encoder {
    world.remove_resource::<PreparedFrame>();
    world
        .remove_resource::<Encoder>()
        .expect("graph owns encoder")
}
#[cfg(target_arch = "wasm32")]
fn take_encoder(world: &mut World) -> Encoder {
    world.remove_non_send::<PreparedFrame>();
    world
        .remove_non_send::<Encoder>()
        .expect("graph owns encoder")
}
impl FrameGraph {
    pub fn with_timings(device: &wgpu::Device, queue: &wgpu::Queue) -> Result<Self> {
        let graph = Self::default();
        graph
            .0
            .lock()
            .map_err(|_| crate::timing::GpuTimingError::State)?
            .timings = Some(crate::timing::Profiler::new(device, queue)?);
        Ok(graph)
    }
    pub fn take_timing(
        &self,
    ) -> std::result::Result<Option<crate::timing::GpuFrameTiming>, crate::timing::GpuTimingError>
    {
        let state = self
            .0
            .lock()
            .map_err(|_| crate::timing::GpuTimingError::State)?;
        state.timings.as_ref().map_or(Ok(None), |p| p.take())
    }
    pub fn timing_stats(
        &self,
    ) -> std::result::Result<crate::timing::GpuTimingStats, crate::timing::GpuTimingError> {
        let state = self
            .0
            .lock()
            .map_err(|_| crate::timing::GpuTimingError::State)?;
        state
            .timings
            .as_ref()
            .map_or(Ok(Default::default()), |p| p.stats())
    }
    pub fn encode(
        &self,
        device: &wgpu::Device,
        frame: PreparedFrame,
    ) -> Result<wgpu::CommandBuffer> {
        let mut state = self
            .0
            .lock()
            .map_err(|_| ResourceError::CacheLock("frame graph"))?;
        let GraphState {
            world,
            schedule,
            timings,
        } = &mut *state;
        let encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Incant frame graph"),
        });
        let capture = timings.as_ref().map(|p| p.begin()).transpose()?.flatten();
        insert_frame(world, frame, Encoder(encoder, capture));
        schedule.run(world);
        // No frame handles remain in the scheduler after encoding. Wgpu retains
        // the referenced GPU resources in the returned command buffer instead.
        let mut encoder = take_encoder(world);
        if let Some(capture) = encoder.1.take() {
            capture.finish(&mut encoder.0);
        }
        Ok(encoder.0.finish())
    }
}
fn initialize_scene(frame: FrameInput<'_>, mut encoder: EncoderInput<'_>) {
    encoder.timed(crate::timing::GpuPass::InitializeScene, |e, q| {
        frame.diagnostics.encode(e, q)
    });
}
fn assign_lights(frame: FrameInput<'_>, mut encoder: EncoderInput<'_>) {
    if let Some(pass) = frame
        .models
        .as_ref()
        .and_then(|m| m.lighting.compute.as_ref())
    {
        encoder.timed(crate::timing::GpuPass::AssignLights, |e, q| {
            pass.encode(e, q)
        });
    }
}
fn models(frame: FrameInput<'_>, mut encoder: EncoderInput<'_>) {
    if let Some(pass) = &frame.models {
        encoder.timed(crate::timing::GpuPass::Models, |e, q| pass.encode(e, q));
    }
}
fn display(frame: FrameInput<'_>, mut encoder: EncoderInput<'_>) {
    encoder.timed(crate::timing::GpuPass::Display, |e, q| {
        frame.output.encode(e, q)
    });
}

fn shadows(frame: FrameInput<'_>, mut encoder: EncoderInput<'_>) {
    if let Some(pass) = &frame.models
        && pass.shadows.has_passes()
    {
        encoder.timed(crate::timing::GpuPass::Shadows, |e, q| {
            pass.shadows.encode(e, q)
        });
    }
}

fn sprites(frame: FrameInput<'_>, mut encoder: EncoderInput<'_>) {
    if let Some(pass) = &frame.sprites {
        encoder.timed(crate::timing::GpuPass::Sprites, |e, q| pass.encode(e, q));
    }
}

fn text(frame: FrameInput<'_>, mut encoder: EncoderInput<'_>) {
    if let Some(pass) = &frame.text
        && pass.has_passes()
    {
        encoder.timed(crate::timing::GpuPass::Text, |e, q| pass.encode(e, q));
    }
}
#[cfg(test)]
#[path = "../tests/graph_ownership/mod.rs"]
mod ownership_tests;

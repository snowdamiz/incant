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
struct Encoder(wgpu::CommandEncoder);
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
        insert_frame(
            world,
            frame,
            Encoder(
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Incant frame graph"),
                }),
            ),
        );
        schedule.run(world);
        // No frame handles remain in the scheduler after encoding. Wgpu retains
        // the referenced GPU resources in the returned command buffer instead.
        let encoder = take_encoder(world);
        Ok(encoder.0.finish())
    }
}
fn initialize_scene(frame: FrameInput<'_>, mut encoder: EncoderInput<'_>) {
    frame.diagnostics.encode(&mut encoder.0);
}
fn assign_lights(frame: FrameInput<'_>, mut encoder: EncoderInput<'_>) {
    if let Some(pass) = frame
        .models
        .as_ref()
        .and_then(|m| m.lighting.compute.as_ref())
    {
        pass.encode(&mut encoder.0);
    }
}
fn models(frame: FrameInput<'_>, mut encoder: EncoderInput<'_>) {
    if let Some(pass) = &frame.models {
        pass.encode(&mut encoder.0);
    }
}
fn display(frame: FrameInput<'_>, mut encoder: EncoderInput<'_>) {
    frame.output.encode(&mut encoder.0);
}

fn shadows(frame: FrameInput<'_>, mut encoder: EncoderInput<'_>) {
    if let Some(pass) = &frame.models {
        pass.shadows.encode(&mut encoder.0);
    }
}

fn sprites(frame: FrameInput<'_>, mut encoder: EncoderInput<'_>) {
    if let Some(pass) = &frame.sprites {
        pass.encode(&mut encoder.0);
    }
}

fn text(frame: FrameInput<'_>, mut encoder: EncoderInput<'_>) {
    if let Some(pass) = &frame.text {
        pass.encode(&mut encoder.0);
    }
}
#[cfg(test)]
#[path = "../tests/graph_ownership/mod.rs"]
mod ownership_tests;

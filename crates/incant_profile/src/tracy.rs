//! Process-owned transport only. Defaults disable broadcast, sampling, source/code
//! transfer, crash handlers and discovery. The pinned client has no safe shutdown;
//! never call its unsafe shutdown or start it for an embedded summary session.
use crate::{Allocations, HeapSample, Phase};
use tracy_client::{Client, Span, plot_name, span_location};
pub fn start() {
    let _ = Client::start();
}
pub fn begin(phase: Phase, source: Option<usize>) -> Option<Span> {
    let client = Client::running()?;
    let location = match phase {
        Phase::Frame => span_location!("player.frame"),
        Phase::NativeTick => span_location!("native.tick"),
        Phase::WorldStep => span_location!("world.step"),
        Phase::RootMotion => span_location!("world.root_motion"),
        Phase::Transforms => span_location!("world.transforms"),
        Phase::Navigation => span_location!("world.navigation"),
        Phase::PhysicsSync => span_location!("physics.sync"),
        Phase::Physics2d => span_location!("physics.2d"),
        Phase::Physics3d => span_location!("physics.3d"),
        Phase::EcsSchedule => span_location!("ecs.schedule"),
        Phase::Integrate => span_location!("ecs.integrate"),
        Phase::Globals => span_location!("ecs.globals"),
        Phase::Flipbook => span_location!("ecs.flipbook"),
        Phase::Animation => span_location!("ecs.animation"),
        Phase::Input => span_location!("native.input"),
        Phase::ScriptTick => span_location!("script.tick"),
        Phase::ScriptSource => span_location!("script.source"),
        Phase::ScriptApply => span_location!("script.apply"),
        Phase::StaticModule => span_location!("script.static_module"),
        Phase::DevelopmentModule => span_location!("script.development_module"),
        Phase::UiStage => span_location!("game.ui_stage"),
        Phase::AudioSync => span_location!("game.audio_sync"),
        Phase::AudioRender => span_location!("game.audio_render"),
        Phase::HostSync => span_location!("game.host_sync"),
        Phase::Restart => span_location!("player.restart"),
        Phase::Presentation => span_location!("player.presentation"),
    };
    let span = client.span(location, 0);
    if let Some(source) = source {
        span.emit_value(source as u64);
    }
    Some(span)
}
pub fn heap(value: HeapSample) {
    if let Some(c) = Client::running() {
        if let Some(used) = value.estimated_used_bytes {
            c.plot(
                plot_name!("QuickJS sampled estimated used bytes"),
                used as f64,
            );
        }
        c.plot(
            plot_name!("QuickJS sampled logical allocated bytes"),
            value.allocated_bytes as f64,
        );
    }
}
pub fn frame(value: Allocations) {
    if let Some(c) = Client::running() {
        c.plot(
            plot_name!("Rust requested live bytes"),
            value.live_requested_bytes as f64,
        );
        c.plot(
            plot_name!("Rust allocation calls"),
            value.allocations as f64,
        );
        c.frame_mark();
    }
}

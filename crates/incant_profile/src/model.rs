/// Compile-time phase inventory. No caller can create arbitrary trace labels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum Phase {
    Frame,
    NativeTick,
    WorldStep,
    RootMotion,
    Transforms,
    Navigation,
    PhysicsSync,
    Physics2d,
    Physics3d,
    EcsSchedule,
    Integrate,
    Globals,
    Flipbook,
    Animation,
    Input,
    ScriptTick,
    ScriptSource,
    ScriptApply,
    StaticModule,
    DevelopmentModule,
    UiStage,
    AudioSync,
    AudioRender,
    HostSync,
    Restart,
    Presentation,
}
pub const PHASE_COUNT: usize = 26;
pub const MAX_SOURCES: usize = 64;
impl Phase {
    pub const ALL: [Self; PHASE_COUNT] = [
        Self::Frame,
        Self::NativeTick,
        Self::WorldStep,
        Self::RootMotion,
        Self::Transforms,
        Self::Navigation,
        Self::PhysicsSync,
        Self::Physics2d,
        Self::Physics3d,
        Self::EcsSchedule,
        Self::Integrate,
        Self::Globals,
        Self::Flipbook,
        Self::Animation,
        Self::Input,
        Self::ScriptTick,
        Self::ScriptSource,
        Self::ScriptApply,
        Self::StaticModule,
        Self::DevelopmentModule,
        Self::UiStage,
        Self::AudioSync,
        Self::AudioRender,
        Self::HostSync,
        Self::Restart,
        Self::Presentation,
    ];
    pub const fn name(self) -> &'static str {
        match self {
            Self::Frame => "player.frame",
            Self::NativeTick => "native.tick",
            Self::WorldStep => "world.step",
            Self::RootMotion => "world.root_motion",
            Self::Transforms => "world.transforms",
            Self::Navigation => "world.navigation",
            Self::PhysicsSync => "physics.sync",
            Self::Physics2d => "physics.2d",
            Self::Physics3d => "physics.3d",
            Self::EcsSchedule => "ecs.schedule",
            Self::Integrate => "ecs.integrate",
            Self::Globals => "ecs.globals",
            Self::Flipbook => "ecs.flipbook",
            Self::Animation => "ecs.animation",
            Self::Input => "native.input",
            Self::ScriptTick => "script.tick",
            Self::ScriptSource => "script.source",
            Self::ScriptApply => "script.apply",
            Self::StaticModule => "script.static_module",
            Self::DevelopmentModule => "script.development_module",
            Self::UiStage => "game.ui_stage",
            Self::AudioSync => "game.audio_sync",
            Self::AudioRender => "game.audio_render",
            Self::HostSync => "game.host_sync",
            Self::Restart => "player.restart",
            Self::Presentation => "player.presentation",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Disabled,
    Storage,
    Busy,
    Sequence,
    ClockUnavailable,
    TracyUnavailable,
    TracyProcessOnly,
    UnsupportedRoute,
    Options,
    SessionBusy,
    FrameRange,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::SessionBusy => "an owner-thread profiling tick is already active",
            Self::FrameRange => "profile read requires 1..=256 available retained frames",
            Self::UnsupportedRoute => "profiling is unavailable for the requested player route",
            Self::Options => "duplicate profiling option",
            Self::Storage => "profile histogram storage unavailable",
            Self::Disabled => "CPU profiling requires the profile build feature",
            Self::Busy => "a process profile capture is already active",
            Self::Sequence => "profile capture sequence exhausted",
            Self::ClockUnavailable => "native profile clock unavailable on this target",
            Self::TracyUnavailable => "Tracy capture requires the profile-tracy build feature",
            Self::TracyProcessOnly => {
                "Tracy transport is supported only by the standalone CLI process"
            }
        })
    }
}
impl std::error::Error for Error {}
#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "summary", derive(serde::Serialize))]
pub struct Totals {
    pub calls: u64,
    pub wall_ns: u64,
    pub thread_cpu_ns: u64,
    pub max_wall_ns: u64,
    pub unavailable_cpu_samples: u64,
    pub unwound: u64,
}
#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "summary", derive(serde::Serialize))]
pub struct Allocations {
    pub installed: bool,
    pub allocations: u64,
    pub reallocations: u64,
    pub frees: u64,
    pub failures: u64,
    pub allocated_requested_bytes: u64,
    pub freed_requested_bytes: u64,
    pub live_requested_bytes: u64,
    pub process_peak_requested_bytes: u64,
    pub saturated: bool,
}
#[derive(Debug)]
#[cfg_attr(feature = "summary", derive(serde::Serialize))]
pub struct PhaseReport {
    pub name: &'static str,
    pub totals: Totals,
}
#[derive(Debug)]
#[cfg_attr(feature = "summary", derive(serde::Serialize))]
pub struct SourceReport {
    pub slot: usize,
    pub source: Option<[u8; 16]>,
    pub totals: Totals,
}
#[derive(Debug)]
#[cfg_attr(feature = "summary", derive(serde::Serialize))]
pub struct Report {
    pub version: u32,
    #[cfg(feature = "diagnostic-script-phases")]
    pub script_phase_diagnostic: crate::diagnostic::Report,
    pub frame_wall: crate::FrameWallReport,
    pub scope: &'static str,
    pub generation: u64,
    pub zones: Vec<PhaseReport>,
    pub scripts: Vec<SourceReport>,
    pub dropped_source_samples: u64,
    pub completed_frames: u64,
    pub failed_frames: u64,
    pub quickjs_heap: crate::QuickJsHeap,
    pub rust_allocations: Allocations,
    pub tracy_process_transport: bool,
}

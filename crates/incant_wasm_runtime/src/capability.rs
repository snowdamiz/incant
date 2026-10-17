use crate::{
    BoundaryError as Error, limits,
    transaction::{self, Chunk},
};
use incant_runtime::{
    AnimationSnapshot, CookedEntity, FlipbookControl, FlipbookSnapshot, FrameCommands, NativeWorld,
    StableId, StructuralCommand, WorldIdentity,
};
use incant_types::{Transform, Velocity};
#[path = "persistent_capability.rs"]
mod persistent;

// The native component representation and generated wire record must agree on
// every numeric field. These compile on each target, not only the test host.
const _: () = {
    use incant_wasm_abi as abi;
    assert!(incant_runtime::TRANSFORM_STRIDE == abi::TRANSFORM_STRIDE);
    assert!(incant_runtime::VELOCITY_STRIDE == abi::VELOCITY_STRIDE);
    assert!(std::mem::size_of::<Transform>() == abi::TransformRow::SIZE);
    assert!(std::mem::offset_of!(Transform, translation) == abi::TransformRow::TRANSLATION_OFFSET);
    assert!(std::mem::offset_of!(Transform, rotation) == abi::TransformRow::ROTATION_OFFSET);
    assert!(std::mem::offset_of!(Transform, scale) == abi::TransformRow::SCALE_OFFSET);
    assert!(std::mem::size_of::<Velocity>() == abi::VelocityRow::SIZE);
    assert!(std::mem::offset_of!(Velocity, linear) == abi::VelocityRow::LINEAR_OFFSET);
};

/// Trusted host adapter operation; all integer arguments are guest offsets/indices.
#[derive(Clone, Copy)]
pub enum Call {
    TimerLen,
    ReadTimers {
        ptr: i32,
        capacity: i32,
    },
    CommandTimer {
        ptr: i32,
        len: i32,
    },
    HapticLen,
    ReadHaptics {
        ptr: i32,
        capacity: i32,
    },
    CommandHaptics {
        ptr: i32,
        len: i32,
    },
    ReadPersistent {
        ptr: i32,
        offset: i32,
        len: i32,
    },
    WritePersistent {
        ptr: i32,
        offset: i32,
        len: i32,
    },
    RequestRestart,
    CommandLight {
        ptr: i32,
        len: i32,
    },
    AudioLen,
    ReadAudio {
        ptr: i32,
        capacity: i32,
    },
    CommandAudio {
        ptr: i32,
        len: i32,
    },
    HudLen,
    ReadHud {
        ptr: i32,
        capacity: i32,
    },
    ControlHud {
        ptr: i32,
        len: i32,
    },
    RootMotionLen,
    ReadRootMotion {
        ptr: i32,
        capacity: i32,
    },
    ControlRootMotion {
        ptr: i32,
        len: i32,
    },
    AnimationLenV5,
    ReadAnimationsV5 {
        ptr: i32,
        capacity: i32,
    },
    AnimationIkLenV2,
    ReadAnimationIkV2 {
        ptr: i32,
        capacity: i32,
    },
    AnimationGraphLenV2,
    ReadAnimationGraphsV2 {
        ptr: i32,
        capacity: i32,
    },
    PhysicsQuery {
        ptr: i32,
        len: i32,
    },
    PhysicsEvents {
        dimension: i32,
    },
    ReadPhysics {
        ptr: i32,
        capacity: i32,
    },
    FindPath {
        ptr: i32,
        len: i32,
    },
    FindGridPath {
        ptr: i32,
        len: i32,
    },
    ReadPath {
        ptr: i32,
        capacity: i32,
    },
    AnimationIkLen,
    ReadAnimationIk {
        ptr: i32,
        capacity: i32,
    },
    ControlAnimationIk {
        ptr: i32,
        len: i32,
    },
    AnimationLenV4,
    ReadAnimationsV4 {
        ptr: i32,
        capacity: i32,
    },
    AnimationLenV3,
    ReadAnimationsV3 {
        ptr: i32,
        capacity: i32,
    },
    AnimationGraphLen,
    ReadAnimationGraphs {
        ptr: i32,
        capacity: i32,
    },
    ControlAnimationGraph {
        id: i32,
        ptr: i32,
    },
    Steer {
        ptr: i32,
        len: i32,
    },
    ReadSteering {
        ptr: i32,
        capacity: i32,
    },
    AnimationLenV2,
    ReadAnimationsV2 {
        ptr: i32,
        capacity: i32,
    },
    ControlAnimationV2 {
        id: i32,
        ptr: i32,
    },
    AnimationLen,
    ReadAnimations {
        ptr: i32,
        capacity: i32,
    },
    ControlAnimation {
        id: i32,
        ptr: i32,
    },
    Locale,
    Format {
        ptr: i32,
        len: i32,
    },
    Localize {
        ptr: i32,
        len: i32,
    },
    ReadLocalized {
        ptr: i32,
        capacity: i32,
    },
    SetLocale {
        ptr: i32,
        len: i32,
    },
    Log {
        level: i32,
        ptr: i32,
        len: i32,
    },
    FlipbookLen,
    ReadFlipbooks {
        ptr: i32,
        capacity: i32,
    },
    ControlFlipbook {
        id: i32,
        kind: i32,
        frame: i32,
    },
    InputLen,
    ReadInput {
        ptr: i32,
        capacity: i32,
    },
    ChunkCount,
    ChunkLen(i32),
    ChunkFlags(i32),
    ReadIds(i32, i32),
    Column {
        chunk: i32,
        ptr: i32,
        velocity: bool,
        write: bool,
    },
    Spawn {
        id: i32,
        parent: i32,
        transform: i32,
        velocity: i32,
    },
    Despawn(i32),
    SetVelocity(i32, i32),
    SetParent(i32, i32),
}
/// Adapter state for one module. Only copied values enter guest linear memory.
/// Hosts must keep exclusive world ownership from begin through finish/abort.
/// Failed acceptance is terminal, including unwinds; reload explicitly replaces
/// this state. No guest access or native pointer is retained by this type.
#[derive(Default)]
pub struct Capability {
    timers: crate::timers::State,
    haptics: crate::haptics::State,
    persistent: Option<crate::persistent::PersistentState>,
    persistent_rejected: bool,
    restart_enabled: bool,
    restart_calls: usize,
    restart_rejected: bool,
    restart_committed: bool,
    audio: Vec<u8>,
    audio_rejected: bool,
    light_rejected: bool,
    identity: Option<WorldIdentity>,
    failed: bool,
    active: bool,
    epoch: i64,
    chunks: Vec<Chunk>,
    commands: FrameCommands,
    input: Vec<u8>,
    input_available: bool,
    hud: Vec<u8>,
    hud_rejected: bool,
    flipbook_rows: Vec<FlipbookSnapshot>,
    flipbooks: Vec<u8>,
    animation_rows: Vec<AnimationSnapshot>,
    animations: Vec<u8>,
    animations_v2: Vec<u8>,
    animations_v3: Vec<u8>,
    animation_scene: Option<StableId>,
    animations_v4: Vec<u8>,
    animations_v5: Vec<u8>,
    animation_ik_v2: Vec<u8>,
    animation_graphs_v2: Vec<u8>,
    animation_graph_rows: Vec<incant_runtime::AnimationGraphSnapshot>,
    animation_graphs: Vec<u8>,
    animation_rejected: bool,
    animation_ik_present: bool,
    root_motion_rows: Vec<incant_runtime::RootMotionSnapshot>,
    root_motion: Vec<u8>,
    ik_rows: Vec<incant_runtime::IkSnapshot>,
    animation_ik: Vec<u8>,
    transferred: usize,
    calls: usize,
    tick: u64,
    logs: crate::NativeLogs,
    tick_logs: crate::logging::TickLogs,
    localization: Option<crate::localization::State>,
    steering: Option<crate::steering::State>,
    query_units: usize,
    navigation_queries: usize,
    navigation: Option<crate::navigation_paths::State>,
    physics: Option<crate::physics_queries::State>,
}
impl Capability {
    pub fn is_failed(&self) -> bool {
        self.failed
    }
    /// Successful trusted reload keeps committed history and its accepted clock.
    pub fn replacement(&mut self) -> Self {
        // Direct trusted callers must not revive a failed or active persistent
        // transaction. Public host reload validates its fresh candidate first.
        let failed =
            self.persistent.is_some() && (self.failed || self.active || self.restart_committed);
        Self {
            failed,
            persistent: self.persistent.take(),
            timers: self.timers.replacement(),
            haptics: self.haptics.replacement(),
            epoch: self.epoch,
            restart_enabled: self.restart_enabled,
            identity: self.identity.clone(),
            tick: self.tick,
            logs: std::mem::take(&mut self.logs),
            ..Self::default()
        }
    }
    /// Trusted owner opt-in; the owner must implement whole-session replacement.
    pub fn enable_restart_requests(&mut self) -> Result<(), Error> {
        if self.failed || self.active {
            return Err(Error::Failed);
        }
        self.restart_enabled = true;
        Ok(())
    }
    /// Consume only after the complete composed tick (including audio and its
    /// mandatory consumer) succeeds. Requests never interrupt a guest callback.
    pub fn take_restart_request(&mut self) -> Result<bool, Error> {
        if self.failed || self.active {
            return Err(Error::Failed);
        }
        Ok(std::mem::take(&mut self.restart_committed))
    }
    /// Trusted current device authority, never derived from input or save data.
    pub fn set_haptic_capabilities(
        &mut self,
        value: incant_input::haptics::HapticCapabilities,
    ) -> Result<(), Error> {
        self.haptics_ready()?;
        self.haptics.set(value)
    }
    pub fn haptics_ready(&self) -> Result<(), Error> {
        if self.failed || self.active {
            return Err(Error::Failed);
        }
        self.haptics.ready()
    }
    /// Defer drain until the complete composed host/audio transaction accepts.
    pub fn take_haptics(&mut self) -> Result<incant_input::haptics::HapticRequests, Error> {
        if self.failed || self.active {
            return Err(Error::Failed);
        }
        Ok(self.haptics.take())
    }
    pub fn clear_haptics(&mut self) {
        self.haptics.clear();
    }
    pub fn logs(&self) -> Result<&crate::NativeLogs, Error> {
        if self.failed {
            Err(Error::Failed)
        } else {
            Ok(&self.logs)
        }
    }
    pub fn begin(&mut self, world: &mut NativeWorld, dt: f64) -> Result<i64, Error> {
        self.begin_hud(world, dt, &incant_input::UiSnapshot::default())
    }
    fn begin_hud(
        &mut self,
        world: &mut NativeWorld,
        dt: f64,
        ui: &incant_input::UiSnapshot,
    ) -> Result<i64, Error> {
        self.haptics_ready()?;
        if !dt.is_finite() || dt <= 0. || dt > 1. {
            return Err(Error::Abi("invalid dt"));
        }
        let identity = world.spatial_view()?.identity().clone();
        if self.identity.as_ref().is_some_and(|old| old != &identity) {
            return Err(Error::Abi("module is bound to a different world"));
        }
        self.identity = Some(identity);
        self.tick
            .checked_add(1)
            .filter(|&tick| tick <= incant_runtime::logging::MAX_LOG_TICK)
            .ok_or(Error::Limit("log clock"))?;
        self.failed = true;
        self.input_available = false;
        self.input.clear();
        self.epoch = self.epoch.checked_add(1).ok_or(Error::Limit("epoch"))?;
        self.chunks = world.with_numeric_columns(transaction::capture)?;
        if let Err(error) =
            crate::flipbooks::encode(world, &mut self.flipbook_rows, &mut self.flipbooks)
        {
            self.abort();
            return Err(error);
        }
        self.commands.clear();
        self.haptics.begin();
        if let Err(error) = self.timers.begin(self.tick + 1, dt) {
            self.abort();
            return Err(error);
        }
        self.restart_calls = 0;
        self.restart_rejected = false;
        self.persistent_rejected = false;
        if let Some(state) = &mut self.persistent {
            state.begin();
        }
        self.audio_rejected = false;
        self.light_rejected = false;
        if let Err(error) = crate::audio::encode(world, &mut self.audio) {
            self.abort();
            return Err(error);
        }
        self.hud_rejected = false;
        if let Err(error) = crate::hud::encode(world, ui, &mut self.hud) {
            self.abort();
            return Err(error);
        }
        if let Err(error) = crate::animations::encode(
            world,
            &mut self.animation_rows,
            &mut self.animations,
            &mut self.animations_v2,
        ) {
            self.abort();
            return Err(error);
        }
        self.animation_rejected = false;
        if let Err(error) = world.snapshot_ik(&mut self.ik_rows) {
            self.abort();
            return Err(error.into());
        }
        if let Err(error) = world.snapshot_root_motion(&mut self.root_motion_rows) {
            self.abort();
            return Err(error.into());
        }
        self.root_motion.clear();
        self.animation_ik_present = !self.ik_rows.is_empty();
        self.animation_ik.clear();
        self.animation_scene = Some(world.scene_id());
        self.animations_v3.clear();
        self.animations_v4.clear();
        self.animations_v5.clear();
        self.animation_ik_v2.clear();
        self.animation_graphs_v2.clear();
        self.animation_graphs.clear();
        if let Err(error) = world.snapshot_animation_graphs(&mut self.animation_graph_rows) {
            self.abort();
            return Err(error.into());
        }
        self.animation_graph_rows.sort_unstable_by_key(|r| r.entity);
        self.transferred = 0;
        self.calls = 0;
        self.tick_logs.clear();
        self.localization = Some(crate::localization::State::new(
            world.localization().clone(),
        ));
        self.steering = Some(crate::steering::State::new(dt));
        self.query_units = 0;
        self.navigation_queries = 0;
        self.physics = Some(crate::physics_queries::State::new(dt));
        match world.navigation_snapshot() {
            Ok(snapshot) => self.navigation = Some(crate::navigation_paths::State::new(snapshot)),
            Err(error) => {
                self.abort();
                return Err(error.into());
            }
        }
        self.active = true;
        Ok(self.epoch)
    }
    /// Optional v1 input capability. Only an engine-validated runtime is accepted.
    pub fn begin_with_input(
        &mut self,
        world: &mut NativeWorld,
        dt: f64,
        input: &incant_input::InputRuntime,
    ) -> Result<i64, Error> {
        let epoch = self.begin_hud(world, dt, &input.frame().ui)?;
        if let Err(error) = crate::input::encode(input, world.tick(), &mut self.input) {
            self.abort();
            return Err(error);
        }
        self.input_available = true;
        Ok(epoch)
    }
    /// Bind the exact owning world's synchronous query loan once per callback.
    /// A stale token cannot poison the current callback. Active misuse is sticky.
    pub fn bind_physics(
        &mut self,
        epoch: i64,
        queries: incant_runtime::NativeQueries,
    ) -> Result<(), Error> {
        if !self.active || self.epoch != epoch {
            return Err(Error::Abi("expired physics epoch"));
        }
        let result = if Some(queries.identity()) != self.identity.as_ref() {
            Err(Error::Abi("different physics world"))
        } else {
            self.physics
                .as_mut()
                .ok_or(Error::Abi("missing physics state"))?
                .bind(queries)
        };
        if result.is_err()
            && let Some(state) = &mut self.physics
        {
            state.reject();
        }
        result
    }
    /// Must run after the guest returns and before native commit. The owning
    /// native loan independently revokes retained query capabilities on unwind.
    pub fn revoke_physics(&mut self) {
        if let Some(state) = &mut self.physics {
            state.revoke();
        }
    }
    fn physics_state(&mut self) -> Result<&mut crate::physics_queries::State, Error> {
        self.physics
            .as_mut()
            .ok_or(Error::Abi("missing physics state"))
    }
    pub fn abort(&mut self) {
        self.timers.abort();
        self.haptics.abort();
        if let Some(state) = &mut self.persistent {
            state.abort();
        }
        self.restart_calls = 0;
        self.restart_committed = false;
        self.audio.clear();
        self.hud.clear();
        self.root_motion_rows.clear();
        self.root_motion.clear();
        self.animation_rows.clear();
        self.ik_rows.clear();
        self.animation_ik.clear();
        self.animations.clear();
        self.animations_v2.clear();
        self.animations_v3.clear();
        self.animations_v4.clear();
        self.animations_v5.clear();
        self.animation_ik_v2.clear();
        self.animation_graphs_v2.clear();
        self.animation_graph_rows.clear();
        self.animation_graphs.clear();
        self.active = false;
        self.input_available = false;
        self.input.clear();
        self.flipbook_rows.clear();
        self.flipbooks.clear();
        self.chunks.clear();
        self.commands.clear();
        self.tick_logs.clear();
        self.localization = None;
        self.steering = None;
        self.navigation = None;
        self.physics = None;
    }
    pub fn finish(&mut self, world: &mut NativeWorld) -> Result<(), Error> {
        if !self.active {
            return Err(Error::Abi("inactive ECS transaction"));
        }
        if self.timers.rejected {
            self.abort();
            return Err(Error::Abi("rejected timer transaction"));
        }
        if self.haptics.rejected {
            self.abort();
            return Err(Error::Abi("rejected haptic transaction"));
        }
        if self.restart_rejected {
            self.abort();
            return Err(Error::Abi("rejected session request transaction"));
        }
        if self.persistent_rejected {
            self.abort();
            return Err(Error::Abi("rejected persistent state transaction"));
        }
        if self
            .physics
            .as_ref()
            .is_some_and(|s| s.rejected || s.bound())
        {
            self.abort();
            return Err(Error::Abi("rejected or unrevoked physics loan"));
        }
        if self.steering.as_ref().is_some_and(|s| s.rejected)
            || self.navigation.as_ref().is_some_and(|s| s.rejected)
        {
            self.abort();
            return Err(Error::Abi("rejected navigation transaction"));
        }
        if self.hud_rejected {
            self.abort();
            return Err(Error::Abi("rejected HUD transaction"));
        }
        self.hud.clear();
        if self.light_rejected {
            self.abort();
            return Err(Error::Abi("rejected light transaction"));
        }
        if self.audio_rejected {
            self.abort();
            return Err(Error::Abi("rejected audio transaction"));
        }
        self.audio.clear();
        if self.animation_rejected {
            self.abort();
            return Err(Error::Abi("rejected animation transaction"));
        }
        self.root_motion_rows.clear();
        self.root_motion.clear();
        self.animation_rows.clear();
        self.ik_rows.clear();
        self.animation_ik.clear();
        self.animations.clear();
        self.animations_v2.clear();
        self.animations_v3.clear();
        self.animations_v4.clear();
        self.animations_v5.clear();
        self.animation_ik_v2.clear();
        self.animation_graphs_v2.clear();
        self.animation_graph_rows.clear();
        self.animation_graphs.clear();
        if self.tick_logs.rejected {
            self.abort();
            return Err(Error::Abi("rejected log transaction"));
        }
        if self.localization.as_ref().is_some_and(|s| s.rejected) {
            self.abort();
            return Err(Error::Abi("rejected localization transaction"));
        }
        self.localization = None;
        self.steering = None;
        self.navigation = None;
        self.physics = None;
        self.active = false;
        self.input_available = false;
        self.input.clear();
        self.flipbook_rows.clear();
        self.flipbooks.clear();
        if Some(world.spatial_view()?.identity()) != self.identity.as_ref() {
            self.abort();
            return Err(Error::Abi("different commit world"));
        }
        let result =
            world.with_numeric_columns(|columns| transaction::commit(columns, &self.chunks));
        self.chunks.clear();
        let mut commands = std::mem::take(&mut self.commands);
        let result = result.and_then(|undo| {
            if let Err(error) = world.apply(&mut commands) {
                world.with_numeric_columns(|columns| undo.restore(columns));
                return Err(error.into());
            }
            Ok(())
        });
        if result.is_ok() {
            self.haptics.commit();
            self.timers.commit();
            if let Some(state) = &mut self.persistent {
                state.commit();
            }
            self.restart_committed |= self.restart_calls != 0;
            self.tick += 1; // checked before entering guest code or publishing writes
            self.logs.commit(self.tick, self.tick_logs.take());
            self.failed = false;
        } else {
            self.timers.abort();
            self.haptics.abort();
            if let Some(state) = &mut self.persistent {
                state.abort();
            }
            self.tick_logs.clear();
        }
        result
    }
    fn enter(&mut self, epoch: i64) -> Result<(), Error> {
        if !self.active || self.epoch != epoch {
            return Err(Error::Abi("inactive or expired ECS epoch"));
        }
        self.calls += 1;
        if self.calls > limits::CALLS {
            return Err(Error::Limit("host call quota"));
        }
        Ok(())
    }
    fn transfer(&mut self, count: usize) -> Result<(), Error> {
        self.transferred = self
            .transferred
            .checked_add(count)
            .filter(|n| *n <= limits::TRANSFERS)
            .ok_or(Error::Limit("transfer quota"))?;
        Ok(())
    }
    fn command(&mut self, command: StructuralCommand) -> Result<(), Error> {
        if self.commands.len() + self.restart_calls + self.haptics.len() >= limits::COMMANDS {
            return Err(Error::Limit("structural command quota"));
        }
        self.commands.push(command)?;
        Ok(())
    }
    fn index(&self, index: i32) -> Result<usize, Error> {
        usize::try_from(index)
            .ok()
            .filter(|i| *i < self.chunks.len())
            .ok_or(Error::Abi("invalid numeric chunk"))
    }
    fn read<const N: usize>(&mut self, memory: &[u8], ptr: i32) -> Result<[u8; N], Error> {
        self.transfer(N)?;
        Ok(memory[range(ptr, N, memory.len())?].try_into().unwrap())
    }
    fn stable(&mut self, memory: &[u8], ptr: i32) -> Result<StableId, Error> {
        Ok(StableId(self.read(memory, ptr)?))
    }
    fn optional_id(&mut self, memory: &[u8], ptr: i32) -> Result<Option<StableId>, Error> {
        if ptr == -1 {
            Ok(None)
        } else {
            self.stable(memory, ptr).map(Some)
        }
    }
    fn velocity(&mut self, memory: &[u8], ptr: i32) -> Result<Option<Velocity>, Error> {
        if ptr == -1 {
            return Ok(None);
        }
        Ok(Some(Velocity {
            linear: values(&self.read::<24>(memory, ptr)?),
        }))
    }
    /// Synchronous import; memory contains initialized guest bytes only and is
    /// borrowed for this call. No world observation/publication occurs here.
    pub fn invoke(&mut self, memory: &mut [u8], epoch: i64, call: Call) -> Result<i32, Error> {
        let timer_authorized = self.active
            && self.epoch == epoch
            && matches!(
                call,
                Call::TimerLen | Call::ReadTimers { .. } | Call::CommandTimer { .. }
            );
        let haptic_authorized = self.active
            && self.epoch == epoch
            && matches!(
                call,
                Call::HapticLen | Call::ReadHaptics { .. } | Call::CommandHaptics { .. }
            );
        let persistent_authorized = self.active
            && self.epoch == epoch
            && matches!(
                call,
                Call::ReadPersistent { .. } | Call::WritePersistent { .. }
            );
        let restart_authorized =
            self.active && self.epoch == epoch && matches!(call, Call::RequestRestart);
        let hud_authorized = self.active
            && self.epoch == epoch
            && matches!(
                call,
                Call::HudLen | Call::ReadHud { .. } | Call::ControlHud { .. }
            );
        let light_authorized =
            self.active && self.epoch == epoch && matches!(call, Call::CommandLight { .. });
        let audio_authorized = self.active
            && self.epoch == epoch
            && matches!(
                call,
                Call::AudioLen | Call::ReadAudio { .. } | Call::CommandAudio { .. }
            );
        let physics_authorized = self.active
            && self.epoch == epoch
            && matches!(
                call,
                Call::PhysicsQuery { .. } | Call::PhysicsEvents { .. } | Call::ReadPhysics { .. }
            );
        let steering_authorized = self.active
            && self.epoch == epoch
            && matches!(
                call,
                Call::Steer { .. }
                    | Call::ReadSteering { .. }
                    | Call::FindPath { .. }
                    | Call::FindGridPath { .. }
                    | Call::ReadPath { .. }
            );
        let animation_authorized = self.active
            && self.epoch == epoch
            && matches!(
                call,
                Call::RootMotionLen
                    | Call::ReadRootMotion { .. }
                    | Call::ControlRootMotion { .. }
                    | Call::AnimationLen
                    | Call::ReadAnimations { .. }
                    | Call::ControlAnimation { .. }
                    | Call::AnimationIkLen
                    | Call::ReadAnimationIk { .. }
                    | Call::ControlAnimationIk { .. }
                    | Call::AnimationLenV5
                    | Call::ReadAnimationsV5 { .. }
                    | Call::AnimationIkLenV2
                    | Call::ReadAnimationIkV2 { .. }
                    | Call::AnimationGraphLenV2
                    | Call::ReadAnimationGraphsV2 { .. }
                    | Call::AnimationLenV2
                    | Call::ReadAnimationsV2 { .. }
                    | Call::ControlAnimationV2 { .. }
                    | Call::AnimationLenV4
                    | Call::ReadAnimationsV4 { .. }
                    | Call::AnimationLenV3
                    | Call::ReadAnimationsV3 { .. }
                    | Call::AnimationGraphLen
                    | Call::ReadAnimationGraphs { .. }
                    | Call::ControlAnimationGraph { .. }
            );
        // Only an authorized localization call can poison its active epoch. This
        // wraps enter/memory/transfer checks as well as parsing/query dispatch;
        // stale tokens and unrelated operations retain their existing semantics.
        let authorized = self.active
            && self.epoch == epoch
            && matches!(
                call,
                Call::Locale
                    | Call::Format { .. }
                    | Call::Localize { .. }
                    | Call::ReadLocalized { .. }
                    | Call::SetLocale { .. }
            );
        let result = self.invoke_inner(memory, epoch, call);
        if timer_authorized && result.is_err() {
            self.timers.rejected = true;
        }
        if haptic_authorized && result.is_err() {
            self.haptics.rejected = true;
        }
        if persistent_authorized && result.is_err() {
            self.persistent_rejected = true;
        }
        if restart_authorized && result.is_err() {
            self.restart_rejected = true;
        }
        if physics_authorized
            && result.is_err()
            && let Some(state) = &mut self.physics
        {
            state.reject();
        }
        if steering_authorized
            && result.is_err()
            && let Some(state) = &mut self.steering
        {
            state.reject();
        }
        if steering_authorized
            && result.is_err()
            && let Some(state) = &mut self.navigation
        {
            state.reject();
        }
        if hud_authorized && result.is_err() {
            self.hud_rejected = true;
        }
        if light_authorized && result.is_err() {
            self.light_rejected = true;
        }
        if audio_authorized && result.is_err() {
            self.audio_rejected = true;
        }
        if animation_authorized && result.is_err() {
            self.animation_rejected = true;
        }
        if authorized
            && result.is_err()
            && let Some(state) = &mut self.localization
        {
            state.reject();
        }
        result
    }
    fn invoke_inner(&mut self, memory: &mut [u8], epoch: i64, call: Call) -> Result<i32, Error> {
        self.enter(epoch)?;
        if memory.len() > limits::MEMORY {
            return Err(Error::Limit("linear memory"));
        }
        if matches!(
            call,
            Call::AnimationGraphLen | Call::ReadAnimationGraphs { .. }
        ) && self.animation_graph_rows.iter().any(|g| g.fade.is_some())
        {
            return Err(Error::Abi(
                "animation graph v1 cannot represent compound fades",
            ));
        }
        // Root stripping is active even when movement is disabled or extract-only.
        // Guard before any shared-cache read: older complete frames cannot carry it.
        if !self.root_motion_rows.is_empty()
            && matches!(
                call,
                Call::AnimationLen
                    | Call::ReadAnimations { .. }
                    | Call::AnimationLenV2
                    | Call::ReadAnimationsV2 { .. }
                    | Call::AnimationLenV3
                    | Call::ReadAnimationsV3 { .. }
                    | Call::AnimationLenV4
                    | Call::ReadAnimationsV4 { .. }
                    | Call::AnimationLenV5
                    | Call::ReadAnimationsV5 { .. }
                    | Call::AnimationIkLen
                    | Call::ReadAnimationIk { .. }
                    | Call::AnimationIkLenV2
                    | Call::ReadAnimationIkV2 { .. }
            )
        {
            return Err(Error::Abi(
                "animation snapshots cannot represent root stripping",
            ));
        }
        // These pose protocols have no post-sample IK payload. Refuse only
        // pose observations; other module capabilities and native rendering
        // remain usable. Every later pose ABI must preserve this admission.
        if self.animation_ik_present
            && matches!(
                call,
                Call::AnimationLen
                    | Call::ReadAnimations { .. }
                    | Call::AnimationLenV2
                    | Call::ReadAnimationsV2 { .. }
                    | Call::AnimationLenV3
                    | Call::ReadAnimationsV3 { .. }
                    | Call::AnimationLenV4
                    | Call::ReadAnimationsV4 { .. }
                    | Call::AnimationLenV5
                    | Call::ReadAnimationsV5 { .. }
            )
        {
            return Err(Error::Abi(
                "animation snapshots cannot represent IK constraints",
            ));
        }
        let units = match call {
            Call::Locale | Call::Format { .. } | Call::Localize { .. } => {
                incant_wasm_abi::LOCALIZATION_QUERY_UNITS
            }
            Call::Steer { .. }
            | Call::FindPath { .. }
            | Call::FindGridPath { .. }
            | Call::PhysicsEvents { .. } => 1,
            _ => 0,
        };
        self.query_units = self
            .query_units
            .checked_add(units)
            .filter(|v| *v <= incant_wasm_abi::NATIVE_MAX_QUERY_UNITS)
            .ok_or(Error::Limit("query units"))?;
        if matches!(
            call,
            Call::Steer { .. } | Call::FindPath { .. } | Call::FindGridPath { .. }
        ) {
            self.navigation_queries = self
                .navigation_queries
                .checked_add(1)
                .filter(|&n| n <= incant_wasm_abi::NAVIGATION_MAX_QUERIES)
                .ok_or(Error::Limit("navigation query quota"))?;
        }
        match call {
            Call::TimerLen => return Ok(self.timers.snapshot_len() as i32),
            Call::ReadTimers { ptr, capacity } => {
                let len = self.timers.snapshot_len();
                if usize::try_from(capacity).ok().is_none_or(|n| n < len) {
                    return Err(Error::Abi("timer snapshot capacity"));
                }
                self.transfer(len)?;
                let region = range(ptr, len, memory.len())?;
                self.timers.snapshot(&mut memory[region])?;
                return Ok(len as i32);
            }
            Call::CommandTimer { ptr, len } => {
                if len != incant_wasm_abi::TimerCommand::SIZE as i32 {
                    return Err(Error::Abi("timer command framing"));
                }
                let bytes = self.read::<{ incant_wasm_abi::TimerCommand::SIZE }>(memory, ptr)?;
                let row = incant_wasm_abi::TimerCommand::decode(&bytes)
                    .ok_or(Error::Abi("timer command"))?;
                self.timers.command(row)?;
                return Ok(0);
            }
            Call::HapticLen | Call::ReadHaptics { .. } => {
                let len = self.haptics.snapshot_len();
                if let Call::ReadHaptics { ptr, capacity } = call {
                    let capacity = usize::try_from(capacity)
                        .ok()
                        .filter(|&n| n >= len)
                        .ok_or(Error::Abi("haptic destination capacity"))?;
                    range(ptr, capacity, memory.len())?;
                    let region = range(ptr, len, memory.len())?;
                    self.transfer(len)?;
                    self.haptics.snapshot(&mut memory[region]);
                }
                return Ok(len as i32);
            }
            Call::CommandHaptics { ptr, len } => {
                let len = usize::try_from(len)
                    .ok()
                    .filter(|n| {
                        (incant_wasm_abi::HapticCommandHeader::SIZE
                            ..=incant_wasm_abi::HAPTIC_MAX_COMMAND_BYTES)
                            .contains(n)
                    })
                    .ok_or(Error::Abi("haptic command length"))?;
                let region = range(ptr, len, memory.len())?;
                self.transfer(len)?;
                let commands = incant_wasm_abi::haptics::Commands::parse(&memory[region])
                    .map_err(|_| Error::Abi("haptic command values"))?;
                let used = self.commands.len() + self.restart_calls + self.haptics.len();
                self.haptics.queue(commands, limits::COMMANDS - used)?;
                return Ok(0);
            }
            Call::ReadPersistent { ptr, offset, len }
            | Call::WritePersistent { ptr, offset, len } => {
                let len =
                    usize::try_from(len).map_err(|_| Error::Abi("persistent state length"))?;
                let region = range(ptr, len, memory.len())?;
                self.transfer(len)?;
                let state = self
                    .persistent
                    .as_mut()
                    .ok_or(Error::Abi("module has no persistent state declaration"))?;
                if matches!(call, Call::WritePersistent { .. }) {
                    state.write(offset, &memory[region])?;
                } else {
                    state.read(offset, &mut memory[region])?;
                }
                return Ok(0);
            }
            Call::RequestRestart => {
                if !self.restart_enabled {
                    return Err(Error::Abi("host does not accept session restart requests"));
                }
                if self.commands.len() + self.restart_calls + self.haptics.len() >= limits::COMMANDS
                {
                    return Err(Error::Limit("structural command quota"));
                }
                self.restart_calls += 1;
                return Ok(0);
            }
            Call::AudioLen | Call::ReadAudio { .. } => {
                let len = self.audio.len();
                if let Call::ReadAudio { ptr, capacity } = call {
                    let capacity = usize::try_from(capacity)
                        .ok()
                        .filter(|&n| n >= len)
                        .ok_or(Error::Abi("audio destination capacity"))?;
                    // Validate the complete declared destination before any copy.
                    range(ptr, capacity, memory.len())?;
                    let region = range(ptr, len, memory.len())?;
                    self.transfer(len)?;
                    memory[region].copy_from_slice(&self.audio);
                }
                return Ok(len as i32);
            }
            Call::CommandLight { ptr, len } => {
                let len = usize::try_from(len)
                    .ok()
                    .filter(|&n| {
                        (incant_wasm_abi::LightCommandHeader::SIZE
                            ..=incant_wasm_abi::LIGHT_MAX_COMMAND_BYTES)
                            .contains(&n)
                    })
                    .ok_or(Error::Abi("light command length"))?;
                let region = range(ptr, len, memory.len())?;
                self.transfer(len)?;
                let commands = incant_wasm_abi::light::Commands::parse(&memory[region])
                    .map_err(|_| Error::Abi("light command values"))?;
                if commands.iter().len() > limits::COMMANDS - self.commands.len() {
                    return Err(Error::Limit("structural command quota"));
                }
                for row in commands.iter() {
                    let control = match incant_wasm_abi::light::validate_command(&row)
                        .expect("validated light batch")
                    {
                        incant_wasm_abi::LightControlKind::Color => {
                            incant_runtime::LightControl::Color(row.values)
                        }
                        incant_wasm_abi::LightControlKind::Intensity => {
                            incant_runtime::LightControl::Intensity(row.values[0])
                        }
                    };
                    self.command(StructuralCommand::ControlLight {
                        entity: StableId(row.entity),
                        control,
                    })?;
                }
                return Ok(0);
            }
            Call::CommandAudio { ptr, len } => {
                let len = usize::try_from(len)
                    .ok()
                    .filter(|&n| {
                        (incant_wasm_abi::AudioCommandHeader::SIZE
                            ..=incant_wasm_abi::AUDIO_MAX_COMMAND_BYTES)
                            .contains(&n)
                    })
                    .ok_or(Error::Abi("audio command length"))?;
                let region = range(ptr, len, memory.len())?;
                self.transfer(len)?;
                let commands = incant_wasm_abi::audio::Commands::parse(&memory[region])
                    .map_err(|_| Error::Abi("audio command values"))?;
                if commands.iter().len() > limits::COMMANDS - self.commands.len() {
                    return Err(Error::Limit("structural command quota"));
                }
                for row in commands.iter() {
                    self.command(crate::audio::command(row))?;
                }
                return Ok(0);
            }
            Call::HudLen | Call::ReadHud { .. } => {
                let len = self.hud.len();
                if let Call::ReadHud { ptr, capacity } = call {
                    if usize::try_from(capacity).ok().is_none_or(|n| n < len) {
                        return Err(Error::Abi("HUD destination capacity"));
                    }
                    let region = range(ptr, len, memory.len())?;
                    self.transfer(len)?;
                    memory[region].copy_from_slice(&self.hud);
                }
                return Ok(len as i32);
            }
            Call::ControlHud { ptr, len } => {
                let len = usize::try_from(len)
                    .ok()
                    .filter(|&n| {
                        (incant_wasm_abi::HudCommand::SIZE
                            ..=incant_wasm_abi::HudCommand::SIZE
                                + incant_wasm_abi::HUD_MAX_TEXT_BYTES)
                            .contains(&n)
                    })
                    .ok_or(Error::Abi("HUD command size"))?;
                let region = range(ptr, len, memory.len())?;
                self.transfer(len)?;
                let command = crate::hud::command(&memory[region])?;
                self.command(command)?;
                return Ok(0);
            }
            Call::RootMotionLen | Call::ReadRootMotion { .. } => {
                if self.root_motion.is_empty() {
                    let scene = self
                        .animation_scene
                        .ok_or(Error::Abi("animation scene absent"))?;
                    if self.animations_v5.is_empty() {
                        crate::animation_poses_v5::encode(
                            scene,
                            &mut self.animation_rows,
                            &mut self.animations_v5,
                        )?;
                    }
                    if self.animation_ik_v2.is_empty() {
                        crate::animation_ik_v2::encode(
                            &self.animation_rows,
                            &self.animations_v5,
                            &self.ik_rows,
                            &mut self.animation_ik_v2,
                        )?;
                    }
                    crate::root_motion::encode(
                        scene,
                        &self.animation_rows,
                        &self.animation_ik_v2,
                        &self.root_motion_rows,
                        &mut self.root_motion,
                    )?;
                }
                let len = self.root_motion.len();
                if let Call::ReadRootMotion { ptr, capacity } = call {
                    let capacity = usize::try_from(capacity)
                        .map_err(|_| Error::Abi("root-motion destination capacity"))?;
                    if capacity < len {
                        return Err(Error::Abi("root-motion destination capacity"));
                    }
                    range(ptr, capacity, memory.len())?;
                    let region = range(ptr, len, memory.len())?;
                    self.transfer(len)?;
                    memory[region].copy_from_slice(&self.root_motion);
                }
                return Ok(len as i32);
            }
            Call::ControlRootMotion { ptr, len } => {
                if usize::try_from(len).ok() != Some(incant_wasm_abi::RootMotionCommand::SIZE) {
                    return Err(Error::Abi("root-motion command length"));
                }
                let bytes =
                    self.read::<{ incant_wasm_abi::RootMotionCommand::SIZE }>(memory, ptr)?;
                self.command(crate::root_motion::command(&bytes)?)?;
            }
            Call::PhysicsQuery { ptr, len } => {
                let len = usize::try_from(len)
                    .ok()
                    .filter(|n| *n <= incant_wasm_abi::PHYSICS_MAX_REQUEST_BYTES)
                    .ok_or(Error::Abi("physics request length"))?;
                let region = range(ptr, len, memory.len())?;
                self.transfer(len)?;
                let batch = incant_wasm_abi::physics::decode_batch(&memory[region])
                    .map_err(|_| Error::Abi("physics request"))?;
                self.query_units = self
                    .query_units
                    .checked_add(batch.len())
                    .filter(|n| *n <= incant_wasm_abi::NATIVE_MAX_QUERY_UNITS)
                    .ok_or(Error::Limit("query units"))?;
                return self.physics_state()?.query(batch);
            }
            Call::PhysicsEvents { dimension } => return self.physics_state()?.events(dimension),
            Call::ReadPhysics { ptr, capacity } => {
                let len = self.physics_state()?.reply()?.len();
                let capacity = usize::try_from(capacity)
                    .map_err(|_| Error::Abi("physics destination capacity"))?;
                if capacity < len {
                    return Err(Error::Abi("physics destination capacity"));
                }
                range(ptr, capacity, memory.len())?;
                let region = range(ptr, len, memory.len())?;
                self.transfer(len)?;
                memory[region].copy_from_slice(self.physics_state()?.reply()?);
                return Ok(len as i32);
            }
            Call::FindPath { ptr, len } | Call::FindGridPath { ptr, len } => {
                let grid = matches!(call, Call::FindGridPath { .. });
                let expected = if grid {
                    incant_wasm_abi::navigation_paths::GRID_REQUEST_BYTES
                } else {
                    incant_wasm_abi::navigation_paths::PATH_REQUEST_BYTES
                };
                if usize::try_from(len).ok() != Some(expected) {
                    return Err(Error::Abi("navigation request length"));
                }
                let region = range(ptr, expected, memory.len())?;
                self.transfer(expected)?;
                return self.navigation_state()?.query(&memory[region], grid);
            }
            Call::ReadPath { ptr, capacity } => {
                let len = self.navigation_state()?.reply()?.len();
                let capacity = usize::try_from(capacity)
                    .map_err(|_| Error::Abi("navigation destination capacity"))?;
                if capacity < len {
                    return Err(Error::Abi("navigation destination capacity"));
                }
                range(ptr, capacity, memory.len())?;
                let region = range(ptr, len, memory.len())?;
                self.transfer(len)?;
                memory[region].copy_from_slice(self.navigation_state()?.reply()?);
                return Ok(len as i32);
            }
            Call::AnimationLenV5
            | Call::ReadAnimationsV5 { .. }
            | Call::AnimationIkLenV2
            | Call::ReadAnimationIkV2 { .. }
            | Call::AnimationGraphLenV2
            | Call::ReadAnimationGraphsV2 { .. } => {
                let graph = matches!(
                    call,
                    Call::AnimationGraphLenV2 | Call::ReadAnimationGraphsV2 { .. }
                );
                let ik = matches!(
                    call,
                    Call::AnimationIkLenV2 | Call::ReadAnimationIkV2 { .. }
                );
                if graph {
                    if self.animation_graphs_v2.is_empty() {
                        self.prepare_animation_snapshot(true);
                        crate::animation_graphs_v2::encode(
                            &self.animation_graph_rows,
                            &self.animation_graphs,
                            &mut self.animation_graphs_v2,
                        );
                    }
                } else {
                    if self.animations_v5.is_empty() {
                        crate::animation_poses_v5::encode(
                            self.animation_scene
                                .ok_or(Error::Abi("animation scene absent"))?,
                            &mut self.animation_rows,
                            &mut self.animations_v5,
                        )?;
                    }
                    if ik && self.animation_ik_v2.is_empty() {
                        crate::animation_ik_v2::encode(
                            &self.animation_rows,
                            &self.animations_v5,
                            &self.ik_rows,
                            &mut self.animation_ik_v2,
                        )?;
                    }
                }
                let len = if graph {
                    self.animation_graphs_v2.len()
                } else if ik {
                    self.animation_ik_v2.len()
                } else {
                    self.animations_v5.len()
                };
                if let Call::ReadAnimationsV5 { ptr, capacity }
                | Call::ReadAnimationIkV2 { ptr, capacity }
                | Call::ReadAnimationGraphsV2 { ptr, capacity } = call
                {
                    let capacity = usize::try_from(capacity)
                        .map_err(|_| Error::Abi("animation destination capacity"))?;
                    if capacity < len {
                        return Err(Error::Abi("animation destination capacity"));
                    }
                    range(ptr, capacity, memory.len())?;
                    let region = range(ptr, len, memory.len())?;
                    self.transfer(len)?;
                    let bytes = if graph {
                        &self.animation_graphs_v2
                    } else if ik {
                        &self.animation_ik_v2
                    } else {
                        &self.animations_v5
                    };
                    memory[region].copy_from_slice(bytes);
                }
                return Ok(len as i32);
            }
            Call::AnimationIkLen | Call::ReadAnimationIk { .. } => {
                if self.animation_ik.is_empty() {
                    if self.animations_v4.is_empty() {
                        crate::animation_poses::encode(
                            self.animation_scene
                                .ok_or(Error::Abi("animation scene absent"))?,
                            &mut self.animation_rows,
                            &mut self.animations_v4,
                        )?;
                    }
                    crate::animation_ik::encode(
                        &self.animation_rows,
                        &self.animations_v4,
                        &self.ik_rows,
                        &mut self.animation_ik,
                    )?;
                }
                let len = self.animation_ik.len();
                if let Call::ReadAnimationIk { ptr, capacity } = call {
                    let capacity = usize::try_from(capacity)
                        .map_err(|_| Error::Abi("animation IK destination capacity"))?;
                    if capacity < len {
                        return Err(Error::Abi("animation IK destination capacity"));
                    }
                    range(ptr, capacity, memory.len())?;
                    let region = range(ptr, len, memory.len())?;
                    self.transfer(len)?;
                    memory[region].copy_from_slice(&self.animation_ik);
                }
                return Ok(len as i32);
            }
            Call::ControlAnimationIk { ptr, len } => {
                if usize::try_from(len).ok() != Some(incant_wasm_abi::AnimationIkCommand::SIZE) {
                    return Err(Error::Abi("animation IK command length"));
                }
                let bytes =
                    self.read::<{ incant_wasm_abi::AnimationIkCommand::SIZE }>(memory, ptr)?;
                self.command(crate::animation_ik::command(&bytes)?)?;
            }
            Call::AnimationLenV4 | Call::ReadAnimationsV4 { .. } => {
                if self.animations_v4.is_empty() {
                    crate::animation_poses::encode(
                        self.animation_scene
                            .ok_or(Error::Abi("animation scene absent"))?,
                        &mut self.animation_rows,
                        &mut self.animations_v4,
                    )?;
                }
                let len = self.animations_v4.len();
                if let Call::ReadAnimationsV4 { ptr, capacity } = call {
                    let capacity = usize::try_from(capacity)
                        .map_err(|_| Error::Abi("animation pose destination capacity"))?;
                    if capacity < len {
                        return Err(Error::Abi("animation pose destination capacity"));
                    }
                    range(ptr, capacity, memory.len())?;
                    let region = range(ptr, len, memory.len())?;
                    self.transfer(len)?;
                    memory[region].copy_from_slice(&self.animations_v4);
                }
                return Ok(len as i32);
            }
            Call::AnimationLenV3 | Call::AnimationGraphLen => {
                let graph = matches!(call, Call::AnimationGraphLen);
                self.prepare_animation_snapshot(graph);
                if !graph && self.animations_v3.is_empty() {
                    return Err(Error::Abi(
                        "animation v3 cannot represent weighted tree poses",
                    ));
                }
                return Ok(if graph {
                    self.animation_graphs.len()
                } else {
                    self.animations_v3.len()
                } as i32);
            }
            Call::ReadAnimationsV3 { ptr, capacity }
            | Call::ReadAnimationGraphs { ptr, capacity } => {
                let graph = matches!(call, Call::ReadAnimationGraphs { .. });
                self.prepare_animation_snapshot(graph);
                if !graph && self.animations_v3.is_empty() {
                    return Err(Error::Abi(
                        "animation v3 cannot represent weighted tree poses",
                    ));
                }
                let len = if graph {
                    self.animation_graphs.len()
                } else {
                    self.animations_v3.len()
                };
                let capacity = usize::try_from(capacity)
                    .map_err(|_| Error::Abi("animation destination capacity"))?;
                if capacity < len {
                    return Err(Error::Abi("animation destination capacity"));
                }
                range(ptr, capacity, memory.len())?;
                let region = range(ptr, len, memory.len())?;
                self.transfer(len)?;
                memory[region].copy_from_slice(if graph {
                    &self.animation_graphs
                } else {
                    &self.animations_v3
                });
                return Ok(len as i32);
            }
            Call::ControlAnimationGraph { id, ptr } => {
                let bytes =
                    self.read::<{ incant_wasm_abi::AnimationGraphCommand::SIZE }>(memory, ptr)?;
                let control = crate::animation_graphs::command(&bytes)?;
                let entity = self.stable(memory, id)?;
                self.command(StructuralCommand::ControlAnimationGraph { entity, control })?;
            }
            Call::Steer { ptr, len } => {
                let len = usize::try_from(len).map_err(|_| Error::Abi("steering byte length"))?;
                if len > incant_wasm_abi::STEERING_MAX_REQUEST_BYTES {
                    return Err(Error::Limit("steering frame bounds"));
                }
                let region = range(ptr, len, memory.len())?;
                self.transfer(len)?;
                return self.steering_state()?.steer(&memory[region]);
            }
            Call::ReadSteering { ptr, capacity } => {
                let len = self.steering_state()?.reply()?.len();
                let capacity = usize::try_from(capacity)
                    .map_err(|_| Error::Abi("steering destination capacity"))?;
                if capacity < len {
                    return Err(Error::Abi("steering destination capacity"));
                }
                range(ptr, capacity, memory.len())?;
                let region = range(ptr, len, memory.len())?;
                self.transfer(len)?;
                memory[region].copy_from_slice(self.steering_state()?.reply()?);
                return Ok(len as i32);
            }
            Call::AnimationLen | Call::AnimationLenV2 => {
                let v2 = matches!(call, Call::AnimationLenV2);
                if self.animations_v2.is_empty() {
                    return Err(Error::Abi(
                        "animation graph blend requires graph snapshot ABI",
                    ));
                }
                if !v2 && self.animations.is_empty() {
                    return Err(Error::Abi("active transitions require animation v2"));
                }
                return Ok(if v2 {
                    self.animations_v2.len()
                } else {
                    self.animations.len()
                } as i32);
            }
            Call::ReadAnimations { ptr, capacity } | Call::ReadAnimationsV2 { ptr, capacity } => {
                let v2 = matches!(call, Call::ReadAnimationsV2 { .. });
                if self.animations_v2.is_empty() {
                    return Err(Error::Abi(
                        "animation graph blend requires graph snapshot ABI",
                    ));
                }
                if !v2 && self.animations.is_empty() {
                    return Err(Error::Abi("active transitions require animation v2"));
                }
                let len = if v2 {
                    self.animations_v2.len()
                } else {
                    self.animations.len()
                };
                let capacity = usize::try_from(capacity)
                    .map_err(|_| Error::Abi("animation destination capacity"))?;
                if capacity < len {
                    return Err(Error::Abi("animation destination capacity"));
                }
                range(ptr, capacity, memory.len())?;
                let region = range(ptr, len, memory.len())?;
                self.transfer(len)?;
                memory[region].copy_from_slice(if v2 {
                    &self.animations_v2
                } else {
                    &self.animations
                });
                return Ok(len as i32);
            }
            Call::ControlAnimation { id, ptr } | Call::ControlAnimationV2 { id, ptr } => {
                let bytes =
                    self.read::<{ incant_wasm_abi::AnimationCommand::SIZE }>(memory, ptr)?;
                let control = if matches!(call, Call::ControlAnimationV2 { .. }) {
                    crate::animations::command_v2(&bytes)?
                } else {
                    crate::animations::command(&bytes)?
                };
                let entity = self.stable(memory, id)?;
                self.command(StructuralCommand::ControlAnimation { entity, control })?;
            }
            Call::Locale => return self.locale_state()?.locale(),
            Call::Format { ptr, len } => {
                let len =
                    usize::try_from(len).map_err(|_| Error::Abi("localization byte length"))?;
                if len != incant_wasm_abi::LOCALIZATION_FORMAT_BYTES {
                    return Err(Error::Abi("localization format length"));
                }
                let region = range(ptr, len, memory.len())?;
                self.transfer(len)?;
                return self.locale_state()?.format(&memory[region]);
            }
            Call::Localize { ptr, len } | Call::SetLocale { ptr, len } => {
                let len =
                    usize::try_from(len).map_err(|_| Error::Abi("localization byte length"))?;
                let max = if matches!(call, Call::Localize { .. }) {
                    incant_wasm_abi::LOCALIZATION_MAX_REQUEST_BYTES
                } else {
                    incant_wasm_abi::LOCALIZATION_MAX_SETTINGS_BYTES
                };
                if len > max {
                    return Err(Error::Limit("localization frame bounds"));
                }
                let region = range(ptr, len, memory.len())?;
                self.transfer(len)?;
                if matches!(call, Call::Localize { .. }) {
                    return self.locale_state()?.localize(&memory[region]);
                }
                let settings = self.locale_state()?.settings(&memory[region])?;
                self.command(StructuralCommand::SetLocale(settings))?;
            }
            Call::ReadLocalized { ptr, capacity } => {
                let len = self.locale_state()?.reply()?.len();
                let capacity = usize::try_from(capacity)
                    .map_err(|_| Error::Abi("localization destination capacity"))?;
                if capacity < len {
                    return Err(Error::Abi("localization destination capacity"));
                }
                range(ptr, capacity, memory.len())?;
                let region = range(ptr, len, memory.len())?;
                self.transfer(len)?;
                memory[region].copy_from_slice(self.locale_state()?.reply()?);
                return i32::try_from(len).map_err(|_| Error::Abi("localization result length"));
            }
            Call::Log { level, ptr, len } => {
                let result = (|| {
                    let len = usize::try_from(len).map_err(|_| Error::Abi("log byte length"))?;
                    if len > incant_wasm_abi::LOG_MAX_MESSAGE_BYTES {
                        return Err(Error::Limit("log message"));
                    }
                    let region = range(ptr, len, memory.len())?;
                    self.transfer(len)?;
                    self.tick_logs.push(level, &memory[region])
                })();
                if result.is_err() {
                    self.tick_logs.rejected = true;
                }
                result?;
            }
            Call::FlipbookLen => return Ok(self.flipbooks.len() as i32),
            Call::ReadFlipbooks { ptr, capacity } => {
                let len = self.flipbooks.len();
                if usize::try_from(capacity).ok().is_none_or(|n| n < len) {
                    return Err(Error::Abi("flipbook destination capacity"));
                }
                range(ptr, capacity as usize, memory.len())?;
                let region = range(ptr, len, memory.len())?;
                self.transfer(len)?;
                memory[region].copy_from_slice(&self.flipbooks);
                return Ok(len as i32);
            }
            Call::ControlFlipbook { id, kind, frame } => {
                let control = match (kind, frame) {
                    (0, 0) => FlipbookControl::Play,
                    (1, 0) => FlipbookControl::Pause,
                    (2, 0) => FlipbookControl::Restart,
                    (3, frame) => FlipbookControl::Seek(
                        u16::try_from(frame).map_err(|_| Error::Abi("flipbook seek frame"))?,
                    ),
                    _ => return Err(Error::Abi("flipbook control tag/reserved frame")),
                };
                let entity = self.stable(memory, id)?;
                self.command(StructuralCommand::ControlFlipbook { entity, control })?;
            }
            Call::InputLen => {
                if !self.input_available {
                    return Err(Error::Abi("input capability unavailable"));
                }
                return Ok(self.input.len() as i32);
            }
            Call::ReadInput { ptr, capacity } => {
                if !self.input_available {
                    return Err(Error::Abi("input capability unavailable"));
                }
                let len = self.input.len();
                if usize::try_from(capacity).ok().is_none_or(|n| n < len) {
                    return Err(Error::Abi("input destination capacity"));
                }
                range(ptr, capacity as usize, memory.len())?;
                let region = range(ptr, len, memory.len())?;
                self.transfer(len)?;
                memory[region].copy_from_slice(&self.input);
                return Ok(len as i32);
            }
            Call::ChunkCount => return Ok(self.chunks.len() as i32),
            Call::ChunkLen(i) => return Ok(self.chunks[self.index(i)?].ids.len() as i32),
            Call::ChunkFlags(i) => {
                return Ok(i32::from(self.chunks[self.index(i)?].velocities.is_some()));
            }
            Call::ReadIds(i, ptr) => {
                let i = self.index(i)?;
                let len = self.chunks[i].ids.len() * 16;
                self.transfer(len)?;
                let region = range(ptr, len, memory.len())?;
                for (id, raw) in self.chunks[i]
                    .ids
                    .iter()
                    .zip(memory[region].as_chunks_mut::<16>().0)
                {
                    raw.copy_from_slice(&id.0);
                }
            }
            Call::Column {
                chunk,
                ptr,
                velocity,
                write,
            } => {
                let i = self.index(chunk)?;
                let len = if velocity {
                    self.chunks[i]
                        .velocities
                        .as_ref()
                        .ok_or(Error::Abi("chunk has no Velocity"))?
                        .len()
                } else {
                    self.chunks[i].transforms.len()
                };
                self.transfer(len * 8)?;
                let region = range(ptr, len * 8, memory.len())?;
                let chunk = &mut self.chunks[i];
                let column = if velocity {
                    chunk.velocities.as_mut().unwrap()
                } else {
                    &mut chunk.transforms
                };
                if write {
                    for (number, raw) in column.iter_mut().zip(memory[region].as_chunks::<8>().0) {
                        *number = f64::from_le_bytes(*raw);
                    }
                    if velocity {
                        chunk.dirty_velocity = true;
                    } else {
                        chunk.dirty_transform = true;
                    }
                } else {
                    for (number, raw) in column.iter().zip(memory[region].as_chunks_mut::<8>().0) {
                        raw.copy_from_slice(&number.to_le_bytes());
                    }
                }
            }
            Call::Spawn {
                id,
                parent,
                transform,
                velocity,
            } => {
                let id = self.stable(memory, id)?;
                let parent = self.optional_id(memory, parent)?;
                let numbers: [f64; 10] = values(&self.read::<80>(memory, transform)?);
                let velocity = self.velocity(memory, velocity)?;
                self.command(StructuralCommand::Spawn(CookedEntity {
                    id,
                    parent,
                    velocity,
                    transform: Transform {
                        translation: numbers[..3].try_into().unwrap(),
                        rotation: numbers[3..7].try_into().unwrap(),
                        scale: numbers[7..].try_into().unwrap(),
                    },
                }))?;
            }
            Call::Despawn(ptr) => {
                let id = self.stable(memory, ptr)?;
                self.command(StructuralCommand::Despawn(id))?;
            }
            Call::SetVelocity(id, ptr) => {
                let entity = self.stable(memory, id)?;
                let velocity = self.velocity(memory, ptr)?;
                self.command(StructuralCommand::SetVelocity { entity, velocity })?;
            }
            Call::SetParent(id, ptr) => {
                let entity = self.stable(memory, id)?;
                let parent = self.optional_id(memory, ptr)?;
                self.command(StructuralCommand::SetParent { entity, parent })?;
            }
        }
        Ok(0)
    }
    fn prepare_animation_snapshot(&mut self, graph: bool) {
        if graph {
            if self.animation_graphs.is_empty() {
                crate::animation_graphs::encode(
                    &self.animation_graph_rows,
                    &mut self.animation_graphs,
                );
            }
        } else if self.animations_v3.is_empty() {
            crate::animations::encode_v3(&self.animation_rows, &mut self.animations_v3);
        }
    }
    fn navigation_state(&mut self) -> Result<&mut crate::navigation_paths::State, Error> {
        self.navigation
            .as_mut()
            .ok_or(Error::Abi("navigation capability absent"))
    }
    fn steering_state(&mut self) -> Result<&mut crate::steering::State, Error> {
        self.steering
            .as_mut()
            .ok_or(Error::Abi("steering epoch unavailable"))
    }
    fn locale_state(&mut self) -> Result<&mut crate::localization::State, Error> {
        self.localization
            .as_mut()
            .ok_or(Error::Abi("localization epoch unavailable"))
    }
}
fn range(ptr: i32, len: usize, total: usize) -> Result<std::ops::Range<usize>, Error> {
    let begin = usize::try_from(ptr).map_err(|_| Error::Abi("negative memory offset"))?;
    let end = begin
        .checked_add(len)
        .filter(|end| *end <= total)
        .ok_or(Error::Abi("memory range out of bounds"))?;
    Ok(begin..end)
}
fn values<const N: usize>(bytes: &[u8]) -> [f64; N] {
    std::array::from_fn(|i| f64::from_le_bytes(bytes[i * 8..i * 8 + 8].try_into().unwrap()))
}

#[cfg(test)]
mod log_clock_tests {
    use super::*;
    #[test]
    fn last_exact_json_tick_accepts_then_overflow_refuses_before_publication() {
        use incant_runtime::{CookedScene, logging::MAX_LOG_TICK};
        let mut world = NativeWorld::new(CookedScene::new(StableId([0; 16]), 64, vec![]).unwrap());
        let mut cap = Capability {
            tick: MAX_LOG_TICK - 1,
            ..Capability::default()
        };
        let epoch = cap.begin(&mut world, 0.25).unwrap();
        cap.invoke(
            &mut [],
            epoch,
            Call::Log {
                level: 1,
                ptr: 0,
                len: 0,
            },
        )
        .unwrap();
        cap.finish(&mut world).unwrap();
        assert_eq!(cap.logs().unwrap().entries()[0].tick, MAX_LOG_TICK);
        let before = world.snapshot();
        assert!(matches!(
            cap.begin(&mut world, 0.25),
            Err(Error::Limit("log clock"))
        ));
        assert!(!cap.active && !cap.is_failed());
        assert_eq!(cap.logs().unwrap().entries().len(), 1);
        assert_eq!(world.snapshot(), before);
        assert_eq!(world.tick(), 0);
        // Reload cannot reset the accepted diagnostic clock to bypass its limit.
        let mut next = cap.replacement();
        assert!(matches!(
            next.begin(&mut world, 0.25),
            Err(Error::Limit("log clock"))
        ));
        assert_eq!(next.logs().unwrap().entries()[0].tick, MAX_LOG_TICK);
    }
}

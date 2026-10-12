mod compound;
mod ik;
mod root_motion;
use crate::{EntityId, Error, FlipbookSnapshot, InputSnapshot, Result, Transform, Velocity, raw};
use core::marker::PhantomData;
use incant_wasm_abi as abi;
/// Valid only during one exported callback. Copies never borrow native storage.
/// An ignored method error still poisons the callback and prevents publication.
pub struct Tick<'tick> {
    epoch: i64,
    failed: bool,
    lifetime: PhantomData<fn(&'tick mut ()) -> &'tick mut ()>,
}
/// A table chunk from this callback, with aligned entity/component row ordering.
/// It cannot escape the exported callback, even through safe retained storage.
/// ```compile_fail
/// let mut retained = None;
/// incant_module::__private::run(1, 0.1, |tick, _| {
///     retained = Some(tick.chunk(0)?);
///     Ok(())
/// });
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Chunk<'tick> {
    index: i32,
    len: usize,
    velocity: bool,
    lifetime: PhantomData<fn(&'tick mut ()) -> &'tick mut ()>,
}
impl Chunk<'_> {
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    pub fn has_velocity(&self) -> bool {
        self.velocity
    }
}
impl<'tick> Tick<'tick> {
    pub(crate) fn new(epoch: i64) -> Self {
        Self {
            epoch,
            failed: false,
            lifetime: PhantomData,
        }
    }
    pub(crate) fn failed(&self) -> bool {
        self.failed
    }
    fn check<T>(&mut self, result: Result<T>) -> Result<T> {
        if self.failed {
            return Err(Error::Failed);
        }
        if result.is_err() {
            self.failed = true;
        }
        result
    }
    fn ready(&mut self) -> Result<()> {
        self.check(Ok(()))
    }
    /// Up to 64 heterogeneous queries over the committed solvers. Results retain
    /// input order and borrow only result scratch, never the Tick or native data.
    pub fn query_physics<'a>(
        &mut self,
        queries: &[crate::physics::Query],
        request_scratch: &mut [u8],
        result_scratch: &'a mut [u8],
    ) -> Result<crate::physics::Results<'a>> {
        self.ready()?;
        self.check(if queries.len() <= abi::PHYSICS_MAX_BATCH {
            Ok(())
        } else {
            Err(Error::Bounds)
        })?;
        let mut rows = [crate::physics::EMPTY; abi::PHYSICS_MAX_BATCH];
        for (to, from) in rows.iter_mut().zip(queries) {
            *to = from.wire();
        }
        let len = self.check(
            abi::physics::encode_batch(&rows[..queries.len()], request_scratch)
                .map_err(crate::physics::error),
        )?;
        // SAFETY: the complete initialized request is copied synchronously.
        let len = unsafe {
            raw::incant_physics_v1_query_batch(
                self.epoch,
                request_scratch.as_ptr() as usize as i32,
                len as i32,
            )
        };
        let len = self.read_physics_reply(len, result_scratch, abi::PHYSICS_MAX_RESULT_BYTES)?;
        let result = self.check(
            abi::physics::decode_results(&result_scratch[..len]).map_err(crate::physics::error),
        )?;
        self.check(
            if result.len() == queries.len()
                && result.iter().zip(&rows).all(|((r, _), q)| r.kind == q.kind)
            {
                Ok(())
            } else {
                Err(Error::Malformed)
            },
        )?;
        Ok(crate::physics::Results(result))
    }
    /// Complete sorted enter/exit events from the last committed step. Reading
    /// is nondestructive, bounded, and shares the native query/transfer quotas.
    pub fn physics_events<'a>(
        &mut self,
        dimension: crate::physics::Dimension,
        scratch: &'a mut [u8],
    ) -> Result<crate::physics::Events<'a>> {
        self.ready()?;
        // SAFETY: scalar selector only; the host retains its copied reply.
        let len = unsafe { raw::incant_physics_v1_trigger_events(self.epoch, dimension.wire()) };
        let len = self.read_physics_reply(len, scratch, abi::PHYSICS_MAX_EVENT_BYTES)?;
        let result = self
            .check(abi::physics::decode_events(&scratch[..len]).map_err(crate::physics::error))?;
        self.check(if result.dimension as i32 == dimension.wire() {
            Ok(())
        } else {
            Err(Error::Malformed)
        })?;
        Ok(crate::physics::Events(result))
    }
    fn read_physics_reply(&mut self, len: i32, scratch: &mut [u8], limit: usize) -> Result<usize> {
        let len = self.snapshot_length(len, scratch.len(), limit)?;
        // SAFETY: checked initialized destination prefix; no native pointer.
        let actual = unsafe {
            raw::incant_physics_v1_read_result(
                self.epoch,
                scratch.as_mut_ptr() as usize as i32,
                len as i32,
            )
        };
        self.check(if usize::try_from(actual).ok() == Some(len) {
            Ok(len)
        } else {
            Err(Error::Malformed)
        })
    }
    /// Find an owned-copy path on the active scene's committed navmesh. A miss
    /// returns None; invalid owners/requests and exhausted budgets reject the tick.
    pub fn find_path<'a>(
        &mut self,
        owner: &EntityId,
        request: crate::navigation::PathRequest,
        scratch: &'a mut [u8],
    ) -> Result<Option<crate::navigation::Path<'a>>> {
        self.ready()?;
        let bytes = self.check(
            abi::navigation_paths::encode_path(abi::PathRequest {
                owner: owner.0,
                start: request.start,
                end: request.end,
                snap_distance: request.snap_distance,
                max_visited: request.max_visited,
                reserved: 0,
            })
            .map_err(crate::navigation::error),
        )?;
        // SAFETY: the initialized fixed frame is copied synchronously.
        let len = unsafe {
            raw::incant_navigation_v1_find_path(
                self.epoch,
                bytes.as_ptr() as usize as i32,
                bytes.len() as i32,
            )
        };
        let len = self.read_path_reply(len, scratch, abi::PATH_MAX_RESULT_BYTES)?;
        self.check(
            abi::navigation_paths::decode_path_result(&scratch[..len])
                .map_err(crate::navigation::error),
        )
    }
    /// Adjacent grid-cell route with exact integer cost and captured generation.
    pub fn find_grid_path<'a>(
        &mut self,
        owner: &EntityId,
        request: crate::navigation::GridPathRequest,
        scratch: &'a mut [u8],
    ) -> Result<Option<crate::navigation::GridPath<'a>>> {
        self.ready()?;
        let bytes = self.check(
            abi::navigation_paths::encode_grid(abi::GridPathRequest {
                owner: owner.0,
                start: request.start.map(u32::from),
                end: request.end.map(u32::from),
                flags: u32::from(request.diagonal),
                max_expansions: request.max_expansions,
            })
            .map_err(crate::navigation::error),
        )?;
        // SAFETY: the initialized fixed frame is copied synchronously.
        let len = unsafe {
            raw::incant_navigation_v1_find_grid_path(
                self.epoch,
                bytes.as_ptr() as usize as i32,
                bytes.len() as i32,
            )
        };
        let len = self.read_path_reply(len, scratch, abi::GRID_PATH_MAX_RESULT_BYTES)?;
        self.check(
            abi::navigation_paths::decode_grid_result(&scratch[..len])
                .map_err(crate::navigation::error),
        )
    }
    fn read_path_reply(&mut self, len: i32, scratch: &mut [u8], limit: usize) -> Result<usize> {
        let len = self.snapshot_length(len, scratch.len(), limit)?;
        // SAFETY: checked initialized destination prefix; host validates whole range.
        let actual = unsafe {
            raw::incant_navigation_v1_read_path(
                self.epoch,
                scratch.as_mut_ptr() as usize as i32,
                len as i32,
            )
        };
        self.check(if usize::try_from(actual).ok() == Some(len) {
            Ok(len)
        } else {
            Err(Error::Malformed)
        })
    }
    /// Compute sorted local-avoidance proposals using this callback's fixed dt.
    /// Aggregate work is bounded to 128 agents, 128 obstacle edges and 32 calls
    /// per update. Returned rows borrow only the caller's result scratch.
    pub fn steer_agents<'a>(
        &mut self,
        request: crate::steering::Request<'_>,
        request_scratch: &mut [u8],
        result_scratch: &'a mut [u8],
    ) -> Result<crate::steering::Velocities<'a>> {
        self.ready()?;
        let len = self.check(
            abi::steering::encode_request(request, request_scratch).map_err(crate::steering::error),
        )?;
        // SAFETY: only the initialized, bounded request prefix is copied during
        // this synchronous import; neither host retains a pointer to the guest.
        let result = unsafe {
            raw::incant_navigation_v1_steer(
                self.epoch,
                request_scratch.as_ptr() as usize as i32,
                len as i32,
            )
        };
        let result_len = self.check(
            usize::try_from(result)
                .ok()
                .filter(|&len| len <= abi::STEERING_MAX_RESULT_BYTES)
                .ok_or(Error::Bounds),
        )?;
        self.check(if result_scratch.len() < result_len {
            Err(Error::Capacity)
        } else {
            Ok(())
        })?;
        // SAFETY: the initialized mutable destination prefix is exclusively
        // borrowed for a synchronous copied read. Complete ranges are rechecked.
        let written = unsafe {
            raw::incant_navigation_v1_read_steering(
                self.epoch,
                result_scratch.as_mut_ptr() as usize as i32,
                result_len as i32,
            )
        };
        self.check(if usize::try_from(written).ok() == Some(result_len) {
            Ok(())
        } else {
            Err(Error::Malformed)
        })?;
        self.check(
            abi::steering::decode_result(&result_scratch[..result_len])
                .map_err(crate::steering::error),
        )
    }
    fn localization_result<'a>(&mut self, len: i32, scratch: &'a mut [u8]) -> Result<&'a [u8]> {
        let len = self.check(
            usize::try_from(len)
                .ok()
                .filter(|&n| n <= abi::LOCALIZATION_MAX_RESULT_BYTES)
                .ok_or(Error::Bounds),
        )?;
        self.check(if scratch.len() < len {
            Err(Error::Capacity)
        } else {
            Ok(())
        })?;
        // SAFETY: exactly this initialized destination prefix is borrowed for the
        // synchronous copied read. Host validates the epoch and complete range.
        let written = unsafe {
            raw::incant_localization_v1_read_result(
                self.epoch,
                scratch.as_mut_ptr() as usize as i32,
                len as i32,
            )
        };
        self.check(if usize::try_from(written).ok() == Some(len) {
            Ok(())
        } else {
            Err(Error::Malformed)
        })?;
        Ok(&scratch[..len])
    }
    /// Observe the committed locale at the start of this callback. A queued
    /// setting change becomes visible after acceptance, in the next callback.
    pub fn locale<'a>(
        &mut self,
        scratch: &'a mut [u8],
    ) -> Result<crate::localization::LocaleSnapshot<'a>> {
        self.ready()?;
        // SAFETY: scalar query only; the host retains its bounded copied result.
        let len = unsafe { raw::incant_localization_v1_locale(self.epoch) };
        let bytes = self.localization_result(len, scratch)?;
        self.check(abi::localization::decode_locale(bytes).map_err(crate::localization::error))
    }
    /// Format a message using the shared native catalog/ICU rules. Arguments are
    /// named Text, finite Number or CalendarDate values. No allocation is needed
    /// in the guest; request/result scratch must be distinct borrowed slices.
    pub fn localize<'a>(
        &mut self,
        request: crate::localization::LocalizeRequest<'_>,
        request_scratch: &mut [u8],
        result_scratch: &'a mut [u8],
    ) -> Result<crate::localization::LocalizedText<'a>> {
        self.ready()?;
        let len = self.check(
            abi::localization::encode_request(request, request_scratch)
                .map_err(crate::localization::error),
        )?;
        // SAFETY: complete initialized versioned frame, synchronously copied.
        let result = unsafe {
            raw::incant_localization_v1_localize(
                self.epoch,
                request_scratch.as_ptr() as usize as i32,
                len as i32,
            )
        };
        let bytes = self.localization_result(result, result_scratch)?;
        self.check(abi::localization::decode_localized(bytes).map_err(crate::localization::error))
    }
    fn format<'a>(
        &mut self,
        request: abi::localization::FormatRequest,
        scratch: &'a mut [u8],
    ) -> Result<&'a str> {
        self.ready()?;
        let mut input = [0; abi::LOCALIZATION_FORMAT_BYTES];
        let len = self.check(
            abi::localization::encode_format(request, &mut input)
                .map_err(crate::localization::error),
        )?;
        // SAFETY: complete fixed-size initialized request, synchronously copied.
        let result = unsafe {
            raw::incant_localization_v1_format(
                self.epoch,
                input.as_ptr() as usize as i32,
                len as i32,
            )
        };
        let bytes = self.localization_result(result, scratch)?;
        self.check(abi::localization::decode_formatted(bytes).map_err(crate::localization::error))
    }
    /// ICU number formatting in the callback's committed locale. Finite values
    /// must be within JavaScript's safe integer magnitude. No pseudo wrapping.
    pub fn format_number<'a>(&mut self, value: f64, scratch: &'a mut [u8]) -> Result<&'a str> {
        self.format(abi::localization::FormatRequest::Number(value), scratch)
    }
    /// ICU calendar-date formatting in the callback's committed locale. Use
    /// DateLength::Medium for the TypeScript API's default length. The native
    /// formatter checks actual Gregorian validity, including leap years.
    pub fn format_date<'a>(
        &mut self,
        date: crate::localization::CalendarDate,
        length: crate::localization::DateLength,
        scratch: &'a mut [u8],
    ) -> Result<&'a str> {
        self.format(
            abi::localization::FormatRequest::Date(date, length),
            scratch,
        )
    }
    /// Queue a validated locale/fallback/pseudo setting in the ordinary atomic
    /// structural transaction. This does not edit an authoring document.
    pub fn set_locale(
        &mut self,
        settings: crate::localization::LocaleSettings<'_>,
        scratch: &mut [u8],
    ) -> Result<()> {
        self.ready()?;
        let len = self.check(
            abi::localization::encode_settings(settings, scratch)
                .map_err(crate::localization::error),
        )?;
        // SAFETY: initialized bounded frame copied before this borrow expires;
        // the host independently validates every setting and all command quotas.
        unsafe {
            raw::incant_localization_v1_set_locale(
                self.epoch,
                scratch.as_ptr() as usize as i32,
                len as i32,
            );
        }
        Ok(())
    }
    /// Copy a primitive UTF-8 diagnostic into this update's bounded log buffer.
    /// Publication waits for acceptance; history retains a prefix and counts drops.
    /// An invalid message poisons this callback even if its error is ignored.
    pub fn log(&mut self, message: &str, level: abi::LogLevel) -> Result<()> {
        self.ready()?;
        self.check(if message.len() > abi::LOG_MAX_MESSAGE_BYTES {
            Err(Error::Bounds)
        } else if message
            .chars()
            .any(|c| c.is_control() && c != '\n' && c != '\t')
        {
            Err(Error::Malformed)
        } else {
            Ok(())
        })?;
        // SAFETY: initialized UTF-8 bytes borrowed through this synchronous copy;
        // the host independently checks the epoch, range, level and all quotas.
        unsafe {
            raw::incant_log_v1_write(
                self.epoch,
                level as i32,
                message.as_ptr() as usize as i32,
                message.len() as i32,
            );
        }
        Ok(())
    }
    pub fn chunk_count(&mut self) -> Result<usize> {
        self.ready()?;
        // SAFETY: scalar import only. Host checks active epoch/call quota.
        let count = unsafe { raw::incant_v1_chunk_count(self.epoch) };
        self.check(
            usize::try_from(count)
                .ok()
                .filter(|&n| n <= abi::CHUNKS)
                .ok_or(Error::Bounds),
        )
    }
    pub fn chunk(&mut self, index: usize) -> Result<Chunk<'tick>> {
        if index >= self.chunk_count()? {
            return self.check(Err(Error::Bounds));
        }
        // SAFETY: bounded index, scalar imports; no guest/native pointer involved.
        let (len, flags) = unsafe {
            (
                raw::incant_v1_chunk_len(self.epoch, index as i32),
                raw::incant_v1_chunk_flags(self.epoch, index as i32),
            )
        };
        let len = self.check(
            usize::try_from(len)
                .ok()
                .filter(|n| *n <= abi::COLUMNS / abi::ID_BYTES)
                .ok_or(Error::Bounds),
        )?;
        if flags & !1 != 0 {
            return self.check(Err(Error::Malformed));
        }
        Ok(Chunk {
            index: index as i32,
            len,
            velocity: flags & 1 != 0,
            lifetime: PhantomData,
        })
    }
    fn capacity<T>(&mut self, chunk: Chunk<'tick>, values: &[T]) -> Result<()> {
        self.check(if values.len() >= chunk.len {
            Ok(())
        } else {
            Err(Error::Capacity)
        })
    }
    pub fn read_ids<'a>(
        &mut self,
        chunk: Chunk<'tick>,
        values: &'a mut [EntityId],
    ) -> Result<&'a mut [EntityId]> {
        self.capacity(chunk, values)?;
        // SAFETY: EntityId is transparent initialized [u8;16]; whole prefix fits.
        unsafe {
            raw::incant_v1_read_ids(self.epoch, chunk.index, values.as_mut_ptr() as usize as i32);
        }
        Ok(&mut values[..chunk.len])
    }
    pub fn read_transforms<'a>(
        &mut self,
        chunk: Chunk<'tick>,
        values: &'a mut [Transform],
    ) -> Result<&'a mut [Transform]> {
        self.capacity(chunk, values)?;
        // SAFETY: initialized arrays of f64, exact numeric stride and whole prefix.
        unsafe {
            raw::incant_v1_read_transforms(
                self.epoch,
                chunk.index,
                values.as_mut_ptr() as usize as i32,
            );
        }
        Ok(&mut values[..chunk.len])
    }
    pub fn write_transforms(&mut self, chunk: Chunk<'tick>, values: &[Transform]) -> Result<()> {
        self.capacity(chunk, values)?;
        // SAFETY: initialized fixed-stride f64 prefix. Host validates numeric domain.
        unsafe {
            raw::incant_v1_write_transforms(
                self.epoch,
                chunk.index,
                values.as_ptr() as usize as i32,
            );
        }
        Ok(())
    }
    fn velocity(&mut self, chunk: Chunk<'tick>) -> Result<()> {
        self.check(if chunk.velocity {
            Ok(())
        } else {
            Err(Error::MissingVelocity)
        })
    }
    pub fn read_velocities<'a>(
        &mut self,
        chunk: Chunk<'tick>,
        values: &'a mut [Velocity],
    ) -> Result<&'a mut [Velocity]> {
        self.velocity(chunk)?;
        self.capacity(chunk, values)?;
        // SAFETY: initialized f64 arrays and validated present column/prefix.
        unsafe {
            raw::incant_v1_read_velocities(
                self.epoch,
                chunk.index,
                values.as_mut_ptr() as usize as i32,
            );
        }
        Ok(&mut values[..chunk.len])
    }
    pub fn write_velocities(&mut self, chunk: Chunk<'tick>, values: &[Velocity]) -> Result<()> {
        self.velocity(chunk)?;
        self.capacity(chunk, values)?;
        // SAFETY: initialized fixed-stride prefix; no pointer escapes this call.
        unsafe {
            raw::incant_v1_write_velocities(
                self.epoch,
                chunk.index,
                values.as_ptr() as usize as i32,
            );
        }
        Ok(())
    }
    pub fn spawn(
        &mut self,
        id: &EntityId,
        parent: Option<&EntityId>,
        transform: &Transform,
        velocity: Option<&Velocity>,
    ) -> Result<()> {
        self.ready()?;
        // SAFETY: borrowed fixed arrays live through the synchronous copy. -1 is
        // the registry's null sentinel, never a fabricated native pointer.
        unsafe {
            raw::incant_v1_spawn(
                self.epoch,
                ptr(id),
                optional(parent),
                ptr(transform),
                optional(velocity),
            );
        }
        Ok(())
    }
    pub fn despawn(&mut self, id: &EntityId) -> Result<()> {
        self.ready()?;
        // SAFETY: fixed-size identity is initialized and borrowed for this call.
        unsafe {
            raw::incant_v1_despawn(self.epoch, ptr(id));
        }
        Ok(())
    }
    pub fn set_parent(&mut self, id: &EntityId, parent: Option<&EntityId>) -> Result<()> {
        self.ready()?;
        // SAFETY: identities are borrowed initialized bytes; -1 is the null tag.
        unsafe {
            raw::incant_v1_set_parent(self.epoch, ptr(id), optional(parent));
        }
        Ok(())
    }
    pub fn set_velocity(&mut self, id: &EntityId, velocity: Option<&Velocity>) -> Result<()> {
        self.ready()?;
        // SAFETY: identity and optional f64 row stay live until the host copies them.
        unsafe {
            raw::incant_v1_set_velocity(self.epoch, ptr(id), optional(velocity));
        }
        Ok(())
    }
    fn control(&mut self, id: &EntityId, kind: abi::FlipbookControl, frame: u16) -> Result<()> {
        self.ready()?;
        // SAFETY: initialized identity, generated tag, and scalar frame.
        unsafe {
            raw::incant_flipbook_v1_control(self.epoch, ptr(id), kind as i32, i32::from(frame));
        }
        Ok(())
    }
    pub fn play(&mut self, id: &EntityId) -> Result<()> {
        self.control(id, abi::FlipbookControl::Play, 0)
    }
    pub fn pause(&mut self, id: &EntityId) -> Result<()> {
        self.control(id, abi::FlipbookControl::Pause, 0)
    }
    pub fn restart(&mut self, id: &EntityId) -> Result<()> {
        self.control(id, abi::FlipbookControl::Restart, 0)
    }
    pub fn seek(&mut self, id: &EntityId, frame: u16) -> Result<()> {
        self.control(id, abi::FlipbookControl::Seek, frame)
    }
    pub fn input<'a>(&mut self, scratch: &'a mut [u8]) -> Result<InputSnapshot<'a>> {
        self.ready()?;
        // SAFETY: length import has no pointers; the complete returned byte count
        // is validated against initialized scratch before exposing its pointer.
        let len = unsafe { raw::incant_input_v1_snapshot_len(self.epoch) };
        let len = self.snapshot_length(len, scratch.len(), abi::INPUT_MAX_BYTES)?;
        let actual = unsafe {
            raw::incant_input_v1_read_snapshot(
                self.epoch,
                scratch.as_mut_ptr() as usize as i32,
                len as i32,
            )
        };
        if actual as usize != len {
            return self.check(Err(Error::Malformed));
        }
        self.check(InputSnapshot::parse(&scratch[..len]))
    }
    pub fn flipbooks<'a>(&mut self, scratch: &'a mut [u8]) -> Result<FlipbookSnapshot<'a>> {
        self.ready()?;
        // SAFETY: scalar length query; the following copy uses checked scratch.
        let len = unsafe { raw::incant_flipbook_v1_snapshot_len(self.epoch) };
        let len = self.snapshot_length(len, scratch.len(), abi::FLIPBOOK_MAX_BYTES)?;
        let actual = unsafe {
            raw::incant_flipbook_v1_read_snapshot(
                self.epoch,
                scratch.as_mut_ptr() as usize as i32,
                len as i32,
            )
        };
        if actual as usize != len {
            return self.check(Err(Error::Malformed));
        }
        self.check(FlipbookSnapshot::parse(&scratch[..len]))
    }
    /// Copy the complete start-of-callback local pose with explicit interpolation
    /// semantics and scene identity. Returned samples borrow only caller scratch.
    /// Queued controls remain invisible until the entire callback commits.
    pub fn animation_poses<'a>(
        &mut self,
        scratch: &'a mut [u8],
    ) -> Result<crate::AnimationPoseSnapshot<'a>> {
        self.ready()?;
        // SAFETY: scalar length query followed by a checked initialized buffer.
        let len = unsafe { raw::incant_animation_v4_snapshot_len(self.epoch) };
        let len = self.snapshot_length(len, scratch.len(), abi::ANIMATION_V4_MAX_BYTES)?;
        let actual = unsafe {
            raw::incant_animation_v4_read_snapshot(
                self.epoch,
                scratch.as_mut_ptr() as usize as i32,
                len as i32,
            )
        };
        self.check(if usize::try_from(actual).ok() == Some(len) {
            Ok(())
        } else {
            Err(Error::Malformed)
        })?;
        self.check(crate::AnimationPoseSnapshot::parse(&scratch[..len]))
    }
    /// Copy the v3 start-of-callback playback snapshot; weighted trees require
    /// animation_poses instead. Queued controls stay private
    /// until the entire callback commits; the returned data borrows only scratch.
    pub fn animations<'a>(
        &mut self,
        scratch: &'a mut [u8],
    ) -> Result<crate::AnimationSnapshot<'a>> {
        self.ready()?;
        // SAFETY: scalar length query, then a checked initialized destination.
        let len = unsafe { raw::incant_animation_v3_snapshot_len(self.epoch) };
        let len = self.snapshot_length(len, scratch.len(), abi::ANIMATION_V3_MAX_BYTES)?;
        let actual = unsafe {
            raw::incant_animation_v3_read_snapshot(
                self.epoch,
                scratch.as_mut_ptr() as usize as i32,
                len as i32,
            )
        };
        if actual as usize != len {
            return self.check(Err(Error::Malformed));
        }
        self.check(crate::AnimationSnapshot::parse(&scratch[..len]))
    }
    /// Explicit v2 snapshot, which refuses parameter blends it cannot represent.
    pub fn transition_animations<'a>(
        &mut self,
        scratch: &'a mut [u8],
    ) -> Result<crate::AnimationSnapshot<'a>> {
        self.ready()?;
        // SAFETY: scalar length query followed by checked initialized scratch.
        let len = unsafe { raw::incant_animation_v2_snapshot_len(self.epoch) };
        let len = self.snapshot_length(len, scratch.len(), abi::ANIMATION_V2_MAX_BYTES)?;
        let actual = unsafe {
            raw::incant_animation_v2_read_snapshot(
                self.epoch,
                scratch.as_mut_ptr() as usize as i32,
                len as i32,
            )
        };
        if actual as usize != len {
            return self.check(Err(Error::Malformed));
        }
        self.check(crate::AnimationSnapshot::parse(&scratch[..len]))
    }
    /// Explicit legacy snapshot. The unchanged v1 host refuses when any active
    /// transition cannot be represented; an ignored refusal rejects this update.
    pub fn legacy_animations<'a>(
        &mut self,
        scratch: &'a mut [u8],
    ) -> Result<crate::AnimationSnapshot<'a>> {
        self.ready()?;
        // SAFETY: scalar length query followed by checked initialized scratch.
        let len = unsafe { raw::incant_animation_v1_snapshot_len(self.epoch) };
        let len = self.snapshot_length(len, scratch.len(), abi::ANIMATION_MAX_BYTES)?;
        let actual = unsafe {
            raw::incant_animation_v1_read_snapshot(
                self.epoch,
                scratch.as_mut_ptr() as usize as i32,
                len as i32,
            )
        };
        if actual as usize != len {
            return self.check(Err(Error::Malformed));
        }
        self.check(crate::AnimationSnapshot::parse(&scratch[..len]))
    }
    /// Queue one validated control on an existing authored animation declaration.
    /// Clip-specific time bounds and entity existence are checked at atomic commit.
    pub fn control_animation(
        &mut self,
        id: &EntityId,
        control: crate::AnimationControl,
    ) -> Result<()> {
        self.ready()?;
        let bytes = self.check(control.encode())?;
        // SAFETY: both fixed-size initialized records remain live for this copied
        // synchronous call; the host also checks epoch, ranges and every field.
        unsafe {
            raw::incant_animation_v2_control(self.epoch, ptr(id), bytes.as_ptr() as usize as i32);
        }
        Ok(())
    }
    /// Explicit old-host control path; crossfades require the v2 import.
    pub fn legacy_control_animation(
        &mut self,
        id: &EntityId,
        control: crate::AnimationControl,
    ) -> Result<()> {
        self.ready()?;
        if matches!(control, crate::AnimationControl::Crossfade { .. }) {
            return self.check(Err(Error::Malformed));
        }
        let bytes = self.check(control.encode())?;
        // SAFETY: fixed initialized records remain live during the copied call.
        unsafe {
            raw::incant_animation_v1_control(self.epoch, ptr(id), bytes.as_ptr() as usize as i32);
        }
        Ok(())
    }
    /// Copy canonical graph state and parameter rows at callback start. This
    /// snapshot has no native references and queued controls do not change it.
    pub fn animation_graphs<'a>(
        &mut self,
        scratch: &'a mut [u8],
    ) -> Result<crate::AnimationGraphSnapshot<'a>> {
        self.ready()?;
        // SAFETY: scalar length query followed by a checked initialized slice.
        let len = unsafe { raw::incant_animation_graph_v1_snapshot_len(self.epoch) };
        let len = self.snapshot_length(len, scratch.len(), abi::ANIMATION_GRAPH_MAX_BYTES)?;
        let actual = unsafe {
            raw::incant_animation_graph_v1_read_snapshot(
                self.epoch,
                scratch.as_mut_ptr() as usize as i32,
                len as i32,
            )
        };
        if actual as usize != len {
            return self.check(Err(Error::Malformed));
        }
        self.check(crate::AnimationGraphSnapshot::parse(&scratch[..len]))
    }
    /// Queue a graph parameter or state selection in the shared transaction.
    /// Authored key membership and parameter ranges are checked at commit.
    pub fn control_animation_graph(
        &mut self,
        id: &EntityId,
        control: crate::AnimationGraphControl,
    ) -> Result<()> {
        self.ready()?;
        let bytes = self.check(control.encode())?;
        // SAFETY: both fixed initialized records remain live for this copied call.
        unsafe {
            raw::incant_animation_graph_v1_control(
                self.epoch,
                ptr(id),
                bytes.as_ptr() as usize as i32,
            );
        }
        Ok(())
    }
    fn snapshot_length(&mut self, len: i32, capacity: usize, limit: usize) -> Result<usize> {
        let len = self.check(
            usize::try_from(len)
                .ok()
                .filter(|&n| n <= limit)
                .ok_or(Error::Bounds),
        )?;
        self.check(if len <= capacity {
            Ok(len)
        } else {
            Err(Error::Capacity)
        })
    }
}
fn ptr<T>(value: &T) -> i32 {
    value as *const T as usize as i32
}
fn optional<T>(value: Option<&T>) -> i32 {
    value.map_or(abi::OPTIONAL_NONE, ptr)
}

//! Foreign numeric views. The outer ECS scope, rooted buffers and undo values
//! share one owner so detachment always precedes Rust access or scope release.
use crate::ScriptError;
use incant_runtime::{NativeWorld, NumericColumns, TRANSFORM_STRIDE, VELOCITY_STRIDE};
use rquickjs::{Array, ArrayBuffer, Ctx, Function, Object, TypedArray, Value, qjs};
use std::{mem::size_of, ptr};

mod ids;
mod scratch;
pub(crate) use ids::IdCache;
pub(crate) use scratch::Scratch;

#[cfg(all(test, feature = "compiler"))]
mod tests;

const MAX_LOAN_BYTES: usize = 32 * 1024 * 1024;
struct Buffer {
    pointer: *mut f64,
    len: usize,
}
impl Buffer {
    fn byte_len(&self) -> usize {
        self.len * size_of::<f64>()
    }
}
struct Chunk {
    transforms: Buffer,
    velocities: Option<Buffer>,
}
pub(crate) struct Loan<'js, 'scope, 'world, 'cache> {
    // Retain the exclusive outer scope. No world operation or second visit can
    // occur while these native pointers or foreign buffer objects remain live.
    _columns: &'scope mut NumericColumns<'world>,
    chunks: Vec<Chunk>,
    saved: Option<scratch::Lease<'cache>>,
    roots: Vec<ArrayBuffer<'js>>,
    committed: bool,
}

impl<'js, 'scope, 'world, 'cache> Loan<'js, 'scope, 'world, 'cache> {
    pub fn new(
        columns: &'scope mut NumericColumns<'world>,
        scratch: &'cache mut Scratch,
    ) -> Result<Self, ScriptError> {
        Self::capture(columns, scratch, MAX_LOAN_BYTES)
    }

    fn capture(
        columns: &'scope mut NumericColumns<'world>,
        scratch: &'cache mut Scratch,
        limit: usize,
    ) -> Result<Self, ScriptError> {
        // Tests can exercise a partial constructor refusal on small native
        // tables; no caller can raise the production numeric-copy ceiling.
        let limit = limit.min(MAX_LOAN_BYTES);
        let mut saved = scratch.lease();
        let mut chunks = Vec::new();
        let mut ranges = Vec::new();
        let mut total = 0usize;
        let mut buffer = |values: &mut [f64]| -> Result<Buffer, ScriptError> {
            let bytes = values
                .len()
                .checked_mul(size_of::<f64>())
                .ok_or(ScriptError::InputLimit)?;
            total = total
                .checked_add(bytes)
                .filter(|n| *n <= limit)
                .ok_or(ScriptError::InputLimit)?;
            let pointer = values.as_mut_ptr();
            let end = pointer
                .addr()
                .checked_add(bytes)
                .ok_or(ScriptError::InputLimit)?;
            ranges.push((pointer.addr(), end));
            Ok(Buffer {
                pointer,
                len: values.len(),
            })
        };
        columns.try_for_each_chunk(|mut chunk| {
            // Admit all numeric bytes before copying this chunk. IDs remain
            // outside the historical 32 MiB numeric-copy limit, but both caches
            // retain only exact active row sizes and evict unused tails.
            let transforms = buffer(chunk.transforms)?;
            let velocities = chunk
                .velocities
                .as_deref_mut()
                .map(&mut buffer)
                .transpose()?;
            saved.capture(chunk.ids, chunk.transforms, chunk.velocities.as_deref());
            chunks.push(Chunk {
                transforms,
                velocities,
            });
            Ok::<_, ScriptError>(())
        })?;
        saved.chunks.truncate(chunks.len());
        // Only one external buffer may cover each native byte. Distinct objects
        // with overlapping backing storage violate some JS copy assumptions.
        ranges.sort_unstable();
        if ranges.windows(2).any(|pair| pair[0].1 > pair[1].0) {
            return Err(ScriptError::Execution);
        }
        Ok(Self {
            _columns: columns,
            chunks,
            saved: Some(saved),
            roots: Vec::new(),
            committed: false,
        })
    }

    pub fn expose(
        &mut self,
        ctx: &Ctx<'js>,
        freeze: &Function<'js>,
        ids: &mut IdCache,
    ) -> rquickjs::Result<Array<'js>> {
        ids.truncate(self.chunks.len());
        let chunks = Array::new(ctx.clone())?;
        for (index, chunk) in self.chunks.iter().enumerate() {
            let object = Object::new(ctx.clone())?;
            let saved = &self.saved.as_ref().expect("active loan").chunks[index];
            object.set("ids", ids.get(ctx, freeze, index, &saved.ids)?)?;
            object.set("transformStride", TRANSFORM_STRIDE)?;
            object.set("velocityStride", VELOCITY_STRIDE)?;
            object.set("transforms", view(ctx, &chunk.transforms, &mut self.roots)?)?;
            object.set(
                "velocities",
                chunk
                    .velocities
                    .as_ref()
                    .map(|buffer| view(ctx, buffer, &mut self.roots))
                    .transpose()?,
            )?;
            chunks.set(index, object)?;
        }
        Ok(chunks)
    }

    pub fn revoke(&mut self) {
        for buffer in &mut self.roots {
            buffer.detach();
        }
    }
    /// Called only after revocation: safe native reads cannot overlap JS views.
    /// Failure leaves this loan uncommitted so Drop restores the whole write set.
    pub fn validate_numeric(&mut self) -> Result<(), ScriptError> {
        self.revoke();
        self._columns.validate().map_err(ScriptError::Structural)
    }

    pub fn finish(mut self) -> Undo<'cache> {
        self.revoke();
        self.committed = true;
        Undo(self.saved.take().expect("active loan"))
    }
}

/// Owned numeric data and identity/layout metadata only. No table pointer or JS
/// value survives the exclusive loan scope in this record.
pub(crate) struct Undo<'cache>(scratch::Lease<'cache>);

impl Undo<'_> {
    pub fn apply(
        self,
        world: &mut NativeWorld,
        commands: &mut incant_runtime::FrameCommands,
    ) -> Result<(), ScriptError> {
        if let Err(error) = world.apply(commands) {
            // NativeWorld::apply rejects before changing ECS storage. Validate
            // the entire saved layout anyway, before writing any restored byte.
            world.with_numeric_columns(|columns| {
                let mut saved = self.0.chunks.iter();
                columns.for_each_chunk(|chunk| {
                    let old = saved.next().expect("rejected batch retains table count");
                    assert_eq!(
                        chunk.ids, &*old.ids,
                        "rejected batch retains row identities"
                    );
                    assert_eq!(chunk.transforms.len(), old.transforms.len());
                    assert_eq!(
                        chunk.velocities.as_ref().map(|v| v.len()),
                        old.velocities.as_ref().map(|v| v.len())
                    );
                });
                assert!(saved.next().is_none(), "rejected batch retains table count");
                let mut saved = self.0.chunks.iter();
                columns.for_each_chunk(|chunk| {
                    let old = saved.next().expect("validated layout");
                    chunk.transforms.copy_from_slice(&old.transforms);
                    if let (Some(current), Some(old)) = (chunk.velocities, &old.velocities) {
                        current.copy_from_slice(old);
                    }
                });
            });
            return Err(ScriptError::Structural(error));
        }
        Ok(())
    }
}

impl Drop for Loan<'_, '_, '_, '_> {
    fn drop(&mut self) {
        self.revoke();
        if !self.committed {
            // Numeric validation may have created new mutable slices. Do not
            // dereference raw pointers retained from the initial visit: restore
            // through fresh safe borrows after every foreign view is detached.
            // Verify the entire layout before restoring any prefix.
            let chunks = &self.saved.as_ref().expect("active loan").chunks;
            let mut saved = chunks.iter();
            self._columns.for_each_chunk(|chunk| {
                let old = saved.next().expect("exclusive loan retains table count");
                assert_eq!(
                    chunk.ids, &*old.ids,
                    "exclusive loan retains row identities"
                );
                assert_eq!(chunk.transforms.len(), old.transforms.len());
                assert_eq!(
                    chunk.velocities.as_ref().map(|v| v.len()),
                    old.velocities.as_ref().map(|v| v.len())
                );
            });
            assert!(saved.next().is_none(), "exclusive loan retains table count");
            let mut saved = chunks.iter();
            self._columns.for_each_chunk(|chunk| {
                let old = saved.next().expect("validated layout");
                chunk.transforms.copy_from_slice(&old.transforms);
                if let (Some(current), Some(old)) = (chunk.velocities, &old.velocities) {
                    current.copy_from_slice(old);
                }
            });
        }
    }
}

fn view<'js>(
    ctx: &Ctx<'js>,
    buffer: &Buffer,
    roots: &mut Vec<ArrayBuffer<'js>>,
) -> rquickjs::Result<TypedArray<'js, f64>> {
    // SAFETY: called only by Loan, which holds the exclusive outer ECS scope.
    // Numeric storage is aligned, initialized, disjoint and fixed until Drop.
    // No Rust component references are read/written during JS execution. The
    // fresh VM has disabled transfer/realm entry points before any project code.
    // The null realloc callback makes the fixed buffer non-owning. Every root
    // is detached before the scope ends, including exception/allocation/unwind.
    let raw = unsafe {
        qjs::JS_NewArrayBuffer(
            ctx.as_raw().as_ptr(),
            buffer.pointer.cast(),
            buffer.byte_len() as _,
            0,
            None,
            ptr::null_mut(),
            false,
        )
    };
    // SAFETY: this owned result belongs to exactly ctx.
    let value = unsafe { Value::from_raw(ctx.clone(), raw) };
    if value.is_exception() {
        return Err(rquickjs::Error::Exception);
    }
    let buffer = ArrayBuffer::from_value(value).ok_or(rquickjs::Error::Unknown)?;
    roots.push(buffer.clone()); // root before the next fallible VM operation
    TypedArray::from_arraybuffer(buffer)
}

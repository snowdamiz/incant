//! Foreign numeric views. The outer ECS scope, rooted buffers and undo values
//! share one owner so detachment always precedes Rust access or scope release.
use crate::ScriptError;
use incant_runtime::{NumericColumns, TRANSFORM_STRIDE, VELOCITY_STRIDE};
use rquickjs::{Array, ArrayBuffer, Ctx, Object, TypedArray, Value, qjs};
use std::{mem::size_of, ptr};

const MAX_LOAN_BYTES: usize = 32 * 1024 * 1024;
struct Buffer {
    pointer: *mut f64,
    undo: Vec<f64>,
}
impl Buffer {
    fn byte_len(&self) -> usize {
        self.undo.len() * size_of::<f64>()
    }
}
struct Chunk {
    ids: Vec<String>,
    transforms: Buffer,
    velocities: Option<Buffer>,
}
pub(crate) struct Loan<'js, 'scope, 'world> {
    // Retain the exclusive outer scope. No world operation or second visit can
    // occur while these native pointers or foreign buffer objects remain live.
    _columns: &'scope mut NumericColumns<'world>,
    chunks: Vec<Chunk>,
    roots: Vec<ArrayBuffer<'js>>,
    committed: bool,
}

impl<'js, 'scope, 'world> Loan<'js, 'scope, 'world> {
    pub fn new(columns: &'scope mut NumericColumns<'world>) -> Result<Self, ScriptError> {
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
                .filter(|n| *n <= MAX_LOAN_BYTES)
                .ok_or(ScriptError::InputLimit)?;
            let pointer = values.as_mut_ptr();
            let end = pointer
                .addr()
                .checked_add(bytes)
                .ok_or(ScriptError::InputLimit)?;
            ranges.push((pointer.addr(), end));
            Ok(Buffer {
                pointer,
                undo: values.to_vec(),
            })
        };
        columns.try_for_each_chunk(|chunk| {
            chunks.push(Chunk {
                ids: chunk
                    .ids
                    .iter()
                    .map(|id| ulid::Ulid::from_bytes(id.0).to_string())
                    .collect(),
                transforms: buffer(chunk.transforms)?,
                velocities: chunk.velocities.map(&mut buffer).transpose()?,
            });
            Ok::<_, ScriptError>(())
        })?;
        // Only one external buffer may cover each native byte. Distinct objects
        // with overlapping backing storage violate some JS copy assumptions.
        ranges.sort_unstable();
        if ranges.windows(2).any(|pair| pair[0].1 > pair[1].0) {
            return Err(ScriptError::Execution);
        }
        Ok(Self {
            _columns: columns,
            chunks,
            roots: Vec::new(),
            committed: false,
        })
    }

    pub fn expose(&mut self, ctx: &Ctx<'js>) -> rquickjs::Result<Array<'js>> {
        let chunks = Array::new(ctx.clone())?;
        for (index, chunk) in self.chunks.iter().enumerate() {
            let object = Object::new(ctx.clone())?;
            object.set("ids", chunk.ids.clone())?;
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
    pub fn commit(&mut self) {
        self.revoke();
        self.committed = true;
    }
}

impl Drop for Loan<'_, '_, '_> {
    fn drop(&mut self) {
        self.revoke();
        if !self.committed {
            for chunk in &self.chunks {
                for buffer in std::iter::once(&chunk.transforms).chain(chunk.velocities.as_ref()) {
                    // SAFETY: all foreign views were detached first. The outer
                    // exclusive scope still keeps these initialized allocations
                    // fixed. Undo data has the exact original length, is disjoint
                    // from ECS storage, and contains only initialized f64 values.
                    unsafe {
                        ptr::copy_nonoverlapping(
                            buffer.undo.as_ptr(),
                            buffer.pointer,
                            buffer.undo.len(),
                        );
                    }
                }
            }
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

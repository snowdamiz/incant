use crate::{
    abi::{self, State},
    bounds,
};
use incant_runtime::{NativeWorld, RuntimeError};
use incant_wasm_runtime::{BoundaryError, Capability};
use thiserror::Error;
use wasmtime::{Config, Engine, Linker, Module, Store, StoreLimitsBuilder, TypedFunc};

#[derive(Debug, Error)]
pub enum ModuleError {
    #[error("development Wasm ABI: {0}")]
    Abi(&'static str),
    #[error("development Wasm quota: {0}")]
    Limit(&'static str),
    #[error("development Wasm rejected or trapped: {0}")]
    Engine(#[from] wasmtime::Error),
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
    #[error("development module failed; construct or reload a new instance")]
    Failed,
}
/// Desktop development conformance host over bounded typed transfers. No WASI,
/// serialized native cache, raw native pointer, JSON projection or world clone.
/// `tick` runs only the module phase; the owner schedules native simulation.
/// Traps/invalid numeric or structural writes leave component values unchanged,
/// discard commands and permanently fail this instance. Guest memory is not
/// rolled back. Reload explicitly replaces the instance, preserving the world.
/// This copied ABI is not a zero-copy or phone-performance/AOT claim.
pub struct DevModuleHost {
    store: Store<State>,
    update: TypedFunc<(i64, f64), ()>,
}
impl DevModuleHost {
    /// Validate raw core Wasm then compile it locally. Never deserialize a
    /// project-supplied Wasmtime executable cache. Startup code has fuel but no
    /// active ECS capability. Compilation has size/declaration caps, not a hard
    /// wall-time or total-process-memory deadline.
    pub fn new(bytes: &[u8]) -> Result<Self, ModuleError> {
        bounds::preflight(bytes)?;
        let mut config = Config::new();
        config
            .consume_fuel(true)
            .max_wasm_stack(256 * 1024)
            .wasm_features(
                wasmparser::WasmFeatures::THREADS
                    | wasmparser::WasmFeatures::SHARED_EVERYTHING_THREADS
                    | wasmparser::WasmFeatures::MEMORY64
                    | wasmparser::WasmFeatures::MULTI_MEMORY
                    | wasmparser::WasmFeatures::RELAXED_SIMD
                    | wasmparser::WasmFeatures::SIMD
                    | wasmparser::WasmFeatures::FUNCTION_REFERENCES
                    | wasmparser::WasmFeatures::GC
                    | wasmparser::WasmFeatures::TAIL_CALL
                    | wasmparser::WasmFeatures::STACK_SWITCHING
                    | wasmparser::WasmFeatures::EXCEPTIONS
                    | wasmparser::WasmFeatures::LEGACY_EXCEPTIONS
                    | wasmparser::WasmFeatures::COMPACT_IMPORTS,
                false,
            )
            .cranelift_nan_canonicalization(true)
            .wasm_backtrace_max_frames(None)
            .memory_reservation(bounds::MEMORY as u64)
            .memory_reservation_for_growth(0);
        let engine = Engine::new(&config)?;
        let module = Module::from_binary(&engine, bytes)?;
        let mut linker = Linker::new(&engine);
        abi::install(&mut linker)?;
        // Linker rejects unknown imports and mismatched signatures. Imported
        // memories/tables/globals are never supplied by this host.
        let limits = StoreLimitsBuilder::new()
            .memory_size(bounds::MEMORY)
            .memories(1)
            .tables(1)
            .table_elements(4096)
            .instances(1)
            .trap_on_grow_failure(true)
            .build();
        let mut store = Store::new(
            &engine,
            State {
                limits,
                capability: Capability::default(),
            },
        );
        store.limiter(|state| &mut state.limits);
        store.set_fuel(bounds::START_FUEL)?;
        let instance = linker.instantiate(&mut store, &module)?;
        let version = instance.get_typed_func::<(), i32>(&mut store, "incant_abi")?;
        if version.call(&mut store, ())? != incant_wasm_abi::ABI_VERSION {
            return Err(ModuleError::Abi("unsupported incant_abi"));
        }
        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or(ModuleError::Abi("missing exported memory"))?;
        if memory.ty(&store).is_64() || memory.ty(&store).is_shared() {
            return Err(ModuleError::Abi("expected unshared 32-bit memory"));
        }
        let update = instance.get_typed_func::<(i64, f64), ()>(&mut store, "update")?;
        Ok(Self { store, update })
    }
    pub fn is_failed(&self) -> bool {
        self.store.data().capability.is_failed()
    }
    /// Bounded committed prefix; a failed instance exposes no live diagnostics.
    pub fn logs(&self) -> Result<&incant_wasm_runtime::NativeLogs, ModuleError> {
        Ok(self.store.data().capability.logs()?)
    }
    /// A failed candidate leaves the current instance untouched. Successful
    /// replacement resets guest globals/memory; no VM-state migration is claimed.
    pub fn reload(&mut self, bytes: &[u8]) -> Result<(), ModuleError> {
        let mut candidate = Self::new(bytes)?;
        candidate.store.data_mut().capability = self.store.data_mut().capability.replacement();
        *self = candidate;
        Ok(())
    }
    pub fn tick(&mut self, world: &mut NativeWorld, dt: f64) -> Result<(), ModuleError> {
        let epoch = self.store.data_mut().capability.begin(world, dt)?;
        self.run(world, dt, epoch)
    }
    /// Run against an engine-validated snapshot through `incant_input_v1`.
    /// The owner must advance this input and native simulation on the same fixed
    /// timeline. This method does not advance either clock. `tick` intentionally
    /// supplies no input authority; imports there fail explicitly.
    pub fn tick_with_input(
        &mut self,
        world: &mut NativeWorld,
        dt: f64,
        input: &incant_input::InputRuntime,
    ) -> Result<(), ModuleError> {
        let epoch = self
            .store
            .data_mut()
            .capability
            .begin_with_input(world, dt, input)?;
        self.run(world, dt, epoch)
    }
    fn run(&mut self, world: &mut NativeWorld, dt: f64, epoch: i64) -> Result<(), ModuleError> {
        let result = (|| {
            self.store.set_fuel(bounds::TICK_FUEL)?;
            self.update.call(&mut self.store, (epoch, dt))?;
            self.store.data_mut().capability.finish(world)?;
            Ok(())
        })();
        if result.is_err() {
            self.store.data_mut().capability.abort();
        }
        result
    }
}
impl From<BoundaryError> for ModuleError {
    fn from(error: BoundaryError) -> Self {
        match error {
            BoundaryError::Abi(message) => Self::Abi(message),
            BoundaryError::Limit(message) => Self::Limit(message),
            BoundaryError::Runtime(error) => Self::Runtime(error),
            BoundaryError::Failed => Self::Failed,
        }
    }
}

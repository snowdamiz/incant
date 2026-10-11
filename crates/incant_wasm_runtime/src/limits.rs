//! Shared fixed resource limits. Not a total-process memory or wall-time limit.
pub use incant_wasm_abi::{CALLS, CHUNKS, COLUMNS, COMMANDS, MEMORY, TRANSFERS};
pub const START_FUEL: u64 = 1_000_000;
pub const TICK_FUEL: u64 = 10_000_000;

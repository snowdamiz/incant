//! Shared fixed resource limits. Not a total-process memory or wall-time limit.
pub const MEMORY: usize = 32 * 1024 * 1024;
pub const COLUMNS: usize = 32 * 1024 * 1024;
pub const TRANSFERS: usize = 128 * 1024 * 1024;
pub const CALLS: usize = 65_536;
pub const COMMANDS: usize = 1024;
pub const CHUNKS: usize = 4096;
pub const START_FUEL: u64 = 1_000_000;
pub const TICK_FUEL: u64 = 10_000_000;

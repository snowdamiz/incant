//! Desktop CLI for the same packaged runtime used by embedded platform hosts.
#[cfg(feature = "profile")]
#[global_allocator]
static ALLOCATOR: incant_profile::CountingAllocator<std::alloc::System> =
    incant_profile::CountingAllocator(std::alloc::System);
fn main() -> Result<(), Box<dyn std::error::Error>> {
    incant_game_player::run_cli(std::env::args().skip(1).collect(), |report| {
        println!("{report}");
    })
}

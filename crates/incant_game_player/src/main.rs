//! Desktop CLI for the same packaged runtime used by embedded platform hosts.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    incant_game_player::run(std::env::args().skip(1).collect(), |report| {
        println!("{report}");
    })
}

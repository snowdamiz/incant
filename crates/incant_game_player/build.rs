fn main() -> Result<(), Box<dyn std::error::Error>> {
    incant_cook::embed_script("fixtures/game.js")
}

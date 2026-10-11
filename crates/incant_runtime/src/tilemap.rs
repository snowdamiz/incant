//! Binary identities and retained local map preparation below authoring.
use crate::StableId;
pub type NativeTilemap = incant_2d::Tilemap<StableId>;
pub type NativeTilemapLayer = incant_2d::TilemapLayer<StableId>;
pub type PreparedTilemap = incant_2d::tilemap::PreparedTilemap<StableId>;
pub const MAX_TILEMAP_ENTITIES: usize = 1024;
pub const MAX_SCENE_TILE_SLOTS: usize = 1_048_576;

#[derive(bevy_ecs::prelude::Component)]
pub(crate) struct Prepared(pub PreparedTilemap);

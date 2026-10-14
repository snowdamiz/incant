//! Fixed fixture types for the opt-in real editor workflow, never project policy.
use incant_runtime::VisualAssetKind;
use serde_json::json;

#[derive(Clone, Copy)]
pub(in crate::play::probe) enum Kind {
    Texture,
    Model,
}
impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Texture => "texture",
            Self::Model => "model",
        }
    }
    pub fn source(self) -> &'static str {
        match self {
            Self::Texture => "refresh.png",
            Self::Model => "refresh.gltf",
        }
    }
    pub fn replacement(self) -> &'static str {
        match self {
            Self::Texture => "refresh-blue.png",
            Self::Model => "refresh-replacement.gltf",
        }
    }
    pub fn fixture(self) -> &'static str {
        match self {
            Self::Texture => "texture-workflow.fixture.json",
            Self::Model => "model-workflow.fixture.json",
        }
    }
    pub fn format(self) -> &'static str {
        match self {
            Self::Texture => "incant-native-texture-workflow",
            Self::Model => "incant-native-model-workflow",
        }
    }
    pub fn visual_kind(self) -> VisualAssetKind {
        match self {
            Self::Texture => VisualAssetKind::Texture,
            Self::Model => VisualAssetKind::Model,
        }
    }
    pub fn texture_usage(self) -> Option<incant_assets::TextureUsage> {
        match self {
            Self::Texture => Some(incant_assets::TextureUsage::Color),
            Self::Model => None,
        }
    }
    pub fn stage(self, index: usize) -> &'static str {
        let stages = match self {
            Self::Texture => [
                "manual_import_red",
                "watched_blue",
                "undo_red",
                "redo_blue",
                "malformed_source_retained_blue",
                "source_recovered_blue",
            ],
            Self::Model => [
                "manual_import_original",
                "watched_replacement",
                "undo_original",
                "redo_replacement",
                "malformed_source_retained_replacement",
                "source_recovered_replacement",
            ],
        };
        stages[index]
    }
    pub fn entity_command(
        self,
        scene: &str,
        id: &str,
        asset: &str,
    ) -> Result<incant_cmd::Command, String> {
        let mut components =
            json!({"Transform":{"translation":[0,0,-2],"rotation":[0,0,0,1],"scale":[1,1,1]}});
        match self {
            Self::Texture => {
                components["SpriteRenderer"] = json!({"texture":asset,"region":[0,0,1,1],"size":[1,1],"pivot":[0.5,0.5],"tint":[1,1,1,1],"sampling":"nearest","mode":{"kind":"simple"}})
            }
            Self::Model => {
                components["MeshRenderer"] =
                    json!({"mesh":asset,"materials":[],"cast_shadows":true})
            }
        }
        serde_json::from_value(json!({"op":"create_entity","scene_id":scene,"entity":{
            "id":id,"name":"Refresh asset","parent":null,"provenance":null,"components":components
        }}))
        .map_err(|e| e.to_string())
    }
}

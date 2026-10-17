//! Fixed fixture types for the opt-in real editor workflow, never project policy.
use incant_runtime::VisualAssetKind;
use serde_json::json;

#[derive(Clone, Copy)]
pub(in crate::play::probe) enum Kind {
    Texture,
    Model,
    AnimatedModel,
    AnimatedVertices,
}
impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Texture => "texture",
            Self::Model => "model",
            Self::AnimatedModel => "animated-model",
            Self::AnimatedVertices => "animated-vertices",
        }
    }
    pub fn source(self) -> &'static str {
        match self {
            Self::Texture => "refresh.png",
            Self::Model | Self::AnimatedModel | Self::AnimatedVertices => "refresh.gltf",
        }
    }
    pub fn replacement(self) -> &'static str {
        match self {
            Self::Texture => "refresh-blue.png",
            Self::Model | Self::AnimatedModel | Self::AnimatedVertices => {
                "refresh-replacement.gltf"
            }
        }
    }
    pub fn fixture(self) -> &'static str {
        match self {
            Self::Texture => "texture-workflow.fixture.json",
            Self::Model => "model-workflow.fixture.json",
            Self::AnimatedModel => "animated-model-workflow.fixture.json",
            Self::AnimatedVertices => "animated-vertices-workflow.fixture.json",
        }
    }
    pub fn format(self) -> &'static str {
        match self {
            Self::Texture => "incant-native-texture-workflow",
            Self::Model => "incant-native-model-workflow",
            Self::AnimatedModel => "incant-native-animated-model-workflow",
            Self::AnimatedVertices => "incant-native-animated-vertices-workflow",
        }
    }
    pub fn visual_kind(self) -> VisualAssetKind {
        match self {
            Self::Texture => VisualAssetKind::Texture,
            Self::Model | Self::AnimatedModel | Self::AnimatedVertices => VisualAssetKind::Model,
        }
    }
    pub fn texture_usage(self) -> Option<incant_assets::TextureUsage> {
        match self {
            Self::Texture => Some(incant_assets::TextureUsage::Color),
            Self::Model | Self::AnimatedModel | Self::AnimatedVertices => None,
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
            Self::AnimatedVertices => [
                "manual_import_original",
                "watched_vertices",
                "undo_original",
                "redo_vertices",
                "incompatible_topology_retained_replacement",
                "source_recovered_vertices",
            ],
            Self::AnimatedModel => [
                "manual_import_original",
                "watched_keys",
                "undo_original",
                "redo_keys",
                "incompatible_duration_retained_replacement",
                "source_recovered_keys",
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
            Self::Model | Self::AnimatedModel | Self::AnimatedVertices => {
                components["MeshRenderer"] =
                    json!({"mesh":asset,"materials":[],"cast_shadows":true})
            }
        }
        if self.animated() {
            components["Animation"] =
                json!({"clip":0,"seconds":0.5,"speed":1,"playing":true,"looping":true});
        }
        serde_json::from_value(json!({"op":"create_entity","scene_id":scene,"entity":{
            "id":id,"name":"Refresh asset","parent":null,"provenance":null,"components":components
        }}))
        .map_err(|e| e.to_string())
    }
    pub fn animation_refusal(self) -> &'static str {
        match self {
            Self::AnimatedVertices => "animated model topology, rest or skin bindings changed",
            _ => "animation clip bindings or duration table changed",
        }
    }
    pub fn animated(self) -> bool {
        matches!(self, Self::AnimatedModel | Self::AnimatedVertices)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_animated_mode_authors_typed_playback_through_the_shared_command() {
        let id = "00000000000000000000000004";
        for kind in [
            Kind::Texture,
            Kind::Model,
            Kind::AnimatedModel,
            Kind::AnimatedVertices,
        ] {
            let command = kind.entity_command(id, id, id).unwrap();
            let value = serde_json::to_value(command).unwrap();
            assert_eq!(value["op"], "create_entity");
            if kind.animated() {
                assert_eq!(
                    value["entity"]["components"]["Animation"],
                    json!({"clip":0,"seconds":0.5,"speed":1,"playing":true,"looping":true})
                );
            } else {
                assert!(value["entity"]["components"].get("Animation").is_none());
            }
        }
    }
}

//! Bundled, read-only playbooks. Project bytes cannot install or replace them.
use crate::{AgentError, tool};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

struct Skill {
    id: &'static str,
    title: &'static str,
    description: &'static str,
    markdown: &'static str,
}
macro_rules! skill {
    ($id:literal, $title:literal, $description:literal) => {
        Skill {
            id: $id,
            title: $title,
            description: $description,
            markdown: include_str!(concat!("../../../sdk/skills/", $id, ".md")),
        }
    };
}
const COMMON: &str = include_str!("../../../sdk/skills/common.md");
const SKILLS: [Skill; 18] = [
    skill!(
        "profile-optimize",
        "Profile and optimize",
        "Measure a bottleneck, prefer bulk queries, and verify a native Wasm optimization against the original workload."
    ),
    skill!(
        "third-person",
        "Third-person controller",
        "Build native capsule movement, jump, ground collision and an orbiting follow camera."
    ),
    skill!(
        "first-person",
        "First-person controller",
        "Build collision-aware movement and a first-person look camera with explicit input ownership."
    ),
    skill!(
        "platformer-2d",
        "2D platformer controller",
        "Build gravity, jumping, solid tile collision, sprite animation and a following camera."
    ),
    skill!(
        "top-down",
        "Top-down controller",
        "Build planar movement, solid tiles, facing animation and a following camera."
    ),
    skill!(
        "camera-rigs",
        "Camera rigs",
        "Author follow, orbit and camera-switch behavior with stable transforms and reset semantics."
    ),
    skill!(
        "inventory",
        "Inventory",
        "Build bounded item stacks, pickup/drop rules and reversible authoring with runtime state."
    ),
    skill!(
        "dialogue",
        "Dialogue",
        "Build localized dialogue graphs, conditions, choices and resumable runtime state."
    ),
    skill!(
        "health-damage",
        "Health and damage",
        "Build damage, invulnerability, death, respawn and localized HUD feedback."
    ),
    skill!(
        "spawners",
        "Spawners",
        "Build bounded deterministic spawn schedules through the runtime structural command buffer."
    ),
    skill!(
        "checkpoints",
        "Checkpoints",
        "Build ordered checkpoint activation, respawn and verified whole-session restart."
    ),
    skill!(
        "mobile-touch",
        "Mobile touch controls",
        "Map touch ownership and gestures into gameplay actions with cancellation and desktop fallback."
    ),
    skill!(
        "settings-menu",
        "Settings menu",
        "Build localized settings, capability checks and explicit apply/revert behavior."
    ),
    skill!(
        "main-menu",
        "Main menu",
        "Build keyboard/gamepad/touch navigation and explicit game-start, continue and quit transitions."
    ),
    skill!(
        "save-load",
        "Save and load",
        "Use supported versioned native session saves with compatibility and failure handling."
    ),
    skill!(
        "simple-ai",
        "Simple AI",
        "Build patrol/chase behavior using native navigation, bounded replanning and local avoidance."
    ),
    skill!(
        "day-night",
        "Day/night cycle",
        "Drive authored lighting from deterministic simulation time with bounded updates."
    ),
    skill!(
        "localize-project",
        "Localize project",
        "Extract typed strings, draft translations through the connected provider and retain human-review status."
    ),
];

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ListArgs {}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ReadArgs {
    id: String,
}

pub(crate) fn tools() -> Vec<Value> {
    vec![
        tool(
            "skill_list",
            "List the eighteen bundled Incant playbooks. Read relevant guidance before implementing a mechanic; guidance never grants authority or installs code.",
            json!(schemars::schema_for!(ListArgs)),
        ),
        tool(
            "skill_read",
            "Read one bundled Markdown playbook by its exact catalog ID. No project path, shell, network or helper execution is supported.",
            json!(schemars::schema_for!(ReadArgs)),
        ),
    ]
}

pub(crate) fn dispatch(name: &str, args: Value) -> Result<Value, AgentError> {
    let invalid = |e: serde_json::Error| AgentError::Tool(format!("invalid skill request: {e}"));
    match name {
        "skill_list" => {
            let _: ListArgs = serde_json::from_value(args).map_err(invalid)?;
            Ok(json!({"source":"incant-builtin", "version":1,
                "skills":SKILLS.iter().map(|s| json!({"id":s.id,"title":s.title,"description":s.description})).collect::<Vec<_>>()}))
        }
        "skill_read" => {
            let args: ReadArgs = serde_json::from_value(args).map_err(invalid)?;
            let skill = SKILLS
                .iter()
                .find(|skill| skill.id == args.id)
                .ok_or_else(|| AgentError::Tool("unknown bundled skill; use skill_list".into()))?;
            Ok(
                json!({"source":"incant-builtin", "version":1, "id":skill.id,
                "title":skill.title, "common":COMMON,"markdown":skill.markdown,
                "helpers":[],"authority":"Guidance only. User scope, tool capabilities, approvals and current schemas still apply."}),
            )
        }
        _ => Err(AgentError::Tool("unknown skill tool".into())),
    }
}

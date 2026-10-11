use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const DEFAULT_AGENT_TOKEN_CAP: u64 = 1_000_000;
/// Largest integer that JSON consumers can represent exactly in JavaScript.
pub const MAX_AGENT_TOKEN_CAP: u64 = 9_007_199_254_740_991;

/// User-authored token allowance for this project. Consumption is stored outside
/// the project; zero disables new agent turns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AgentBudgetSettings {
    #[schemars(range(min = 0, max = MAX_AGENT_TOKEN_CAP))]
    pub max_tokens: u64,
}

impl Default for AgentBudgetSettings {
    fn default() -> Self {
        Self {
            max_tokens: DEFAULT_AGENT_TOKEN_CAP,
        }
    }
}

impl AgentBudgetSettings {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

//! Optional editor observations, never project authority or provider instructions.
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EditorPanel {
    Viewport,
    Hierarchy,
    Assets,
    Inspector,
    Agent,
    Problems,
    Console,
    History,
}

/// An observation at submission time. No labels, host paths, pixels, account
/// data, or arbitrary instructions cross this boundary. Recheck project state
/// through tools before editing; visibility does not authorize a mutation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EditorViewContext {
    pub project_id: String,
    pub open_panels: Vec<EditorPanel>,
}

impl EditorViewContext {
    pub fn is_valid(&self) -> bool {
        self.project_id.len() == 26
            && self.project_id.as_bytes()[0] <= b'7'
            && self
                .project_id
                .bytes()
                .all(|c| b"0123456789ABCDEFGHJKMNPQRSTVWXYZ".contains(&c))
            && self.open_panels.len() <= 8
            && self.open_panels.iter().collect::<BTreeSet<_>>().len() == self.open_panels.len()
    }
}

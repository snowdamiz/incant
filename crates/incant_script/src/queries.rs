//! Read-only engine capabilities share the script deadline and one weighted budget.
use crate::{ScriptError, ScriptHost};
use incant_core::{CharacterMovement, CharacterQuery, PhysicsError, RayHit, RayQuery};
use serde::{Serialize, de::DeserializeOwned};
use std::sync::{Arc, atomic::Ordering};

type Query<Q, R, E = PhysicsError> = Arc<dyn Fn(Q) -> Result<R, E> + Send + Sync>;
pub(super) type Navigator = Query<
    incant_core::NavigationQuery,
    Option<incant_core::NavigationPath>,
    incant_core::NavigationError,
>;
pub(super) type GridNavigator = Query<
    incant_core::GridNavigationQuery,
    Option<incant_core::GridPath>,
    incant_core::NavigationError,
>;
pub(super) type Raycaster = Query<RayQuery, Option<RayHit>>;
pub(super) type CharacterMover = Query<CharacterQuery, CharacterMovement>;
pub(super) type Steerer = Query<
    incant_core::SteeringQuery,
    Vec<incant_core::SteeringVelocity>,
    incant_core::NavigationError,
>;

struct Budget {
    cost: usize,
    bytes: usize,
}

impl ScriptHost {
    pub(super) fn install_queries(&mut self) -> Result<(), ScriptError> {
        self.install_query(
            "__incantRaycast",
            self.raycaster.clone(),
            Budget {
                cost: 1,
                bytes: 4096,
            },
            "hit",
            "physics",
        )?;
        self.install_query(
            "__incantCharacterMotion",
            self.character_mover.clone(),
            Budget {
                cost: 16,
                bytes: 4096,
            },
            "movement",
            "physics",
        )?;
        self.install_query(
            "__incantFindPath",
            self.navigator.clone(),
            Budget {
                cost: 64,
                bytes: 4096,
            },
            "path",
            "navigation",
        )?;
        self.install_query(
            "__incantFindGridPath",
            self.grid_navigator.clone(),
            Budget {
                cost: 64,
                bytes: 4096,
            },
            "path",
            "grid navigation",
        )?;
        self.install_query(
            "__incantSteerAgents",
            self.steerer.clone(),
            Budget {
                cost: 128,
                bytes: 65536,
            },
            "velocities",
            "steering",
        )
    }

    fn install_query<
        Q: DeserializeOwned + 'static,
        R: Serialize + 'static,
        E: std::fmt::Display + 'static,
    >(
        &self,
        name: &str,
        query: Option<Query<Q, R, E>>,
        budget: Budget,
        output_key: &'static str,
        family: &'static str,
    ) -> Result<(), ScriptError> {
        let count = self.query_count.clone();
        let deadline = self.deadline.clone();
        self.context
            .with(|ctx| {
                let function =
                    rquickjs::Function::new(ctx.clone(), move |text: String| -> String {
                        let result = (|| -> Result<_, String> {
                            // Every native query shares the 256-unit tick allowance.
                            if text.len() > budget.bytes
                                || count.fetch_add(budget.cost, Ordering::Relaxed)
                                    > 256 - budget.cost
                                || deadline.lock().unwrap_or_else(|e| e.into_inner()).expired()
                            {
                                return Err(format!("{family} query budget exceeded"));
                            }
                            let request = serde_json::from_str(&text)
                                .map_err(|e| format!("invalid {family} query: {e}"))?;
                            let value = query.as_ref().ok_or_else(|| {
                                format!("{family} queries require a play session")
                            })?(request)
                            .map_err(|e| e.to_string())?;
                            if deadline.lock().unwrap_or_else(|e| e.into_inner()).expired() {
                                return Err(format!("{family} query deadline exceeded"));
                            }
                            Ok(value)
                        })();
                        match result {
                            Ok(value) => serde_json::json!({(output_key):value}).to_string(),
                            Err(error) => serde_json::json!({"error":error}).to_string(),
                        }
                    })?;
                ctx.globals().set(name, function)
            })
            .map_err(|_| ScriptError::Execution)
    }
}

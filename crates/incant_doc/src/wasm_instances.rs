use crate::{Id, Scene};
use incant_types::{CoroutineValue, properties::PropertyBudget};
use std::collections::BTreeMap;

pub(super) fn validate(
    scene: &Scene,
    instances: &BTreeMap<Id, BTreeMap<String, CoroutineValue>>,
    path: &str,
    issue: &mut impl FnMut(String, &str),
) {
    if instances.len() > incant_types::properties::MAX_INSTANCES {
        issue(
            format!("{path}/program/instances"),
            "too many Wasm instances",
        );
        return;
    }
    let mut budget = PropertyBudget::default();
    for (id, props) in instances {
        if !scene.entities.contains_key(id) {
            issue(
                format!("{path}/program/instances/{id}"),
                "Wasm instance owner is absent",
            );
        }
        if budget.object(props, 0, &mut |_| {}).is_err() {
            issue(
                format!("{path}/program/instances/{id}"),
                "invalid or excessive Wasm properties",
            );
            return;
        }
    }
}

//! Label/coarse-bundle dependency contract (KGD-131).

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DependencyBundle {
    pub label: String,
    pub bundle: String,
    pub dependencies: BTreeSet<String>,
}

impl DependencyBundle {
    pub fn closure<'a>(&'a self, known: &'a [DependencyBundle]) -> BTreeSet<String> {
        let mut result = self.dependencies.clone();
        let mut changed = true;
        while changed {
            changed = false;
            for dependency in result.clone() {
                if let Some(bundle) = known.iter().find(|candidate| candidate.label == dependency) {
                    let before = result.len();
                    result.extend(bundle.dependencies.iter().cloned());
                    changed |= result.len() != before;
                }
            }
        }
        result
    }
}

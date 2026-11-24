//! Keeps normal and OptiFine canonical values apart in the shared tables.

use crate::state::InstanceMetadata;
use std::collections::HashMap;

pub(in crate::api::instance) const OPTIFINE_PREFIX: &str = "optifine:";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::api::instance) enum GameOptionsScope {
    Normal,
    OptiFine,
}

impl GameOptionsScope {
    pub(in crate::api::instance) fn for_instance(
        metadata: &InstanceMetadata,
    ) -> Self {
        let loader = &metadata.applied_content_set.loader;
        if loader.as_str() == "optifine" || loader.as_meta_str() == "optifine" {
            Self::OptiFine
        } else {
            Self::Normal
        }
    }

    pub(super) fn is_optifine(self) -> bool {
        self == Self::OptiFine
    }

    pub(super) fn scoped_id(self, id: &str) -> String {
        if self.is_optifine() {
            format!("{OPTIFINE_PREFIX}{id}")
        } else {
            id.to_string()
        }
    }

    pub(super) fn matches_id(self, id: &str) -> bool {
        id.starts_with(OPTIFINE_PREFIX) == self.is_optifine()
    }
}

pub(in crate::api::instance) fn unscoped_option_id(id: &str) -> &str {
    id.strip_prefix(OPTIFINE_PREFIX).unwrap_or(id)
}

pub(in crate::api::instance) fn filter_scope_map<T>(
    values: HashMap<String, T>,
    scope: GameOptionsScope,
) -> HashMap<String, T> {
    values
        .into_iter()
        .filter(|(id, _)| scope.matches_id(id))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn option_ids_are_scoped() {
        let normal = GameOptionsScope::Normal;
        let optifine = GameOptionsScope::OptiFine;
        assert_eq!(normal.scoped_id("fullscreen"), "fullscreen");
        assert_eq!(optifine.scoped_id("fullscreen"), "optifine:fullscreen");
        assert_eq!(unscoped_option_id("optifine:external:key"), "external:key");
        assert!(normal.matches_id("fullscreen"));
        assert!(!normal.matches_id("optifine:fullscreen"));
        assert!(optifine.matches_id("optifine:fullscreen"));
        assert!(!optifine.matches_id("fullscreen"));
        let values = HashMap::from([
            ("fullscreen".to_string(), 1),
            ("optifine:fullscreen".to_string(), 2),
        ]);
        assert_eq!(filter_scope_map(values.clone(), normal).len(), 1);
        assert_eq!(filter_scope_map(values, optifine).len(), 1);
    }
}

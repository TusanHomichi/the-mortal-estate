pub mod script;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use tme_rules::content::{
    ActorSeedDef, GroundItemSeedDef, ItemInstanceSeedDef, MerchantInventorySeedDef,
    ResearchBoundary, ServiceInstanceSeedDef, WorldSeedDef,
};

use self::script::ScriptStep;

pub const SIMULATION_SCENARIO_SCHEMA_VERSION: u32 = 1;
pub const SIMULATION_SCENARIO_KIND: &str = "simulation_scenario";
pub const SIMULATION_SEED_SCHEMA_VERSION: u32 = 3;
pub const SIMULATION_SEED_KIND: &str = "simulation_seed";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SimulationScenarioV1 {
    pub schema_version: u32,
    pub kind: String,
    pub id: String,
    pub name: String,
    pub description: String,
    pub clean_content: bool,
    pub research_boundary: ResearchBoundary,
    pub rng_seed: u64,
    pub catalog: String,
    pub catalog_profile: String,
    pub world_template: String,
    pub simulation_seed: String,
    pub script: Vec<ScriptStep>,
}

impl SimulationScenarioV1 {
    pub fn effective_rng_seed(&self, override_seed: Option<u64>) -> u64 {
        override_seed.unwrap_or(self.rng_seed)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SimulationSeedV3 {
    pub schema_version: u32,
    pub kind: String,
    pub id: String,
    pub actors: Vec<ActorSeedDef>,
    pub item_instances: BTreeMap<String, ItemInstanceSeedDef>,
    pub ground_items: Vec<GroundItemSeedDef>,
    pub service_instances: Vec<ServiceInstanceSeedDef>,
    pub merchant_inventories: Vec<MerchantInventorySeedDef>,
    pub ecology_sites: Vec<tme_rules::content::EcologySiteDef>,
}

impl SimulationSeedV3 {
    pub fn into_world_seed(self) -> WorldSeedDef {
        WorldSeedDef {
            actors: self.actors,
            item_instances: self.item_instances,
            ground_items: self.ground_items,
            service_instances: self.service_instances,
            merchant_inventories: self.merchant_inventories,
            ecology_sites: self.ecology_sites,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scenario_requires_its_rng_seed_and_rejects_inline_content() {
        let mut value: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../content/test-corpus/first_room.json"
        ))
        .unwrap();
        value.as_object_mut().unwrap().remove("rng_seed");
        assert!(serde_json::from_value::<SimulationScenarioV1>(value).is_err());

        let mut value: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../content/test-corpus/first_room.json"
        ))
        .unwrap();
        value
            .as_object_mut()
            .unwrap()
            .insert("rooms".to_string(), serde_json::json!({}));
        assert!(serde_json::from_value::<SimulationScenarioV1>(value).is_err());
    }

    #[test]
    fn command_line_seed_overrides_required_scenario_seed() {
        let scenario: SimulationScenarioV1 = serde_json::from_str(include_str!(
            "../../../../content/test-corpus/first_room.json"
        ))
        .unwrap();
        assert_eq!(scenario.effective_rng_seed(None), 7);
        assert_eq!(scenario.effective_rng_seed(Some(91)), 91);
    }

    #[test]
    fn simulation_seed_requires_every_explicit_collection() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../content/test-corpus/simulation_seeds/first_room.json");
        let valid: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(root).expect("seed source"))
                .expect("seed JSON");
        serde_json::from_value::<SimulationSeedV3>(valid.clone())
            .expect("tracked seed should decode");

        for field in [
            "ground_items",
            "service_instances",
            "merchant_inventories",
            "ecology_sites",
        ] {
            let mut missing = valid.clone();
            missing.as_object_mut().expect("seed object").remove(field);
            assert!(
                serde_json::from_value::<SimulationSeedV3>(missing).is_err(),
                "missing {field} must be rejected"
            );
        }
    }
}

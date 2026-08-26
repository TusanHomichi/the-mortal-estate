use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::model::WorldPosition;

use super::{ActorSeedDef, EcologySiteDef, GroundItemSeedDef, ItemInstanceSeedDef};

/// Rules-owned initial-state payload carried by the sim-owned
/// `simulation_seed` envelope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldSeedDef {
    pub actors: Vec<ActorSeedDef>,
    pub item_instances: BTreeMap<String, ItemInstanceSeedDef>,
    pub ground_items: Vec<GroundItemSeedDef>,
    pub service_instances: Vec<ServiceInstanceSeedDef>,
    pub merchant_inventories: Vec<MerchantInventorySeedDef>,
    pub ecology_sites: Vec<EcologySiteDef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceInstanceSeedDef {
    pub id: String,
    pub service_definition_id: String,
    pub location: WorldPosition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MerchantInventorySeedDef {
    pub service_instance_id: String,
    pub capability_id: String,
    pub stock: Vec<MerchantStockSeedDef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MerchantStockSeedDef {
    pub item_instance_id: String,
    pub price_gold: i64,
}

#[cfg(test)]
mod tests {
    use super::WorldSeedDef;

    #[test]
    fn every_world_seed_collection_is_required_even_when_empty() {
        let valid = serde_json::json!({
            "actors": [],
            "item_instances": {},
            "ground_items": [],
            "service_instances": [],
            "merchant_inventories": []
            ,"ecology_sites": []
        });
        serde_json::from_value::<WorldSeedDef>(valid.clone())
            .expect("all explicit empty seed collections should decode");

        for field in [
            "ground_items",
            "service_instances",
            "merchant_inventories",
            "ecology_sites",
        ] {
            let mut missing = valid.clone();
            missing.as_object_mut().expect("seed object").remove(field);
            assert!(
                serde_json::from_value::<WorldSeedDef>(missing).is_err(),
                "missing {field} must be rejected"
            );
        }
    }
}

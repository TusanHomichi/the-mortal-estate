use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::content::{
    SeedWorldPositionStatus, SelectedCatalog, ValidationError, WorldSeedValidationContext,
    WorldTemplateV3,
};
use crate::events::Event;
use crate::model::*;
use crate::rng::DeterministicRng;

use super::{Engine, GameDefinition};

pub const FACET_CHECKPOINT_SCHEMA_VERSION: u32 = 5;
const FACET_CHECKPOINT_KIND: &str = "facet_checkpoint";
pub const MAX_FACET_CHECKPOINT_BYTES: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContentIdentityV1 {
    pub catalog_id: String,
    pub catalog_profile: String,
    pub world_template_id: String,
    pub definition_sha256: String,
}

impl ContentIdentityV1 {
    pub(crate) fn from_selected(
        selected: &SelectedCatalog,
        template: &WorldTemplateV3,
    ) -> Result<Self, ValidationError> {
        let bytes = serde_json::to_vec(&(selected, template)).map_err(|error| {
            ValidationError::new(vec![format!(
                "selected content could not be serialized for identity: {error}"
            )])
        })?;
        Ok(Self {
            catalog_id: selected.catalog_id.clone(),
            catalog_profile: selected.profile_key.as_str().to_string(),
            world_template_id: template.id.clone(),
            definition_sha256: hex_lower(&Sha256::digest(bytes)),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FacetCheckpointV5 {
    bytes: Vec<u8>,
}

impl FacetCheckpointV5 {
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, CheckpointError> {
        if bytes.is_empty() || bytes.len() > MAX_FACET_CHECKPOINT_BYTES {
            return Err(CheckpointError::new(
                "checkpoint byte count is out of bounds",
            ));
        }
        let payload: FacetCheckpointPayloadV1 =
            serde_json::from_slice(&bytes).map_err(CheckpointError::json)?;
        payload.validate_header()?;
        let canonical = serde_json::to_vec(&payload).map_err(CheckpointError::json)?;
        if canonical != bytes {
            return Err(CheckpointError::new(
                "checkpoint bytes are not canonical JSON",
            ));
        }
        Ok(Self { bytes })
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn sha256(&self) -> [u8; 32] {
        Sha256::digest(&self.bytes).into()
    }

    fn decode(&self) -> Result<FacetCheckpointPayloadV1, CheckpointError> {
        serde_json::from_slice(&self.bytes).map_err(CheckpointError::json)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckpointError(String);

impl CheckpointError {
    fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }

    fn json(error: serde_json::Error) -> Self {
        Self::new(format!("invalid checkpoint JSON: {error}"))
    }

    pub fn message(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CheckpointError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for CheckpointError {}

impl Engine {
    pub fn export_checkpoint(&self) -> Result<FacetCheckpointV5, CheckpointError> {
        let payload = FacetCheckpointPayloadV1::from_engine(self);
        let bytes = serde_json::to_vec(&payload).map_err(CheckpointError::json)?;
        FacetCheckpointV5::from_bytes(bytes)
    }

    pub fn hydrate_checkpoint(
        definition: Arc<GameDefinition>,
        checkpoint: &FacetCheckpointV5,
    ) -> Result<Self, CheckpointError> {
        let payload = checkpoint.decode()?;
        payload.validate_header()?;
        if &payload.content != definition.content_identity() {
            return Err(CheckpointError::new("checkpoint content identity mismatch"));
        }
        let engine = payload.into_engine(definition)?;
        engine
            .validate_world_item_locations()
            .map_err(|error| CheckpointError::new(error.to_string()))?;
        engine
            .validate_bow_readiness_invariants()
            .map_err(|error| CheckpointError::new(error.to_string()))?;
        engine
            .validate_world_item_burden()
            .map_err(|error| CheckpointError::new(error.to_string()))?;
        validate_character_ownership(&engine)?;
        validate_checkpoint_references(&engine)?;
        validate_social_checkpoint_state(&engine)?;
        let reencoded = engine.export_checkpoint()?;
        if reencoded.as_bytes() != checkpoint.as_bytes() {
            return Err(CheckpointError::new(
                "hydrated checkpoint does not re-export byte-identically",
            ));
        }
        Ok(engine)
    }
}

macro_rules! copy_checkpoint {
    ($checkpoint:ident, $runtime:ty, { $($field:ident : $kind:ty),+ $(,)? }) => {
        #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
        #[serde(deny_unknown_fields)]
        pub(super) struct $checkpoint { $( pub(super) $field: $kind, )+ }

        impl From<&$runtime> for $checkpoint {
            fn from(value: &$runtime) -> Self {
                Self { $( $field: value.$field.clone(), )+ }
            }
        }

        impl From<$checkpoint> for $runtime {
            fn from(value: $checkpoint) -> Self {
                Self { $( $field: value.$field, )+ }
            }
        }
    };
}

mod validation;
use validation::*;
mod world;
use world::*;
mod actors_inventory;
use actors_inventory::*;
mod secondary;
use secondary::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::setup::test_engine;

    #[test]
    fn checkpoint_round_trip_is_byte_exact_and_preserves_rng() {
        let mut engine = test_engine("first_room");
        let actor_id = engine
            .world()
            .controlled_actors()
            .next()
            .unwrap()
            .id
            .clone();
        let _ = engine
            .apply_actor_intent(&actor_id, PlayerIntent::Wait)
            .unwrap();
        let checkpoint = engine.export_checkpoint().unwrap();
        let hydrated =
            Engine::hydrate_checkpoint(engine.definition().clone(), &checkpoint).unwrap();
        assert_eq!(checkpoint, hydrated.export_checkpoint().unwrap());

        let mut expected = engine.clone();
        let mut actual = hydrated;
        assert_eq!(
            expected.apply_actor_intent(&actor_id, PlayerIntent::Wait),
            actual.apply_actor_intent(&actor_id, PlayerIntent::Wait)
        );
        assert_eq!(
            expected.export_checkpoint().unwrap(),
            actual.export_checkpoint().unwrap()
        );

        for case_id in [
            "world_topology_gallery",
            "town_adventure_loop_gallery",
            "magic_profession_gallery",
        ] {
            let representative = test_engine(case_id);
            let checkpoint = representative.export_checkpoint().unwrap();
            let hydrated =
                Engine::hydrate_checkpoint(representative.definition().clone(), &checkpoint)
                    .unwrap();
            assert_eq!(
                checkpoint,
                hydrated.export_checkpoint().unwrap(),
                "{case_id}"
            );
        }
    }

    #[test]
    fn checkpoint_rejects_noncanonical_and_content_mismatch() {
        let engine = test_engine("first_room");
        let checkpoint = engine.export_checkpoint().unwrap();
        let mut spaced = checkpoint.as_bytes().to_vec();
        spaced.insert(1, b' ');
        assert!(FacetCheckpointV5::from_bytes(spaced).is_err());

        let other = test_engine("ranged_attack");
        assert!(Engine::hydrate_checkpoint(other.definition().clone(), &checkpoint).is_err());
    }

    #[test]
    fn checkpoint_rejects_size_corruption_unknown_missing_and_wrong_schema() {
        assert!(FacetCheckpointV5::from_bytes(Vec::new()).is_err());
        assert!(FacetCheckpointV5::from_bytes(vec![b'x'; MAX_FACET_CHECKPOINT_BYTES + 1]).is_err());
        assert!(FacetCheckpointV5::from_bytes(b"not-json".to_vec()).is_err());

        let checkpoint = test_engine("first_room").export_checkpoint().unwrap();
        let mut unknown: serde_json::Value = serde_json::from_slice(checkpoint.as_bytes()).unwrap();
        unknown["unknown"] = serde_json::json!(true);
        assert!(FacetCheckpointV5::from_bytes(serde_json::to_vec(&unknown).unwrap()).is_err());

        let mut missing: serde_json::Value = serde_json::from_slice(checkpoint.as_bytes()).unwrap();
        missing.as_object_mut().unwrap().remove("rng_state");
        assert!(FacetCheckpointV5::from_bytes(serde_json::to_vec(&missing).unwrap()).is_err());

        let mut schema: serde_json::Value = serde_json::from_slice(checkpoint.as_bytes()).unwrap();
        schema["schema_version"] = serde_json::json!(2);
        assert!(FacetCheckpointV5::from_bytes(serde_json::to_vec(&schema).unwrap()).is_err());
    }

    #[test]
    fn checkpoint_three_rejects_both_empty_and_nonempty_pre_slot_ecology_maps() {
        let checkpoint = test_engine("creature_ecology_gallery")
            .export_checkpoint()
            .unwrap();
        let current: serde_json::Value =
            serde_json::from_slice(checkpoint.as_bytes()).expect("checkpoint JSON");
        assert_eq!(current["world"]["ecology"]["kind"], "slot_lifecycle");
        assert!(
            current["world"]["ecology"]["sites"]
                .as_object()
                .expect("ecology sites")
                .contains_key("gallery_pack")
        );

        for old_sites in [
            current["world"]["ecology"]["sites"].clone(),
            serde_json::json!({}),
        ] {
            let mut old = current.clone();
            old["world"]
                .as_object_mut()
                .expect("checkpoint world")
                .remove("ecology");
            old["world"]["ecology_sites"] = old_sites;
            assert!(
                FacetCheckpointV5::from_bytes(serde_json::to_vec(&old).unwrap()).is_err(),
                "Checkpoint 3 must reject the retired bare ecology_sites shape"
            );
        }
    }

    #[test]
    fn checkpoint_hydration_rejects_broken_content_references_and_sequences() {
        let engine = test_engine("world_topology_gallery");
        let checkpoint = engine.export_checkpoint().unwrap();
        let payload: FacetCheckpointPayloadV1 =
            serde_json::from_slice(checkpoint.as_bytes()).unwrap();

        let mut unknown_location = payload.clone();
        unknown_location.world.actors[0].location.realm = "missing_realm".to_string();
        let unknown_location =
            FacetCheckpointV5::from_bytes(serde_json::to_vec(&unknown_location).unwrap()).unwrap();
        assert!(
            Engine::hydrate_checkpoint(engine.definition().clone(), &unknown_location).is_err()
        );

        let item_engine = test_engine("first_room");
        let mut unknown_item: FacetCheckpointPayloadV1 =
            serde_json::from_slice(item_engine.export_checkpoint().unwrap().as_bytes()).unwrap();
        unknown_item
            .world
            .item_instances
            .first_entry()
            .unwrap()
            .get_mut()
            .definition_id = "missing_item_definition".to_string();
        let unknown_item =
            FacetCheckpointV5::from_bytes(serde_json::to_vec(&unknown_item).unwrap()).unwrap();
        assert!(
            Engine::hydrate_checkpoint(item_engine.definition().clone(), &unknown_item).is_err()
        );

        let mut invalid_sequence = payload;
        invalid_sequence.world.next_gold_sequence = 0;
        let invalid_sequence =
            FacetCheckpointV5::from_bytes(serde_json::to_vec(&invalid_sequence).unwrap()).unwrap();
        assert!(
            Engine::hydrate_checkpoint(engine.definition().clone(), &invalid_sequence).is_err()
        );
    }
}

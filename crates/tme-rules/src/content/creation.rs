use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{ActorSeedDef, ValidationError, WorldSeedDef};
use super::{CarriedLayoutDef, ItemInstanceSeedDef, StarterAttributeBoundsDef};
use crate::model::{
    ActorId, CharacterAttributes, CharacterId, CharacterSheetV1, ItemBindingState, WorldPosition,
};

/// Immutable creation choices. Item keys are local to this profile and are
/// replaced with server-character-scoped identities when the rules instantiate it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CharacterCreationProfileDef {
    pub id: String,
    pub actor_definition_id: String,
    pub arrival_id: String,
    pub attribute_bounds: StarterAttributeBoundsDef,
    pub attribute_points: u32,
    pub character: CharacterSheetV1,
    pub carried: CarriedLayoutDef,
    pub item_instances: BTreeMap<String, ItemInstanceSeedDef>,
    pub open_evidence: Vec<String>,
}

impl CharacterCreationProfileDef {
    pub(crate) fn materialize(
        &self,
        character_id: &CharacterId,
        attributes: CharacterAttributes,
        location: Option<&WorldPosition>,
        actor_is_player: bool,
        binds_to_character: impl Fn(&str) -> bool,
    ) -> Result<WorldSeedDef, ValidationError> {
        let profile = self;
        let error = || {
            ValidationError::new(vec![
                "invalid character creation profile or allocation".into(),
            ])
        };
        let class = &profile.character.identity;
        if !matches!(
            class.base_class_id.as_str(),
            "fighter" | "martial_artist" | "thief" | "wizard" | "thaumaturge"
        ) || class.base_class_id != class.current_class_id
            || !profile.character.promotion_history.is_empty()
            || profile.id.trim().is_empty()
            || profile
                .open_evidence
                .iter()
                .any(|entry| entry.trim().is_empty())
            || !actor_is_player
        {
            return Err(error());
        }
        let bounds = &profile.attribute_bounds;
        let rows = [
            (attributes.strength, &bounds.strength),
            (attributes.dexterity, &bounds.dexterity),
            (attributes.constitution, &bounds.constitution),
            (attributes.intelligence, &bounds.intelligence),
            (attributes.wisdom, &bounds.wisdom),
            (attributes.charisma, &bounds.charisma),
        ];
        let mut spent = 0_i64;
        for (value, range) in rows {
            if range.inborn <= 0 || value < range.inborn || value > range.creation_cap {
                return Err(error());
            }
            spent += i64::from(value) - i64::from(range.inborn);
        }
        if spent != i64::from(profile.attribute_points) {
            return Err(error());
        }
        let location = location.ok_or_else(error)?.clone();
        let mut character = profile.character.clone();
        character.attributes = attributes;
        let instance_id = |local: &str| format!("created/{}/{local}", character_id.as_str());
        let mut carried = profile.carried.clone();
        for item in &mut carried.items {
            item.item_instance_id = instance_id(&item.item_instance_id);
        }
        let mut items = std::collections::BTreeMap::new();
        for (local, source) in &profile.item_instances {
            // A profile cannot confer ownership of an existing character's gear.
            if !matches!(source.binding, ItemBindingState::Unrestricted) || local.is_empty() {
                return Err(error());
            }
            let mut item = source.clone();
            if binds_to_character(&source.definition_id) {
                item.binding = ItemBindingState::Bound {
                    character_id: character_id.clone(),
                };
            }
            items.insert(instance_id(local), item);
        }
        Ok(WorldSeedDef {
            actors: vec![ActorSeedDef {
                id: ActorId::new(format!("created/{}", character_id.as_str())),
                actor_definition_id: profile.actor_definition_id.clone(),
                location,
                npc: None,
                character_id: Some(character_id.clone()),
                character: Some(character),
                starter_character: None,
                carried,
                active_effects: Vec::new(),
            }],
            item_instances: items,
            ground_items: Vec::new(),
            service_instances: Vec::new(),
            merchant_inventories: Vec::new(),
            ecology_sites: Vec::new(),
        })
    }
}

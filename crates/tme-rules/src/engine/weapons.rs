use crate::events::{Event, ItemRelocationReason};
use crate::model::{
    BlockSourceKind, BowReadiness, BowReadinessChangeReason, CarriedPosition, CharacterAlignment,
    ItemBindingState, ItemLocation, PhysicalAttackMode, WeaponFumbleReason, WeaponFumbleResult,
    WeaponHandedness,
};

use crate::content::WeaponAttackModeDef;

use super::{Engine, StepError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PhysicalWeaponSelection {
    pub item_instance_id: Option<String>,
    pub item_definition_id: Option<String>,
    pub skill_track_id: String,
    pub skill_level: u8,
    pub default_attack_mode: PhysicalAttackMode,
    pub attack_modes: Vec<WeaponAttackModeDef>,
    pub cooldown_units: u32,
    pub combat_add_rating: i32,
    pub handedness: Option<WeaponHandedness>,
    pub block_value: i32,
    pub nocking_unloads_on_movement: Option<bool>,
    pub bow_readiness: Option<BowReadiness>,
    pub required_alignment: Option<CharacterAlignment>,
    pub binding_usable: bool,
    pub alignment_usable: bool,
    pub offhand_occupied: bool,
    pub full_two_handed_effect: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct BlockCandidate {
    pub source: BlockSourceKind,
    pub carried_position: Option<CarriedPosition>,
    pub item_instance_id: Option<String>,
    pub block_value: i32,
    pub skill_track_id: Option<String>,
    pub skill_level: Option<u8>,
    pub chance_percent: u32,
}

impl PhysicalWeaponSelection {
    pub fn is_bow(&self) -> bool {
        self.handedness == Some(WeaponHandedness::Bow)
    }

    pub fn attack_mode(&self, mode: PhysicalAttackMode) -> Option<&WeaponAttackModeDef> {
        self.attack_modes.iter().find(|row| row.mode == mode)
    }
}

impl Engine {
    pub(super) fn martial_attack_selection(
        &self,
        actor_index: usize,
    ) -> Result<PhysicalWeaponSelection, StepError> {
        let offhand_occupied = self
            .item_at_position(actor_index, CarriedPosition::LeftHand)?
            .is_some();
        Ok(PhysicalWeaponSelection {
            item_instance_id: None,
            item_definition_id: None,
            skill_track_id: "hand".to_string(),
            skill_level: self.skill_level_for_actor(actor_index, "hand"),
            default_attack_mode: PhysicalAttackMode::Fight,
            attack_modes: vec![WeaponAttackModeDef {
                mode: PhysicalAttackMode::Fight,
                maximum_range: 0,
                damage_kind: crate::model::PhysicalDamageKind::Crushing,
            }],
            cooldown_units: 1,
            combat_add_rating: 0,
            handedness: None,
            block_value: 0,
            nocking_unloads_on_movement: None,
            bow_readiness: None,
            required_alignment: None,
            binding_usable: true,
            alignment_usable: true,
            offhand_occupied,
            full_two_handed_effect: true,
        })
    }

    pub(super) fn physical_weapon_selection(
        &self,
        actor_index: usize,
    ) -> Result<PhysicalWeaponSelection, StepError> {
        let offhand_occupied = self
            .item_at_position(actor_index, CarriedPosition::LeftHand)?
            .is_some();
        let Some(item_instance_id) = self
            .item_at_position(actor_index, CarriedPosition::RightHand)?
            .map(str::to_string)
        else {
            return self.martial_attack_selection(actor_index);
        };
        let item_definition_id = self.item_instance(&item_instance_id)?.definition_id.clone();
        let item = self.item_definition(&item_instance_id)?;
        let weapon = item.weapon.as_ref().ok_or_else(|| {
            StepError::new(format!(
                "right-hand item {item_instance_id:?} is not a weapon"
            ))
        })?;
        let skill_level = self.skill_level_for_actor(actor_index, &weapon.skill_track_id);
        let actor = &self.world.actors[actor_index];
        let binding_usable = match &self.item_instance(&item_instance_id)?.binding {
            ItemBindingState::Unrestricted => true,
            ItemBindingState::BindOnFirstCharacterTouch => false,
            ItemBindingState::Bound { character_id } => {
                actor.character_id.as_ref() == Some(character_id)
            }
        };
        let alignment_usable = weapon.required_alignment.is_none_or(|required| {
            actor
                .character
                .as_ref()
                .is_some_and(|character| character.alignment_state.alignment == required)
        });
        let bow_readiness = self.item_instance(&item_instance_id)?.bow_readiness;
        Ok(PhysicalWeaponSelection {
            item_instance_id: Some(item_instance_id),
            item_definition_id: Some(item_definition_id),
            skill_track_id: weapon.skill_track_id.clone(),
            skill_level,
            default_attack_mode: weapon.default_attack_mode,
            attack_modes: weapon.attack_modes.clone(),
            cooldown_units: weapon.cooldown_units,
            combat_add_rating: weapon.combat_add_rating,
            handedness: Some(weapon.handedness),
            block_value: weapon.block_value,
            nocking_unloads_on_movement: weapon
                .nocking
                .as_ref()
                .map(|nocking| nocking.unloads_on_movement),
            bow_readiness,
            required_alignment: weapon.required_alignment,
            binding_usable,
            alignment_usable,
            offhand_occupied,
            full_two_handed_effect: weapon.handedness != WeaponHandedness::TwoHanded
                || !offhand_occupied,
        })
    }

    pub(super) fn effective_combat_add_rating(
        &self,
        selection: &PhysicalWeaponSelection,
    ) -> Result<i32, StepError> {
        let Some(weapon_instance_id) = selection.item_instance_id.as_deref() else {
            return Ok(0);
        };
        self.world
            .item_enchantments
            .iter()
            .filter(|enchantment| enchantment.item_instance_id == weapon_instance_id)
            .try_fold(selection.combat_add_rating, |total, enchantment| {
                total
                    .checked_add(enchantment.combat_add_rating_bonus)
                    .ok_or_else(|| StepError::new("effective combat-add rating overflow"))
            })
    }

    pub(super) fn selected_weapon_is_usable(
        &self,
        _actor_index: usize,
        selection: &PhysicalWeaponSelection,
    ) -> bool {
        selection.binding_usable && selection.alignment_usable
    }

    pub(super) fn selected_weapon_restriction(
        &self,
        _actor_index: usize,
        selection: &PhysicalWeaponSelection,
    ) -> Option<WeaponFumbleReason> {
        selection.item_instance_id.as_deref()?;
        if !selection.binding_usable {
            return Some(WeaponFumbleReason::TiedToOtherCharacter);
        }
        if !selection.alignment_usable {
            return Some(WeaponFumbleReason::AlignmentMismatch);
        }
        None
    }

    pub(super) fn selected_weapon_skill_level(
        &self,
        _actor_index: usize,
        selection: &PhysicalWeaponSelection,
    ) -> u8 {
        selection.skill_level
    }

    pub(super) fn general_fumble_percent(
        &self,
        actor_index: usize,
        selection: &PhysicalWeaponSelection,
    ) -> u32 {
        let rules = &self.definition.catalog.rules.combat.fumble;
        let reductions = u32::from(self.selected_weapon_skill_level(actor_index, selection))
            / rules.skill_levels_per_reduction;
        rules
            .base_percent
            .saturating_sub(reductions)
            .max(rules.minimum_percent)
    }

    pub(super) fn change_bow_readiness(
        &mut self,
        actor_index: usize,
        item_instance_id: &str,
        to: BowReadiness,
        reason: BowReadinessChangeReason,
        events: &mut Vec<Event>,
    ) -> Result<bool, StepError> {
        let from = self
            .item_instance(item_instance_id)?
            .bow_readiness
            .ok_or_else(|| StepError::new("item is not a bow"))?;
        if from == to {
            return Ok(false);
        }
        self.item_instance_mut(item_instance_id)?.bow_readiness = Some(to);
        let actor = &self.world.actors[actor_index];
        events.push(Event::BowReadinessChanged {
            actor_id: actor.id.clone(),
            actor: actor.name.clone(),
            item_instance_id: item_instance_id.to_string(),
            from,
            to,
            reason,
        });
        Ok(true)
    }

    pub(super) fn apply_actor_nock(
        &mut self,
        actor_index: usize,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let selection = self.physical_weapon_selection(actor_index)?;
        if !selection.is_bow() {
            return Err(StepError::new("right hand is not holding a bow"));
        }
        if selection.offhand_occupied {
            return Err(StepError::new("left hand must be empty to nock a bow"));
        }
        if selection.bow_readiness == Some(BowReadiness::Nocked) {
            return Err(StepError::new("bow is already nocked"));
        }
        let item_instance_id = selection.item_instance_id.expect("bow must have item id");
        self.change_bow_readiness(
            actor_index,
            &item_instance_id,
            BowReadiness::Nocked,
            BowReadinessChangeReason::Nocked,
            events,
        )?;
        Ok(())
    }

    pub(super) fn apply_actor_unload_bow(
        &mut self,
        actor_index: usize,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let selection = self.physical_weapon_selection(actor_index)?;
        if !selection.is_bow() {
            return Err(StepError::new("right hand is not holding a bow"));
        }
        if selection.bow_readiness != Some(BowReadiness::Nocked) {
            return Err(StepError::new("bow is not nocked"));
        }
        let item_instance_id = selection.item_instance_id.expect("bow must have item id");
        self.change_bow_readiness(
            actor_index,
            &item_instance_id,
            BowReadiness::Unnocked,
            BowReadinessChangeReason::ExplicitUnload,
            events,
        )?;
        Ok(())
    }

    pub(super) fn unload_item_if_nocked(
        &mut self,
        actor_index: usize,
        item_instance_id: &str,
        reason: BowReadinessChangeReason,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        if self.item_instance(item_instance_id)?.bow_readiness == Some(BowReadiness::Nocked) {
            self.change_bow_readiness(
                actor_index,
                item_instance_id,
                BowReadiness::Unnocked,
                reason,
                events,
            )?;
        }
        Ok(())
    }

    pub(super) fn unload_actor_bow_after_movement(
        &mut self,
        actor_index: usize,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let Ok(selection) = self.physical_weapon_selection(actor_index) else {
            return Ok(());
        };
        let Some(item_instance_id) = selection.item_instance_id else {
            return Ok(());
        };
        if selection.nocking_unloads_on_movement == Some(true) {
            self.unload_item_if_nocked(
                actor_index,
                &item_instance_id,
                BowReadinessChangeReason::Movement,
                events,
            )?;
        }
        Ok(())
    }

    pub(super) fn block_candidates(
        &self,
        defender_index: usize,
        armor_encumbrance: i32,
        attacking_combat_add_rating: i32,
    ) -> Result<Vec<BlockCandidate>, StepError> {
        let mut candidates = Vec::new();
        if let Some(left_id) = self
            .item_at_position(defender_index, CarriedPosition::LeftHand)?
            .map(str::to_string)
        {
            let item = self.item_definition(&left_id)?;
            let (source, value, skill_track_id) = if let Some(weapon) = item.weapon.as_ref() {
                (
                    BlockSourceKind::LeftWeapon,
                    weapon.block_value,
                    Some(weapon.skill_track_id.clone()),
                )
            } else {
                (
                    BlockSourceKind::LeftShield,
                    item.capability
                        .as_ref()
                        .and_then(|capability| capability.block_value)
                        .unwrap_or(0),
                    None,
                )
            };
            if value > 0 {
                let skill_level = skill_track_id
                    .as_deref()
                    .map(|track_id| self.skill_level_for_actor(defender_index, track_id));
                candidates.push(BlockCandidate {
                    source,
                    carried_position: Some(CarriedPosition::LeftHand),
                    item_instance_id: Some(left_id),
                    block_value: value,
                    skill_track_id,
                    skill_level,
                    chance_percent: self.block_percent_for_value(value),
                });
            }
        }
        if let Some(right_id) = self
            .item_at_position(defender_index, CarriedPosition::RightHand)?
            .map(str::to_string)
        {
            if let Some(weapon) = self.item_definition(&right_id)?.weapon.as_ref()
                && weapon.block_value > 0
            {
                candidates.push(BlockCandidate {
                    source: BlockSourceKind::RightWeapon,
                    carried_position: Some(CarriedPosition::RightHand),
                    item_instance_id: Some(right_id),
                    block_value: weapon.block_value,
                    skill_track_id: Some(weapon.skill_track_id.clone()),
                    skill_level: Some(
                        self.skill_level_for_actor(defender_index, &weapon.skill_track_id),
                    ),
                    chance_percent: self.block_percent_for_value(weapon.block_value),
                });
            }
        } else if let Some(chance) = self.martial_hand_block_chance_percent(defender_index) {
            candidates.push(BlockCandidate {
                source: BlockSourceKind::RightMartialHand,
                carried_position: Some(CarriedPosition::RightHand),
                item_instance_id: None,
                block_value: 0,
                skill_track_id: Some("hand".to_string()),
                skill_level: Some(self.actor_hand_level(defender_index)),
                chance_percent: u32::try_from(chance).unwrap_or(0),
            });
        }
        let encumbrance_penalty = u32::try_from(armor_encumbrance)
            .unwrap_or(0)
            .checked_mul(
                self.definition
                    .catalog
                    .rules
                    .combat
                    .block
                    .armor_encumbrance_percent_per_point,
            )
            .ok_or_else(|| StepError::new("armor encumbrance block penalty overflow"))?;
        let combat_add_penalty = u32::try_from(attacking_combat_add_rating)
            .unwrap_or(0)
            .checked_mul(
                self.definition
                    .catalog
                    .rules
                    .combat
                    .block
                    .combat_add_penetration_percent_per_rating,
            )
            .ok_or_else(|| StepError::new("combat-add block penetration overflow"))?;
        let total_penalty = encumbrance_penalty
            .checked_add(combat_add_penalty)
            .ok_or_else(|| StepError::new("hand block penalty overflow"))?;
        for candidate in &mut candidates {
            candidate.chance_percent = candidate.chance_percent.saturating_sub(total_penalty);
        }
        Ok(candidates)
    }

    fn block_percent_for_value(&self, value: i32) -> u32 {
        u32::try_from(value)
            .unwrap_or(0)
            .saturating_mul(
                self.definition
                    .catalog
                    .rules
                    .combat
                    .block
                    .shield_percent_per_point,
            )
            .min(
                self.definition
                    .catalog
                    .rules
                    .combat
                    .block
                    .shield_percent_cap,
            )
    }

    pub(super) fn choose_block_candidate(
        &mut self,
        candidates: &[BlockCandidate],
    ) -> Result<Option<BlockCandidate>, StepError> {
        match candidates {
            [] => Ok(None),
            [candidate] => Ok(Some(candidate.clone())),
            [left, right] => {
                let left_weight = self
                    .definition
                    .catalog
                    .rules
                    .combat
                    .block
                    .left_hand_selection_percent;
                let right_weight = 100_u32.saturating_sub(left_weight);
                let index = self
                    .rng
                    .weighted_index(&[left_weight, right_weight])
                    .map_err(StepError::new)?;
                Ok(Some([left, right][index].clone()))
            }
            _ => Err(StepError::new(
                "weapon owner produced too many block candidates",
            )),
        }
    }

    pub(super) fn resolve_weapon_fumble(
        &mut self,
        actor_index: usize,
        selection: &PhysicalWeaponSelection,
        mode: PhysicalAttackMode,
        reason: WeaponFumbleReason,
        events: &mut Vec<Event>,
    ) -> Result<WeaponFumbleResult, StepError> {
        let item_instance_id = selection
            .item_instance_id
            .as_deref()
            .ok_or_else(|| StepError::new("Martial Arts cannot fumble a weapon"))?;
        let result = if selection.is_bow() {
            WeaponFumbleResult::BowUnnocked
        } else {
            WeaponFumbleResult::Dropped
        };
        let actor = &self.world.actors[actor_index];
        events.push(Event::WeaponFumbled {
            attacker_id: actor.id.clone(),
            attacker: actor.name.clone(),
            item_instance_id: item_instance_id.to_string(),
            mode,
            reason,
            result,
        });
        if selection.is_bow() {
            self.unload_item_if_nocked(
                actor_index,
                item_instance_id,
                BowReadinessChangeReason::Fumble,
                events,
            )?;
        } else {
            let actor = &self.world.actors[actor_index];
            let holder = actor.item_holder_id();
            let ground = actor.location.clone();
            self.relocate_item_with_event(
                actor_index,
                item_instance_id,
                ItemLocation::Carried {
                    holder,
                    position: CarriedPosition::RightHand,
                },
                ItemLocation::Ground { position: ground },
                ItemRelocationReason::WeaponFumble,
                events,
            )?;
        }
        Ok(result)
    }

    pub(super) fn validate_bow_readiness_invariants(&self) -> Result<(), StepError> {
        for (item_instance_id, instance) in &self.world.item_instances {
            let is_bow = self
                .definition
                .catalog
                .item_catalog
                .get(&instance.definition_id)
                .and_then(|item| item.weapon.as_ref())
                .is_some_and(|weapon| weapon.handedness == WeaponHandedness::Bow);
            if is_bow != instance.bow_readiness.is_some() {
                return Err(StepError::new(format!(
                    "item instance {item_instance_id:?} has invalid bow readiness shape"
                )));
            }
            if instance.bow_readiness == Some(BowReadiness::Nocked) {
                let location = self.item_location(item_instance_id)?;
                let ItemLocation::Carried { holder, position } = location else {
                    return Err(StepError::new("nocked bow must be carried"));
                };
                if position != CarriedPosition::RightHand {
                    return Err(StepError::new("nocked bow must be in the right hand"));
                }
                let actor_index = self.actor_index_for_item_holder(&holder)?;
                if self
                    .item_at_position(actor_index, CarriedPosition::LeftHand)?
                    .is_some()
                {
                    return Err(StepError::new("nocked bow requires an empty left hand"));
                }
            }
        }
        Ok(())
    }
}

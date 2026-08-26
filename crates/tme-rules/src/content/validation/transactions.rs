use std::collections::{HashMap, HashSet};

use crate::content::{
    TransactionCostDef, TransactionDef, TransactionRequirementDef, TransactionRewardDef,
};
use crate::model::MAX_SKILL_LEVEL;

use super::ValidationBundle;

const CLASS_IDS: &[&str] = &[
    "fighter",
    "knight",
    "martial_artist",
    "thaumaturge",
    "thief",
    "wizard",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TransactionPolicy {
    Promotion,
    GenericService,
    Restoration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TransactionSummary {
    pub source_class_id: Option<String>,
    pub target_class_id: Option<String>,
    pub item_grant_instance_ids: Vec<String>,
}

fn non_empty(label: &str, value: &str, errors: &mut Vec<String>) {
    if value.trim().is_empty() {
        errors.push(format!("{label} must be non-empty"));
    }
}

fn requirement_key(requirement: &TransactionRequirementDef) -> String {
    match requirement {
        TransactionRequirementDef::CurrentClass { .. } => "current_class".to_string(),
        TransactionRequirementDef::MinimumLevel { .. } => "minimum_level".to_string(),
        TransactionRequirementDef::ExactKarma { .. } => "exact_karma".to_string(),
        TransactionRequirementDef::ExactAlignment { .. } => "exact_alignment".to_string(),
        TransactionRequirementDef::MinimumSkillLevel { track_id, .. } => {
            format!("minimum_skill_level:{track_id}")
        }
        TransactionRequirementDef::MinimumCarriedGold { .. } => "minimum_carried_gold".to_string(),
        TransactionRequirementDef::CarriedItem {
            item_definition_id, ..
        } => format!("carried_item:{item_definition_id}"),
        TransactionRequirementDef::CarriedPositionEmpty { position } => {
            format!("carried_position_empty:{}", position.label())
        }
        TransactionRequirementDef::SpellUnknown { spell_id } => {
            format!("spell_unknown:{spell_id}")
        }
        TransactionRequirementDef::QuestUnstarted { quest_id } => {
            format!("quest_unstarted:{quest_id}")
        }
        TransactionRequirementDef::QuestAtStage { quest_id, .. } => {
            format!("quest_at_stage:{quest_id}")
        }
        TransactionRequirementDef::NpcAccompanying { npc_actor_id } => {
            format!("npc_accompanying:{npc_actor_id}")
        }
    }
}

impl ValidationBundle {
    pub(super) fn validate_transaction(
        &self,
        label: &str,
        transaction: &TransactionDef,
        policy: TransactionPolicy,
        errors: &mut Vec<String>,
    ) -> TransactionSummary {
        non_empty(&format!("{label}.id"), &transaction.id, errors);
        non_empty(&format!("{label}.label"), &transaction.label, errors);
        if policy != TransactionPolicy::Restoration
            && transaction.costs.is_empty()
            && transaction.rewards.is_empty()
        {
            errors.push(format!("{label} must contain at least one cost or reward"));
        }
        if policy == TransactionPolicy::Restoration && !transaction.rewards.is_empty() {
            errors.push(format!(
                "{label}.rewards must be empty for restoration because its typed outcome is the reward"
            ));
        }

        let mut requirement_keys = HashSet::new();
        let mut source_class_id = None;
        let mut minimum_level_count = 0;
        let mut exact_karma_count = 0;
        let mut empty_positions = Vec::new();
        let mut spell_requirements = Vec::new();
        let mut carried_item = None;
        let mut minimum_gold = None;
        let mut quest_gates: HashMap<&str, Option<&str>> = HashMap::new();

        for (index, requirement) in transaction.requirements.iter().enumerate() {
            let requirement_label = format!("{label}.requirements[{index}]");
            if !requirement_keys.insert(requirement_key(requirement)) {
                errors.push(format!("{requirement_label} duplicates a requirement fact"));
            }
            match requirement {
                TransactionRequirementDef::CurrentClass { class_id } => {
                    non_empty(&format!("{requirement_label}.class_id"), class_id, errors);
                    if !CLASS_IDS.contains(&class_id.as_str()) {
                        errors.push(format!(
                            "{requirement_label}.class_id references unknown class {class_id:?}"
                        ));
                    }
                    source_class_id = Some(class_id.clone());
                }
                TransactionRequirementDef::MinimumLevel { level } => {
                    minimum_level_count += 1;
                    if *level <= 0 {
                        errors.push(format!("{requirement_label}.level must be positive"));
                    }
                }
                TransactionRequirementDef::ExactKarma { .. } => exact_karma_count += 1,
                TransactionRequirementDef::ExactAlignment { .. } => {}
                TransactionRequirementDef::MinimumSkillLevel { track_id, level } => {
                    non_empty(&format!("{requirement_label}.track_id"), track_id, errors);
                    if *level == 0 || *level > MAX_SKILL_LEVEL {
                        errors.push(format!(
                            "{requirement_label}.level must be between 1 and {MAX_SKILL_LEVEL}"
                        ));
                    }
                    if self
                        .skill_catalog
                        .as_ref()
                        .and_then(|catalog| catalog.track(track_id))
                        .is_none()
                    {
                        errors.push(format!(
                            "{requirement_label}.track_id references unknown skill track {track_id:?}"
                        ));
                    }
                }
                TransactionRequirementDef::MinimumCarriedGold { amount } => {
                    if *amount <= 0 {
                        errors.push(format!("{requirement_label}.amount must be positive"));
                    }
                    minimum_gold = Some(*amount);
                }
                TransactionRequirementDef::CarriedItem {
                    item_definition_id,
                    quantity,
                } => {
                    non_empty(
                        &format!("{requirement_label}.item_definition_id"),
                        item_definition_id,
                        errors,
                    );
                    if *quantity == 0 {
                        errors.push(format!("{requirement_label}.quantity must be positive"));
                    }
                    self.validate_item_definition_reference(
                        &format!("{requirement_label}.item_definition_id"),
                        item_definition_id,
                        errors,
                    );
                    if carried_item.is_some() {
                        errors.push(format!(
                            "{label} may contain at most one carried_item requirement"
                        ));
                    }
                    carried_item = Some((item_definition_id.as_str(), *quantity));
                }
                TransactionRequirementDef::CarriedPositionEmpty { position } => {
                    empty_positions.push(*position);
                }
                TransactionRequirementDef::SpellUnknown { spell_id } => {
                    non_empty(&format!("{requirement_label}.spell_id"), spell_id, errors);
                    if !self.spells.iter().any(|spell| spell.id == *spell_id) {
                        errors.push(format!(
                            "{requirement_label}.spell_id references unknown spell {spell_id:?}"
                        ));
                    }
                    spell_requirements.push(spell_id.as_str());
                }
                TransactionRequirementDef::QuestUnstarted { quest_id } => {
                    non_empty(&format!("{requirement_label}.quest_id"), quest_id, errors);
                    if !self.quests.iter().any(|quest| quest.id == *quest_id) {
                        errors.push(format!(
                            "{requirement_label}.quest_id references unknown quest {quest_id:?}"
                        ));
                    }
                    if quest_gates.insert(quest_id, None).is_some() {
                        errors.push(format!(
                            "{label} may contain only one quest gate for {quest_id:?}"
                        ));
                    }
                }
                TransactionRequirementDef::QuestAtStage { quest_id, stage_id } => {
                    non_empty(&format!("{requirement_label}.quest_id"), quest_id, errors);
                    non_empty(&format!("{requirement_label}.stage_id"), stage_id, errors);
                    match self.quests.iter().find(|quest| quest.id == *quest_id) {
                        Some(quest) if quest.stages.iter().any(|stage| stage.id == *stage_id) => {}
                        Some(_) => errors.push(format!(
                            "{requirement_label}.stage_id references unknown stage {stage_id:?}"
                        )),
                        None => errors.push(format!(
                            "{requirement_label}.quest_id references unknown quest {quest_id:?}"
                        )),
                    }
                    if quest_gates.insert(quest_id, Some(stage_id)).is_some() {
                        errors.push(format!(
                            "{label} may contain only one quest gate for {quest_id:?}"
                        ));
                    }
                }
                TransactionRequirementDef::NpcAccompanying { npc_actor_id } => {
                    non_empty(
                        &format!("{requirement_label}.npc_actor_id"),
                        npc_actor_id.as_str(),
                        errors,
                    );
                }
            }
        }

        let mut selected_item_cost = None;
        let mut carried_gold_costs = 0;
        for (index, cost) in transaction.costs.iter().enumerate() {
            let cost_label = format!("{label}.costs[{index}]");
            match cost {
                TransactionCostDef::CarriedGold { amount } => {
                    carried_gold_costs += 1;
                    if *amount <= 0 {
                        errors.push(format!("{cost_label}.amount must be positive"));
                    }
                    if minimum_gold.is_some_and(|minimum| *amount > minimum) {
                        errors.push(format!(
                            "{cost_label}.amount must not exceed the minimum_carried_gold requirement"
                        ));
                    }
                }
                TransactionCostDef::SelectedCarriedItem { quantity } => {
                    if *quantity == 0 {
                        errors.push(format!("{cost_label}.quantity must be positive"));
                    }
                    if selected_item_cost.is_some() {
                        errors.push(format!(
                            "{label} may contain at most one selected_carried_item cost"
                        ));
                    }
                    selected_item_cost = Some(*quantity);
                }
            }
        }
        if carried_gold_costs > 1 {
            errors.push(format!("{label} may contain at most one carried_gold cost"));
        }
        match (carried_item, selected_item_cost) {
            (None, Some(_)) => errors.push(format!(
                "{label} selected_carried_item cost requires one carried_item requirement"
            )),
            (Some((_, required)), Some(cost)) if cost > required => errors.push(format!(
                "{label} selected_carried_item cost must not exceed its carried_item requirement"
            )),
            _ => {}
        }

        let mut target_class_id = None;
        let mut class_reward_count = 0;
        let mut item_reward_count = 0;
        let mut spell_rewards = Vec::new();
        let mut item_grant_instance_ids = Vec::new();
        let mut reward_ids = HashSet::new();
        let mut quest_reward_ids = HashSet::new();
        for (index, reward) in transaction.rewards.iter().enumerate() {
            let reward_label = format!("{label}.rewards[{index}]");
            match reward {
                TransactionRewardDef::Experience { amount } => {
                    if *amount <= 0 {
                        errors.push(format!("{reward_label}.amount must be positive"));
                    }
                    if policy == TransactionPolicy::Promotion {
                        errors.push(format!(
                            "{reward_label}.kind experience is not legal for class_promotion"
                        ));
                    }
                }
                TransactionRewardDef::Item {
                    item_instance_id,
                    item_definition_id,
                    position,
                } => {
                    item_reward_count += 1;
                    non_empty(
                        &format!("{reward_label}.item_instance_id"),
                        item_instance_id,
                        errors,
                    );
                    non_empty(
                        &format!("{reward_label}.item_definition_id"),
                        item_definition_id,
                        errors,
                    );
                    if !reward_ids.insert(format!("item:{item_instance_id}")) {
                        errors.push(format!("{reward_label}.item_instance_id must be unique"));
                    }
                    if self.item_instances.contains_key(item_instance_id) {
                        errors.push(format!(
                            "{reward_label}.item_instance_id must not already be registered"
                        ));
                    }
                    self.validate_item_definition_reference(
                        &format!("{reward_label}.item_definition_id"),
                        item_definition_id,
                        errors,
                    );
                    if let Some(item) = self
                        .items
                        .iter()
                        .find(|item| item.id == *item_definition_id)
                        && !item.valid_placements.contains(&position.placement_kind())
                    {
                        errors.push(format!(
                            "{reward_label}.position is invalid for item definition {item_definition_id:?}"
                        ));
                    }
                    item_grant_instance_ids.push(item_instance_id.clone());
                }
                TransactionRewardDef::Class {
                    to_class_id,
                    to_class_display,
                } => {
                    class_reward_count += 1;
                    non_empty(&format!("{reward_label}.to_class_id"), to_class_id, errors);
                    non_empty(
                        &format!("{reward_label}.to_class_display"),
                        to_class_display,
                        errors,
                    );
                    if !CLASS_IDS.contains(&to_class_id.as_str()) {
                        errors.push(format!(
                            "{reward_label}.to_class_id references unknown class {to_class_id:?}"
                        ));
                    }
                    if policy != TransactionPolicy::Promotion {
                        errors.push(format!(
                            "{reward_label}.kind class is legal only for class_promotion"
                        ));
                    }
                    target_class_id = Some(to_class_id.clone());
                }
                TransactionRewardDef::Spell { spell_id } => {
                    non_empty(&format!("{reward_label}.spell_id"), spell_id, errors);
                    if !self.spells.iter().any(|spell| spell.id == *spell_id) {
                        errors.push(format!(
                            "{reward_label}.spell_id references unknown spell {spell_id:?}"
                        ));
                    }
                    if !reward_ids.insert(format!("spell:{spell_id}")) {
                        errors.push(format!("{reward_label}.spell_id must be unique"));
                    }
                    if policy != TransactionPolicy::Promotion {
                        errors.push(format!(
                            "{reward_label}.kind spell is legal only for class_promotion"
                        ));
                    }
                    spell_rewards.push(spell_id.as_str());
                }
                TransactionRewardDef::QuestStage { quest_id, stage_id } => {
                    non_empty(&format!("{reward_label}.quest_id"), quest_id, errors);
                    non_empty(&format!("{reward_label}.stage_id"), stage_id, errors);
                    if !quest_reward_ids.insert(quest_id.as_str()) {
                        errors.push(format!(
                            "{label} may change quest {quest_id:?} at most once"
                        ));
                    }
                    match self.quests.iter().find(|quest| quest.id == *quest_id) {
                        Some(quest) if quest.stages.iter().any(|stage| stage.id == *stage_id) => {}
                        Some(_) => errors.push(format!(
                            "{reward_label}.stage_id references unknown stage {stage_id:?}"
                        )),
                        None => errors.push(format!(
                            "{reward_label}.quest_id references unknown quest {quest_id:?}"
                        )),
                    }
                    match quest_gates.get(quest_id.as_str()) {
                        Some(Some(required_stage)) if *required_stage == stage_id => errors.push(
                            format!("{reward_label}.stage_id must advance beyond its quest gate"),
                        ),
                        Some(_) => {}
                        None => errors.push(format!(
                            "{reward_label} requires exactly one quest gate for {quest_id:?}"
                        )),
                    }
                    if policy == TransactionPolicy::Promotion {
                        errors.push(format!(
                            "{reward_label}.kind quest_stage is not legal for class_promotion"
                        ));
                    }
                }
            }
        }

        if policy == TransactionPolicy::Promotion {
            if source_class_id.is_none() {
                errors.push(format!(
                    "{label} must contain exactly one current_class requirement"
                ));
            }
            if minimum_level_count != 1 {
                errors.push(format!(
                    "{label} must contain exactly one minimum_level requirement"
                ));
            }
            if exact_karma_count != 1 {
                errors.push(format!(
                    "{label} must contain exactly one exact_karma requirement"
                ));
            }
            if empty_positions.len() != 1 {
                errors.push(format!(
                    "{label} must contain exactly one carried_position_empty requirement"
                ));
            }
            if !transaction.costs.is_empty() {
                errors.push(format!("{label}.costs must be empty for class_promotion"));
            }
            if class_reward_count != 1 {
                errors.push(format!("{label} must contain exactly one class reward"));
            }
            if item_reward_count != 1 {
                errors.push(format!("{label} must contain exactly one item reward"));
            }
            if spell_rewards.is_empty() {
                errors.push(format!("{label} must contain at least one spell reward"));
            }
            if spell_requirements != spell_rewards {
                errors.push(format!(
                    "{label} spell_unknown requirements must exactly match spell rewards"
                ));
            }
        }

        TransactionSummary {
            source_class_id,
            target_class_id,
            item_grant_instance_ids,
        }
    }

    pub(super) fn record_transaction_grants(
        &self,
        label: &str,
        summary: &TransactionSummary,
        grants: &mut HashMap<String, String>,
        errors: &mut Vec<String>,
    ) {
        for instance_id in &summary.item_grant_instance_ids {
            if let Some(previous) = grants.insert(instance_id.clone(), label.to_string()) {
                errors.push(format!(
                    "{label} item grant {instance_id:?} duplicates {previous}"
                ));
            }
        }
    }
}

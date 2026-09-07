//! Mutable seed validation: npcs.
use super::*;

pub(super) fn validate_npcs(
    seed: &WorldSeedDef,
    context: &impl WorldSeedValidationContext,
    errors: &mut Vec<String>,
) {
    for (actor_index, actor) in seed.actors.iter().enumerate() {
        let Some(npc) = &actor.npc else {
            continue;
        };
        let prefix = format!("actors[{actor_index}].npc");
        if npc.follow_cadence_units == 0 {
            errors.push(format!("{prefix}.follow_cadence_units must be positive"));
        }
        if !npc.patrol.is_empty() {
            if npc.patrol.len() < 2 || npc.patrol.len() > 64 {
                errors.push(format!(
                    "{prefix}.patrol must contain 2 to 64 circuit cells"
                ));
            }
            if !npc.patrol.contains(&actor.location.position) {
                errors.push(format!(
                    "{prefix}.patrol must include the initial actor square"
                ));
            }
            for (index, cell) in npc.patrol.iter().enumerate() {
                let mut at = actor.location.clone();
                at.position = *cell;
                validate_world_position(context, &at, &format!("{prefix}.patrol[{index}]"), errors);
                let next = npc.patrol[(index + 1) % npc.patrol.len()];
                if cell.x.abs_diff(next.x) + cell.y.abs_diff(next.y) != 1 {
                    errors.push(format!(
                        "{prefix}.patrol must be a cardinal connected circuit"
                    ));
                }
            }
        }
        if npc.interactions.is_empty() {
            errors.push(format!("{prefix}.interactions must be non-empty"));
        }
        let mut interaction_ids = HashMap::new();
        for (index, interaction) in npc.interactions.iter().enumerate() {
            let label = format!("{prefix}.interactions[{index}]");
            if let Some(previous) =
                interaction_ids.insert(interaction.transaction.id.as_str(), index)
            {
                errors.push(format!(
                    "{label}.transaction.id duplicates {prefix}.interactions[{previous}].transaction.id"
                ));
            }
            if interaction.response.trim().is_empty() {
                errors.push(format!("{label}.response must be non-empty"));
            }
            validate_transaction(
                &interaction.transaction,
                seed,
                context,
                &format!("{label}.transaction"),
                errors,
            );
            let accompanies = |npc_actor_id: &crate::model::ActorId| {
                interaction.transaction.requirements.iter().any(|requirement| {
                    matches!(requirement, TransactionRequirementDef::NpcAccompanying { npc_actor_id: required } if required == npc_actor_id)
                })
            };
            match &interaction.outcome {
                NpcInteractionOutcomeDef::Speak | NpcInteractionOutcomeDef::BeginFollow => {}
                NpcInteractionOutcomeDef::EndFollow | NpcInteractionOutcomeDef::Climb { .. } => {
                    if !accompanies(&actor.id) {
                        errors.push(format!(
                            "{label}.outcome requires npc_accompanying for the provider"
                        ));
                    }
                }
                NpcInteractionOutcomeDef::CompleteEscort { npc_actor_id } => {
                    if npc_actor_id == &actor.id {
                        errors.push(format!(
                            "{label}.outcome.npc_actor_id must differ from the provider"
                        ));
                    }
                    if !seed.actors.iter().any(|candidate| {
                        candidate.id == *npc_actor_id
                            && context.actor_definition_kind(&candidate.actor_definition_id)
                                == Some(ActorKind::Npc)
                    }) {
                        errors.push(format!(
                            "{label}.outcome.npc_actor_id references unknown NPC {npc_actor_id:?}"
                        ));
                    }
                    if !accompanies(npc_actor_id) {
                        errors.push(format!(
                            "{label}.outcome requires a matching npc_accompanying gate"
                        ));
                    }
                }
            }
        }
    }
}

pub(super) fn validate_transaction(
    transaction: &TransactionDef,
    seed: &WorldSeedDef,
    context: &impl WorldSeedValidationContext,
    label: &str,
    errors: &mut Vec<String>,
) {
    if transaction.id.trim().is_empty() {
        errors.push(format!("{label}.id must be non-empty"));
    }
    if transaction.label.trim().is_empty() {
        errors.push(format!("{label}.label must be non-empty"));
    }
    let mut requirement_keys = HashSet::new();
    let mut carried_item_requirement = None;
    let mut minimum_gold = None;
    let mut quest_gates = HashMap::<&str, Option<&str>>::new();
    for (index, requirement) in transaction.requirements.iter().enumerate() {
        let row = format!("{label}.requirements[{index}]");
        let key = match requirement {
            TransactionRequirementDef::CurrentClass { class_id } => {
                if class_id.trim().is_empty() {
                    errors.push(format!("{row}.class_id must be non-empty"));
                }
                if !TRANSACTION_CLASS_IDS.contains(&class_id.as_str()) {
                    errors.push(format!(
                        "{row}.class_id references unknown class {class_id:?}"
                    ));
                }
                "current_class".to_string()
            }
            TransactionRequirementDef::MinimumLevel { level } => {
                if *level <= 0 {
                    errors.push(format!("{row}.level must be positive"));
                }
                "minimum_level".to_string()
            }
            TransactionRequirementDef::ExactKarma { .. } => "exact_karma".to_string(),
            TransactionRequirementDef::ExactAlignment { .. } => "exact_alignment".to_string(),
            TransactionRequirementDef::MinimumSkillLevel { track_id, level } => {
                if track_id.trim().is_empty() {
                    errors.push(format!("{row}.track_id must be non-empty"));
                }
                if *level == 0 || *level > crate::model::MAX_SKILL_LEVEL {
                    errors.push(format!("{row}.level must be between 1 and 19"));
                }
                if context
                    .skill_catalog()
                    .and_then(|catalog| catalog.track(track_id))
                    .is_none()
                {
                    errors.push(format!(
                        "{row}.track_id references unknown skill track {track_id:?}"
                    ));
                }
                format!("minimum_skill_level:{track_id}")
            }
            TransactionRequirementDef::MinimumCarriedGold { amount } => {
                if *amount <= 0 {
                    errors.push(format!("{row}.amount must be positive"));
                }
                minimum_gold = Some(*amount);
                "minimum_carried_gold".to_string()
            }
            TransactionRequirementDef::CarriedItem {
                item_definition_id,
                quantity,
            } => {
                if context.item(item_definition_id).is_none() {
                    errors.push(format!(
                        "{row}.item_definition_id references unknown item definition {item_definition_id:?}"
                    ));
                }
                if *quantity == 0 {
                    errors.push(format!("{row}.quantity must be positive"));
                }
                if carried_item_requirement.is_some() {
                    errors.push(format!(
                        "{label} may contain at most one carried_item requirement"
                    ));
                }
                carried_item_requirement = Some((item_definition_id.as_str(), *quantity));
                format!("carried_item:{item_definition_id}")
            }
            TransactionRequirementDef::CarriedPositionEmpty { position } => {
                format!("carried_position_empty:{}", position.label())
            }
            TransactionRequirementDef::SpellUnknown { spell_id } => {
                if context.spell(spell_id).is_none() {
                    errors.push(format!(
                        "{row}.spell_id references unknown spell {spell_id:?}"
                    ));
                }
                format!("spell_unknown:{spell_id}")
            }
            TransactionRequirementDef::QuestUnstarted { quest_id } => {
                if !context.quest_exists(quest_id) {
                    errors.push(format!(
                        "{row}.quest_id references unknown quest {quest_id:?}"
                    ));
                }
                if quest_gates.insert(quest_id, None).is_some() {
                    errors.push(format!(
                        "{label} may contain only one quest gate for {quest_id:?}"
                    ));
                }
                format!("quest_unstarted:{quest_id}")
            }
            TransactionRequirementDef::QuestAtStage { quest_id, stage_id } => {
                if !context.quest_stage_exists(quest_id, stage_id) {
                    errors.push(format!(
                        "{row} references unknown quest/stage {quest_id:?}/{stage_id:?}"
                    ));
                }
                if quest_gates.insert(quest_id, Some(stage_id)).is_some() {
                    errors.push(format!(
                        "{label} may contain only one quest gate for {quest_id:?}"
                    ));
                }
                format!("quest_at_stage:{quest_id}")
            }
            TransactionRequirementDef::NpcAccompanying { npc_actor_id } => {
                if !seed.actors.iter().any(|actor| {
                    actor.id == *npc_actor_id
                        && context.actor_definition_kind(&actor.actor_definition_id)
                            == Some(ActorKind::Npc)
                }) {
                    errors.push(format!(
                        "{row}.npc_actor_id references unknown NPC {npc_actor_id:?}"
                    ));
                }
                format!("npc_accompanying:{npc_actor_id}")
            }
        };
        if !requirement_keys.insert(key) {
            errors.push(format!("{row} duplicates a requirement kind/target"));
        }
    }
    let mut selected_item_cost = None;
    let mut carried_gold_costs = 0;
    for (index, cost) in transaction.costs.iter().enumerate() {
        let row = format!("{label}.costs[{index}]");
        match cost {
            TransactionCostDef::CarriedGold { amount } => {
                carried_gold_costs += 1;
                if *amount <= 0 {
                    errors.push(format!("{row}.amount must be positive"));
                }
                if minimum_gold.is_some_and(|minimum| *amount > minimum) {
                    errors.push(format!(
                        "{row}.amount must not exceed the minimum_carried_gold requirement"
                    ));
                }
            }
            TransactionCostDef::SelectedCarriedItem { quantity } => {
                if *quantity == 0 {
                    errors.push(format!("{row}.quantity must be positive"));
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
    match (carried_item_requirement, selected_item_cost) {
        (None, Some(_)) => errors.push(format!(
            "{label} selected_carried_item cost requires a carried_item requirement"
        )),
        (Some((_, required)), Some(cost)) if cost > required => errors.push(format!(
            "{label} selected_carried_item cost must not exceed its carried_item requirement"
        )),
        _ => {}
    }
    let mut reward_ids = HashSet::new();
    let mut quest_reward_ids = HashSet::new();
    for (index, reward) in transaction.rewards.iter().enumerate() {
        let row = format!("{label}.rewards[{index}]");
        match reward {
            TransactionRewardDef::Item {
                item_instance_id,
                item_definition_id,
                position,
            } => {
                if item_instance_id.trim().is_empty() {
                    errors.push(format!("{row}.item_instance_id must be non-empty"));
                }
                if seed.item_instances.contains_key(item_instance_id) {
                    errors.push(format!(
                        "{row}.item_instance_id collides with an initial item instance"
                    ));
                }
                if !reward_ids.insert(format!("item:{item_instance_id}")) {
                    errors.push(format!("{row}.item_instance_id must be unique"));
                }
                match context.item(item_definition_id) {
                    None => errors.push(format!(
                        "{row}.item_definition_id references unknown item definition {item_definition_id:?}"
                    )),
                    Some(item)
                        if !item
                            .valid_placements
                            .contains(&position.placement_kind()) =>
                    {
                        errors.push(format!(
                            "{row}.position is invalid for item definition {item_definition_id:?}"
                        ));
                    }
                    Some(_) => {}
                }
            }
            TransactionRewardDef::Class {
                to_class_id,
                to_class_display,
            } => {
                if to_class_id.trim().is_empty() || to_class_display.trim().is_empty() {
                    errors.push(format!("{row} class identifiers must be non-empty"));
                }
                if !context.progression_profile_exists(to_class_id) {
                    errors.push(format!(
                        "{row}.to_class_id has no progression growth profile"
                    ));
                }
                errors.push(format!(
                    "{row}.kind class is legal only for class_promotion"
                ));
            }
            TransactionRewardDef::Spell { spell_id } => {
                if context.spell(spell_id).is_none() {
                    errors.push(format!(
                        "{row}.spell_id references unknown spell {spell_id:?}"
                    ));
                }
                if !reward_ids.insert(format!("spell:{spell_id}")) {
                    errors.push(format!("{row}.spell_id must be unique"));
                }
                errors.push(format!(
                    "{row}.kind spell is legal only for class_promotion"
                ));
            }
            TransactionRewardDef::Experience { amount } if *amount <= 0 => {
                errors.push(format!("{row}.amount must be positive"));
            }
            TransactionRewardDef::QuestStage { quest_id, stage_id } => {
                if !quest_reward_ids.insert(quest_id.as_str()) {
                    errors.push(format!(
                        "{label} may change quest {quest_id:?} at most once"
                    ));
                }
                if !context.quest_stage_exists(quest_id, stage_id) {
                    errors.push(format!(
                        "{row} references unknown quest/stage {quest_id:?}/{stage_id:?}"
                    ));
                }
                match quest_gates.get(quest_id.as_str()) {
                    Some(Some(required_stage)) if *required_stage == stage_id => {
                        errors.push(format!("{row}.stage_id must advance beyond its quest gate"))
                    }
                    Some(_) => {}
                    None => errors.push(format!(
                        "{row} requires exactly one quest gate for {quest_id:?}"
                    )),
                }
            }
            TransactionRewardDef::Experience { .. } => {}
        }
    }
}

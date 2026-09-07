//! Mutable seed validation: actors.
use super::*;

pub(super) fn validate_actors(
    seed: &WorldSeedDef,
    context: &impl WorldSeedValidationContext,
    errors: &mut Vec<String>,
) {
    let mut actor_ids = HashMap::new();
    let mut character_ids = HashMap::new();
    let mut player_count = 0;
    for (index, actor) in seed.actors.iter().enumerate() {
        let prefix = format!("actors[{index}]");
        if actor.id.as_str().trim().is_empty() {
            errors.push(format!("{prefix}.id must be non-empty"));
        } else if let Some(previous) = actor_ids.insert(actor.id.as_str(), index) {
            errors.push(format!("{prefix}.id duplicates actors[{previous}].id"));
        }
        let Some(actor_kind) = context.actor_definition_kind(&actor.actor_definition_id) else {
            errors.push(format!(
                "{prefix}.actor_definition_id references unknown or unselected actor definition {:?}",
                actor.actor_definition_id
            ));
            continue;
        };
        let has_character = actor.character.is_some() || actor.starter_character.is_some();
        let uses_character_alignment = context
            .actor_definition_uses_character_alignment(&actor.actor_definition_id)
            .unwrap_or(false);
        if uses_character_alignment && !has_character {
            errors.push(format!(
                "{prefix} actor definition alignment_source character requires a character-backed actor"
            ));
        }
        if has_character && !uses_character_alignment {
            errors.push(format!(
                "{prefix} actor definition alignment_source must be character for a character-backed actor"
            ));
        }
        if actor.character.is_some() && actor.starter_character.is_some() {
            errors.push(format!(
                "{prefix} must not contain both character and starter_character"
            ));
        }
        match (&actor.character_id, has_character) {
            (None, true) => errors.push(format!(
                "{prefix}.character_id is required when a character role is present"
            )),
            (Some(_), false) => errors.push(format!(
                "{prefix}.character_id is only valid when character is present"
            )),
            _ => {}
        }
        if let Some(character_id) = &actor.character_id {
            let id = character_id.as_str();
            if id.trim().is_empty() {
                errors.push(format!("{prefix}.character_id must be non-empty"));
            } else if id == actor.id.as_str() {
                errors.push(format!(
                    "{prefix}.character_id must differ from transient actor id"
                ));
            } else if let Some(previous) = character_ids.insert(id, index) {
                errors.push(format!(
                    "{prefix}.character_id duplicates actors[{previous}].character_id"
                ));
            }
        }
        if actor_kind == ActorKind::Player {
            player_count += 1;
        }
        match (actor_kind, actor.npc.as_ref()) {
            (ActorKind::Npc, None) => errors.push(format!("{prefix}.npc is required for NPCs")),
            (ActorKind::Npc, Some(_)) => {}
            (_, Some(_)) => errors.push(format!("{prefix}.npc is only valid for NPCs")),
            _ => {}
        }
        validate_world_position(
            context,
            &actor.location,
            &format!("{prefix}.location"),
            errors,
        );
        validate_character(actor, actor_kind, context, index, errors);
        validate_starter(actor, actor_kind, seed, context, index, errors);
        validate_active_effects(
            &actor.active_effects,
            context,
            &format!("{prefix}.active_effects"),
            errors,
        );
        validate_burden_strength(actor, context, &prefix, errors);
    }
    if player_count == 0 {
        errors.push("actors must contain at least one player".to_string());
    }
}

pub(super) fn validate_ecology_sites(
    seed: &WorldSeedDef,
    context: &impl WorldSeedValidationContext,
    errors: &mut Vec<String>,
) {
    let mut site_ids = HashSet::new();
    let actor_ids = seed
        .actors
        .iter()
        .map(|actor| actor.id.as_str())
        .collect::<HashSet<_>>();
    for (index, site) in seed.ecology_sites.iter().enumerate() {
        let prefix = format!("ecology_sites[{index}]");
        if site.id.trim().is_empty() {
            errors.push(format!("{prefix}.id must be non-empty"));
        } else if !site_ids.insert(site.id.as_str()) {
            errors.push(format!("{prefix}.id must be unique"));
        }
        let Some(group) = context.ecology_group(&site.source) else {
            errors.push(format!(
                "{prefix}.source references unknown or unselected ecology definition"
            ));
            continue;
        };
        let expected = group
            .member_ids
            .iter()
            .map(String::as_str)
            .collect::<HashSet<_>>();
        let actual = site
            .member_locations
            .keys()
            .map(String::as_str)
            .collect::<HashSet<_>>();
        if actual != expected {
            errors.push(format!(
                "{prefix}.member_locations keys must exactly equal spawn-group members"
            ));
        }
        for (member_id, location) in &site.member_locations {
            validate_world_position(
                context,
                location,
                &format!("{prefix}.member_locations[{member_id:?}]"),
                errors,
            );
            let generated_id = format!("ecology:{}:{}:0", site.id, member_id);
            if actor_ids.contains(generated_id.as_str()) {
                errors.push(format!(
                    "{prefix} generation-zero actor ID collides with an explicit actor"
                ));
            }
        }
    }
}

pub(super) fn validate_burden_strength(
    actor: &ActorSeedDef,
    context: &impl WorldSeedValidationContext,
    prefix: &str,
    errors: &mut Vec<String>,
) {
    let Some(attributes) = actor.effective_attributes() else {
        return;
    };
    let strength = u64::try_from(attributes.strength).unwrap_or(0);
    for (name, per_strength) in [
        (
            "lightly_loaded_max_per_strength",
            context.burden_limits_per_strength()[0],
        ),
        (
            "moderately_loaded_max_per_strength",
            context.burden_limits_per_strength()[1],
        ),
        (
            "heavily_loaded_max_per_strength",
            context.burden_limits_per_strength()[2],
        ),
    ] {
        if per_strength.checked_mul(strength).is_none() {
            errors.push(format!(
                "rules.burden.{name} * {prefix} effective character strength must not overflow"
            ));
        }
    }
}

pub(super) fn validate_character(
    actor: &ActorSeedDef,
    actor_kind: ActorKind,
    context: &impl WorldSeedValidationContext,
    index: usize,
    errors: &mut Vec<String>,
) {
    let Some(character) = &actor.character else {
        return;
    };
    let prefix = format!("actors[{index}].character");
    if actor_kind != ActorKind::Player {
        errors.push(format!("{prefix} is only valid for players"));
        return;
    }
    validate_skill_ledger(
        &character.skill_ledger,
        &character.identity.current_class_id,
        context,
        &format!("{prefix}.skill_ledger"),
        errors,
    );
    validate_attributes_and_resources(character, &prefix, errors);
    validate_progression(
        character.progression.level,
        character.progression.experience,
        &character.identity.current_class_id,
        context,
        &prefix,
        errors,
    );
}

pub(super) fn validate_attributes_and_resources(
    character: &crate::model::CharacterSheetV1,
    prefix: &str,
    errors: &mut Vec<String>,
) {
    for (name, value) in [
        ("strength", character.attributes.strength),
        ("dexterity", character.attributes.dexterity),
        ("constitution", character.attributes.constitution),
        ("intelligence", character.attributes.intelligence),
        ("wisdom", character.attributes.wisdom),
        ("charisma", character.attributes.charisma),
    ] {
        if !(3..=18).contains(&value) {
            errors.push(format!(
                "{prefix}.attributes.{name} must be between 3 and 18, got {value}"
            ));
        }
    }
    let resources = &character.resources;
    for (name, value) in [
        ("hp", resources.hp),
        ("max_hp", resources.max_hp),
        ("peak_hp", resources.peak_hp),
        ("mp", resources.mp),
        ("max_mp", resources.max_mp),
        ("stamina", resources.stamina),
        ("max_stamina", resources.max_stamina),
    ] {
        if value < 0 {
            errors.push(format!("{prefix}.resources.{name} must be non-negative"));
        }
    }
    if resources.hp <= 0 || resources.max_hp <= 0 {
        errors.push(format!(
            "{prefix}.resources.hp and max_hp must be positive for a living character"
        ));
    }
    if resources.hp > resources.max_hp {
        errors.push(format!("{prefix}.resources.hp must not exceed max_hp"));
    }
    if resources.max_hp > resources.peak_hp {
        errors.push(format!("{prefix}.resources.max_hp must not exceed peak_hp"));
    }
    if resources.mp > resources.max_mp {
        errors.push(format!("{prefix}.resources.mp must not exceed max_mp"));
    }
    if resources.max_stamina <= 0 {
        errors.push(format!(
            "{prefix}.resources.max_stamina must be positive for a living character"
        ));
    }
    if resources.stamina > resources.max_stamina {
        errors.push(format!(
            "{prefix}.resources.stamina must not exceed max_stamina"
        ));
    }
    if character.physical_attribute_adds.strength_adds < 0
        || character.physical_attribute_adds.dexterity_adds < 0
    {
        errors.push(format!(
            "{prefix}.physical_attribute_adds values must be non-negative"
        ));
    }
    for (index, entry) in character.promotion_history.iter().enumerate() {
        if entry.level < 1 {
            errors.push(format!(
                "{prefix}.promotion_history[{index}].level must be >= 1"
            ));
        }
    }
}

pub(super) fn validate_progression(
    level: i32,
    experience: i64,
    class_id: &str,
    context: &impl WorldSeedValidationContext,
    prefix: &str,
    errors: &mut Vec<String>,
) {
    let thresholds = context.progression_thresholds();
    if let (Some(first), Some(last)) = (thresholds.first(), thresholds.last()) {
        if level < first.0 || level > last.0 {
            errors.push(format!(
                "{prefix}.progression.level must be within authored threshold range"
            ));
        } else if thresholds
            .iter()
            .rev()
            .find(|(_, threshold)| experience >= *threshold)
            .is_none_or(|(earned, _)| level > *earned)
        {
            errors.push(format!(
                "{prefix}.progression.level must not exceed the XP-earned level"
            ));
        }
    }
    if experience < 0 {
        errors.push(format!("{prefix}.progression.experience must be >= 0"));
    }
    if !context.progression_profile_exists(class_id) {
        errors.push(format!(
            "rules.progression.growth_profiles must contain class_id {class_id:?}"
        ));
    }
}

pub(super) fn validate_skill_ledger(
    entries: &[SkillEntry],
    class_id: &str,
    context: &impl WorldSeedValidationContext,
    prefix: &str,
    errors: &mut Vec<String>,
) {
    let mut tracks = HashSet::new();
    for (index, entry) in entries.iter().enumerate() {
        let label = format!("{prefix}[{index}]");
        if entry.track_id.trim().is_empty() {
            errors.push(format!("{label}.track_id must be non-empty"));
        } else if !tracks.insert(entry.track_id.as_str()) {
            errors.push(format!("{label}.track_id must be unique"));
        }
        if !entry.is_valid_position() {
            errors.push(format!(
                "{label} must use level 0/critique 0 or level 1..=19/critique 0..=10"
            ));
        }
        if !entry.has_valid_learning_rate() {
            errors.push(format!("{label}.learning_rate must be positive"));
        } else if entry.learning_rate < context.base_learning_rate() {
            errors.push(format!(
                "{label}.learning_rate must be at least rules.skills.base_learning_rate"
            ));
        }
        let expected_magic = match class_id {
            "wizard" => Some("wizard_magic"),
            "thaumaturge" => Some("thaumaturge_magic"),
            "thief" => Some("thief_magic"),
            _ => None,
        };
        if matches!(
            entry.track_id.as_str(),
            "wizard_magic" | "thaumaturge_magic" | "thief_magic" | "knight_magic"
        ) && expected_magic != Some(entry.track_id.as_str())
        {
            errors.push(format!(
                "{label}.track_id is not a magic skill track for class {class_id:?}"
            ));
        }
        if let Some(catalog) = context.skill_catalog() {
            if catalog.track(&entry.track_id).is_none() {
                errors.push(format!(
                    "{label}.track_id references unknown skill catalog track {:?}",
                    entry.track_id
                ));
            } else if !catalog.track_is_eligible_for_class(&entry.track_id, class_id) {
                errors.push(format!(
                    "{label}.track_id is not eligible for class {class_id:?}"
                ));
            }
        }
    }
}

pub(super) fn validate_starter(
    actor: &ActorSeedDef,
    actor_kind: ActorKind,
    seed: &WorldSeedDef,
    context: &impl WorldSeedValidationContext,
    actor_index: usize,
    errors: &mut Vec<String>,
) {
    let Some(starter) = &actor.starter_character else {
        return;
    };
    let prefix = format!("actors[{actor_index}].starter_character");
    if context.boundary_policy() == ContentBoundaryPolicy::Clean {
        errors.push(format!(
            "{prefix} is only valid in an internal_parity_fixture"
        ));
    }
    if actor_kind != ActorKind::Player {
        errors.push(format!("{prefix} is only valid for players"));
        return;
    }
    starter.validate_intrinsic(&prefix, errors);
    validate_skill_ledger(
        &starter.initial_skills,
        starter.current_class_id(),
        context,
        &format!("{prefix}.initial_skills"),
        errors,
    );
    for (index, known) in starter.initial_known_spells.iter().enumerate() {
        let label = format!("{prefix}.initial_known_spells[{index}]");
        match context.spell(&known.spell_id) {
            Some(spell) if spell.lane.as_deref() == Some(known.lane.as_str()) => {}
            Some(_) => errors.push(format!("{label}.lane must match the referenced spell lane")),
            None => errors.push(format!(
                "{label}.spell_id references unknown spell {:?}",
                known.spell_id
            )),
        }
    }
    validate_progression(
        starter.progression.level,
        starter.progression.experience,
        starter.current_class_id(),
        context,
        &prefix,
        errors,
    );
    validate_starter_loadout(actor, starter, seed, actor_index, errors);
}

pub(super) fn validate_starter_loadout(
    actor: &ActorSeedDef,
    starter: &StarterCharacterDef,
    seed: &WorldSeedDef,
    actor_index: usize,
    errors: &mut Vec<String>,
) {
    let prefix = format!("actors[{actor_index}].starter_character.loadout");
    if actor.carried.gold != starter.loadout.gold {
        errors.push(format!(
            "actors[{actor_index}].carried.gold must equal {prefix}.gold"
        ));
    }
    let positions = actor
        .carried
        .items
        .iter()
        .map(|item| (item.item_instance_id.as_str(), item.position))
        .collect::<HashMap<_, _>>();
    let expected = starter
        .expected_carried_instance_ids()
        .into_iter()
        .collect::<HashSet<_>>();
    if positions.keys().copied().collect::<HashSet<_>>() != expected {
        errors.push(format!(
            "actors[{actor_index}].carried.items must equal the starter resolved loadout"
        ));
    }
    let mut check = |instance_id: &str,
                     definition_id: &str,
                     position_ok: fn(CarriedPosition) -> bool,
                     label: String| {
        let Some(instance) = seed.item_instances.get(instance_id) else {
            errors.push(format!(
                "{label}.item_instance_id references unknown item instance {instance_id:?}"
            ));
            return;
        };
        if instance.definition_id != definition_id {
            errors.push(format!(
                "{label}.item_definition_id does not match item instance definition"
            ));
        }
        match positions.get(instance_id).copied() {
            Some(position) if position_ok(position) => {}
            Some(position) => errors.push(format!(
                "{label}.item_instance_id has invalid carried position {:?}",
                position.label()
            )),
            None => errors.push(format!(
                "{label}.item_instance_id is not present in the actor carried layout"
            )),
        }
    };
    check(
        &starter.loadout.right_hand.item_instance_id,
        &starter.loadout.right_hand.item_definition_id,
        |position| position == CarriedPosition::RightHand,
        format!("{prefix}.right_hand"),
    );
    for (index, row) in starter.loadout.ordered_belt.iter().enumerate() {
        check(
            &row.item_instance_id,
            &row.item_definition_id,
            CarriedPosition::is_belt,
            format!("{prefix}.ordered_belt[{index}]"),
        );
    }
    check(
        &starter.loadout.inner_armor.item_instance_id,
        &starter.loadout.inner_armor.item_definition_id,
        |position| position == CarriedPosition::InnerArmor,
        format!("{prefix}.inner_armor"),
    );
    if let Some(book) = &starter.loadout.spell_book {
        check(
            &book.item_instance_id,
            &book.item_definition_id,
            CarriedPosition::is_sack_item,
            format!("{prefix}.spell_book"),
        );
    }
}

pub(super) fn validate_active_effects(
    effects: &[ActiveEffectDef],
    context: &impl WorldSeedValidationContext,
    prefix: &str,
    errors: &mut Vec<String>,
) {
    const SOURCE_KINDS: &[&str] = &["actor", "fixture", "item", "spell"];
    const STACKING: &[&str] = &["replace_same_kind", "stack_instance", "refresh_duration"];
    let mut instances = HashMap::new();
    for (index, effect) in effects.iter().enumerate() {
        let label = format!("{prefix}[{index}]");
        if effect.instance_id.trim().is_empty() {
            errors.push(format!("{label}.instance_id must be non-empty"));
        } else if let Some(previous) = instances.insert(effect.instance_id.as_str(), index) {
            errors.push(format!(
                "{label}.instance_id duplicates {prefix}[{previous}].instance_id"
            ));
        }
        if effect.effect_id.trim().is_empty() {
            errors.push(format!("{label}.effect_id must be non-empty"));
        }
        if !SOURCE_KINDS.contains(&effect.source.kind.as_str()) {
            errors.push(format!("{label}.source.kind is invalid"));
        }
        if effect.source.id.trim().is_empty() {
            errors.push(format!("{label}.source.id must be non-empty"));
        }
        if effect.kind.trim().is_empty() {
            errors.push(format!("{label}.kind must be non-empty"));
        }
        if effect.tags.iter().any(|tag| tag.trim().is_empty()) {
            errors.push(format!("{label}.tags must contain non-empty strings"));
        }
        let mut resistance_tags = HashSet::new();
        for (boost_index, boost) in effect.resistance_boosts.iter().enumerate() {
            if boost.tag.trim().is_empty() {
                errors.push(format!(
                    "{label}.resistance_boosts[{boost_index}].tag must be non-empty"
                ));
            }
            if boost.bonus_twentieths == 0
                || boost.bonus_twentieths > context.magic_resistance_denominator()
            {
                errors.push(format!(
                    "{label}.resistance_boosts[{boost_index}].bonus_twentieths must be in range"
                ));
            }
            if !resistance_tags.insert(boost.tag.as_str()) {
                errors.push(format!("{label}.resistance_boosts tags must be unique"));
            }
        }
        if effect.potency < 0 {
            errors.push(format!("{label}.potency must be non-negative"));
        }
        if effect.remaining_rounds.is_some_and(|rounds| rounds <= 0) {
            errors.push(format!("{label}.remaining_rounds must be positive"));
        }
        if effect
            .until_condition
            .as_ref()
            .is_some_and(|condition| condition.trim().is_empty())
        {
            errors.push(format!("{label}.until_condition must be non-empty"));
        }
        if !STACKING.contains(&effect.stacking.as_str()) {
            errors.push(format!("{label}.stacking is invalid"));
        }
        if effect.start_delay_rounds < 0 {
            errors.push(format!("{label}.start_delay_rounds must be non-negative"));
        }
        if effect.tick_interval_rounds <= 0 {
            errors.push(format!("{label}.tick_interval_rounds must be positive"));
        }
    }
}

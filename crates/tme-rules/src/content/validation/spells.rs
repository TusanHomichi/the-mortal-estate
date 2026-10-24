use std::collections::{HashMap, HashSet};

use super::*;

const SPELL_TERRAIN_OVERLAY_PASSABILITY: &[&str] = &[
    "blocked",
    "passable",
    "hindered",
    "unknown",
    "remove_overlay",
];
const SPELL_TERRAIN_OVERLAY_SIGHT: &[&str] =
    &["blocked", "obscured", "clear", "unknown", "remove_overlay"];
const SPELL_TERRAIN_OVERLAY_HAZARD: &[&str] =
    &["fire", "cold", "storm", "poison", "lava", "unknown"];
const SPELL_DOOR_CONTROL_ACTIONS: &[&str] = &["open", "close", "reveal_secret", "hide_secret"];
const SPELL_ITEM_UTILITY_ACTIONS: &[&str] = &["identify", "enchant_weapon", "transform_item"];
const SPELL_LOCATE_SUBJECTS: &[&str] = &["actor", "item", "level"];
const SPELL_SCRY_SCOPES: &[&str] = &["level", "coordinate"];
const ACTIVE_EFFECT_STACKING: &[&str] =
    &["replace_same_kind", "stack_instance", "refresh_duration"];

fn is_status_like_spell_family(family: SpellEffectFamily) -> bool {
    matches!(
        family,
        SpellEffectFamily::AttributeBuff
            | SpellEffectFamily::Concealment
            | SpellEffectFamily::ControlStatus
            | SpellEffectFamily::Curse
            | SpellEffectFamily::Darkness
            | SpellEffectFamily::FallProtection
            | SpellEffectFamily::Light
            | SpellEffectFamily::Poison
            | SpellEffectFamily::Protection
            | SpellEffectFamily::Resistance
            | SpellEffectFamily::Speed
            | SpellEffectFamily::Vision
            | SpellEffectFamily::WaterBreathing
            | SpellEffectFamily::WeaponEnchant
    )
}

fn requires_spell_potency(family: SpellEffectFamily) -> bool {
    matches!(
        family,
        SpellEffectFamily::AttributeBuff
            | SpellEffectFamily::Curse
            | SpellEffectFamily::DirectDamage
            | SpellEffectFamily::Healing
            | SpellEffectFamily::Poison
    )
}

impl ValidationBundle {
    pub(super) fn validate_spells(&self, internal_parity_fixture: bool, errors: &mut Vec<String>) {
        let mut ids = HashMap::new();
        let catalog_mode = self
            .spells
            .iter()
            .any(|spell| spell.catalog_entry.is_some());
        if catalog_mode && !internal_parity_fixture {
            errors.push(
                "spell catalog entries are only valid in a marked internal_parity_fixture"
                    .to_string(),
            );
        }
        let mut catalog_row_ids = HashMap::new();
        for (index, spell) in self.spells.iter().enumerate() {
            if spell.id.trim().is_empty() {
                errors.push(format!("spells[{index}].id must be non-empty"));
            }
            if let Some(previous) = ids.insert(spell.id.clone(), index) {
                errors.push(format!(
                    "spells[{index}].id duplicates spells[{previous}].id"
                ));
            }
            if spell.name.trim().is_empty() {
                errors.push(format!("spells[{index}].name must be non-empty"));
            }
            if spell.status != "stub" && spell.status != "draft" {
                errors.push(format!("spells[{index}].status must be stub or draft"));
            }
            validate_spell_social_definition(spell, index, errors);
            if catalog_mode && spell.catalog_entry.is_none() {
                errors.push(format!(
                    "spells[{index}].catalog_entry is required when any spell has catalog metadata"
                ));
            }
            if let Some(entry) = &spell.catalog_entry {
                self.validate_spell_catalog_entry(
                    index,
                    spell,
                    entry,
                    &mut catalog_row_ids,
                    errors,
                );
            }
            let topic_only_catalog_row = internal_parity_fixture
                && spell
                    .catalog_entry
                    .as_ref()
                    .is_some_and(|entry| entry.acquisition_row_id.is_none());
            if spell.casting.is_none() && !topic_only_catalog_row {
                errors.push(format!(
                    "spells[{index}].casting is required for operational spells"
                ));
            }
            if spell
                .effect
                .as_ref()
                .is_some_and(|effect| effect.family != SpellEffectFamily::DirectDamage)
                && spell.casting.as_ref().is_some_and(|casting| {
                    matches!(
                        casting.cast_class,
                        SpellCastClass::Path | SpellCastClass::PathOrCharacter
                    )
                })
            {
                errors.push(format!(
                    "spells[{index}].casting.cast_class may be path or path_or_character only for direct_damage"
                ));
            }
            if let Some(req) = spell.skill_requirement
                && req < 1
            {
                errors.push(format!(
                    "spells[{index}].skill_requirement must be >= 1, got {req}"
                ));
            }
            if spell.lane.as_deref() == Some("knight_magic") {
                if spell.skill_requirement.is_some() {
                    errors.push(format!(
                        "spells[{index}].skill_requirement must be absent for knight_magic"
                    ));
                }
                if spell.mp_cost != Some(3) {
                    errors.push(format!(
                        "spells[{index}].mp_cost must be 3 for knight_magic"
                    ));
                }
                if spell.acquisition.is_some() {
                    errors.push(format!(
                        "spells[{index}].acquisition must be absent for knight_magic"
                    ));
                }
                if spell.casting.as_ref().map(|casting| casting.method)
                    != Some(SpellCastingMethod::Direct)
                {
                    errors.push(format!(
                        "spells[{index}].casting.method must be direct for knight_magic"
                    ));
                }
            }
            self.validate_spell_effect(index, spell, errors);
            self.validate_spell_target(index, spell, errors);
            self.validate_spell_acquisition(index, spell, errors);
        }
    }

    fn validate_spell_catalog_entry<'a>(
        &self,
        index: usize,
        spell: &'a SpellDef,
        entry: &'a SpellCatalogEntryDef,
        catalog_row_ids: &mut HashMap<&'a str, usize>,
        errors: &mut Vec<String>,
    ) {
        let label = format!("spells[{index}].catalog_entry");
        if entry.row_id.trim().is_empty() {
            errors.push(format!("{label}.row_id must be non-empty"));
        } else {
            if entry.row_id != spell.id {
                errors.push(format!("{label}.row_id must equal spells[{index}].id"));
            }
            if let Some(previous) = catalog_row_ids.insert(entry.row_id.as_str(), index) {
                errors.push(format!(
                    "{label}.row_id duplicates spells[{previous}].catalog_entry.row_id"
                ));
            }
        }
        if entry.topic_id.trim().is_empty() {
            errors.push(format!("{label}.topic_id must be non-empty"));
        }
        if entry.variant_id.trim().is_empty() {
            errors.push(format!("{label}.variant_id must be non-empty"));
        }
        if entry
            .acquisition_row_id
            .as_ref()
            .is_some_and(|row_id| row_id.trim().is_empty())
        {
            errors.push(format!(
                "{label}.acquisition_row_id must be non-empty when present"
            ));
        }
        let mut question_ids = HashSet::new();
        for (question_index, question_id) in entry.open_question_ids.iter().enumerate() {
            if question_id.trim().is_empty() {
                errors.push(format!(
                    "{label}.open_question_ids[{question_index}] must be non-empty"
                ));
            } else if !question_ids.insert(question_id.as_str()) {
                errors.push(format!(
                    "{label}.open_question_ids[{question_index}] duplicates {question_id:?}"
                ));
            }
        }
        let mut resistance_tags = HashSet::new();
        for (tag_index, tag) in entry.resistance_tags.iter().enumerate() {
            if tag.trim().is_empty() {
                errors.push(format!(
                    "{label}.resistance_tags[{tag_index}] must be non-empty"
                ));
            } else if !resistance_tags.insert(tag.as_str()) {
                errors.push(format!(
                    "{label}.resistance_tags[{tag_index}] duplicates {tag:?}"
                ));
            }
        }
        if entry.resistance_mitigation_mode.is_some() && entry.resistance_tags.is_empty() {
            errors.push(format!(
                "{label}.resistance_mitigation_mode requires at least one resistance tag"
            ));
        }
        let client_fields_present = [
            entry.client_row_id.is_some(),
            entry.client_spell_id.is_some(),
            entry.client_verb_type.is_some(),
            entry.client_powerable.is_some(),
            entry.client_spell_poem_id.is_some(),
            entry.client_offensive.is_some(),
        ];
        if client_fields_present.iter().any(|present| *present)
            && !client_fields_present.iter().all(|present| *present)
        {
            errors.push(format!(
                "{label} client metadata must be complete when linked"
            ));
        }
        if entry
            .client_row_id
            .as_ref()
            .is_some_and(|value| value.trim().is_empty())
        {
            errors.push(format!(
                "{label}.client_row_id must be non-empty when present"
            ));
        }
        if entry.client_spell_id == Some(0) {
            errors.push(format!(
                "{label}.client_spell_id must be positive when present"
            ));
        }
        if entry.client_spell_poem_id == Some(0) {
            errors.push(format!(
                "{label}.client_spell_poem_id must be positive when present"
            ));
        }
        if entry.target_kind.is_none() && entry.state != SpellCatalogState::OpenEvidence {
            errors.push(format!(
                "{label}.target_kind may be absent only for open_evidence"
            ));
        }
        if let Some(effect) = &spell.effect
            && effect.family != entry.effect_family
        {
            errors.push(format!(
                "spells[{index}].effect.family must match {label}.effect_family"
            ));
        }
        if let Some(target) = &spell.target
            && Some(target.kind) != entry.target_kind
        {
            errors.push(format!(
                "spells[{index}].target.kind must match {label}.target_kind"
            ));
        }

        if entry.acquisition_row_id.is_none() {
            for (field, present) in [
                ("lane", spell.lane.is_some()),
                ("skill_requirement", spell.skill_requirement.is_some()),
                ("mp_cost", spell.mp_cost.is_some()),
                ("acquisition", spell.acquisition.is_some()),
            ] {
                if present {
                    errors.push(format!(
                        "spells[{index}].{field} must be absent for a topic-only catalog entry"
                    ));
                }
            }
            return;
        }

        let lane = spell.lane.as_deref();
        if !matches!(
            lane,
            Some("knight_magic" | "thaumaturge_magic" | "thief_magic" | "wizard_magic")
        ) {
            errors.push(format!(
                "spells[{index}].lane must be a current magic lane for a class-linked catalog entry"
            ));
        }
        if !matches!(spell.mp_cost, Some(cost) if cost > 0) {
            errors.push(format!(
                "spells[{index}].mp_cost must be positive for a class-linked catalog entry"
            ));
        }
        if lane != Some("knight_magic") {
            if !matches!(spell.skill_requirement, Some(level) if level > 0) {
                errors.push(format!(
                    "spells[{index}].skill_requirement must be positive for a non-Knight class-linked catalog entry"
                ));
            }
            if spell.acquisition.is_none() {
                errors.push(format!(
                    "spells[{index}].acquisition must be present for a non-Knight class-linked catalog entry"
                ));
            }
        }
    }

    fn validate_spell_effect(&self, index: usize, spell: &SpellDef, errors: &mut Vec<String>) {
        let Some(effect) = &spell.effect else {
            return;
        };

        if let Some(duration) = &effect.duration {
            if duration.policy == SpellDurationPolicy::Rounds && duration.rounds.is_none() {
                errors.push(format!(
                    "spells[{index}].effect.duration.rounds must be positive for rounds duration"
                ));
            }
            if let Some(rounds) = duration.rounds
                && rounds <= 0
            {
                errors.push(format!(
                    "spells[{index}].effect.duration.rounds must be positive for rounds duration"
                ));
            }
            if let Some(tick_interval_rounds) = duration.tick_interval_rounds
                && tick_interval_rounds <= 0
            {
                errors.push(format!(
                    "spells[{index}].effect.duration.tick_interval_rounds must be positive"
                ));
            }
        }

        if is_status_like_spell_family(effect.family)
            && !(effect.family == SpellEffectFamily::Concealment && effect.door_control.is_some())
        {
            if effect.status_kind.is_none() {
                errors.push(format!(
                    "spells[{index}].effect.status_kind must be present for {} spells",
                    effect.family
                ));
            }
            if effect.duration.is_none() {
                errors.push(format!(
                    "spells[{index}].effect.duration must be present for {} spells",
                    effect.family
                ));
            }
            if matches!(
                effect.family,
                SpellEffectFamily::Concealment
                    | SpellEffectFamily::FallProtection
                    | SpellEffectFamily::Speed
                    | SpellEffectFamily::Vision
                    | SpellEffectFamily::WaterBreathing
            ) && effect.stacking.is_none()
            {
                errors.push(format!(
                    "spells[{index}].effect.stacking must be present for {} spells",
                    effect.family
                ));
            }
        }

        let required_status_kind = match effect.family {
            SpellEffectFamily::Concealment if effect.door_control.is_none() => Some("hidden"),
            SpellEffectFamily::FallProtection => Some("fall_protection"),
            SpellEffectFamily::Speed => Some("speed"),
            SpellEffectFamily::Vision => Some("night_vision"),
            SpellEffectFamily::WaterBreathing => Some("water_breathing"),
            _ => None,
        };
        if let Some(required) = required_status_kind
            && effect.status_kind.as_deref() != Some(required)
        {
            errors.push(format!(
                "spells[{index}].effect.status_kind must be {required} for {} spells",
                effect.family
            ));
        }

        if requires_spell_potency(effect.family)
            && !matches!(effect.potency, Some(potency) if potency > 0)
        {
            errors.push(format!(
                "spells[{index}].effect.potency must be positive for {} spells",
                effect.family
            ));
        }

        if let Some(start_delay_rounds) = effect.start_delay_rounds
            && start_delay_rounds < 0
        {
            errors.push(format!(
                "spells[{index}].effect.start_delay_rounds must be non-negative"
            ));
        }

        let incoming_family = matches!(
            effect.family,
            SpellEffectFamily::DirectDamage
                | SpellEffectFamily::InstantDeath
                | SpellEffectFamily::ControlStatus
                | SpellEffectFamily::Poison
        );
        let boost_family = matches!(
            effect.family,
            SpellEffectFamily::Protection | SpellEffectFamily::Resistance
        );
        match effect.resistance.as_ref() {
            Some(SpellResistanceDef::Incoming { tag, mitigation }) => {
                if !incoming_family {
                    errors.push(format!(
                        "spells[{index}].effect.resistance incoming role is not valid for {}",
                        effect.family
                    ));
                }
                if tag.trim().is_empty() {
                    errors.push(format!(
                        "spells[{index}].effect.resistance.tag must be non-empty"
                    ));
                }
                if matches!(
                    effect.family,
                    SpellEffectFamily::ControlStatus | SpellEffectFamily::Poison
                ) && !matches!(mitigation, SpellResistanceMitigation::Negate)
                {
                    errors.push(format!(
                        "spells[{index}].effect.resistance.mitigation must be negate for {}",
                        effect.family
                    ));
                }
                match mitigation {
                    SpellResistanceMitigation::HalfDamage { minimum_damage, .. } if *minimum_damage <= 0 => errors.push(format!("spells[{index}].effect.resistance.mitigation.minimum_damage must be positive")),
                    SpellResistanceMitigation::MinimumDamage { damage } if *damage <= 0 => errors.push(format!("spells[{index}].effect.resistance.mitigation.damage must be positive")),
                    _ => {}
                }
            }
            Some(SpellResistanceDef::Boost { boosts }) => {
                if !boost_family {
                    errors.push(format!(
                        "spells[{index}].effect.resistance boost role is not valid for {}",
                        effect.family
                    ));
                }
                if boosts.is_empty() {
                    errors.push(format!(
                        "spells[{index}].effect.resistance.boosts must be non-empty"
                    ));
                }
                let mut tags = HashSet::new();
                for (boost_index, boost) in boosts.iter().enumerate() {
                    if boost.tag.trim().is_empty() {
                        errors.push(format!("spells[{index}].effect.resistance.boosts[{boost_index}].tag must be non-empty"));
                    }
                    if boost.bonus_twentieths == 0
                        || boost.bonus_twentieths > self.rules.magic.resistance.denominator
                    {
                        errors.push(format!("spells[{index}].effect.resistance.boosts[{boost_index}].bonus_twentieths must be in 1..=rules.magic.resistance.denominator"));
                    }
                    if !tags.insert(boost.tag.as_str()) {
                        errors.push(format!(
                            "spells[{index}].effect.resistance.boosts tags must be unique"
                        ));
                    }
                }
            }
            None if incoming_family => errors.push(format!(
                "spells[{index}].effect.resistance must use the incoming role for {}",
                effect.family
            )),
            None if boost_family => errors.push(format!(
                "spells[{index}].effect.resistance must use the boost role for {}",
                effect.family
            )),
            None => {}
        }

        let payloads = [
            ("banish", effect.banish.is_some()),
            ("instant_death", effect.instant_death.is_some()),
            ("raise_dead", effect.raise_dead.is_some()),
            ("turn_undead", effect.turn_undead.is_some()),
        ];
        for (family, present) in payloads {
            if present && effect.family.label() != family {
                errors.push(format!(
                    "spells[{index}].effect.{family} is valid only for {family} spells"
                ));
            }
        }
        match effect.family {
            SpellEffectFamily::Banish => match &effect.banish {
                Some(banish) => {
                    let expected = [
                        crate::model::CreatureTrait::Demon,
                        crate::model::CreatureTrait::Phantasm,
                    ];
                    if banish.eligible_traits.as_slice() != expected {
                        errors.push(format!(
                            "spells[{index}].effect.banish.eligible_traits must equal demon, phantasm"
                        ));
                    }
                }
                None => errors.push(format!(
                    "spells[{index}].effect.banish must be present for banish spells"
                )),
            },
            SpellEffectFamily::InstantDeath => match &effect.instant_death {
                Some(definition) if definition.damage_per_magic_level > 0 => {}
                Some(_) => errors.push(format!(
                    "spells[{index}].effect.instant_death.damage_per_magic_level must be positive"
                )),
                None => errors.push(format!(
                    "spells[{index}].effect.instant_death must be present for instant_death spells"
                )),
            },
            SpellEffectFamily::RaiseDead => match &effect.raise_dead {
                Some(definition)
                    if definition.method == crate::model::ResurrectionMethod::Thaumaturge => {}
                Some(_) => errors.push(format!(
                    "spells[{index}].effect.raise_dead.method must be thaumaturge"
                )),
                None => errors.push(format!(
                    "spells[{index}].effect.raise_dead must be present for raise_dead spells"
                )),
            },
            SpellEffectFamily::TurnUndead => match &effect.turn_undead {
                Some(definition)
                    if definition.eligible_trait == crate::model::CreatureTrait::Undead => {}
                Some(_) => errors.push(format!(
                    "spells[{index}].effect.turn_undead.eligible_trait must be undead"
                )),
                None => errors.push(format!(
                    "spells[{index}].effect.turn_undead must be present for turn_undead spells"
                )),
            },
            _ => {}
        }

        if effect.family == SpellEffectFamily::Concealment && effect.door_control.is_some() {
            if effect.duration.is_none() {
                errors.push(format!(
                    "spells[{index}].effect.duration must be present for door concealment"
                ));
            }
            if effect
                .door_control
                .as_ref()
                .is_some_and(|control| control.action != "hide_secret")
            {
                errors.push(format!(
                    "spells[{index}].effect.door_control.action must be hide_secret for door concealment"
                ));
            }
            if effect.status_kind.is_some() || effect.stacking.is_some() {
                errors.push(format!(
                    "spells[{index}].effect.status_kind and stacking must be absent for door concealment"
                ));
            }
        }

        if let Some(stacking) = effect.stacking.as_deref()
            && !ACTIVE_EFFECT_STACKING.contains(&stacking)
        {
            errors.push(format!(
                "spells[{index}].effect.stacking must be one of {}",
                ACTIVE_EFFECT_STACKING.join(", ")
            ));
        }

        if effect.family == SpellEffectFamily::Summon && effect.summon_actor_id.is_none() {
            errors.push(format!(
                "spells[{index}].effect.summon_actor_id must be present for summon spells"
            ));
        }
        if effect.family == SpellEffectFamily::Summon && effect.duration.is_none() {
            errors.push(format!(
                "spells[{index}].effect.duration must be present for summon spells"
            ));
        }
        if effect.family == SpellEffectFamily::Summon
            && let Some(summon_actor_id) = effect.summon_actor_id.as_deref()
            && !self
                .summon_templates
                .iter()
                .any(|template| template.id == summon_actor_id)
        {
            errors.push(format!(
                "spells[{index}].effect.summon_actor_id '{summon_actor_id}' is not a summon_templates id"
            ));
        }

        if effect.family == SpellEffectFamily::TerrainOverlay {
            match &effect.terrain_overlay {
                Some(terrain_overlay)
                    if terrain_overlay.passability.is_some()
                        || terrain_overlay.sight.is_some()
                        || terrain_overlay.hazard.is_some()
                        || terrain_overlay.move_cost.is_some() => {}
                _ => errors.push(format!(
                    "spells[{index}].effect.terrain_overlay must declare passability, sight, hazard, or move_cost"
                )),
            }
        }

        if let Some(terrain_overlay) = &effect.terrain_overlay {
            if let Some(passability) = terrain_overlay.passability.as_deref()
                && !SPELL_TERRAIN_OVERLAY_PASSABILITY.contains(&passability)
            {
                errors.push(format!(
                    "spells[{index}].effect.terrain_overlay.passability must be one of {}",
                    SPELL_TERRAIN_OVERLAY_PASSABILITY.join(", ")
                ));
            }
            if let Some(sight) = terrain_overlay.sight.as_deref()
                && !SPELL_TERRAIN_OVERLAY_SIGHT.contains(&sight)
            {
                errors.push(format!(
                    "spells[{index}].effect.terrain_overlay.sight must be one of {}",
                    SPELL_TERRAIN_OVERLAY_SIGHT.join(", ")
                ));
            }
            if let Some(hazard) = terrain_overlay.hazard.as_deref()
                && !SPELL_TERRAIN_OVERLAY_HAZARD.contains(&hazard)
            {
                errors.push(format!(
                    "spells[{index}].effect.terrain_overlay.hazard must be one of {}",
                    SPELL_TERRAIN_OVERLAY_HAZARD.join(", ")
                ));
            }
            if let Some(move_cost) = terrain_overlay.move_cost
                && move_cost <= 0
            {
                errors.push(format!(
                    "spells[{index}].effect.terrain_overlay.move_cost must be positive"
                ));
            }
        }

        if let Some(door_control) = &effect.door_control
            && !SPELL_DOOR_CONTROL_ACTIONS.contains(&door_control.action.as_str())
        {
            errors.push(format!(
                "spells[{index}].effect.door_control.action must be one of {}",
                SPELL_DOOR_CONTROL_ACTIONS.join(", ")
            ));
        }

        if let Some(item_utility) = &effect.item_utility {
            if !SPELL_ITEM_UTILITY_ACTIONS.contains(&item_utility.action.as_str()) {
                errors.push(format!(
                    "spells[{index}].effect.item_utility.action must be one of {}",
                    SPELL_ITEM_UTILITY_ACTIONS.join(", ")
                ));
            }
            for (tag_index, tag) in item_utility.tags.iter().enumerate() {
                if tag.trim().is_empty() {
                    errors.push(format!(
                        "spells[{index}].effect.item_utility.tags[{tag_index}] must be non-empty"
                    ));
                }
            }
            if let Some(output_item_definition_id) = &item_utility.output_item_definition_id {
                let item_label =
                    format!("spells[{index}].effect.item_utility.output_item_definition_id");
                if output_item_definition_id.trim().is_empty() {
                    errors.push(format!("{item_label} must be non-empty"));
                } else {
                    self.validate_item_definition_reference(
                        &item_label,
                        output_item_definition_id,
                        errors,
                    );
                }
            }
        }

        if let Some(locate) = &effect.locate {
            if !SPELL_LOCATE_SUBJECTS.contains(&locate.subject.as_str()) {
                errors.push(format!(
                    "spells[{index}].effect.locate.subject must be one of {}",
                    SPELL_LOCATE_SUBJECTS.join(", ")
                ));
            }
            if locate.id.trim().is_empty() {
                errors.push(format!(
                    "spells[{index}].effect.locate.id must be non-empty"
                ));
            }
        }

        if effect.family == SpellEffectFamily::Scry && effect.scry.is_none() {
            errors.push(format!(
                "spells[{index}].effect.scry must be present for scry spells"
            ));
        } else if let Some(scry) = &effect.scry {
            self.validate_spell_scry(index, scry, errors);
        }

        if let Some(portal) = &effect.portal {
            self.validate_spell_portal(index, portal, errors);
        }
    }

    fn validate_spell_portal(
        &self,
        index: usize,
        portal: &SpellPortalDef,
        errors: &mut Vec<String>,
    ) {
        let location = match &portal.target {
            TopologyTargetDef::Position { location } => location,
            TopologyTargetDef::Arrival { arrival_id } => {
                if arrival_id.trim().is_empty() {
                    errors.push(format!(
                        "spells[{index}].effect.portal.target.arrival_id must be non-empty"
                    ));
                    return;
                }
                let Some(location) = self.world_template.arrivals.get(arrival_id) else {
                    errors.push(format!(
                        "spells[{index}].effect.portal.target.arrival_id {arrival_id:?} does not exist"
                    ));
                    return;
                };
                location
            }
        };
        let terrain_map = world_template::terrain_map(&self.terrains);
        match world_template::position_status(&self.world_template, &terrain_map, location) {
            None => errors.push(format!(
                "spells[{index}].effect.portal.target references missing realm/level {}/{}",
                location.realm, location.level
            )),
            Some(world_template::WorldPositionStatus::OutOfBounds) => errors.push(format!(
                "spells[{index}].effect.portal.target is out of bounds at {}",
                location.label()
            )),
            Some(world_template::WorldPositionStatus::Blocked) => errors.push(format!(
                "spells[{index}].effect.portal.target is not traversable at {}",
                location.label()
            )),
            Some(world_template::WorldPositionStatus::Passable) => {}
        }
    }

    fn validate_spell_scry(&self, index: usize, scry: &SpellScryDef, errors: &mut Vec<String>) {
        if !SPELL_SCRY_SCOPES.contains(&scry.scope.as_str()) {
            errors.push(format!(
                "spells[{index}].effect.scry.scope must be one of {}",
                SPELL_SCRY_SCOPES.join(", ")
            ));
        }
        if scry.site.realm.trim().is_empty() || scry.site.level.trim().is_empty() {
            errors.push(format!(
                "spells[{index}].effect.scry.site realm and level must be non-empty"
            ));
            return;
        }
        let Some(level_def) = self
            .world_template
            .realms
            .get(&scry.site.realm)
            .and_then(|realm| realm.levels.get(&scry.site.level))
        else {
            errors.push(format!(
                "spells[{index}].effect.scry.site '{}' is not a level",
                scry.site.label()
            ));
            return;
        };

        match scry.scope.as_str() {
            "level" => {
                if scry.position.is_some() {
                    errors.push(format!(
                        "spells[{index}].effect.scry.position is invalid for level scope"
                    ));
                }
            }
            "coordinate" => {
                let Some(position) = scry.position else {
                    errors.push(format!(
                        "spells[{index}].effect.scry.position must be present for coordinate scope"
                    ));
                    return;
                };
                if position.x < 0
                    || position.y < 0
                    || position.x >= level_def.width
                    || position.y >= level_def.height
                {
                    errors.push(format!(
                        "spells[{index}].effect.scry.position [{},{}] out of bounds for level '{}' ({}x{})",
                        position.x, position.y, scry.site.label(), level_def.width, level_def.height
                    ));
                }
            }
            _ => {}
        }
    }

    fn validate_spell_acquisition(&self, index: usize, spell: &SpellDef, errors: &mut Vec<String>) {
        let Some(acquisition) = &spell.acquisition else {
            return;
        };

        if acquisition.gold_cost < 0 {
            errors.push(format!(
                "spells[{index}].acquisition.gold_cost must be >= 0"
            ));
        }
    }
}

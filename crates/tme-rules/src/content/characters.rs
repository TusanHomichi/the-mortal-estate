use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::model::{
    CharacterAlignmentState, CharacterAttributes, CharacterIdentity, CharacterProgression,
    CharacterResources, CharacterSheetV1, KnownSpell, PhysicalAttributeAdds, SkillEntry,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StarterCharacterDef {
    pub profile_id: String,
    pub class: StarterClassDef,
    pub nationality: StarterNationalityDef,
    pub creation: StarterCreationDef,
    pub progression: StarterProgressionDef,
    pub runtime_defaults: StarterRuntimeDefaultsDef,
    #[serde(default)]
    pub initial_skills: Vec<SkillEntry>,
    pub initial_known_spells: Vec<KnownSpell>,
    pub loadout: StarterLoadoutDef,
    pub open_evidence: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StarterClassDef {
    pub id: String,
    pub display: String,
    pub is_starter: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StarterNationalityDef {
    pub id: String,
    pub display: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StarterCreationDef {
    pub attributes: CharacterAttributes,
    pub bounds: StarterAttributeBoundsDef,
    pub creation_points_available: i32,
    pub creation_points_spent: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StarterAttributeBoundsDef {
    pub strength: StarterAttributeRangeDef,
    pub dexterity: StarterAttributeRangeDef,
    pub constitution: StarterAttributeRangeDef,
    pub intelligence: StarterAttributeRangeDef,
    pub wisdom: StarterAttributeRangeDef,
    pub charisma: StarterAttributeRangeDef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StarterAttributeRangeDef {
    pub inborn: i32,
    pub creation_cap: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StarterProgressionDef {
    pub level: i32,
    pub experience: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StarterRuntimeDefaultsDef {
    pub alignment_state: CharacterAlignmentState,
    pub resources: CharacterResources,
    pub physical_attribute_adds: PhysicalAttributeAdds,
    pub open_question_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StarterLoadoutDef {
    pub gold: crate::model::CarriedGold,
    pub right_hand: StarterEquipmentRowDef,
    pub ordered_belt: Vec<StarterEquipmentRowDef>,
    pub inner_armor: StarterItemRefDef,
    pub loot_sack_present: bool,
    #[serde(default)]
    pub spell_book: Option<StarterItemRefDef>,
    pub documented_skills: Vec<StarterSkillRowDef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StarterEquipmentRowDef {
    pub source_row_id: String,
    pub item_definition_id: String,
    pub item_instance_id: String,
    pub rating_scale_id: String,
    pub documented_rating_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StarterItemRefDef {
    pub item_definition_id: String,
    pub item_instance_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StarterSkillRowDef {
    pub source_row_id: String,
    pub skill_id: String,
    pub rating_scale_id: String,
    pub documented_rating_id: String,
}

impl StarterCharacterDef {
    pub(crate) fn build_character_sheet(&self) -> CharacterSheetV1 {
        CharacterSheetV1 {
            identity: CharacterIdentity {
                base_class_id: self.class.id.clone(),
                current_class_id: self.class.id.clone(),
                display_class: self.class.display.clone(),
                nationality_id: self.nationality.id.clone(),
                sex_or_gender_display: None,
            },
            alignment_state: self.runtime_defaults.alignment_state.clone(),
            attributes: self.creation.attributes.clone(),
            resources: self.runtime_defaults.resources.clone(),
            progression: CharacterProgression {
                level: self.progression.level,
                experience: self.progression.experience,
            },
            physical_attribute_adds: self.runtime_defaults.physical_attribute_adds.clone(),
            promotion_history: Vec::new(),
            skill_ledger: self.initial_skills.clone(),
            known_spells: self.initial_known_spells.clone(),
        }
    }

    pub(crate) fn attributes(&self) -> &CharacterAttributes {
        &self.creation.attributes
    }

    pub(crate) fn current_class_id(&self) -> &str {
        &self.class.id
    }

    pub(crate) fn expected_carried_instance_ids(&self) -> Vec<&str> {
        let mut ids = vec![
            self.loadout.right_hand.item_instance_id.as_str(),
            self.loadout.inner_armor.item_instance_id.as_str(),
        ];
        ids.extend(
            self.loadout
                .ordered_belt
                .iter()
                .map(|row| row.item_instance_id.as_str()),
        );
        if let Some(spell_book) = &self.loadout.spell_book {
            ids.push(spell_book.item_instance_id.as_str());
        }
        ids
    }

    pub(crate) fn validate_intrinsic(&self, prefix: &str, errors: &mut Vec<String>) {
        require_non_empty(&self.profile_id, &format!("{prefix}.profile_id"), errors);
        require_non_empty(&self.class.id, &format!("{prefix}.class.id"), errors);
        require_non_empty(
            &self.class.display,
            &format!("{prefix}.class.display"),
            errors,
        );
        if !self.class.is_starter {
            errors.push(format!("{prefix}.class.is_starter must be true"));
        }
        require_non_empty(
            &self.nationality.id,
            &format!("{prefix}.nationality.id"),
            errors,
        );
        require_non_empty(
            &self.nationality.display,
            &format!("{prefix}.nationality.display"),
            errors,
        );

        let attributes = &self.creation.attributes;
        let bounds = &self.creation.bounds;
        let rows = [
            ("strength", attributes.strength, &bounds.strength),
            ("dexterity", attributes.dexterity, &bounds.dexterity),
            (
                "constitution",
                attributes.constitution,
                &bounds.constitution,
            ),
            (
                "intelligence",
                attributes.intelligence,
                &bounds.intelligence,
            ),
            ("wisdom", attributes.wisdom, &bounds.wisdom),
            ("charisma", attributes.charisma, &bounds.charisma),
        ];
        let mut computed_spent = 0_i64;
        for (attribute_id, value, range) in rows {
            let range_prefix = format!("{prefix}.creation.bounds.{attribute_id}");
            if range.inborn > range.creation_cap {
                errors.push(format!(
                    "{range_prefix}.inborn must not exceed creation_cap"
                ));
                continue;
            }
            if value < range.inborn {
                errors.push(format!(
                    "{prefix}.creation.attributes.{attribute_id} must be at least {}",
                    range.inborn
                ));
            }
            if value > range.creation_cap {
                errors.push(format!(
                    "{prefix}.creation.attributes.{attribute_id} must be at most {}",
                    range.creation_cap
                ));
            }
            let Some(spent) = i64::from(value).checked_sub(i64::from(range.inborn)) else {
                errors.push(format!(
                    "{prefix}.creation.attributes.{attribute_id} spend must not overflow"
                ));
                continue;
            };
            let Some(total) = computed_spent.checked_add(spent) else {
                errors.push(format!("{prefix}.creation point spend must not overflow"));
                continue;
            };
            computed_spent = total;
        }

        if self.creation.creation_points_available <= 0 {
            errors.push(format!(
                "{prefix}.creation.creation_points_available must be positive"
            ));
        }
        if i64::from(self.creation.creation_points_spent) != computed_spent {
            errors.push(format!(
                "{prefix}.creation.creation_points_spent must equal recomputed spend {computed_spent}"
            ));
        }
        if self.creation.creation_points_spent != self.creation.creation_points_available {
            errors.push(format!(
                "{prefix}.creation must spend the entire creation point pool"
            ));
        }

        if self.progression.level <= 0 {
            errors.push(format!("{prefix}.progression.level must be positive"));
        }
        if self.progression.experience < 0 {
            errors.push(format!(
                "{prefix}.progression.experience must be non-negative"
            ));
        }
        let mut spell_ids = HashSet::new();
        for (index, spell) in self.initial_known_spells.iter().enumerate() {
            let spell_prefix = format!("{prefix}.initial_known_spells[{index}]");
            require_non_empty(&spell.spell_id, &format!("{spell_prefix}.spell_id"), errors);
            require_non_empty(&spell.lane, &format!("{spell_prefix}.lane"), errors);
            if spell.learned_at_level != self.progression.level {
                errors.push(format!(
                    "{spell_prefix}.learned_at_level must equal starter progression level"
                ));
            }
            if !spell.spell_id.is_empty() && !spell_ids.insert(spell.spell_id.as_str()) {
                errors.push(format!("{spell_prefix}.spell_id must be unique"));
            }
        }
        validate_resources(
            &self.runtime_defaults.resources,
            &format!("{prefix}.runtime_defaults.resources"),
            errors,
        );
        if self.runtime_defaults.physical_attribute_adds.strength_adds < 0
            || self.runtime_defaults.physical_attribute_adds.dexterity_adds < 0
        {
            errors.push(format!(
                "{prefix}.runtime_defaults.physical_attribute_adds values must be non-negative"
            ));
        }
        validate_non_empty_unique(
            &self.runtime_defaults.open_question_ids,
            &format!("{prefix}.runtime_defaults.open_question_ids"),
            errors,
            false,
        );

        if self.loadout.gold.left_hand < 0
            || self.loadout.gold.right_hand < 0
            || self.loadout.gold.sack < 0
        {
            errors.push(format!("{prefix}.loadout.gold values must be non-negative"));
        }
        if self.loadout.gold.checked_total().is_none() {
            errors.push(format!(
                "{prefix}.loadout.gold total must fit a signed 64-bit integer"
            ));
        }
        validate_equipment_row(
            &self.loadout.right_hand,
            &format!("{prefix}.loadout.right_hand"),
            errors,
        );
        if self.loadout.ordered_belt.len() != 2 {
            errors.push(format!(
                "{prefix}.loadout.ordered_belt must contain exactly two rows"
            ));
        }
        for (index, row) in self.loadout.ordered_belt.iter().enumerate() {
            validate_equipment_row(
                row,
                &format!("{prefix}.loadout.ordered_belt[{index}]"),
                errors,
            );
        }
        validate_item_ref(
            &self.loadout.inner_armor,
            &format!("{prefix}.loadout.inner_armor"),
            errors,
        );
        if !self.loadout.loot_sack_present {
            errors.push(format!("{prefix}.loadout.loot_sack_present must be true"));
        }
        if let Some(spell_book) = &self.loadout.spell_book {
            validate_item_ref(spell_book, &format!("{prefix}.loadout.spell_book"), errors);
        }

        if self.loadout.documented_skills.len() != 3 {
            errors.push(format!(
                "{prefix}.loadout.documented_skills must contain exactly three rows"
            ));
        }
        let mut skill_ids = HashSet::new();
        let mut skill_source_rows = HashSet::new();
        for (index, row) in self.loadout.documented_skills.iter().enumerate() {
            let row_prefix = format!("{prefix}.loadout.documented_skills[{index}]");
            require_non_empty(
                &row.source_row_id,
                &format!("{row_prefix}.source_row_id"),
                errors,
            );
            require_non_empty(&row.skill_id, &format!("{row_prefix}.skill_id"), errors);
            require_non_empty(
                &row.rating_scale_id,
                &format!("{row_prefix}.rating_scale_id"),
                errors,
            );
            require_non_empty(
                &row.documented_rating_id,
                &format!("{row_prefix}.documented_rating_id"),
                errors,
            );
            if !row.skill_id.is_empty() && !skill_ids.insert(row.skill_id.as_str()) {
                errors.push(format!("{row_prefix}.skill_id must be unique"));
            }
            if !row.source_row_id.is_empty()
                && !skill_source_rows.insert(row.source_row_id.as_str())
            {
                errors.push(format!("{row_prefix}.source_row_id must be unique"));
            }
        }
        let expected_skill_ids = HashSet::from(["martial_arts", "magic", "theft"]);
        if skill_ids != expected_skill_ids {
            errors.push(format!(
                "{prefix}.loadout.documented_skills must contain martial_arts, magic, and theft"
            ));
        }

        let mut instance_ids = HashSet::new();
        for instance_id in self.expected_carried_instance_ids() {
            if !instance_id.is_empty() && !instance_ids.insert(instance_id) {
                errors.push(format!("{prefix}.loadout item instance ids must be unique"));
            }
        }
        let mut equipment_source_rows = HashSet::new();
        for row in std::iter::once(&self.loadout.right_hand).chain(self.loadout.ordered_belt.iter())
        {
            if !row.source_row_id.is_empty()
                && !equipment_source_rows.insert(row.source_row_id.as_str())
            {
                errors.push(format!(
                    "{prefix}.loadout equipment source_row_id values must be unique"
                ));
            }
        }
        validate_non_empty_unique(
            &self.open_evidence,
            &format!("{prefix}.open_evidence"),
            errors,
            false,
        );
    }
}

fn validate_equipment_row(row: &StarterEquipmentRowDef, prefix: &str, errors: &mut Vec<String>) {
    require_non_empty(
        &row.source_row_id,
        &format!("{prefix}.source_row_id"),
        errors,
    );
    require_non_empty(
        &row.item_definition_id,
        &format!("{prefix}.item_definition_id"),
        errors,
    );
    require_non_empty(
        &row.item_instance_id,
        &format!("{prefix}.item_instance_id"),
        errors,
    );
    require_non_empty(
        &row.rating_scale_id,
        &format!("{prefix}.rating_scale_id"),
        errors,
    );
    require_non_empty(
        &row.documented_rating_id,
        &format!("{prefix}.documented_rating_id"),
        errors,
    );
}

fn validate_item_ref(row: &StarterItemRefDef, prefix: &str, errors: &mut Vec<String>) {
    require_non_empty(
        &row.item_definition_id,
        &format!("{prefix}.item_definition_id"),
        errors,
    );
    require_non_empty(
        &row.item_instance_id,
        &format!("{prefix}.item_instance_id"),
        errors,
    );
}

fn validate_resources(resources: &CharacterResources, prefix: &str, errors: &mut Vec<String>) {
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
            errors.push(format!("{prefix}.{name} must be non-negative"));
        }
    }
    if resources.hp > resources.max_hp {
        errors.push(format!("{prefix}.hp must not exceed max_hp"));
    }
    if resources.max_hp > resources.peak_hp {
        errors.push(format!("{prefix}.max_hp must not exceed peak_hp"));
    }
    if resources.hp <= 0 || resources.max_hp <= 0 {
        errors.push(format!(
            "{prefix}.hp and max_hp must be positive for a living character"
        ));
    }
    if resources.max_stamina <= 0 {
        errors.push(format!(
            "{prefix}.max_stamina must be positive for a living character"
        ));
    }
    if resources.mp > resources.max_mp {
        errors.push(format!("{prefix}.mp must not exceed max_mp"));
    }
    if resources.stamina > resources.max_stamina {
        errors.push(format!("{prefix}.stamina must not exceed max_stamina"));
    }
}

fn validate_non_empty_unique(
    values: &[String],
    prefix: &str,
    errors: &mut Vec<String>,
    require_entries: bool,
) {
    if require_entries && values.is_empty() {
        errors.push(format!("{prefix} must be non-empty"));
    }
    let mut seen = HashSet::new();
    for (index, value) in values.iter().enumerate() {
        require_non_empty(value, &format!("{prefix}[{index}]"), errors);
        if !value.is_empty() && !seen.insert(value.as_str()) {
            errors.push(format!("{prefix}[{index}] must be unique"));
        }
    }
}

fn require_non_empty(value: &str, path: &str, errors: &mut Vec<String>) {
    if value.trim().is_empty() {
        errors.push(format!("{path} must be non-empty"));
    }
}

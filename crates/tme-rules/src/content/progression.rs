use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::{ActorSeedDef, ServiceCapabilityDef, ServiceDefinitionDef};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProgressionRulesDef {
    pub level_thresholds: Vec<LevelThresholdDef>,
    pub growth_profiles: Vec<ProgressionGrowthProfileDef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LevelThresholdDef {
    pub level: i32,
    pub cumulative_experience: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProgressionGrowthProfileDef {
    pub class_id: String,
    pub hit_points: GrowthRuleDef,
    pub magic_points: Option<GrowthRuleDef>,
    pub stamina_points: GrowthRuleDef,
    pub physical_attribute_adds_by_level: Vec<CombatAddGrowthDef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum GrowthRuleDef {
    Fixed {
        outcomes: Vec<WeightedGrowthOutcomeDef>,
    },
    AttributeBands {
        attribute: GrowthAttributeDef,
        bands: Vec<AttributeGrowthBandDef>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GrowthAttributeDef {
    Strength,
    Constitution,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttributeGrowthBandDef {
    pub minimum_attribute: i32,
    pub outcomes: Vec<WeightedGrowthOutcomeDef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeightedGrowthOutcomeDef {
    pub amount: i32,
    pub weight: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CombatAddGrowthDef {
    pub level: i32,
    pub strength_adds: i32,
    pub dexterity_adds: i32,
}

impl ProgressionRulesDef {
    pub(crate) fn validate(
        &self,
        actors: &[ActorSeedDef],
        services: &[ServiceDefinitionDef],
        errors: &mut Vec<String>,
    ) {
        self.validate_thresholds(errors);

        let supported_minimum = self.level_thresholds.first().map(|row| row.level);
        let supported_maximum = self.level_thresholds.last().map(|row| row.level);
        let mut profile_ids = HashSet::new();
        for (index, profile) in self.growth_profiles.iter().enumerate() {
            let prefix = format!("rules.progression.growth_profiles[{index}]");
            if profile.class_id.trim().is_empty() {
                errors.push(format!("{prefix}.class_id must be non-empty"));
            } else if !profile_ids.insert(profile.class_id.as_str()) {
                errors.push(format!("{prefix}.class_id must be unique"));
            }
            validate_attribute_rule(
                &profile.hit_points,
                GrowthAttributeDef::Constitution,
                &format!("{prefix}.hit_points"),
                errors,
            );
            if let Some(magic_points) = &profile.magic_points {
                if !matches!(magic_points, GrowthRuleDef::Fixed { .. }) {
                    errors.push(format!("{prefix}.magic_points must use kind fixed"));
                }
                validate_growth_rule(magic_points, &format!("{prefix}.magic_points"), errors);
            }
            validate_attribute_rule(
                &profile.stamina_points,
                GrowthAttributeDef::Strength,
                &format!("{prefix}.stamina_points"),
                errors,
            );
            let mut previous_level = None;
            let mut seen_levels = HashSet::new();
            for (row_index, row) in profile.physical_attribute_adds_by_level.iter().enumerate() {
                let row_prefix = format!("{prefix}.physical_attribute_adds_by_level[{row_index}]");
                if row.level <= 0 {
                    errors.push(format!("{row_prefix}.level must be positive"));
                }
                if supported_minimum.is_some_and(|minimum| row.level < minimum)
                    || supported_maximum.is_some_and(|maximum| row.level > maximum)
                {
                    errors.push(format!(
                        "{row_prefix}.level must be within the authored threshold range"
                    ));
                }
                if previous_level.is_some_and(|previous| row.level <= previous) {
                    errors.push(format!(
                        "{row_prefix}.level must be strictly ascending in authored order"
                    ));
                }
                previous_level = Some(row.level);
                if !seen_levels.insert(row.level) {
                    errors.push(format!("{row_prefix}.level must be unique"));
                }
                if row.strength_adds < 0 || row.dexterity_adds < 0 {
                    errors.push(format!("{row_prefix} additions must be non-negative"));
                }
                if row.strength_adds == 0 && row.dexterity_adds == 0 {
                    errors.push(format!(
                        "{row_prefix} must contain at least one positive addition"
                    ));
                }
            }
        }

        let mut required_classes = HashSet::new();
        for actor in actors {
            if let Some(class_id) = actor.effective_current_class_id()
                && !class_id.trim().is_empty()
            {
                required_classes.insert(class_id);
            }
        }
        for service in services {
            for capability in &service.capabilities {
                if let ServiceCapabilityDef::ClassPromotion { transaction, .. } = capability {
                    for reward in &transaction.rewards {
                        if let crate::content::TransactionRewardDef::Class { to_class_id, .. } =
                            reward
                            && !to_class_id.trim().is_empty()
                        {
                            required_classes.insert(to_class_id.as_str());
                        }
                    }
                }
            }
        }
        for class_id in required_classes {
            if !profile_ids.contains(class_id) {
                errors.push(format!(
                    "rules.progression.growth_profiles must contain class_id {class_id:?}"
                ));
            }
        }

        for (index, actor) in actors.iter().enumerate() {
            let progression = actor
                .character
                .as_ref()
                .map(|character| {
                    (
                        character.progression.level,
                        character.progression.experience,
                    )
                })
                .or_else(|| {
                    actor
                        .starter_character
                        .as_ref()
                        .map(|starter| (starter.progression.level, starter.progression.experience))
                });
            if let Some((level, experience)) = progression {
                self.validate_character_progression(
                    level,
                    experience,
                    &format!("actors[{index}]"),
                    errors,
                );
            }
        }
    }

    fn validate_thresholds(&self, errors: &mut Vec<String>) {
        if self.level_thresholds.len() < 2 {
            errors.push(
                "rules.progression.level_thresholds must contain at least two rows".to_string(),
            );
            return;
        }
        let mut previous_level = None;
        let mut previous_experience = None;
        for (index, row) in self.level_thresholds.iter().enumerate() {
            let prefix = format!("rules.progression.level_thresholds[{index}]");
            if row.level <= 0 {
                errors.push(format!("{prefix}.level must be positive"));
            }
            if let Some(previous) = previous_level
                && row.level != previous + 1
            {
                errors.push(format!(
                    "{prefix}.level must be consecutive in authored order"
                ));
            }
            if index == 0 {
                if row.cumulative_experience < 0 {
                    errors.push(format!(
                        "{prefix}.cumulative_experience must be non-negative"
                    ));
                }
            } else if previous_experience
                .is_some_and(|previous| row.cumulative_experience <= previous)
            {
                errors.push(format!(
                    "{prefix}.cumulative_experience must be strictly increasing"
                ));
            }
            previous_level = Some(row.level);
            previous_experience = Some(row.cumulative_experience);
        }
    }

    fn validate_character_progression(
        &self,
        level: i32,
        experience: i64,
        actor_prefix: &str,
        errors: &mut Vec<String>,
    ) {
        let Some(first) = self.level_thresholds.first() else {
            return;
        };
        let Some(last) = self.level_thresholds.last() else {
            return;
        };
        if level < first.level || level > last.level {
            errors.push(format!(
                "{actor_prefix} progression level must be within authored threshold range"
            ));
            return;
        }
        let earned_level = self
            .level_thresholds
            .iter()
            .rev()
            .find(|row| experience >= row.cumulative_experience)
            .map(|row| row.level);
        if earned_level.is_none_or(|earned| level > earned) {
            errors.push(format!(
                "{actor_prefix} progression level must not exceed the XP-earned level"
            ));
        }
    }
}

fn validate_attribute_rule(
    rule: &GrowthRuleDef,
    expected_attribute: GrowthAttributeDef,
    prefix: &str,
    errors: &mut Vec<String>,
) {
    match rule {
        GrowthRuleDef::AttributeBands { attribute, .. } if *attribute == expected_attribute => {}
        _ => errors.push(format!(
            "{prefix} must use attribute_bands with attribute {}",
            match expected_attribute {
                GrowthAttributeDef::Strength => "strength",
                GrowthAttributeDef::Constitution => "constitution",
            }
        )),
    }
    validate_growth_rule(rule, prefix, errors);
}

fn validate_growth_rule(rule: &GrowthRuleDef, prefix: &str, errors: &mut Vec<String>) {
    match rule {
        GrowthRuleDef::Fixed { outcomes } => validate_outcomes(outcomes, prefix, errors),
        GrowthRuleDef::AttributeBands { bands, .. } => {
            if bands.is_empty() {
                errors.push(format!("{prefix}.bands must be non-empty"));
                return;
            }
            let mut previous_minimum = None;
            for (index, band) in bands.iter().enumerate() {
                let band_prefix = format!("{prefix}.bands[{index}]");
                if index == 0 && band.minimum_attribute != 0 {
                    errors.push(format!("{band_prefix}.minimum_attribute must be zero"));
                }
                if band.minimum_attribute < 0 {
                    errors.push(format!(
                        "{band_prefix}.minimum_attribute must be non-negative"
                    ));
                }
                if previous_minimum.is_some_and(|previous| band.minimum_attribute <= previous) {
                    errors.push(format!(
                        "{band_prefix}.minimum_attribute must be strictly increasing"
                    ));
                }
                previous_minimum = Some(band.minimum_attribute);
                validate_outcomes(&band.outcomes, &band_prefix, errors);
            }
        }
    }
}

fn validate_outcomes(
    outcomes: &[WeightedGrowthOutcomeDef],
    prefix: &str,
    errors: &mut Vec<String>,
) {
    if outcomes.is_empty() {
        errors.push(format!("{prefix}.outcomes must be non-empty"));
        return;
    }
    let mut amounts = HashSet::new();
    let mut total_weight = 0_u32;
    for (index, outcome) in outcomes.iter().enumerate() {
        let outcome_prefix = format!("{prefix}.outcomes[{index}]");
        if outcome.amount <= 0 {
            errors.push(format!("{outcome_prefix}.amount must be positive"));
        }
        if !amounts.insert(outcome.amount) {
            errors.push(format!("{outcome_prefix}.amount must be unique"));
        }
        if outcome.weight == 0 {
            errors.push(format!("{outcome_prefix}.weight must be positive"));
        }
        let Some(next) = total_weight.checked_add(outcome.weight) else {
            errors.push(format!("{prefix}.outcome weights must not overflow u32"));
            return;
        };
        total_weight = next;
    }
}

use super::{
    combat_rules_from_def, magic_rules_from_def, profession_action_from_def,
    progression_rules_from_def, service_capability_from_def, summon_template_from_def,
};
use crate::content::SelectedCatalog;
use crate::engine::{CatalogItem, GameCatalog};
use crate::model::{
    ActorDefinition, ActorMagicResistanceState, BankDefinition, BankId, BurdenRules,
    LockerVaultDefinition, LockerVaultId, MovementRules, PhysicalDamageAffinity,
    PhysicalDamageKind, QuestDefinition, QuestId, QuestStage, QuestStageId, ResourceRules,
    ServiceDefinition, SkillRules, SpellCatalogEntry, TerrainState, TerrainTraversal,
    TrainingRules, WorldRules,
};

pub(super) fn compile(source: &SelectedCatalog) -> GameCatalog {
    let terrains = source
        .terrains
        .iter()
        .map(|terrain| {
            let (passable, move_cost, blocks_sight, traversal, unresolved) =
                match &terrain.navigation {
                    crate::content::TerrainNavigationDef::Walk {
                        move_cost,
                        blocks_sight,
                    } => (
                        true,
                        Some(*move_cost),
                        *blocks_sight,
                        Some(TerrainTraversal::Walk),
                        false,
                    ),
                    crate::content::TerrainNavigationDef::Swim {
                        move_cost,
                        blocks_sight,
                    } => (
                        true,
                        Some(*move_cost),
                        *blocks_sight,
                        Some(TerrainTraversal::Swim),
                        false,
                    ),
                    crate::content::TerrainNavigationDef::Blocked { blocks_sight } => {
                        (false, None, *blocks_sight, None, false)
                    }
                    crate::content::TerrainNavigationDef::Unresolved { .. } => {
                        (false, None, true, None, true)
                    }
                };
            (
                terrain.id.clone(),
                TerrainState {
                    id: terrain.id.clone(),
                    name: terrain.name.clone(),
                    passable,
                    move_cost,
                    blocks_sight,
                    traversal,
                    unresolved,
                },
            )
        })
        .collect();
    let item_catalog = source
        .items
        .iter()
        .map(|item| {
            (
                item.id.clone(),
                CatalogItem {
                    name: item.name.clone(),
                    kind: item.kind.clone(),
                    category: item.category.clone(),
                    weapon: item.weapon.clone(),
                    armor: item.armor.clone(),
                    valid_placements: item.valid_placements.clone(),
                    capability: item.capability.clone(),
                    economy: item.economy.clone(),
                },
            )
        })
        .collect();

    let consumable_heals = source
        .items
        .iter()
        .filter(|item| item.kind == "consumable")
        .filter_map(|item| {
            item.consumable
                .as_ref()
                .map(|consumable| (item.id.clone(), consumable.heal_per_round))
        })
        .collect();

    let spells = source
        .spells
        .iter()
        .map(|spell| (spell.id.clone(), spell.clone()))
        .collect();
    let spell_catalog = source
        .spells
        .iter()
        .filter_map(|spell| {
            spell.catalog_entry.as_ref().map(|entry| {
                (
                    spell.id.clone(),
                    SpellCatalogEntry {
                        spell_id: spell.id.clone(),
                        row_id: entry.row_id.clone(),
                        topic_id: entry.topic_id.clone(),
                        acquisition_row_id: entry.acquisition_row_id.clone(),
                        variant_id: entry.variant_id.clone(),
                        effect_family: entry.effect_family,
                        target_kind: entry.target_kind,
                        state: entry.state,
                        open_question_ids: entry.open_question_ids.clone(),
                        resistance_tags: entry.resistance_tags.clone(),
                        resistance_mitigation_mode: entry.resistance_mitigation_mode,
                        client_row_id: entry.client_row_id.clone(),
                        client_spell_id: entry.client_spell_id,
                        client_verb_type: entry.client_verb_type,
                        client_powerable: entry.client_powerable,
                        client_spell_poem_id: entry.client_spell_poem_id,
                        client_offensive: entry.client_offensive,
                    },
                )
            })
        })
        .collect();

    let affinity_profiles = source
        .physical_damage_affinity_profiles
        .iter()
        .map(|profile| {
            let response = |kind| {
                let row = profile
                    .responses
                    .iter()
                    .find(|row| row.damage_kind == kind)
                    .expect("validated affinity response");
                (row.numerator, row.denominator)
            };
            let (cutting_numerator, cutting_denominator) = response(PhysicalDamageKind::Cutting);
            let (piercing_numerator, piercing_denominator) = response(PhysicalDamageKind::Piercing);
            let (crushing_numerator, crushing_denominator) = response(PhysicalDamageKind::Crushing);
            (
                profile.id.clone(),
                PhysicalDamageAffinity {
                    cutting_numerator,
                    cutting_denominator,
                    piercing_numerator,
                    piercing_denominator,
                    crushing_numerator,
                    crushing_denominator,
                },
            )
        })
        .collect::<std::collections::HashMap<_, _>>();
    let actor_definitions = source
        .actor_definitions
        .iter()
        .map(|definition| {
            let affinity = *affinity_profiles
                .get(&definition.physical_damage_affinity_profile_id)
                .expect("validated actor affinity profile");
            (
                definition.id.clone(),
                ActorDefinition {
                    id: definition.id.clone(),
                    kind: definition.kind,
                    name: definition.name.clone(),
                    creature_traits: definition.creature_traits.clone(),
                    social: super::social_profile_from_def(&definition.social),
                    stats: definition.stats.clone(),
                    magic_resistance: ActorMagicResistanceState {
                        natural_save_twentieths: definition
                            .magic_resistance
                            .natural_save_twentieths,
                        evidence_state: definition.magic_resistance.evidence_state,
                    },
                    corpse_disposition: definition.death.remains,
                    ai: definition.ai.as_ref().map(super::actor_ai_from_def),
                    xp_value: definition.xp_value.unwrap_or(0),
                    physical_damage_affinity_profile_id: definition
                        .physical_damage_affinity_profile_id
                        .clone(),
                    physical_damage_affinity: affinity,
                    scavenging_profile: definition.scavenging_profile_id.as_ref().map(|id| {
                        *source
                            .scavenging_profiles
                            .iter()
                            .find_map(|(key, profile)| (key.as_str() == id).then_some(profile))
                            .expect("validated actor scavenging profile")
                    }),
                    monster_abilities: definition
                        .monster_abilities
                        .iter()
                        .map(super::monster_ability_from_def)
                        .collect(),
                },
            )
        })
        .collect::<std::collections::HashMap<_, _>>();
    let loot_tables = source
        .loot_tables
        .iter()
        .map(|row| (row.id.clone(), row.clone()))
        .collect();
    let spawn_groups = source
        .spawn_groups
        .iter()
        .map(|row| (row.id.clone(), row.clone()))
        .collect();
    let lair_definitions = source
        .lair_definitions
        .iter()
        .map(|row| (row.id.clone(), row.clone()))
        .collect();

    let summon_templates = source
        .summon_templates
        .iter()
        .map(|template| {
            (
                template.id.clone(),
                summon_template_from_def(template, &item_catalog),
            )
        })
        .collect();
    let profession_actions = source
        .profession_actions
        .iter()
        .map(profession_action_from_def)
        .collect();
    let quests = source
        .quests
        .iter()
        .map(|quest| {
            let quest_id = QuestId::new(&quest.id);
            let stages = quest
                .stages
                .iter()
                .map(|stage| {
                    let stage_id = QuestStageId::new(&stage.id);
                    (
                        stage_id.clone(),
                        QuestStage {
                            id: stage_id,
                            label: stage.label.clone(),
                            terminal: stage.terminal,
                        },
                    )
                })
                .collect();
            (
                quest_id.clone(),
                QuestDefinition {
                    id: quest_id,
                    title: quest.title.clone(),
                    stages,
                },
            )
        })
        .collect();

    let service_definitions = source
        .service_definitions
        .iter()
        .map(|definition| ServiceDefinition {
            id: definition.id.clone(),
            name: definition.name.clone(),
            capabilities: definition
                .capabilities
                .iter()
                .map(service_capability_from_def)
                .collect(),
        })
        .collect();
    let bank_definitions = source
        .banks
        .iter()
        .map(|bank| {
            (
                BankId::new(&bank.id),
                BankDefinition {
                    transaction_cap_gold: bank.transaction_cap_gold,
                },
            )
        })
        .collect();
    let locker_vault_definitions = source
        .locker_vaults
        .iter()
        .map(|vault| {
            (
                LockerVaultId::new(&vault.id),
                LockerVaultDefinition {
                    capacity: vault.capacity,
                },
            )
        })
        .collect();

    let rules = &source.rules;
    GameCatalog {
        boundary_policy: if source.clean_content {
            crate::content::ContentBoundaryPolicy::Clean
        } else {
            crate::content::ContentBoundaryPolicy::InternalParity
        },
        profile_key: source.profile_key.clone(),
        rules: WorldRules {
            progression: progression_rules_from_def(&rules.progression),
            movement: MovementRules {
                controlled_path_points: rules.movement.controlled_path_points,
                automatic_step_points: rules.movement.automatic_step_points,
            },
            burden: BurdenRules {
                coin_burden_per_gold: rules.burden.coin_burden_per_gold,
                lightly_loaded_max_per_strength: rules.burden.lightly_loaded_max_per_strength,
                moderately_loaded_max_per_strength: rules.burden.moderately_loaded_max_per_strength,
                heavily_loaded_max_per_strength: rules.burden.heavily_loaded_max_per_strength,
            },
            resources: ResourceRules {
                recovery_interval_units: rules.resources.recovery_interval_units,
                active_hp_recovery: rules.resources.active_hp_recovery,
                inactive_hp_recovery: rules.resources.inactive_hp_recovery,
                inactive_stamina_recovery: rules.resources.inactive_stamina_recovery,
                mp_recovery: rules.resources.mp_recovery,
                normal_movement_stamina_cost: rules.resources.normal_movement_stamina_cost,
                rapid_movement_stamina_cost: rules.resources.rapid_movement_stamina_cost,
            },
            magic: magic_rules_from_def(&rules.magic),
            skills: SkillRules {
                base_learning_rate: rules.skills.base_learning_rate,
                practice_thresholds: rules.skills.practice_thresholds.clone(),
                training: TrainingRules {
                    gold_per_learning_rate: rules.skills.training.gold_per_learning_rate,
                    experience_per_learning_rate: rules
                        .skills
                        .training
                        .experience_per_learning_rate,
                    maximum_learning_rates: rules.skills.training.maximum_learning_rates.clone(),
                },
            },
            combat: combat_rules_from_def(&rules.combat),
        },
        consumable_heals,
        item_catalog,
        skill_catalog: source.skill_catalog.clone(),
        service_definitions,
        bank_definitions,
        locker_vault_definitions,
        quests,
        profession_actions,
        spells,
        spell_catalog,
        summon_templates,
        actor_definitions,
        loot_tables,
        spawn_groups,
        lair_definitions,
        terrains,
    }
}

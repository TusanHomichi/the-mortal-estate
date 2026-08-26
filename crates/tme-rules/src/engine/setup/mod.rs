use std::sync::Arc;

use super::{CatalogItem, Engine, GameDefinition, StepError};
use crate::combat::{
    CombatAttackModeRules, CombatBlockRules, CombatDamageRules, CombatHitRules,
    CombatJumpkickRules, CombatKickRules, CombatPracticeRules, CombatRules, CombatTuningStatus,
    CombatWoundRules,
};
use crate::content::{
    ActorAiDef, ActorSeedDef, CatalogProfileKey, CatalogV6, ServiceCapabilityDef,
    TransactionCostDef, TransactionDef, TransactionRequirementDef, TransactionRewardDef,
    ValidationError, WorldSeedDef, WorldTemplateV3,
};
use crate::events::Event;
use crate::model::{
    ActiveEffectStackingPolicy, ActiveEffectState, ActorAiState, ActorAwarenessState,
    ActorDefinition, ActorId, ActorLifeState, ActorResourceActivity, ActorState, ActorTimingState,
    AttributeGrowthBand, BankCapability, BankId, CarriedLayout, CharacterId, CharacterSheetV1,
    ClassPromotionCapability, CombatAddGrowth, EcologyActorOrigin, GrowthAttribute, GrowthRule,
    HideActionConfig, HideBreakTrigger, ItemInstanceState, ItemKnowledgeState,
    ItemServiceCapability, ItemServiceOperation, LevelThreshold, LockerCapability, LockerVaultId,
    LogicalTime, MartialHandBlockConfig, MerchantCapability, MonsterAbilityKind,
    MonsterAbilityState, MonsterAbilityTargetPolicy, NpcState, PlayerSalesPolicy,
    ProfessionActionConfig, ProgressionGrowthProfile, ProgressionRules, ServiceCapability,
    ServiceTransactionCapability, SkillCritiqueCapability, SkillTrainingCapability,
    SocialAlignmentSource, SocialBehavior, SocialNature, SocialOwnerRelation, SocialProfile,
    SpellTeaching, SpellTeachingCapability, SummonTemplate, SummonedActorState, TrainingOffer,
    Transaction, TransactionCost, TransactionRequirement, TransactionReward, WeightedGrowthOutcome,
    WorldPosition,
};
use crate::rng::DeterministicRng;

mod catalog;
mod world_state;
mod world_template;

pub(super) struct ActorInstanceState {
    pub id: ActorId,
    pub location: WorldPosition,
    pub hp: i32,
    pub mp: i32,
    pub stamina: i32,
    pub timing: ActorTimingState,
    pub attack_ready_at: LogicalTime,
    pub carried: CarriedLayout,
    pub npc: Option<NpcState>,
    pub character_id: Option<CharacterId>,
    pub character: Option<CharacterSheetV1>,
    pub active_effects: Vec<ActiveEffectState>,
    pub summoned: Option<SummonedActorState>,
    pub ecology_origin: Option<EcologyActorOrigin>,
}

pub(super) fn actor_state_from_definition(
    definition: &ActorDefinition,
    instance: ActorInstanceState,
) -> ActorState {
    ActorState {
        id: instance.id,
        definition_id: definition.id.clone(),
        kind: definition.kind,
        creature_traits: definition.creature_traits.clone(),
        social: definition.social.clone(),
        name: definition.name.clone(),
        location: instance.location.clone(),
        home_location: instance.location,
        stats: definition.stats.clone(),
        magic_resistance: definition.magic_resistance.clone(),
        physical_damage_affinity_profile_id: definition.physical_damage_affinity_profile_id.clone(),
        physical_damage_affinity: definition.physical_damage_affinity,
        hp: instance.hp,
        mp: instance.mp,
        stamina: instance.stamina,
        life_state: ActorLifeState::Alive,
        corpse_disposition: definition.corpse_disposition,
        resource_activity: ActorResourceActivity::default(),
        timing: instance.timing,
        attack_ready_at: instance.attack_ready_at,
        carried: instance.carried,
        ai: definition.ai.clone(),
        npc: instance.npc,
        xp_value: definition.xp_value,
        character_id: instance.character_id,
        character: instance.character,
        active_effects: instance.active_effects,
        balm_effect: None,
        warmed_spell: None,
        monster_abilities: definition.monster_abilities.clone(),
        summoned: instance.summoned,
        ecology_origin: instance.ecology_origin,
    }
}

fn active_effect_from_def(def: &crate::content::ActiveEffectDef) -> ActiveEffectState {
    let stacking = match def.stacking.as_str() {
        "stack_instance" => ActiveEffectStackingPolicy::StackInstance,
        "refresh_duration" => ActiveEffectStackingPolicy::RefreshDuration,
        _ => ActiveEffectStackingPolicy::ReplaceSameKind,
    };
    ActiveEffectState {
        instance_id: def.instance_id.clone(),
        effect_id: def.effect_id.clone(),
        source: def.source.clone(),
        source_actor_id: None,
        hostile_authority: None,
        spell_damage_credit: None,
        kind: def.kind.clone(),
        tags: def.tags.clone(),
        potency: def.potency,
        remaining_rounds: def
            .remaining_rounds
            .and_then(|rounds| u32::try_from(rounds).ok()),
        until_condition: def.until_condition.clone(),
        stacking,
        start_delay_rounds: u32::try_from(def.start_delay_rounds).unwrap_or(0),
        tick_interval_rounds: u32::try_from(def.tick_interval_rounds).unwrap_or(1),
        suppresses_action: def.suppresses_action,
        resistance_boosts: def.resistance_boosts.clone(),
        last_ticked_at: LogicalTime::ZERO,
    }
}

fn actor_ai_from_def(def: &ActorAiDef) -> ActorAiState {
    ActorAiState {
        behavior: def.behavior,
        cadence_units: def.cadence_units,
        aggro_radius: def.aggro_radius,
        leash_range: def.leash_range,
        awareness: ActorAwarenessState {
            policy: def.awareness.policy(),
            remembered: None,
        },
        physical_attack_modes: def.physical_attack_modes.clone(),
        returning_home: false,
    }
}

fn social_profile_from_def(def: &crate::content::SocialProfileDef) -> SocialProfile {
    SocialProfile {
        alignment_source: match def.alignment_source {
            crate::content::SocialAlignmentSourceDef::Character {} => {
                SocialAlignmentSource::Character {}
            }
            crate::content::SocialAlignmentSourceDef::Inherent { alignment } => {
                SocialAlignmentSource::Inherent { alignment }
            }
        },
        nature: match def.nature {
            crate::content::SocialNatureDef::Human => SocialNature::Human,
            crate::content::SocialNatureDef::Animal => SocialNature::Animal,
            crate::content::SocialNatureDef::Other => SocialNature::Other,
        },
        behavior: match def.behavior {
            crate::content::SocialBehaviorDef::Adventurer => SocialBehavior::Adventurer,
            crate::content::SocialBehaviorDef::Civilian => SocialBehavior::Civilian,
            crate::content::SocialBehaviorDef::TownEnforcer => SocialBehavior::TownEnforcer,
            crate::content::SocialBehaviorDef::AlignmentCreature => {
                SocialBehavior::AlignmentCreature
            }
            crate::content::SocialBehaviorDef::Passive => SocialBehavior::Passive,
        },
        owner_relation: match def.owner_relation {
            crate::content::SocialOwnerRelationDef::None => SocialOwnerRelation::None,
            crate::content::SocialOwnerRelationDef::Summoner => SocialOwnerRelation::Summoner,
        },
    }
}

fn npc_state_from_def(def: &crate::content::NpcDef) -> crate::model::NpcState {
    crate::model::NpcState {
        follow_cadence_units: def.follow_cadence_units,
        interactions: def
            .interactions
            .iter()
            .map(|interaction| crate::model::NpcInteraction {
                transaction: transaction_from_def(&interaction.transaction),
                response: interaction.response.clone(),
                outcome: match &interaction.outcome {
                    crate::content::NpcInteractionOutcomeDef::Speak => {
                        crate::model::NpcInteractionOutcome::Speak
                    }
                    crate::content::NpcInteractionOutcomeDef::BeginFollow => {
                        crate::model::NpcInteractionOutcome::BeginFollow
                    }
                    crate::content::NpcInteractionOutcomeDef::EndFollow => {
                        crate::model::NpcInteractionOutcome::EndFollow
                    }
                    crate::content::NpcInteractionOutcomeDef::CompleteEscort { npc_actor_id } => {
                        crate::model::NpcInteractionOutcome::CompleteEscort {
                            npc_actor_id: npc_actor_id.clone(),
                        }
                    }
                    crate::content::NpcInteractionOutcomeDef::Climb { direction } => {
                        crate::model::NpcInteractionOutcome::Climb {
                            direction: *direction,
                        }
                    }
                },
            })
            .collect(),
        following_character_id: None,
    }
}

fn monster_ability_from_def(def: &crate::content::MonsterAbilityDef) -> MonsterAbilityState {
    MonsterAbilityState {
        id: def.id.clone(),
        kind: match def.kind.as_str() {
            "spell" => MonsterAbilityKind::Spell,
            _ => MonsterAbilityKind::SpecialAttack,
        },
        spell_id: def.spell_id.clone(),
        cooldown_rounds: def.cooldown_rounds,
        target_policy: match def.target_policy.as_deref() {
            Some("self") => MonsterAbilityTargetPolicy::SelfTarget,
            _ => MonsterAbilityTargetPolicy::NearestHostile,
        },
        ready_at: LogicalTime::ZERO,
    }
}

fn service_capability_from_def(def: &ServiceCapabilityDef) -> ServiceCapability {
    match def {
        ServiceCapabilityDef::SkillTraining { id, offers } => {
            ServiceCapability::SkillTraining(SkillTrainingCapability {
                id: id.clone(),
                offers: offers
                    .iter()
                    .map(|offer| TrainingOffer {
                        track_id: offer.track_id.clone(),
                        eligible_class_ids: offer.eligible_class_ids.clone(),
                        minimum_category_level: offer.minimum_category_level,
                        maximum_category_level: offer.maximum_category_level,
                    })
                    .collect(),
            })
        }
        ServiceCapabilityDef::SkillCritique { id } => {
            ServiceCapability::SkillCritique(SkillCritiqueCapability { id: id.clone() })
        }
        ServiceCapabilityDef::SpellTeaching {
            id,
            training_capability_id,
            teachings,
        } => ServiceCapability::SpellTeaching(SpellTeachingCapability {
            id: id.clone(),
            training_capability_id: training_capability_id.clone(),
            teachings: teachings
                .iter()
                .map(|teaching| SpellTeaching {
                    spell_id: teaching.spell_id.clone(),
                })
                .collect(),
        }),
        ServiceCapabilityDef::ClassPromotion { id, transaction } => {
            ServiceCapability::ClassPromotion(ClassPromotionCapability {
                id: id.clone(),
                transaction: transaction_from_def(transaction),
            })
        }
        ServiceCapabilityDef::ServiceTransaction { id, transactions } => {
            ServiceCapability::ServiceTransaction(ServiceTransactionCapability {
                id: id.clone(),
                transactions: transactions.iter().map(transaction_from_def).collect(),
            })
        }
        ServiceCapabilityDef::Merchant {
            id, player_sales, ..
        } => ServiceCapability::Merchant(MerchantCapability {
            id: id.clone(),
            player_sales: player_sales.as_ref().map(|policy| PlayerSalesPolicy {
                pawn_listing_multiplier: policy.pawn_listing_multiplier,
            }),
        }),
        ServiceCapabilityDef::ItemService { id, operations } => {
            ServiceCapability::ItemService(ItemServiceCapability {
                id: id.clone(),
                operations: operations
                    .iter()
                    .map(|operation| match operation {
                        crate::content::ItemServiceOperationDef::Appraise {} => {
                            ItemServiceOperation::Appraise
                        }
                        crate::content::ItemServiceOperationDef::Identify { gold_cost } => {
                            ItemServiceOperation::Identify {
                                gold_cost: *gold_cost,
                            }
                        }
                        crate::content::ItemServiceOperationDef::EnchantWeapon {
                            gold_cost,
                            combat_add_rating_bonus,
                            tags,
                            remaining_rounds,
                        } => ItemServiceOperation::EnchantWeapon {
                            gold_cost: *gold_cost,
                            combat_add_rating_bonus: *combat_add_rating_bonus,
                            tags: tags.clone(),
                            remaining_rounds: *remaining_rounds,
                        },
                    })
                    .collect(),
            })
        }
        ServiceCapabilityDef::Restoration { id, operations } => {
            ServiceCapability::Restoration(crate::model::RestorationCapability {
                id: id.clone(),
                operations: operations
                    .iter()
                    .map(|operation| crate::model::RestorationOperation {
                        transaction: transaction_from_def(&operation.transaction),
                        outcome: match &operation.outcome {
                            crate::content::RestorationOutcomeDef::RestoreResource { resource } => {
                                crate::model::RestorationOutcome::RestoreResource {
                                    resource: *resource,
                                }
                            }
                            crate::content::RestorationOutcomeDef::CureStatus { status } => {
                                crate::model::RestorationOutcome::CureStatus { status: *status }
                            }
                            crate::content::RestorationOutcomeDef::PriestResurrection => {
                                crate::model::RestorationOutcome::PriestResurrection
                            }
                        },
                    })
                    .collect(),
            })
        }
        ServiceCapabilityDef::Bank { id, bank_id } => ServiceCapability::Bank(BankCapability {
            id: id.clone(),
            bank_id: BankId::new(bank_id),
        }),
        ServiceCapabilityDef::Locker { id, vault_id } => {
            ServiceCapability::Locker(LockerCapability {
                id: id.clone(),
                vault_id: LockerVaultId::new(vault_id),
            })
        }
    }
}

fn transaction_from_def(def: &TransactionDef) -> Transaction {
    Transaction {
        id: def.id.clone(),
        label: def.label.clone(),
        requirements: def
            .requirements
            .iter()
            .map(|requirement| match requirement {
                TransactionRequirementDef::CurrentClass { class_id } => {
                    TransactionRequirement::CurrentClass {
                        class_id: class_id.clone(),
                    }
                }
                TransactionRequirementDef::MinimumLevel { level } => {
                    TransactionRequirement::MinimumLevel { level: *level }
                }
                TransactionRequirementDef::ExactKarma { karma_points } => {
                    TransactionRequirement::ExactKarma {
                        karma_points: *karma_points,
                    }
                }
                TransactionRequirementDef::ExactAlignment { alignment } => {
                    TransactionRequirement::ExactAlignment {
                        alignment: *alignment,
                    }
                }
                TransactionRequirementDef::MinimumSkillLevel { track_id, level } => {
                    TransactionRequirement::MinimumSkillLevel {
                        track_id: track_id.clone(),
                        level: *level,
                    }
                }
                TransactionRequirementDef::MinimumCarriedGold { amount } => {
                    TransactionRequirement::MinimumCarriedGold { amount: *amount }
                }
                TransactionRequirementDef::CarriedItem {
                    item_definition_id,
                    quantity,
                } => TransactionRequirement::CarriedItem {
                    item_definition_id: item_definition_id.clone(),
                    quantity: *quantity,
                },
                TransactionRequirementDef::CarriedPositionEmpty { position } => {
                    TransactionRequirement::CarriedPositionEmpty {
                        position: *position,
                    }
                }
                TransactionRequirementDef::SpellUnknown { spell_id } => {
                    TransactionRequirement::SpellUnknown {
                        spell_id: spell_id.clone(),
                    }
                }
                TransactionRequirementDef::QuestUnstarted { quest_id } => {
                    TransactionRequirement::QuestUnstarted {
                        quest_id: crate::model::QuestId::new(quest_id),
                    }
                }
                TransactionRequirementDef::QuestAtStage { quest_id, stage_id } => {
                    TransactionRequirement::QuestAtStage {
                        quest_id: crate::model::QuestId::new(quest_id),
                        stage_id: crate::model::QuestStageId::new(stage_id),
                    }
                }
                TransactionRequirementDef::NpcAccompanying { npc_actor_id } => {
                    TransactionRequirement::NpcAccompanying {
                        npc_actor_id: npc_actor_id.clone(),
                    }
                }
            })
            .collect(),
        costs: def
            .costs
            .iter()
            .map(|cost| match cost {
                TransactionCostDef::CarriedGold { amount } => {
                    TransactionCost::CarriedGold { amount: *amount }
                }
                TransactionCostDef::SelectedCarriedItem { quantity } => {
                    TransactionCost::SelectedCarriedItem {
                        quantity: *quantity,
                    }
                }
            })
            .collect(),
        rewards: def
            .rewards
            .iter()
            .map(|reward| match reward {
                TransactionRewardDef::Experience { amount } => {
                    TransactionReward::Experience { amount: *amount }
                }
                TransactionRewardDef::Item {
                    item_instance_id,
                    item_definition_id,
                    position,
                } => TransactionReward::Item {
                    item_instance_id: item_instance_id.clone(),
                    item_definition_id: item_definition_id.clone(),
                    position: *position,
                },
                TransactionRewardDef::Class {
                    to_class_id,
                    to_class_display,
                } => TransactionReward::Class {
                    to_class_id: to_class_id.clone(),
                    to_class_display: to_class_display.clone(),
                },
                TransactionRewardDef::Spell { spell_id } => TransactionReward::Spell {
                    spell_id: spell_id.clone(),
                },
                TransactionRewardDef::QuestStage { quest_id, stage_id } => {
                    TransactionReward::QuestStage {
                        quest_id: crate::model::QuestId::new(quest_id),
                        stage_id: crate::model::QuestStageId::new(stage_id),
                    }
                }
            })
            .collect(),
    }
}

fn carried_layout_from_def(def: &crate::content::CarriedLayoutDef) -> CarriedLayout {
    CarriedLayout {
        items: def
            .items
            .iter()
            .map(|item| (item.position, item.item_instance_id.clone()))
            .collect(),
        gold: def.gold,
    }
}

fn character_sheet_from_actor(actor: &ActorSeedDef) -> Option<CharacterSheetV1> {
    actor.character.clone().or_else(|| {
        actor
            .starter_character
            .as_ref()
            .map(crate::content::StarterCharacterDef::build_character_sheet)
    })
}

fn weighted_outcomes_from_def(
    outcomes: &[crate::content::WeightedGrowthOutcomeDef],
) -> Vec<WeightedGrowthOutcome> {
    outcomes
        .iter()
        .map(|outcome| WeightedGrowthOutcome {
            amount: outcome.amount,
            weight: outcome.weight,
        })
        .collect()
}

fn growth_rule_from_def(def: &crate::content::GrowthRuleDef) -> GrowthRule {
    match def {
        crate::content::GrowthRuleDef::Fixed { outcomes } => GrowthRule::Fixed {
            outcomes: weighted_outcomes_from_def(outcomes),
        },
        crate::content::GrowthRuleDef::AttributeBands { attribute, bands } => {
            GrowthRule::AttributeBands {
                attribute: match attribute {
                    crate::content::GrowthAttributeDef::Strength => GrowthAttribute::Strength,
                    crate::content::GrowthAttributeDef::Constitution => {
                        GrowthAttribute::Constitution
                    }
                },
                bands: bands
                    .iter()
                    .map(|band| AttributeGrowthBand {
                        minimum_attribute: band.minimum_attribute,
                        outcomes: weighted_outcomes_from_def(&band.outcomes),
                    })
                    .collect(),
            }
        }
    }
}

fn progression_rules_from_def(def: &crate::content::ProgressionRulesDef) -> ProgressionRules {
    ProgressionRules {
        level_thresholds: def
            .level_thresholds
            .iter()
            .map(|row| LevelThreshold {
                level: row.level,
                cumulative_experience: row.cumulative_experience,
            })
            .collect(),
        growth_profiles: def
            .growth_profiles
            .iter()
            .map(|profile| {
                (
                    profile.class_id.clone(),
                    ProgressionGrowthProfile {
                        class_id: profile.class_id.clone(),
                        hit_points: growth_rule_from_def(&profile.hit_points),
                        magic_points: profile.magic_points.as_ref().map(growth_rule_from_def),
                        stamina_points: growth_rule_from_def(&profile.stamina_points),
                        physical_attribute_adds_by_level: profile
                            .physical_attribute_adds_by_level
                            .iter()
                            .map(|row| CombatAddGrowth {
                                level: row.level,
                                strength_adds: row.strength_adds,
                                dexterity_adds: row.dexterity_adds,
                            })
                            .collect(),
                    },
                )
            })
            .collect(),
    }
}

fn combat_rules_from_def(def: &crate::content::CombatRulesDef) -> CombatRules {
    CombatRules {
        tuning_status: match def.tuning_status {
            crate::content::CombatTuningStatusDef::OriginalProvisional => {
                CombatTuningStatus::OriginalProvisional
            }
        },
        attack_modes: CombatAttackModeRules {
            kick: CombatKickRules {
                maximum_range: def.attack_modes.kick.maximum_range,
                cooldown_units: def.attack_modes.kick.cooldown_units,
                damage_kind: def.attack_modes.kick.damage_kind,
            },
            jumpkick: CombatJumpkickRules {
                maximum_range_cap: def.attack_modes.jumpkick.maximum_range_cap,
                skill_levels_per_extra_hex: def.attack_modes.jumpkick.skill_levels_per_extra_hex,
                stamina_cost: def.attack_modes.jumpkick.stamina_cost,
                cooldown_units: def.attack_modes.jumpkick.cooldown_units,
                damage_kind: def.attack_modes.jumpkick.damage_kind,
            },
        },
        hit: CombatHitRules {
            base_defender_score: def.hit.base_defender_score,
            attacker_attack_stat_divisor: def.hit.attacker_attack_stat_divisor,
            attacker_skill_level_divisor: def.hit.attacker_skill_level_divisor,
            defender_defense_stat_divisor: def.hit.defender_defense_stat_divisor,
            defender_dexterity_divisor: def.hit.defender_dexterity_divisor,
            non_character_defender_dexterity: def.hit.non_character_defender_dexterity,
        },
        block: CombatBlockRules {
            left_hand_selection_percent: def.block.left_hand_selection_percent,
            shield_percent_per_point: def.block.shield_percent_per_point,
            shield_percent_cap: def.block.shield_percent_cap,
            armor_percent_per_point: def.block.armor_percent_per_point,
            armor_percent_cap: def.block.armor_percent_cap,
            strength_penetration_percent_per_add: def.block.strength_penetration_percent_per_add,
            armor_encumbrance_percent_per_point: def.block.armor_encumbrance_percent_per_point,
            combat_add_penetration_percent_per_rating: def
                .block
                .combat_add_penetration_percent_per_rating,
        },
        fumble: crate::combat::CombatFumbleRules {
            base_percent: def.fumble.base_percent,
            minimum_percent: def.fumble.minimum_percent,
            skill_levels_per_reduction: def.fumble.skill_levels_per_reduction,
        },
        damage: CombatDamageRules {
            minimum_damage: def.damage.minimum_damage,
            roll_variation_modulus: def.damage.roll_variation_modulus,
            moderate_label_min_percent: def.damage.moderate_label_min_percent,
            heavy_label_min_percent: def.damage.heavy_label_min_percent,
            severe_label_min_percent: def.damage.severe_label_min_percent,
        },
        wounds: CombatWoundRules {
            near_death_max_percent: def.wounds.near_death_max_percent,
            badly_wounded_max_percent: def.wounds.badly_wounded_max_percent,
            wounded_max_percent: def.wounds.wounded_max_percent,
        },
        practice: CombatPracticeRules {
            practice_raw_points: def.practice.practice_raw_points,
            life_and_death_raw_points: def.practice.life_and_death_raw_points,
            overwhelming_raw_points: def.practice.overwhelming_raw_points,
            fatal_blow_bonus_raw_points: def.practice.fatal_blow_bonus_raw_points,
            life_and_death_minimum_target_xp_per_attacker_level: def
                .practice
                .life_and_death_minimum_target_xp_per_attacker_level,
            life_and_death_required_at_skill_level: def
                .practice
                .life_and_death_required_at_skill_level,
        },
    }
}

fn magic_rules_from_def(def: &crate::content::MagicRulesDef) -> crate::model::MagicRules {
    let evidence_state = |state| match state {
        crate::content::MagicRuleEvidenceStateDef::OriginalProvisional => {
            crate::model::MagicRuleEvidenceState::OriginalProvisional
        }
        crate::content::MagicRuleEvidenceStateDef::TargetRelease => {
            crate::model::MagicRuleEvidenceState::TargetRelease
        }
    };
    crate::model::MagicRules {
        warmup: crate::model::SpellWarmupRules {
            units: def.warmup.units,
            evidence_state: evidence_state(def.warmup.evidence_state),
        },
        damage_interruption: crate::model::SpellDamageInterruptionRules {
            comparison: match def.damage_interruption.comparison {
                crate::content::DamageInterruptionComparisonDef::StrictlyGreater => {
                    crate::model::DamageInterruptionComparison::StrictlyGreater
                }
            },
            numerator: def.damage_interruption.numerator,
            denominator: def.damage_interruption.denominator,
            evidence_state: evidence_state(def.damage_interruption.evidence_state),
        },
        resistance: crate::model::SpellResistanceRules {
            denominator: def.resistance.denominator,
            denominator_evidence_state: evidence_state(def.resistance.denominator_evidence_state),
            success_comparison: match def.resistance.success_comparison {
                crate::content::MagicSaveComparisonDef::RollAtOrBelow => {
                    crate::model::MagicSaveComparison::RollAtOrBelow
                }
            },
            matching_boost_policy: match def.resistance.matching_boost_policy {
                crate::content::MatchingResistanceBoostPolicyDef::HighestMatching => {
                    crate::model::MatchingResistanceBoostPolicy::HighestMatching
                }
            },
            resolution_evidence_state: evidence_state(def.resistance.resolution_evidence_state),
        },
        casting_practice: crate::model::MagicCastingPracticeRules {
            minimum_raw_points: def.casting_practice.minimum_raw_points,
            raw_points_per_mp: def.casting_practice.raw_points_per_mp,
            primary_attribute_points_per_bonus: def
                .casting_practice
                .primary_attribute_points_per_bonus,
            evidence_state: evidence_state(def.casting_practice.evidence_state),
        },
        thaum_above_skill: crate::model::ThaumAboveSkillRules {
            roll_denominator: def.thaum_above_skill.roll_denominator,
            penalty_per_missing_level: def.thaum_above_skill.penalty_per_missing_level,
            minimum_success_threshold: def.thaum_above_skill.minimum_success_threshold,
            evidence_state: evidence_state(def.thaum_above_skill.evidence_state),
        },
        kill_experience: crate::model::MagicKillExperienceRules {
            directed: crate::model::MagicRewardFraction {
                numerator: def.kill_experience.directed.numerator,
                denominator: def.kill_experience.directed.denominator,
            },
            area_or_illusion: crate::model::MagicRewardFraction {
                numerator: def.kill_experience.area_or_illusion.numerator,
                denominator: def.kill_experience.area_or_illusion.denominator,
            },
            fraction_evidence_state: evidence_state(def.kill_experience.fraction_evidence_state),
            rounding: match def.kill_experience.rounding {
                crate::content::MagicArithmeticRoundingDef::Down => {
                    crate::model::MagicArithmeticRounding::Down
                }
            },
            rounding_evidence_state: evidence_state(def.kill_experience.rounding_evidence_state),
        },
        mp_recovery: crate::model::MagicMpRecoveryRules {
            active_item_policy: match def.mp_recovery.active_item_policy {
                crate::content::ActiveMpRecoveryItemPolicyDef::HighestMultiplier => {
                    crate::model::ActiveMpRecoveryItemPolicy::HighestMultiplier
                }
            },
            rounding: match def.mp_recovery.rounding {
                crate::content::MagicArithmeticRoundingDef::Down => {
                    crate::model::MagicArithmeticRounding::Down
                }
            },
            evidence_state: evidence_state(def.mp_recovery.evidence_state),
        },
        effect_families: crate::model::MagicEffectFamilyRules {
            raise_dead: crate::model::RaiseDeadRules {
                roll_denominator: def.effect_families.raise_dead.roll_denominator,
                success_threshold_per_magic_level: def
                    .effect_families
                    .raise_dead
                    .success_threshold_per_magic_level,
                minimum_success_threshold: def.effect_families.raise_dead.minimum_success_threshold,
                evidence_state: evidence_state(def.effect_families.raise_dead.evidence_state),
            },
        },
    }
}

fn summon_template_from_def(
    def: &crate::content::SummonTemplateDef,
    item_catalog: &std::collections::HashMap<String, CatalogItem>,
) -> SummonTemplate {
    SummonTemplate {
        id: def.id.clone(),
        actor_definition_id: def.actor_definition_id.clone(),
        item_instances: def
            .item_instances
            .iter()
            .map(|(instance_id, instance)| {
                (
                    instance_id.clone(),
                    ItemInstanceState {
                        definition_id: instance.definition_id.clone(),
                        quantity: instance.quantity,
                        knowledge: ItemKnowledgeState {
                            identified: instance.knowledge.identified,
                            appraised: instance.knowledge.appraised,
                        },
                        binding: instance.binding.clone(),
                        bow_readiness: item_catalog
                            .get(&instance.definition_id)
                            .and_then(|item| item.weapon.as_ref())
                            .and_then(|weapon| {
                                (weapon.handedness == crate::model::WeaponHandedness::Bow)
                                    .then_some(crate::model::BowReadiness::Unnocked)
                            }),
                    },
                )
            })
            .collect(),
        carried: carried_layout_from_def(&def.carried),
        active_effects: def
            .active_effects
            .iter()
            .map(active_effect_from_def)
            .collect(),
    }
}

fn profession_action_from_def(def: &crate::content::ProfessionActionDef) -> ProfessionActionConfig {
    ProfessionActionConfig {
        id: def.id.clone(),
        class_ids: def.class_ids.clone(),
        hide: def.hide.as_ref().map(|hide| HideActionConfig {
            effect_id: hide.effect_id.clone(),
            duration_rounds: hide.duration_rounds,
            requires_cover_or_darkness: hide.requires_cover_or_darkness,
            break_on: hide
                .break_on
                .iter()
                .filter_map(|trigger| match trigger.as_str() {
                    "move" => Some(HideBreakTrigger::Move),
                    "attack" => Some(HideBreakTrigger::Attack),
                    "active_item_move" => Some(HideBreakTrigger::ActiveItemMove),
                    "cast" => Some(HideBreakTrigger::Cast),
                    "warm" => Some(HideBreakTrigger::Warm),
                    _ => None,
                })
                .collect(),
            disallow_two_handed: hide.disallow_two_handed,
        }),
        martial_hand_block: def
            .martial_hand_block
            .as_ref()
            .map(|block| MartialHandBlockConfig {
                min_hand_level: block.min_hand_level,
                level_divisor: block.level_divisor,
                max_chance_percent: block.max_chance_percent,
            }),
    }
}

#[derive(Debug, Clone)]
pub struct ValidatedWorldSeed {
    definition: Arc<GameDefinition>,
    seed: WorldSeedDef,
}

impl ValidatedWorldSeed {
    pub fn new(
        definition: Arc<GameDefinition>,
        seed: WorldSeedDef,
    ) -> Result<Self, ValidationError> {
        seed.validate_with_context(definition.as_ref())?;
        Ok(Self { definition, seed })
    }

    pub fn definition(&self) -> &Arc<GameDefinition> {
        &self.definition
    }

    pub fn seed(&self) -> &WorldSeedDef {
        &self.seed
    }

    pub fn controlled_actor_ids(&self) -> Vec<crate::model::ActorId> {
        self.seed
            .actors
            .iter()
            .filter(|actor| {
                self.definition
                    .catalog
                    .actor_definitions
                    .get(&actor.actor_definition_id)
                    .is_some_and(|definition| definition.kind == crate::model::ActorKind::Player)
            })
            .map(|actor| actor.id.clone())
            .collect()
    }

    fn into_parts(self) -> (Arc<GameDefinition>, WorldSeedDef) {
        (self.definition, self.seed)
    }
}

impl GameDefinition {
    pub fn from_content(
        catalog: CatalogV6,
        profile_key: CatalogProfileKey,
        template: WorldTemplateV3,
    ) -> Result<Arc<Self>, ValidationError> {
        let selected = catalog.select(&profile_key)?;
        selected.validate_with_template(&template)?;
        let content_identity =
            super::checkpoint::ContentIdentityV1::from_selected(&selected, &template)?;
        let compiled_catalog = catalog::compile(&selected);
        let compiled_template = world_template::compile(&template);
        Ok(Arc::new(Self {
            catalog: compiled_catalog,
            world_template: compiled_template,
            content_identity,
        }))
    }
}

impl Engine {
    pub fn new(validated_seed: ValidatedWorldSeed, seed: u64) -> Result<Self, StepError> {
        let (definition, seed_source) = validated_seed.into_parts();
        let world = world_state::seed(&definition, &seed_source)?;
        let mut initial_events = world
            .actors
            .iter()
            .map(|actor| Event::ActorStatus {
                actor_id: actor.id.clone(),
                actor: actor.name.clone(),
                kind: actor.kind,
                location: actor.location.clone(),
                hp: actor.hp,
                character_identity: actor.character.as_ref().map(|c| c.identity.clone()),
            })
            .collect::<Vec<_>>();
        for actor in &world.actors {
            for effect in &actor.active_effects {
                initial_events.push(Event::EffectApplied {
                    actor_id: actor.id.clone(),
                    actor: actor.name.clone(),
                    location: actor.location.clone(),
                    instance_id: effect.instance_id.clone(),
                    effect_id: effect.effect_id.clone(),
                    source_kind: effect.source.kind.clone(),
                    source_id: effect.source.id.clone(),
                    kind: effect.kind.clone(),
                    tags: effect.tags.clone(),
                    potency: effect.potency,
                    remaining_rounds: effect.remaining_rounds,
                });
            }
        }

        let mut engine = Self {
            definition,
            world,
            rng: DeterministicRng::new(seed),
            initial_events,
            pending_durable_effects: Vec::new(),
        };
        let mut ecology_events = Vec::new();
        engine.initialize_ecology(&mut ecology_events)?;
        engine.initial_events.extend(ecology_events);
        engine.apply_initial_item_bindings()?;
        engine.validate_world_item_locations()?;
        engine.validate_bow_readiness_invariants()?;
        engine.validate_world_item_burden()?;
        Ok(engine)
    }
}

#[cfg(test)]
pub(crate) fn test_parts(
    case_id: &str,
) -> (CatalogV6, CatalogProfileKey, WorldTemplateV3, WorldSeedDef) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/test-corpus");
    let catalog: CatalogV6 = serde_json::from_str(
        &std::fs::read_to_string(root.join("catalogs/prototype_catalog_v6.json"))
            .expect("test catalog should be readable"),
    )
    .expect("test catalog should deserialize");
    let template: WorldTemplateV3 = serde_json::from_str(
        &std::fs::read_to_string(root.join(format!("world_templates/{case_id}.json")))
            .expect("test world template should be readable"),
    )
    .expect("test world template should deserialize");
    let mut seed_value: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(root.join(format!("simulation_seeds/{case_id}.json")))
            .expect("test simulation seed should be readable"),
    )
    .expect("test simulation seed should be JSON");
    let seed_object = seed_value
        .as_object_mut()
        .expect("test simulation seed should be an object");
    seed_object.remove("schema_version");
    seed_object.remove("kind");
    seed_object.remove("id");
    let seed: WorldSeedDef =
        serde_json::from_value(seed_value).expect("test world seed payload should deserialize");
    (
        catalog,
        CatalogProfileKey::from(format!("profile/{case_id}")),
        template,
        seed,
    )
}

#[cfg(test)]
pub(crate) fn test_engine_from_parts(
    catalog: CatalogV6,
    profile_key: CatalogProfileKey,
    template: WorldTemplateV3,
    seed: WorldSeedDef,
) -> Engine {
    let definition = GameDefinition::from_content(catalog, profile_key, template)
        .expect("test definition should validate");
    let seed = ValidatedWorldSeed::new(definition, seed).expect("test seed should validate");
    Engine::new(seed, 7).expect("test engine should initialize")
}

#[cfg(test)]
pub(crate) fn test_engine(case_id: &str) -> Engine {
    let (catalog, profile, template, seed) = test_parts(case_id);
    test_engine_from_parts(catalog, profile, template, seed)
}

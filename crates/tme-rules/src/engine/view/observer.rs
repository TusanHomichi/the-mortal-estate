use std::collections::BTreeSet;

use crate::events::{
    Event, SpellCastFailure, SpellFizzleCause, TransactionCostReceiptV1,
    TransactionRewardReceiptV1, TransactionSourceV1,
};
use crate::model::{ActorId, Coord, ItemBindingState, WorldPosition};
use crate::view::{
    LootClaimViewV1, MAX_FEEDBACK_TEXT_BYTES, MAX_FEEDBACK_TEXT_SCALARS,
    MAX_FEEDBACK_TRANSACTION_COSTS, MAX_FEEDBACK_TRANSACTION_REWARDS, MAX_OBSERVED_EVENTS,
    MAX_OBSERVER_ACTION_OPTIONS, MAX_OBSERVER_ACTORS, MAX_OBSERVER_CORPSES,
    MAX_OBSERVER_GOLD_PILES, MAX_OBSERVER_GROUND_ITEMS, OBSERVER_PROJECTION_CONTRACT_VERSION,
    ObservedEventV1, ObserverActorV1, ObserverCorpseChangeV1, ObserverCorpseV1,
    ObserverEffectChangeV1, ObserverFeedbackActorV1, ObserverFeedbackCueV1, ObserverFrameV1,
    ObserverGoldPileV1, ObserverGroundItemV1, ObserverGroupInvitationV2, ObserverGroupMemberV2,
    ObserverGroupV2, ObserverInspectActorV1, ObserverInspectExitStatusV1, ObserverInspectExitV1,
    ObserverInspectGroundItemV1, ObserverItemBindingV1, ObserverItemV1, ObserverLifeStateV1,
    ObserverPhysicalOutcomeV1, ObserverProjectionV1, ObserverResourceReasonV1, ObserverSocialV2,
    ObserverSpellFailureReasonV1, ObserverSpellFizzleReasonV1, ObserverSpellImpactOutcomeV1,
    ObserverSpellLifecycleStateV1, ObserverTileV1, ObserverTransactionCostV1,
    ObserverTransactionRewardV1, ObserverTransactionSourceV1,
    STATIC_SCENE_CONTEXT_CONTRACT_VERSION, StaticPresentationModeV1, StaticSceneBoundsV1,
    StaticSceneContextV1, StaticScenePropV1, StaticSceneRoleV1, StaticSceneSiteV1,
    StaticSceneTileV1, StaticTransitionApertureV1,
};

use super::super::{Engine, PLAYER_OBSERVATION_RADIUS, StepError};

fn observer_transaction_source(value: &TransactionSourceV1) -> ObserverTransactionSourceV1 {
    match value {
        TransactionSourceV1::SkillTraining {
            service_id,
            capability_id,
            track_id,
        } => ObserverTransactionSourceV1::SkillTraining {
            service_id: service_id.clone(),
            capability_id: capability_id.clone(),
            track_id: track_id.clone(),
        },
        TransactionSourceV1::SpellLearning {
            service_id,
            capability_id,
            spell_id,
        } => ObserverTransactionSourceV1::SpellLearning {
            service_id: service_id.clone(),
            capability_id: capability_id.clone(),
            spell_id: spell_id.clone(),
        },
        TransactionSourceV1::ClassPromotion {
            service_id,
            capability_id,
            transaction_id,
            target_class_id,
        } => ObserverTransactionSourceV1::ClassPromotion {
            service_id: service_id.clone(),
            capability_id: capability_id.clone(),
            transaction_id: transaction_id.clone(),
            target_class_id: target_class_id.clone(),
        },
        TransactionSourceV1::ServiceTransaction {
            service_id,
            capability_id,
            transaction_id,
        } => ObserverTransactionSourceV1::ServiceTransaction {
            service_id: service_id.clone(),
            capability_id: capability_id.clone(),
            transaction_id: transaction_id.clone(),
        },
        TransactionSourceV1::MerchantPurchase {
            service_id,
            capability_id,
            item_instance_ids,
        } => ObserverTransactionSourceV1::MerchantPurchase {
            service_id: service_id.clone(),
            capability_id: capability_id.clone(),
            item_instance_ids: item_instance_ids.clone(),
        },
        TransactionSourceV1::MerchantSale {
            service_id,
            capability_id,
            item_instance_id,
        } => ObserverTransactionSourceV1::MerchantSale {
            service_id: service_id.clone(),
            capability_id: capability_id.clone(),
            item_instance_id: item_instance_id.clone(),
        },
        TransactionSourceV1::ItemService {
            service_id,
            capability_id,
            operation,
            item_instance_id,
        } => ObserverTransactionSourceV1::ItemService {
            service_id: service_id.clone(),
            capability_id: capability_id.clone(),
            operation: *operation,
            item_instance_id: item_instance_id.clone(),
        },
        TransactionSourceV1::RestorationService {
            service_id,
            capability_id,
            operation_id,
            corpse_id,
        } => ObserverTransactionSourceV1::RestorationService {
            service_id: service_id.clone(),
            capability_id: capability_id.clone(),
            operation_id: operation_id.clone(),
            corpse_id: corpse_id.clone(),
        },
        TransactionSourceV1::NpcInteraction {
            npc_actor_id,
            interaction_id,
        } => ObserverTransactionSourceV1::NpcInteraction {
            npc_actor_id: npc_actor_id.clone(),
            interaction_id: interaction_id.clone(),
        },
        TransactionSourceV1::BankDeposit {
            service_id,
            capability_id,
            bank_id,
            gold_pile_id,
        } => ObserverTransactionSourceV1::BankDeposit {
            service_id: service_id.clone(),
            capability_id: capability_id.clone(),
            bank_id: bank_id.clone(),
            gold_pile_id: gold_pile_id.clone(),
        },
        TransactionSourceV1::BankWithdrawal {
            service_id,
            capability_id,
            bank_id,
            amount,
        } => ObserverTransactionSourceV1::BankWithdrawal {
            service_id: service_id.clone(),
            capability_id: capability_id.clone(),
            bank_id: bank_id.clone(),
            amount: *amount,
        },
    }
}

fn observer_transaction_cost(value: &TransactionCostReceiptV1) -> ObserverTransactionCostV1 {
    match value {
        TransactionCostReceiptV1::CarriedGold {
            amount,
            position,
            before,
            after,
        } => ObserverTransactionCostV1::CarriedGold {
            amount: *amount,
            position: *position,
            before: *before,
            after: *after,
        },
        TransactionCostReceiptV1::GroundGoldPile {
            gold_pile_id,
            amount,
            ..
        } => ObserverTransactionCostV1::GroundGoldPile {
            gold_pile_id: gold_pile_id.clone(),
            amount: *amount,
        },
        TransactionCostReceiptV1::BankBalance {
            bank_id,
            amount,
            before,
            after,
            ..
        } => ObserverTransactionCostV1::BankBalance {
            bank_id: bank_id.clone(),
            amount: *amount,
            before: *before,
            after: *after,
        },
        TransactionCostReceiptV1::SelectedCarriedItem {
            item_instance_id,
            item_definition_id,
            consumed_quantity,
            remaining_quantity,
        } => ObserverTransactionCostV1::SelectedCarriedItem {
            item_instance_id: item_instance_id.clone(),
            item_definition_id: item_definition_id.clone(),
            consumed_quantity: *consumed_quantity,
            remaining_quantity: *remaining_quantity,
        },
        TransactionCostReceiptV1::MerchantItem {
            item_instance_id,
            item_definition_id,
            quantity,
            pawn_listing_price_gold,
            ..
        } => ObserverTransactionCostV1::MerchantItem {
            item_instance_id: item_instance_id.clone(),
            item_definition_id: item_definition_id.clone(),
            quantity: *quantity,
            pawn_listing_price_gold: *pawn_listing_price_gold,
        },
    }
}

fn observer_transaction_reward(value: &TransactionRewardReceiptV1) -> ObserverTransactionRewardV1 {
    match value {
        TransactionRewardReceiptV1::LearningRate {
            track_id,
            before,
            after,
        } => ObserverTransactionRewardV1::LearningRate {
            track_id: track_id.clone(),
            before: *before,
            after: *after,
        },
        TransactionRewardReceiptV1::Experience { amount, total_xp } => {
            ObserverTransactionRewardV1::Experience {
                amount: *amount,
                total_xp: *total_xp,
            }
        }
        TransactionRewardReceiptV1::Item {
            item_instance_id,
            item_definition_id,
            position,
            quantity,
        } => ObserverTransactionRewardV1::Item {
            item_instance_id: item_instance_id.clone(),
            item_definition_id: item_definition_id.clone(),
            position: *position,
            quantity: *quantity,
        },
        TransactionRewardReceiptV1::Class {
            from_class_id,
            from_class_display,
            to_class_id,
            to_class_display,
        } => ObserverTransactionRewardV1::Class {
            from_class_id: from_class_id.clone(),
            from_class_display: from_class_display.clone(),
            to_class_id: to_class_id.clone(),
            to_class_display: to_class_display.clone(),
        },
        TransactionRewardReceiptV1::Spell {
            spell_id,
            learned_at_level,
        } => ObserverTransactionRewardV1::Spell {
            spell_id: spell_id.clone(),
            learned_at_level: *learned_at_level,
        },
        TransactionRewardReceiptV1::CarriedGold {
            amount,
            position,
            before,
            after,
        } => ObserverTransactionRewardV1::CarriedGold {
            amount: *amount,
            position: *position,
            before: *before,
            after: *after,
        },
        TransactionRewardReceiptV1::BankBalance {
            bank_id,
            amount,
            before,
            after,
            ..
        } => ObserverTransactionRewardV1::BankBalance {
            bank_id: bank_id.clone(),
            amount: *amount,
            before: *before,
            after: *after,
        },
        TransactionRewardReceiptV1::GroundGoldPile {
            gold_pile_id,
            amount,
            ..
        } => ObserverTransactionRewardV1::GroundGoldPile {
            gold_pile_id: gold_pile_id.clone(),
            amount: *amount,
        },
        TransactionRewardReceiptV1::MerchantItem {
            item_instance_id,
            item_definition_id,
            quantity,
            listing_price_gold,
            ..
        } => ObserverTransactionRewardV1::MerchantItem {
            item_instance_id: item_instance_id.clone(),
            item_definition_id: item_definition_id.clone(),
            quantity: *quantity,
            listing_price_gold: *listing_price_gold,
        },
        TransactionRewardReceiptV1::ItemAppraised {
            item_instance_id,
            item_definition_id,
            unit_value_gold,
            total_value_gold,
        } => ObserverTransactionRewardV1::ItemAppraised {
            item_instance_id: item_instance_id.clone(),
            item_definition_id: item_definition_id.clone(),
            unit_value_gold: *unit_value_gold,
            total_value_gold: *total_value_gold,
        },
        TransactionRewardReceiptV1::ItemIdentified {
            item_instance_id,
            item_definition_id,
        } => ObserverTransactionRewardV1::ItemIdentified {
            item_instance_id: item_instance_id.clone(),
            item_definition_id: item_definition_id.clone(),
        },
        TransactionRewardReceiptV1::ItemEnchanted {
            item_instance_id,
            item_definition_id,
            enchantment_instance_id,
            combat_add_rating_bonus,
            tags,
            remaining_rounds,
        } => ObserverTransactionRewardV1::ItemEnchanted {
            item_instance_id: item_instance_id.clone(),
            item_definition_id: item_definition_id.clone(),
            enchantment_instance_id: enchantment_instance_id.clone(),
            combat_add_rating_bonus: *combat_add_rating_bonus,
            tags: tags.clone(),
            remaining_rounds: *remaining_rounds,
        },
        TransactionRewardReceiptV1::ResourceRestored {
            resource,
            before,
            after,
            maximum,
            ..
        } => ObserverTransactionRewardV1::ResourceRestored {
            resource: *resource,
            before: *before,
            after: *after,
            maximum: *maximum,
        },
        TransactionRewardReceiptV1::StatusCured {
            status,
            removed_count,
            ..
        } => ObserverTransactionRewardV1::StatusCured {
            status: *status,
            removed_count: *removed_count,
        },
        TransactionRewardReceiptV1::PriestResurrection {
            corpse_id,
            method,
            current_hp,
            current_stamina,
            ..
        } => ObserverTransactionRewardV1::PriestResurrection {
            corpse_id: corpse_id.clone(),
            method: *method,
            current_hp: *current_hp,
            current_stamina: *current_stamina,
        },
        TransactionRewardReceiptV1::NpcInteraction {
            npc_actor_id,
            interaction_id,
            outcome,
        } => ObserverTransactionRewardV1::NpcInteraction {
            npc_actor_id: npc_actor_id.clone(),
            interaction_id: interaction_id.clone(),
            outcome: outcome.clone(),
        },
        TransactionRewardReceiptV1::QuestStage {
            quest_id,
            before_stage_id,
            after_stage_id,
            ..
        } => ObserverTransactionRewardV1::QuestStage {
            quest_id: quest_id.clone(),
            before_stage_id: before_stage_id.clone(),
            after_stage_id: after_stage_id.clone(),
        },
    }
}

impl Engine {
    fn static_scene_context(
        &self,
        center: &WorldPosition,
    ) -> Result<StaticSceneContextV1, StepError> {
        const FRAME_HALF_WIDTH: i32 = 7;
        const FRAME_HALF_HEIGHT: i32 = 6;

        let level = self
            .level_at(center)
            .ok_or_else(|| StepError::new("static scene context level is missing"))?;
        let (min, max) = if level.scene_role == crate::model::SceneRole::Interior {
            (
                Coord { x: 0, y: 0 },
                Coord {
                    x: level.width - 1,
                    y: level.height - 1,
                },
            )
        } else {
            (
                Coord {
                    x: (center.position.x - FRAME_HALF_WIDTH).max(0),
                    y: (center.position.y - FRAME_HALF_HEIGHT).max(0),
                },
                Coord {
                    x: (center.position.x + FRAME_HALF_WIDTH).min(level.width - 1),
                    y: (center.position.y + FRAME_HALF_HEIGHT).min(level.height - 1),
                },
            )
        };
        let mut tiles = Vec::new();
        let mut walkable_mask = Vec::new();
        for y in min.y..=max.y {
            for x in min.x..=max.x {
                let position = Coord { x, y };
                let terrain_ids = level.cells[y as usize][x as usize]
                    .iter()
                    .flatten()
                    .cloned()
                    .collect::<Vec<_>>();
                let mut walkable = false;
                let mut blocked = false;
                for terrain_id in &terrain_ids {
                    match self.definition.catalog.terrains.get(terrain_id) {
                        Some(terrain) if terrain.unresolved || !terrain.passable => blocked = true,
                        Some(_) => walkable = true,
                        None => blocked = true,
                    }
                }
                walkable &= !blocked;
                if walkable {
                    walkable_mask.push(position);
                }
                tiles.push(StaticSceneTileV1 {
                    position,
                    terrain_ids,
                    walkable,
                });
            }
        }
        let mut transition_apertures = self
            .definition
            .world_template
            .navigation
            .iter()
            .filter(|(at, _)| {
                at.same_site(center)
                    && at.position.x >= min.x
                    && at.position.x <= max.x
                    && at.position.y >= min.y
                    && at.position.y <= max.y
            })
            .flat_map(|(at, rows)| {
                rows.iter()
                    .filter(|row| !row.hidden)
                    .map(|row| StaticTransitionApertureV1 {
                        at: at.position,
                        navigation: row.kind,
                        target: row.target.clone(),
                    })
            })
            .collect::<Vec<_>>();
        transition_apertures.sort_by(|left, right| {
            left.at
                .cmp(&right.at)
                .then_with(|| left.target.cmp(&right.target))
        });
        Ok(StaticSceneContextV1 {
            contract_version: STATIC_SCENE_CONTEXT_CONTRACT_VERSION,
            site: StaticSceneSiteV1 {
                realm: center.realm.clone(),
                level: center.level.clone(),
            },
            bounds: StaticSceneBoundsV1 { min, max },
            content_digest: self.definition.content_identity().definition_sha256.clone(),
            visual_manifest_digest: self
                .definition
                .world_template
                .visual_manifest_digest
                .clone(),
            scene_role: match level.scene_role {
                crate::model::SceneRole::Overworld => StaticSceneRoleV1::Overworld,
                crate::model::SceneRole::CombatSpace => StaticSceneRoleV1::CombatSpace,
                crate::model::SceneRole::Interior => StaticSceneRoleV1::Interior,
            },
            presentation_mode: match level.presentation_mode {
                crate::model::PresentationMode::OverworldTown => {
                    StaticPresentationModeV1::OverworldTown
                }
                crate::model::PresentationMode::CombatSpace => {
                    StaticPresentationModeV1::CombatSpace
                }
            },
            world_zoom: level.world_zoom,
            tiles,
            walkable_mask,
            static_props: level
                .static_props
                .iter()
                .filter(|prop| {
                    prop.anchor.x >= min.x
                        && prop.anchor.x <= max.x
                        && prop.anchor.y >= min.y
                        && prop.anchor.y <= max.y
                })
                .map(|prop| StaticScenePropV1 {
                    id: prop.id.clone(),
                    visual_family: prop.visual_family.clone(),
                    anchor: prop.anchor,
                    layer: prop.layer,
                })
                .collect(),
            transition_apertures,
        })
    }

    fn observer_feedback_actor(&self, actor_id: &ActorId) -> Option<ObserverFeedbackActorV1> {
        self.world
            .actor(actor_id)
            .map(|actor| ObserverFeedbackActorV1 {
                actor_id: actor.id.clone(),
                name: actor.name.clone(),
                kind: actor.kind,
            })
    }

    fn observer_feedback_actor_is_visible(
        &self,
        actor_id: &ActorId,
        observer_actor_id: &ActorId,
        visible: &BTreeSet<WorldPosition>,
    ) -> bool {
        actor_id == observer_actor_id
            || self
                .world
                .actor(actor_id)
                .is_some_and(|actor| visible.contains(&actor.location))
    }

    fn observer_feedback_source(
        &self,
        actor_id: &ActorId,
        observer_actor_id: &ActorId,
        visible: &BTreeSet<WorldPosition>,
    ) -> Option<ObserverFeedbackActorV1> {
        self.observer_feedback_actor_is_visible(actor_id, observer_actor_id, visible)
            .then(|| self.observer_feedback_actor(actor_id))
            .flatten()
    }

    fn observer_combat_feedback(
        &self,
        event: &Event,
        observer_actor_id: &ActorId,
        visible: &BTreeSet<WorldPosition>,
    ) -> Result<Option<ObserverFeedbackCueV1>, StepError> {
        let actor_visible = |actor_id: &ActorId| {
            self.observer_feedback_actor_is_visible(actor_id, observer_actor_id, visible)
        };
        let actor = |actor_id: &ActorId| self.observer_feedback_actor(actor_id);
        let source = |actor_id: &ActorId| {
            self.observer_feedback_source(actor_id, observer_actor_id, visible)
        };
        let cue = match event {
            Event::Attacked {
                attacker_id,
                defender_id,
                defender_location,
                mode,
                damage,
                armor_reduction,
                wound_before,
                wound_after,
                defender_hp,
                ..
            } if attacker_id == observer_actor_id || visible.contains(defender_location) => {
                Some(ObserverFeedbackCueV1::PhysicalCombat {
                    source: source(attacker_id),
                    target: actor(defender_id).ok_or_else(|| {
                        StepError::new("physical feedback target disappeared before projection")
                    })?,
                    location: Some(defender_location.clone()),
                    mode: *mode,
                    outcome: ObserverPhysicalOutcomeV1::Hit {
                        damage: *damage,
                        armor_reduction: *armor_reduction,
                        wound_before: *wound_before,
                        wound_after: *wound_after,
                        target_hp: *defender_hp,
                    },
                })
            }
            Event::AttackMissed {
                attacker_id,
                defender_id,
                defender_location,
                mode,
                ..
            } if attacker_id == observer_actor_id || visible.contains(defender_location) => {
                Some(ObserverFeedbackCueV1::PhysicalCombat {
                    source: source(attacker_id),
                    target: actor(defender_id).ok_or_else(|| {
                        StepError::new("physical feedback target disappeared before projection")
                    })?,
                    location: Some(defender_location.clone()),
                    mode: *mode,
                    outcome: ObserverPhysicalOutcomeV1::Missed,
                })
            }
            Event::AttackBlocked {
                attacker_id,
                defender_id,
                defender_location,
                mode,
                ..
            } if attacker_id == observer_actor_id || visible.contains(defender_location) => {
                Some(ObserverFeedbackCueV1::PhysicalCombat {
                    source: source(attacker_id),
                    target: actor(defender_id).ok_or_else(|| {
                        StepError::new("physical feedback target disappeared before projection")
                    })?,
                    location: Some(defender_location.clone()),
                    mode: *mode,
                    outcome: ObserverPhysicalOutcomeV1::Blocked,
                })
            }
            Event::AttackBlockedNoSight {
                attacker_id,
                defender_id,
                mode,
                ..
            } if attacker_id == observer_actor_id
                || (actor_visible(attacker_id) && actor_visible(defender_id)) =>
            {
                Some(ObserverFeedbackCueV1::PhysicalCombat {
                    source: source(attacker_id),
                    target: actor(defender_id).ok_or_else(|| {
                        StepError::new("physical feedback target disappeared before projection")
                    })?,
                    location: None,
                    mode: *mode,
                    outcome: ObserverPhysicalOutcomeV1::NoSight,
                })
            }
            Event::AttackNotReady {
                actor_id,
                target_id,
                current_time,
                ready_at,
                mode,
                ..
            } if actor_id == observer_actor_id
                || (actor_visible(actor_id) && actor_visible(target_id)) =>
            {
                Some(ObserverFeedbackCueV1::PhysicalCombat {
                    source: source(actor_id),
                    target: actor(target_id).ok_or_else(|| {
                        StepError::new("physical feedback target disappeared before projection")
                    })?,
                    location: None,
                    mode: *mode,
                    outcome: ObserverPhysicalOutcomeV1::NotReady {
                        current_time: *current_time,
                        ready_at: *ready_at,
                    },
                })
            }
            Event::WeaponFumbled {
                attacker_id,
                mode,
                result,
                ..
            } if actor_visible(attacker_id) => Some(ObserverFeedbackCueV1::WeaponFumbled {
                actor: actor(attacker_id).ok_or_else(|| {
                    StepError::new("fumble feedback actor disappeared before projection")
                })?,
                mode: *mode,
                result: *result,
            }),
            _ => None,
        };
        Ok(cue)
    }

    fn observer_spell_feedback(
        &self,
        event: &Event,
        observer_actor_id: &ActorId,
        visible: &BTreeSet<WorldPosition>,
    ) -> Result<Option<ObserverFeedbackCueV1>, StepError> {
        let actor_visible = |actor_id: &ActorId| {
            self.observer_feedback_actor_is_visible(actor_id, observer_actor_id, visible)
        };
        let actor = |actor_id: &ActorId| self.observer_feedback_actor(actor_id);
        let source = |actor_id: &ActorId| {
            self.observer_feedback_source(actor_id, observer_actor_id, visible)
        };
        let cue = match event {
            Event::SpellWarmed {
                actor_id,
                spell_id,
                spell_name,
                warmed_at,
                ready_at,
                ..
            } if actor_visible(actor_id) => Some(ObserverFeedbackCueV1::SpellLifecycle {
                actor: actor(actor_id).ok_or_else(|| {
                    StepError::new("spell feedback actor disappeared before projection")
                })?,
                spell_id: spell_id.clone(),
                spell_name: spell_name.clone(),
                state: ObserverSpellLifecycleStateV1::Warmed {
                    warmed_at: *warmed_at,
                    ready_at: *ready_at,
                },
            }),
            Event::WarmedSpellReady {
                actor_id,
                spell_id,
                spell_name,
                ready_at,
                ..
            } if actor_visible(actor_id) => Some(ObserverFeedbackCueV1::SpellLifecycle {
                actor: actor(actor_id).ok_or_else(|| {
                    StepError::new("spell feedback actor disappeared before projection")
                })?,
                spell_id: spell_id.clone(),
                spell_name: spell_name.clone(),
                state: ObserverSpellLifecycleStateV1::Ready {
                    ready_at: *ready_at,
                },
            }),
            Event::SpellCastCommitted {
                actor_id,
                spell_id,
                spell_name,
                mp_cost,
                stamina_cost,
                ..
            } if actor_visible(actor_id) => Some(ObserverFeedbackCueV1::SpellLifecycle {
                actor: actor(actor_id).ok_or_else(|| {
                    StepError::new("spell feedback actor disappeared before projection")
                })?,
                spell_id: spell_id.clone(),
                spell_name: spell_name.clone(),
                state: ObserverSpellLifecycleStateV1::Cast {
                    mp_cost: (actor_id == observer_actor_id)
                        .then_some(*mp_cost)
                        .flatten(),
                    stamina_cost: (actor_id == observer_actor_id)
                        .then_some(*stamina_cost)
                        .flatten(),
                },
            }),
            Event::SpellFizzled {
                actor_id,
                spell_id,
                spell_name,
                cause,
                ..
            } if actor_visible(actor_id) => Some(ObserverFeedbackCueV1::SpellLifecycle {
                actor: actor(actor_id).ok_or_else(|| {
                    StepError::new("spell feedback actor disappeared before projection")
                })?,
                spell_id: spell_id.clone(),
                spell_name: spell_name.clone(),
                state: ObserverSpellLifecycleStateV1::Fizzled {
                    reason: match cause {
                        SpellFizzleCause::Replaced { .. } => ObserverSpellFizzleReasonV1::Replaced,
                        SpellFizzleCause::Canceled => ObserverSpellFizzleReasonV1::Canceled,
                        SpellFizzleCause::Rest => ObserverSpellFizzleReasonV1::Rest,
                        SpellFizzleCause::HealingBalm => ObserverSpellFizzleReasonV1::HealingBalm,
                        SpellFizzleCause::Damage { .. } => ObserverSpellFizzleReasonV1::Damage,
                        SpellFizzleCause::Defeat => ObserverSpellFizzleReasonV1::Defeat,
                    },
                },
            }),
            Event::SpellCastFailed {
                actor_id,
                spell_id,
                spell_name,
                failure,
                mp_cost,
                stamina_cost,
                ..
            } if actor_visible(actor_id) => Some(ObserverFeedbackCueV1::SpellLifecycle {
                actor: actor(actor_id).ok_or_else(|| {
                    StepError::new("spell feedback actor disappeared before projection")
                })?,
                spell_id: spell_id.clone(),
                spell_name: spell_name.clone(),
                state: ObserverSpellLifecycleStateV1::Failed {
                    reason: match failure {
                        SpellCastFailure::InvalidPath { .. } => {
                            ObserverSpellFailureReasonV1::InvalidPath
                        }
                        SpellCastFailure::AboveSkillAttempt => {
                            ObserverSpellFailureReasonV1::AboveSkillAttempt
                        }
                    },
                    mp_cost: (actor_id == observer_actor_id)
                        .then_some(*mp_cost)
                        .flatten(),
                    stamina_cost: (actor_id == observer_actor_id)
                        .then_some(*stamina_cost)
                        .flatten(),
                },
            }),
            Event::SpellDamaged {
                caster_id,
                spell_id,
                spell_name,
                target_id,
                location,
                damage,
                hp,
                ..
            } if target_id == observer_actor_id || visible.contains(location) => {
                Some(ObserverFeedbackCueV1::SpellImpact {
                    source: source(caster_id),
                    spell_id: spell_id.clone(),
                    spell_name: spell_name.clone(),
                    target: actor(target_id).ok_or_else(|| {
                        StepError::new("spell impact target disappeared before projection")
                    })?,
                    location: location.clone(),
                    outcome: ObserverSpellImpactOutcomeV1::Damaged {
                        damage: *damage,
                        target_hp: *hp,
                    },
                })
            }
            Event::SpellHealed {
                caster_id,
                spell_id,
                spell_name,
                target_id,
                location,
                amount,
                hp,
                ..
            } if target_id == observer_actor_id || visible.contains(location) => {
                Some(ObserverFeedbackCueV1::SpellImpact {
                    source: source(caster_id),
                    spell_id: spell_id.clone(),
                    spell_name: spell_name.clone(),
                    target: actor(target_id).ok_or_else(|| {
                        StepError::new("spell impact target disappeared before projection")
                    })?,
                    location: location.clone(),
                    outcome: ObserverSpellImpactOutcomeV1::Healed {
                        amount: *amount,
                        target_hp: *hp,
                    },
                })
            }
            _ => None,
        };
        Ok(cue)
    }

    fn observer_spell_resource_feedback(
        &self,
        event: &Event,
        observer_actor_id: &ActorId,
    ) -> Result<Vec<ObserverFeedbackCueV1>, StepError> {
        let (actor_id, mp_cost, stamina_cost) = match event {
            Event::SpellCastCommitted {
                actor_id,
                mp_cost,
                stamina_cost,
                ..
            }
            | Event::SpellCastFailed {
                actor_id,
                mp_cost,
                stamina_cost,
                ..
            } if actor_id == observer_actor_id => (actor_id, *mp_cost, *stamina_cost),
            _ => return Ok(Vec::new()),
        };
        let actor = self
            .world
            .actor(actor_id)
            .ok_or_else(|| StepError::new("spell resource actor disappeared before projection"))?;
        let character = actor.character.as_ref().ok_or_else(|| {
            StepError::new("controlled spell resource actor has no character sheet")
        })?;
        let feedback_actor = self.observer_feedback_actor(actor_id).ok_or_else(|| {
            StepError::new("spell resource feedback actor disappeared before projection")
        })?;
        let mut cues = Vec::with_capacity(2);
        if let Some(amount) = mp_cost.filter(|amount| *amount > 0) {
            cues.push(ObserverFeedbackCueV1::Resource {
                actor: feedback_actor.clone(),
                resource: crate::model::ResourceKind::Mp,
                reason: ObserverResourceReasonV1::SpellCost,
                amount,
                current: Some(actor.mp),
                maximum: character.resources.max_mp,
            });
        }
        if let Some(amount) = stamina_cost.filter(|amount| *amount > 0) {
            cues.push(ObserverFeedbackCueV1::Resource {
                actor: feedback_actor,
                resource: crate::model::ResourceKind::Stamina,
                reason: ObserverResourceReasonV1::SpellCost,
                amount,
                current: Some(actor.stamina),
                maximum: actor.max_stamina(),
            });
        }
        Ok(cues)
    }

    fn observer_effect_feedback(
        &self,
        event: &Event,
        observer_actor_id: &ActorId,
        visible: &BTreeSet<WorldPosition>,
    ) -> Result<Option<ObserverFeedbackCueV1>, StepError> {
        let actor = |actor_id: &ActorId| {
            self.observer_feedback_actor(actor_id)
                .ok_or_else(|| StepError::new("effect actor disappeared before projection"))
        };
        let cue = match event {
            Event::EffectApplied {
                actor_id,
                location,
                effect_id,
                kind,
                remaining_rounds,
                ..
            } if actor_id == observer_actor_id || visible.contains(location) => {
                Some(ObserverFeedbackCueV1::ActorEffect {
                    actor: actor(actor_id)?,
                    location: location.clone(),
                    effect_id: effect_id.clone(),
                    effect_kind: kind.clone(),
                    change: ObserverEffectChangeV1::Applied {
                        remaining_rounds: *remaining_rounds,
                    },
                })
            }
            Event::EffectTicked {
                actor_id,
                location,
                effect_id,
                kind,
                remaining_rounds,
                ..
            } if actor_id == observer_actor_id || visible.contains(location) => {
                Some(ObserverFeedbackCueV1::ActorEffect {
                    actor: actor(actor_id)?,
                    location: location.clone(),
                    effect_id: effect_id.clone(),
                    effect_kind: kind.clone(),
                    change: ObserverEffectChangeV1::Ticked {
                        remaining_rounds: *remaining_rounds,
                    },
                })
            }
            Event::EffectExpired {
                actor_id,
                location,
                effect_id,
                kind,
                ..
            } if actor_id == observer_actor_id || visible.contains(location) => {
                Some(ObserverFeedbackCueV1::ActorEffect {
                    actor: actor(actor_id)?,
                    location: location.clone(),
                    effect_id: effect_id.clone(),
                    effect_kind: kind.clone(),
                    change: ObserverEffectChangeV1::Expired,
                })
            }
            Event::EffectRemoved {
                actor_id,
                location,
                effect_id,
                kind,
                ..
            } if actor_id == observer_actor_id || visible.contains(location) => {
                Some(ObserverFeedbackCueV1::ActorEffect {
                    actor: actor(actor_id)?,
                    location: location.clone(),
                    effect_id: effect_id.clone(),
                    effect_kind: kind.clone(),
                    change: ObserverEffectChangeV1::Removed,
                })
            }
            Event::TileEffectApplied {
                location,
                effect_id,
                kind,
                remaining_rounds,
                ..
            } if visible.contains(location) => Some(ObserverFeedbackCueV1::TileEffect {
                location: location.clone(),
                effect_id: effect_id.clone(),
                effect_kind: kind.clone(),
                change: ObserverEffectChangeV1::Applied {
                    remaining_rounds: *remaining_rounds,
                },
            }),
            Event::TileEffectTicked {
                location,
                effect_id,
                kind,
                remaining_rounds,
                ..
            } if visible.contains(location) => Some(ObserverFeedbackCueV1::TileEffect {
                location: location.clone(),
                effect_id: effect_id.clone(),
                effect_kind: kind.clone(),
                change: ObserverEffectChangeV1::Ticked {
                    remaining_rounds: *remaining_rounds,
                },
            }),
            Event::TileEffectExpired {
                location,
                effect_id,
                kind,
                ..
            } if visible.contains(location) => Some(ObserverFeedbackCueV1::TileEffect {
                location: location.clone(),
                effect_id: effect_id.clone(),
                effect_kind: kind.clone(),
                change: ObserverEffectChangeV1::Expired,
            }),
            Event::TileEffectRemoved {
                location,
                effect_id,
                kind,
                ..
            } if visible.contains(location) => Some(ObserverFeedbackCueV1::TileEffect {
                location: location.clone(),
                effect_id: effect_id.clone(),
                effect_kind: kind.clone(),
                change: ObserverEffectChangeV1::Removed,
            }),
            Event::EffectDamaged {
                actor_id,
                location,
                effect_id,
                kind,
                damage,
                hp,
                ..
            }
            | Event::TileEffectDamaged {
                actor_id,
                location,
                effect_id,
                kind,
                damage,
                hp,
                ..
            } if actor_id == observer_actor_id || visible.contains(location) => {
                Some(ObserverFeedbackCueV1::EffectDamage {
                    actor: actor(actor_id)?,
                    location: location.clone(),
                    effect_id: effect_id.clone(),
                    effect_kind: kind.clone(),
                    damage: *damage,
                    actor_hp: *hp,
                })
            }
            _ => None,
        };
        Ok(cue)
    }

    fn observer_private_feedback(
        &self,
        event: &Event,
        observer_actor_id: &ActorId,
        observer_character_id: &crate::model::CharacterId,
    ) -> Result<Option<ObserverFeedbackCueV1>, StepError> {
        let controlled_actor = |actor_id: &ActorId| -> Result<ObserverFeedbackActorV1, StepError> {
            self.observer_feedback_actor(actor_id).ok_or_else(|| {
                StepError::new("controlled feedback actor disappeared before projection")
            })
        };
        let cue = match event {
            Event::MovementStaminaSpent {
                actor_id,
                amount,
                stamina,
                max_stamina,
                ..
            } if actor_id == observer_actor_id => Some(ObserverFeedbackCueV1::Resource {
                actor: controlled_actor(actor_id)?,
                resource: crate::model::ResourceKind::Stamina,
                reason: ObserverResourceReasonV1::MovementSpend,
                amount: *amount,
                current: Some(*stamina),
                maximum: *max_stamina,
            }),
            Event::PhysicalStaminaSpent {
                actor_id,
                amount,
                stamina,
                max_stamina,
                ..
            } if actor_id == observer_actor_id => Some(ObserverFeedbackCueV1::Resource {
                actor: controlled_actor(actor_id)?,
                resource: crate::model::ResourceKind::Stamina,
                reason: ObserverResourceReasonV1::PhysicalSpend,
                amount: *amount,
                current: Some(*stamina),
                maximum: *max_stamina,
            }),
            Event::ResourceRegenerated {
                actor_id,
                resource,
                amount,
                current,
                maximum,
                ..
            } if actor_id == observer_actor_id => Some(ObserverFeedbackCueV1::Resource {
                actor: controlled_actor(actor_id)?,
                resource: *resource,
                reason: ObserverResourceReasonV1::Regenerated,
                amount: *amount,
                current: Some(*current),
                maximum: *maximum,
            }),
            Event::ResourceRestored {
                actor_id,
                resource,
                before,
                after,
                maximum,
                ..
            } if actor_id == observer_actor_id => Some(ObserverFeedbackCueV1::Resource {
                actor: controlled_actor(actor_id)?,
                resource: *resource,
                reason: ObserverResourceReasonV1::Restored,
                amount: after.saturating_sub(*before),
                current: Some(*after),
                maximum: *maximum,
            }),
            Event::BalmHealed {
                actor_id,
                amount,
                hp,
                ..
            } if actor_id == observer_actor_id => Some(ObserverFeedbackCueV1::Resource {
                actor: controlled_actor(actor_id)?,
                resource: crate::model::ResourceKind::Hp,
                reason: ObserverResourceReasonV1::Balm,
                amount: *amount,
                current: Some(*hp),
                maximum: self
                    .world
                    .actor(actor_id)
                    .ok_or_else(|| StepError::new("resource actor disappeared before projection"))?
                    .max_hp(),
            }),
            Event::TransactionCommitted {
                actor_id,
                source,
                costs,
                rewards,
                ..
            } if actor_id == observer_actor_id => {
                if costs.len() > MAX_FEEDBACK_TRANSACTION_COSTS
                    || rewards.len() > MAX_FEEDBACK_TRANSACTION_REWARDS
                {
                    return Err(StepError::new(
                        "committed transaction exceeds observer feedback receipt bound",
                    ));
                }
                Some(ObserverFeedbackCueV1::Transaction {
                    actor: controlled_actor(actor_id)?,
                    source: observer_transaction_source(source),
                    costs: costs.iter().map(observer_transaction_cost).collect(),
                    rewards: rewards.iter().map(observer_transaction_reward).collect(),
                })
            }
            Event::QuestStateChanged {
                character_id,
                quest_id,
                before_stage_id,
                after_stage_id,
            } if character_id == observer_character_id => {
                let quest = self
                    .definition
                    .catalog
                    .quests
                    .iter()
                    .find(|(id, _)| id.as_str() == quest_id)
                    .map(|(_, quest)| quest)
                    .ok_or_else(|| StepError::new("quest feedback definition is missing"))?;
                let stage = quest
                    .stages
                    .iter()
                    .find(|(id, _)| id.as_str() == after_stage_id)
                    .map(|(_, stage)| stage)
                    .ok_or_else(|| StepError::new("quest feedback stage is missing"))?;
                Some(ObserverFeedbackCueV1::Quest {
                    quest_id: quest_id.clone(),
                    quest_title: quest.title.clone(),
                    before_stage_id: before_stage_id.clone(),
                    after_stage_id: after_stage_id.clone(),
                    after_stage_label: stage.label.clone(),
                    terminal: stage.terminal,
                })
            }
            Event::NpcSpoke {
                npc_actor_id,
                recipient_character_id,
                interaction_id,
                response,
                ..
            } if recipient_character_id == observer_character_id => {
                let scalar_count = response.chars().count();
                if response.is_empty()
                    || scalar_count > MAX_FEEDBACK_TEXT_SCALARS
                    || response.len() > MAX_FEEDBACK_TEXT_BYTES
                    || response.chars().any(char::is_control)
                {
                    return Err(StepError::new(
                        "NPC feedback response violates the bounded text contract",
                    ));
                }
                let npc = self.observer_feedback_actor(npc_actor_id).ok_or_else(|| {
                    StepError::new("NPC feedback actor disappeared before projection")
                })?;
                Some(ObserverFeedbackCueV1::NpcMessage {
                    npc_actor_id: npc.actor_id,
                    npc_name: npc.name,
                    interaction_id: interaction_id.clone(),
                    response: response.clone(),
                })
            }
            _ => None,
        };
        Ok(cue)
    }

    fn observer_death_feedback(
        &self,
        event: &Event,
        observer_actor_id: &ActorId,
        visible: &BTreeSet<WorldPosition>,
    ) -> Result<Option<ObserverFeedbackCueV1>, StepError> {
        let actor_visible = |actor_id: &ActorId| {
            self.observer_feedback_actor_is_visible(actor_id, observer_actor_id, visible)
        };
        let actor = |actor_id: &ActorId| {
            self.observer_feedback_actor(actor_id)
                .ok_or_else(|| StepError::new("death feedback actor disappeared before projection"))
        };
        let cue = match event {
            Event::ActorDefeated {
                actor_id,
                location,
                cause,
                credited_actor_id,
                ..
            } if actor_id == observer_actor_id || visible.contains(location) => {
                Some(ObserverFeedbackCueV1::Defeat {
                    actor: actor(actor_id)?,
                    location: location.clone(),
                    cause: *cause,
                    credited_source: credited_actor_id.as_ref().and_then(|source_actor_id| {
                        self.observer_feedback_source(source_actor_id, observer_actor_id, visible)
                    }),
                })
            }
            Event::CorpseCreated {
                corpse_id,
                origin_actor_id,
                origin_kind,
                origin_name,
                location,
                ..
            } if visible.contains(location) => Some(ObserverFeedbackCueV1::Corpse {
                corpse_id: corpse_id.clone(),
                origin: Some(ObserverFeedbackActorV1 {
                    actor_id: origin_actor_id.clone(),
                    name: origin_name.clone(),
                    kind: *origin_kind,
                }),
                location: location.clone(),
                change: ObserverCorpseChangeV1::Created,
            }),
            Event::CorpseRemoved {
                corpse_id,
                origin_actor_id,
                location,
                method,
            } if visible.contains(location) => Some(ObserverFeedbackCueV1::Corpse {
                corpse_id: corpse_id.clone(),
                origin: self.observer_feedback_source(origin_actor_id, observer_actor_id, visible),
                location: location.clone(),
                change: ObserverCorpseChangeV1::Removed { method: *method },
            }),
            Event::ActorLifeStateChanged {
                actor_id, from, to, ..
            } if actor_visible(actor_id) => Some(ObserverFeedbackCueV1::LifeState {
                actor: actor(actor_id)?,
                from: ObserverLifeStateV1::from(from),
                to: ObserverLifeStateV1::from(to),
            }),
            Event::ActorResurrected {
                actor_id,
                corpse_id,
                method,
                destination,
                current_hp,
                current_stamina,
                ..
            } if actor_id == observer_actor_id || visible.contains(destination) => {
                Some(ObserverFeedbackCueV1::Resurrection {
                    actor: actor(actor_id)?,
                    corpse_id: corpse_id.clone(),
                    method: *method,
                    destination: destination.clone(),
                    current_hp: *current_hp,
                    current_stamina: *current_stamina,
                })
            }
            _ => None,
        };
        Ok(cue)
    }

    fn observer_feedback_cues(
        &self,
        event: &Event,
        observer_actor_id: &ActorId,
        observer_character_id: &crate::model::CharacterId,
        visible: &BTreeSet<WorldPosition>,
    ) -> Result<Vec<ObserverFeedbackCueV1>, StepError> {
        let mut cues = Vec::new();
        if let Some(cue) = self.observer_combat_feedback(event, observer_actor_id, visible)? {
            cues.push(cue);
        }
        if let Some(cue) = self.observer_spell_feedback(event, observer_actor_id, visible)? {
            cues.push(cue);
        }
        cues.extend(self.observer_spell_resource_feedback(event, observer_actor_id)?);
        if let Some(cue) = self.observer_effect_feedback(event, observer_actor_id, visible)? {
            cues.push(cue);
        }
        if let Some(cue) =
            self.observer_private_feedback(event, observer_actor_id, observer_character_id)?
        {
            cues.push(cue);
        }
        if let Some(cue) = self.observer_death_feedback(event, observer_actor_id, visible)? {
            cues.push(cue);
        }
        Ok(cues)
    }

    fn observer_item(&self, item_instance_id: &str) -> Result<ObserverItemV1, StepError> {
        let instance = self.item_instance(item_instance_id)?;
        let definition = self.item_definition(item_instance_id)?;
        Ok(ObserverItemV1 {
            item_instance_id: item_instance_id.to_string(),
            item_definition_id: instance.definition_id.clone(),
            name: definition.name.clone(),
            quantity: instance.quantity,
            binding: match instance.binding {
                ItemBindingState::Unrestricted | ItemBindingState::BindOnFirstCharacterTouch => {
                    ObserverItemBindingV1::Unbound
                }
                ItemBindingState::Bound { .. } => ObserverItemBindingV1::Bound,
            },
        })
    }

    pub fn observer_projection(
        &self,
        observer_actor_id: &ActorId,
        raw_events: &[Event],
    ) -> Result<ObserverProjectionV1, StepError> {
        let observer_index = self.player_actor_index(observer_actor_id)?;
        let observer = &self.world.actors[observer_index];
        let observer_character_id = observer
            .character_id
            .clone()
            .ok_or_else(|| StepError::new("observer has no stable character ID"))?;
        let visible = self.visible_tiles_for_actor_id(observer_actor_id)?;

        let level = self
            .level_at(&observer.location)
            .ok_or_else(|| StepError::new("observer is outside the validated world"))?;
        let center = observer.location.clone();
        let radius = PLAYER_OBSERVATION_RADIUS as i32;
        let mut tiles = Vec::with_capacity(225);
        for y in (center.position.y - radius)..=(center.position.y + radius) {
            for x in (center.position.x - radius)..=(center.position.x + radius) {
                if x < 0 || y < 0 || x >= level.width || y >= level.height {
                    continue;
                }
                let position = Coord { x, y };
                let location = WorldPosition::new(&center.realm, &center.level, position);
                let observed = visible.contains(&location);
                let effective = observed
                    .then(|| self.effective_tile_at(&location))
                    .flatten();
                tiles.push(ObserverTileV1 {
                    position,
                    terrain_id: effective.as_ref().map(|tile| tile.terrain_id.clone()),
                    terrain_name: effective.as_ref().map(|tile| tile.terrain_name.clone()),
                    passable: effective.as_ref().map(|tile| tile.passable),
                    move_cost: effective.as_ref().and_then(|tile| tile.move_cost),
                    transition: observed
                        .then(|| self.transition_view_at(&location))
                        .flatten(),
                });
            }
        }

        let mut visible_actors = self
            .world
            .actors
            .iter()
            .enumerate()
            .filter(|(_, actor)| visible.contains(&actor.location))
            .collect::<Vec<_>>();
        if visible_actors.len() > MAX_OBSERVER_ACTORS {
            return Err(StepError::new(format!(
                "observer projection contains {} visible actors; maximum is {MAX_OBSERVER_ACTORS}",
                visible_actors.len()
            )));
        }
        visible_actors.sort_by(|(_, left), (_, right)| left.id.cmp(&right.id));
        let actors = visible_actors
            .into_iter()
            .map(|(target_index, actor)| {
                Ok(ObserverActorV1 {
                    actor_id: actor.id.clone(),
                    character_id: (actor.kind == crate::model::ActorKind::Player)
                        .then(|| actor.character_id.clone())
                        .flatten(),
                    name: actor.name.clone(),
                    kind: actor.kind,
                    position: actor.location.clone(),
                    life_state: ObserverLifeStateV1::from(&actor.life_state),
                    hp: actor.hp,
                    max_hp: actor.max_hp(),
                    attack_safety: if !actor.is_alive() {
                        crate::model::AttackSafety::Invalid
                    } else {
                        self.attack_safety_assessment(observer_index, target_index)?
                            .safety
                    },
                })
            })
            .collect::<Result<Vec<_>, StepError>>()?;

        let distance =
            |location: &WorldPosition| center.position.chebyshev_distance(location.position);
        let mut visible_corpses = self
            .world
            .corpses
            .values()
            .filter(|corpse| visible.contains(&corpse.location))
            .collect::<Vec<_>>();
        visible_corpses.sort_by(|left, right| {
            distance(&left.location)
                .cmp(&distance(&right.location))
                .then_with(|| left.location.cmp(&right.location))
                .then_with(|| right.sequence.cmp(&left.sequence))
                .then_with(|| left.id.cmp(&right.id))
        });
        let corpses_truncated = visible_corpses.len() > MAX_OBSERVER_CORPSES;
        let corpses = visible_corpses
            .into_iter()
            .take(MAX_OBSERVER_CORPSES)
            .map(|corpse| ObserverCorpseV1 {
                corpse_id: corpse.id.clone(),
                origin_actor_id: corpse.origin_actor_id.clone(),
                origin_kind: corpse.origin_kind,
                origin_name: corpse.origin_name.clone(),
                location: corpse.location.clone(),
                sequence: corpse.sequence,
                searched: corpse.searched,
                loot_claim: corpse.loot_claim.as_ref().map(LootClaimViewV1::from),
            })
            .collect();

        let mut visible_ground_items = self
            .world
            .ground_items
            .iter()
            .filter(|item| visible.contains(&item.location))
            .collect::<Vec<_>>();
        visible_ground_items.sort_by(|left, right| {
            distance(&left.location)
                .cmp(&distance(&right.location))
                .then_with(|| left.location.cmp(&right.location))
                .then_with(|| left.item_instance_id.cmp(&right.item_instance_id))
        });
        let ground_items_truncated = visible_ground_items.len() > MAX_OBSERVER_GROUND_ITEMS;
        let ground_items = visible_ground_items
            .into_iter()
            .take(MAX_OBSERVER_GROUND_ITEMS)
            .map(|item| {
                Ok(ObserverGroundItemV1 {
                    item: self.observer_item(&item.item_instance_id)?,
                    location: item.location.clone(),
                    loot_claim: item.loot_claim.as_ref().map(LootClaimViewV1::from),
                })
            })
            .collect::<Result<Vec<_>, StepError>>()?;

        let mut visible_gold = self
            .world
            .ground_gold
            .values()
            .filter(|pile| visible.contains(&pile.location))
            .collect::<Vec<_>>();
        visible_gold.sort_by(|left, right| {
            distance(&left.location)
                .cmp(&distance(&right.location))
                .then_with(|| left.location.cmp(&right.location))
                .then_with(|| left.id.cmp(&right.id))
        });
        let gold_piles_truncated = visible_gold.len() > MAX_OBSERVER_GOLD_PILES;
        let gold_piles = visible_gold
            .into_iter()
            .take(MAX_OBSERVER_GOLD_PILES)
            .map(|pile| ObserverGoldPileV1 {
                gold_pile_id: pile.id.clone(),
                amount: pile.amount,
                location: pile.location.clone(),
                loot_claim: pile.loot_claim.as_ref().map(LootClaimViewV1::from),
            })
            .collect();

        let controlled_view = self.actor_view(observer_index, false);
        let character = controlled_view
            .character
            .ok_or_else(|| StepError::new("observer has no controlled character sheet"))?;
        let carried = controlled_view.carried;
        let burden = controlled_view.burden;
        let observed_context = self.actor_observed_action_context(observer_actor_id)?;
        let warmed_spell = observed_context.warmed_spell.clone();
        let spell_actions = observed_context.spell_actions.clone();
        let services_here = observed_context.services_here.clone();
        let npcs_here = observed_context.npcs_here.clone();
        let quest_log = observed_context.quest_log.clone();
        let incoming_item_offers = observed_context.incoming_item_offers.clone();
        let outgoing_item_offers = observed_context.outgoing_item_offers.clone();
        let mut action_options = self.actor_action_options(observer_actor_id)?;
        let action_options_truncated = action_options.len() > MAX_OBSERVER_ACTION_OPTIONS;
        action_options.truncate(MAX_OBSERVER_ACTION_OPTIONS);

        let mut events = Vec::new();
        let mut events_truncated = false;
        for event in raw_events {
            let observed = match event {
                Event::Moved {
                    actor_id,
                    from,
                    to,
                    navigation,
                    ..
                } => {
                    let actor_is_visible = actor_id == observer_actor_id
                        || self
                            .world
                            .actor(actor_id)
                            .is_some_and(|actor| visible.contains(&actor.location));
                    (actor_is_visible && (actor_id == observer_actor_id || visible.contains(to)))
                        .then(|| ObservedEventV1::ActorMoved {
                            actor_id: actor_id.clone(),
                            from: from.clone(),
                            to: to.clone(),
                            navigation: *navigation,
                        })
                }
                Event::Inspected {
                    actor_id,
                    location,
                    tile,
                    tile_move_cost,
                    exits,
                    nearby_actors,
                    ground_items,
                    ..
                } if actor_id == observer_actor_id => {
                    let exits = exits
                        .iter()
                        .map(|exit| {
                            let status = match &exit.status {
                                crate::events::InspectExitStatus::Walkable => {
                                    ObserverInspectExitStatusV1::Walkable
                                }
                                crate::events::InspectExitStatus::BlockedTerrain => {
                                    ObserverInspectExitStatusV1::BlockedTerrain
                                }
                                crate::events::InspectExitStatus::Door { state, target } => {
                                    let open = match state.as_str() {
                                        "open" => true,
                                        "closed" => false,
                                        _ => {
                                            return Err(StepError::new(
                                                "inspect event contains invalid door state",
                                            ));
                                        }
                                    };
                                    ObserverInspectExitStatusV1::Door {
                                        open,
                                        target: target.clone(),
                                    }
                                }
                                crate::events::InspectExitStatus::OutOfBounds => {
                                    ObserverInspectExitStatusV1::OutOfBounds
                                }
                            };
                            Ok(ObserverInspectExitV1 {
                                direction: exit.direction,
                                location: exit.location.clone(),
                                terrain: exit.terrain.clone(),
                                move_cost: exit.move_cost,
                                status,
                            })
                        })
                        .collect::<Result<Vec<_>, StepError>>()?;
                    let nearby_actors = nearby_actors
                        .iter()
                        .filter(|actor| visible.contains(&actor.location))
                        .map(|actor| ObserverInspectActorV1 {
                            direction: actor.direction,
                            actor_id: actor.actor_id.clone(),
                            actor: actor.actor.clone(),
                            kind: actor.kind,
                            location: actor.location.clone(),
                            hp: actor.hp,
                        })
                        .collect();
                    let ground_items = ground_items
                        .iter()
                        .filter(|item| visible.contains(&item.location))
                        .map(|item| {
                            Ok(ObserverInspectGroundItemV1 {
                                item: self.observer_item(&item.item.item_instance_id)?,
                                location: item.location.clone(),
                                direction: item.direction,
                            })
                        })
                        .collect::<Result<Vec<_>, StepError>>()?;
                    Some(ObservedEventV1::Inspected {
                        location: location.clone(),
                        tile: tile.clone(),
                        tile_move_cost: *tile_move_cost,
                        exits,
                        nearby_actors,
                        ground_items,
                    })
                }
                Event::GroupChanged {
                    group_id,
                    member_character_ids,
                    subject_character_id,
                    ..
                } => (member_character_ids.contains(&observer_character_id)
                    || subject_character_id.as_ref() == Some(&observer_character_id)
                    || self.group_id_for_character(&observer_character_id) == Some(*group_id))
                .then_some(ObservedEventV1::GroupChanged {
                    group_id: *group_id,
                }),
                Event::GroupInvitationCreated {
                    invitation_id,
                    issuer_character_id,
                    target_character_id,
                    ..
                }
                | Event::GroupInvitationResolved {
                    invitation_id,
                    issuer_character_id,
                    target_character_id,
                    ..
                } => (issuer_character_id == &observer_character_id
                    || target_character_id == &observer_character_id
                    || self
                        .group_id_for_character(issuer_character_id)
                        .is_some_and(|group_id| {
                            self.group_id_for_character(&observer_character_id) == Some(group_id)
                        }))
                .then_some(ObservedEventV1::GroupInvitationChanged {
                    invitation_id: *invitation_id,
                }),
                Event::GroupPresenceChanged {
                    group_id,
                    character_id,
                    connected,
                    ..
                } => (self.group_id_for_character(&observer_character_id) == Some(*group_id))
                    .then_some(ObservedEventV1::GroupPresenceChanged {
                        group_id: *group_id,
                        character_id: character_id.clone(),
                        connected: *connected,
                    }),
                Event::PlayerFollowChanged {
                    follower_character_id,
                    target_character_id,
                    ..
                } => (follower_character_id == &observer_character_id
                    || target_character_id.as_ref() == Some(&observer_character_id))
                .then(|| ObservedEventV1::PlayerFollowChanged {
                    follower_character_id: follower_character_id.clone(),
                    target_character_id: target_character_id.clone(),
                }),
                Event::CommunicationPreferenceChanged { character_id, .. }
                | Event::CharacterBlockChanged { character_id, .. } => (character_id
                    == &observer_character_id)
                    .then_some(ObservedEventV1::CommunicationPreferencesChanged),
                Event::ItemOfferCreated {
                    item_instance_id,
                    sender_character_id,
                    recipient_character_id,
                    ..
                }
                | Event::ItemOfferCompleted {
                    item_instance_id,
                    sender_character_id,
                    recipient_character_id,
                    ..
                } => (sender_character_id == &observer_character_id
                    || recipient_character_id == &observer_character_id)
                    .then(|| ObservedEventV1::ItemOfferChanged {
                        item_instance_id: item_instance_id.clone(),
                    }),
                Event::DefeatRewardShareAwarded {
                    character_id,
                    amount,
                    ..
                } => (character_id == &observer_character_id).then(|| {
                    ObservedEventV1::DefeatRewardShare {
                        character_id: character_id.clone(),
                        amount: *amount,
                    }
                }),
                _ => None,
            };
            if let Some(observed) = observed {
                if events.len() == MAX_OBSERVED_EVENTS {
                    events_truncated = true;
                } else {
                    events.push(observed);
                }
                continue;
            }
            for cue in self.observer_feedback_cues(
                event,
                observer_actor_id,
                &observer_character_id,
                &visible,
            )? {
                if events.len() == MAX_OBSERVED_EVENTS {
                    events_truncated = true;
                    continue;
                }
                events.push(ObservedEventV1::Feedback { cue });
            }
        }

        let group = self
            .group_id_for_character(&observer_character_id)
            .and_then(|group_id| self.world.groups.get(&group_id))
            .map(|group| ObserverGroupV2 {
                group_id: group.id,
                leader_character_id: group.leader_character_id.clone(),
                members: group
                    .members
                    .iter()
                    .map(|member| {
                        let presence = self
                            .world
                            .character_presence
                            .get(&member.character_id)
                            .expect("validated group member presence");
                        ObserverGroupMemberV2 {
                            character_id: member.character_id.clone(),
                            joined_order: member.joined_order,
                            membership_epoch: member.membership_epoch,
                            connected: presence.connected,
                            absent_since: presence.absent_since,
                        }
                    })
                    .collect(),
            });
        let invitations =
            self.world
                .group_invitations
                .values()
                .map(|invitation| ObserverGroupInvitationV2 {
                    invitation_id: invitation.id,
                    issuer_character_id: invitation.issuer_character_id.clone(),
                    target_character_id: invitation.target_character_id.clone(),
                    group_id: invitation.group_id,
                    expires_at: invitation.expires_at,
                });
        let incoming_invitations = invitations
            .clone()
            .filter(|invitation| invitation.target_character_id == observer_character_id)
            .collect();
        let observer_group_id = self.group_id_for_character(&observer_character_id);
        let outgoing_invitations = invitations
            .filter(|invitation| {
                invitation.issuer_character_id == observer_character_id
                    || invitation
                        .group_id
                        .is_some_and(|group_id| observer_group_id == Some(group_id))
            })
            .collect();
        let preferences = self
            .world
            .communication_preferences
            .get(&observer_character_id)
            .ok_or_else(|| StepError::new("observer communication preferences are missing"))?;
        let social = ObserverSocialV2 {
            character_id: observer_character_id.clone(),
            group,
            incoming_invitations,
            outgoing_invitations,
            following_character_id: self
                .world
                .player_follow_targets
                .get(&observer_character_id)
                .cloned(),
            pages_enabled: preferences.pages_enabled,
            blocked_character_ids: preferences.blocked_character_ids.iter().cloned().collect(),
        };
        Ok(ObserverProjectionV1 {
            contract_version: OBSERVER_PROJECTION_CONTRACT_VERSION,
            static_scene_context: self.static_scene_context(&center)?,
            frame: ObserverFrameV1 {
                contract_version: OBSERVER_PROJECTION_CONTRACT_VERSION,
                logical_time: self.world.timing.now,
                ready_at: observer.timing.ready_at,
                observer_actor_id: observer_actor_id.clone(),
                observation_center: center,
                observation_radius: PLAYER_OBSERVATION_RADIUS,
                can_act: self.actor_can_act(observer_index),
                tiles,
                actors,
                corpses,
                corpses_truncated,
                ground_items,
                ground_items_truncated,
                gold_piles,
                gold_piles_truncated,
                character,
                carried,
                burden,
                warmed_spell,
                spell_actions,
                services_here,
                npcs_here,
                quest_log,
                action_options,
                action_options_truncated,
                social,
                incoming_item_offers,
                outgoing_item_offers,
            },
            events,
            events_truncated,
        })
    }
}

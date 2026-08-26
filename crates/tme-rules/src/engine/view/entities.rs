use crate::model::{
    CorpseState, GroundGoldPile, HostilityReason, SocialBehavior, SocialNature,
    SocialOwnerRelation, WorldPosition,
};
use crate::view::{
    ActiveEffectViewV1, ActorLifeStateViewV1, ActorViewV1, ArmorProtectionSourceViewV1,
    ArmorProtectionViewV1, CarriedLayoutViewV1, CharacterProgressionViewV1, CharacterSheetViewV1,
    CorpseViewV1, GroundGoldPileViewV1, KnownSpellViewV1, LootClaimViewV1, ObservedActorViewV1,
    ObservedSocialViewV1, PhysicalBlockCandidateViewV1, PhysicalWeaponModeViewV1,
    PhysicalWeaponViewV1, PublicHostilityReasonV1, SkillEntryViewV1, SocialBehaviorViewV1,
    SocialNatureViewV1, SocialOwnerRelationViewV1, TileEffectViewV1, TrueSocialViewV1,
};

use super::super::Engine;

impl Engine {
    fn social_nature_view(nature: SocialNature) -> SocialNatureViewV1 {
        match nature {
            SocialNature::Human => SocialNatureViewV1::Human,
            SocialNature::Animal => SocialNatureViewV1::Animal,
            SocialNature::Other => SocialNatureViewV1::Other,
        }
    }

    fn social_behavior_view(behavior: SocialBehavior) -> SocialBehaviorViewV1 {
        match behavior {
            SocialBehavior::Adventurer => SocialBehaviorViewV1::Adventurer,
            SocialBehavior::Civilian => SocialBehaviorViewV1::Civilian,
            SocialBehavior::TownEnforcer => SocialBehaviorViewV1::TownEnforcer,
            SocialBehavior::AlignmentCreature => SocialBehaviorViewV1::AlignmentCreature,
            SocialBehavior::Passive => SocialBehaviorViewV1::Passive,
        }
    }

    fn social_owner_relation_view(relation: SocialOwnerRelation) -> SocialOwnerRelationViewV1 {
        match relation {
            SocialOwnerRelation::None => SocialOwnerRelationViewV1::None,
            SocialOwnerRelation::Summoner => SocialOwnerRelationViewV1::Summoner,
        }
    }

    fn public_hostility_reason(reason: HostilityReason) -> PublicHostilityReasonV1 {
        match reason {
            HostilityReason::SameActor => PublicHostilityReasonV1::SameActor,
            HostilityReason::Owner => PublicHostilityReasonV1::OwnerProtected,
            HostilityReason::Passive => PublicHostilityReasonV1::Passive,
            HostilityReason::NpcGrudge => PublicHostilityReasonV1::Retaliation,
            HostilityReason::SelfDefense => PublicHostilityReasonV1::SelfDefense,
            HostilityReason::LawfulHumanResponse => PublicHostilityReasonV1::LawfulResponse,
            HostilityReason::ChaoticOpposition => PublicHostilityReasonV1::ChaoticOpposition,
            HostilityReason::EvilOpposition => PublicHostilityReasonV1::EvilOpposition,
            HostilityReason::NoHostility => PublicHostilityReasonV1::NoHostility,
        }
    }

    fn true_social_view(&self, index: usize) -> TrueSocialViewV1 {
        let actor = &self.world.actors[index];
        TrueSocialViewV1 {
            alignment: self
                .true_actor_alignment(index)
                .expect("validated actor social alignment"),
            nature: Self::social_nature_view(actor.social.nature),
            behavior: Self::social_behavior_view(actor.social.behavior),
            owner_relation: Self::social_owner_relation_view(actor.social.owner_relation),
        }
    }

    pub(in crate::engine) fn observed_social_view(
        &self,
        observer_index: usize,
        target_index: usize,
    ) -> Result<ObservedSocialViewV1, super::super::StepError> {
        let target_identity = self.perceived_social_identity(observer_index, target_index)?;
        let (hostile_to_observer, hostility_reason) = if observer_index == target_index {
            (false, HostilityReason::SameActor)
        } else if !self.world.actors[target_index].is_alive() {
            (false, HostilityReason::NoHostility)
        } else {
            let incoming = self.hostility_assessment(target_index, observer_index)?;
            (incoming.hostile, incoming.reason)
        };
        let attack_safety = if !self.world.actors[target_index].is_alive() {
            crate::model::AttackSafety::Invalid
        } else {
            self.attack_safety_assessment(observer_index, target_index)?
                .safety
        };
        Ok(ObservedSocialViewV1 {
            apparent_behavior: Self::social_behavior_view(target_identity.behavior),
            hostile_to_observer,
            hostility_reason: Self::public_hostility_reason(hostility_reason),
            attack_safety,
        })
    }

    fn corpse_view(corpse: &CorpseState) -> CorpseViewV1 {
        CorpseViewV1 {
            corpse_id: corpse.id.clone(),
            origin_actor_id: corpse.origin_actor_id.clone(),
            origin_character_id: corpse.origin_character_id.clone(),
            origin_kind: corpse.origin_kind,
            origin_name: corpse.origin_name.clone(),
            location: corpse.location.clone(),
            created_at: corpse.created_at,
            sequence: corpse.sequence,
            searched: corpse.searched,
            loot_claim: corpse.loot_claim.as_ref().map(LootClaimViewV1::from),
        }
    }

    pub(in crate::engine) fn ground_gold_view(pile: &GroundGoldPile) -> GroundGoldPileViewV1 {
        GroundGoldPileViewV1 {
            gold_pile_id: pile.id.clone(),
            amount: pile.amount,
            location: pile.location.clone(),
            loot_claim: pile.loot_claim.as_ref().map(LootClaimViewV1::from),
        }
    }

    pub(super) fn sorted_corpse_views(&self) -> Vec<CorpseViewV1> {
        let mut corpses = self
            .world
            .corpses
            .values()
            .map(Self::corpse_view)
            .collect::<Vec<_>>();
        corpses.sort_by(|left, right| {
            left.location
                .level
                .cmp(&right.location.level)
                .then(left.location.position.y.cmp(&right.location.position.y))
                .then(left.location.position.x.cmp(&right.location.position.x))
                .then(right.sequence.cmp(&left.sequence))
                .then(left.corpse_id.cmp(&right.corpse_id))
        });
        corpses
    }

    pub(super) fn sorted_ground_gold_views(&self) -> Vec<GroundGoldPileViewV1> {
        let mut piles = self
            .world
            .ground_gold
            .values()
            .map(Self::ground_gold_view)
            .collect::<Vec<_>>();
        piles.sort_by(|left, right| {
            left.location
                .level
                .cmp(&right.location.level)
                .then(left.location.position.y.cmp(&right.location.position.y))
                .then(left.location.position.x.cmp(&right.location.position.x))
                .then(left.gold_pile_id.cmp(&right.gold_pile_id))
        });
        piles
    }

    pub(super) fn sorted_tile_effect_views(&self) -> Vec<TileEffectViewV1> {
        let mut tile_effects: Vec<TileEffectViewV1> = self
            .world
            .tile_effects
            .iter()
            .filter(|effect| effect.remaining_rounds != Some(0))
            .map(TileEffectViewV1::from)
            .collect();
        tile_effects.sort_by(|a, b| {
            a.location
                .level
                .cmp(&b.location.level)
                .then(a.location.position.y.cmp(&b.location.position.y))
                .then(a.location.position.x.cmp(&b.location.position.x))
                .then(a.instance_id.cmp(&b.instance_id))
        });
        tile_effects
    }

    pub(super) fn visible_tile_effect_views(
        &self,
        visible: &std::collections::BTreeSet<WorldPosition>,
    ) -> Vec<TileEffectViewV1> {
        let mut tile_effects: Vec<TileEffectViewV1> = self
            .world
            .tile_effects
            .iter()
            .filter(|effect| effect.remaining_rounds != Some(0))
            .filter(|effect| visible.contains(&effect.location.clone()))
            .map(TileEffectViewV1::from)
            .collect();
        tile_effects.sort_by(|a, b| {
            a.location
                .level
                .cmp(&b.location.level)
                .then(a.location.position.y.cmp(&b.location.position.y))
                .then(a.location.position.x.cmp(&b.location.position.x))
                .then(a.instance_id.cmp(&b.instance_id))
        });
        tile_effects
    }

    pub(super) fn actor_view(&self, index: usize, include_tie_break_order: bool) -> ActorViewV1 {
        let actor = &self.world.actors[index];
        let armor_plan = self.armor_protection_plan(index).unwrap_or_default();
        let armor_protection = ArmorProtectionViewV1 {
            sources: armor_plan
                .sources
                .iter()
                .map(|source| ArmorProtectionSourceViewV1 {
                    carried_position: source.carried_position,
                    item_instance_id: source.item_instance_id.clone(),
                    item_definition_id: source.item_definition_id.clone(),
                    block_rating: source.block_rating,
                    encumbrance: source.encumbrance,
                    cutting_reduction: source.cutting_reduction,
                    piercing_reduction: source.piercing_reduction,
                    crushing_reduction: source.crushing_reduction,
                })
                .collect(),
            block_rating: armor_plan.block_rating,
            encumbrance: armor_plan.encumbrance,
            cutting_reduction: armor_plan.cutting_reduction,
            piercing_reduction: armor_plan.piercing_reduction,
            crushing_reduction: armor_plan.crushing_reduction,
        };
        let carried_items = actor
            .carried
            .items
            .iter()
            .map(|(position, item_instance_id)| {
                self.positioned_item_view(item_instance_id, *position)
                    .expect("validated carried item instance")
            })
            .collect();

        ActorViewV1 {
            id: actor.id.clone(),
            character_id: actor.character_id.clone(),
            kind: actor.kind,
            creature_traits: actor.creature_traits.clone(),
            social: self.true_social_view(index),
            name: actor.name.clone(),
            location: actor.location.clone(),
            hp: actor.hp,
            max_hp: actor.max_hp(),
            wound_state: self.wound_state(index),
            armor_protection,
            life_state: ActorLifeStateViewV1::from(&actor.life_state),
            ready_at: actor.timing.ready_at,
            last_resource_activity_at: actor.resource_activity.last_active_at,
            tie_break_order: include_tie_break_order.then_some(actor.timing.tie_break_order),
            attack_ready_at: actor.attack_ready_at,
            physical_weapon: self.physical_weapon_selection(index).ok().map(|selection| {
                let restriction_usable = self.selected_weapon_is_usable(index, &selection);
                let effective_combat_add_rating =
                    self.effective_combat_add_rating(&selection).unwrap_or(0);
                let eligible_block_candidates = self
                    .block_candidates(index, armor_plan.encumbrance, 0)
                    .unwrap_or_default()
                    .into_iter()
                    .map(|candidate| PhysicalBlockCandidateViewV1 {
                        source: candidate.source,
                        carried_position: candidate.carried_position,
                        item_instance_id: candidate.item_instance_id,
                        block_value: candidate.block_value,
                        skill_track_id: candidate.skill_track_id,
                        skill_level: candidate.skill_level,
                        chance_percent: candidate.chance_percent,
                    })
                    .collect();
                PhysicalWeaponViewV1 {
                    item_instance_id: selection.item_instance_id,
                    item_definition_id: selection.item_definition_id,
                    skill_track_id: selection.skill_track_id,
                    skill_level: selection.skill_level,
                    default_attack_mode: selection.default_attack_mode,
                    attack_modes: selection
                        .attack_modes
                        .into_iter()
                        .map(|row| PhysicalWeaponModeViewV1 {
                            mode: row.mode,
                            maximum_range: row.maximum_range,
                            damage_kind: row.damage_kind,
                        })
                        .collect(),
                    cooldown_units: selection.cooldown_units,
                    combat_add_rating: selection.combat_add_rating,
                    effective_combat_add_rating,
                    handedness: selection.handedness,
                    block_value: selection.block_value,
                    nocking_unloads_on_movement: selection.nocking_unloads_on_movement,
                    offhand_occupied: selection.offhand_occupied,
                    full_two_handed_effect: selection.full_two_handed_effect,
                    bow_readiness: selection.bow_readiness,
                    required_alignment: selection.required_alignment,
                    binding_usable: selection.binding_usable,
                    alignment_usable: selection.alignment_usable,
                    restriction_usable,
                    eligible_block_candidates,
                }
            }),
            carried: CarriedLayoutViewV1 {
                items: carried_items,
                gold: actor.carried.gold,
            },
            burden: self.burden_view(index).expect("validated actor burden"),
            npc: actor
                .npc
                .as_ref()
                .map(|npc| crate::view::NpcActorStateViewV1 {
                    follow_cadence_units: npc.follow_cadence_units,
                    following_character_id: npc.following_character_id.clone(),
                }),
            character: actor.character.as_ref().map(|character| {
                let mut view = CharacterSheetViewV1 {
                    identity: (&character.identity).into(),
                    alignment_state: (&character.alignment_state).into(),
                    attributes: (&character.attributes).into(),
                    resources: (&character.resources).into(),
                    progression: CharacterProgressionViewV1 {
                        level: character.progression.level,
                        experience: character.progression.experience,
                        pending_target_level: super::super::progression::pending_target_level(
                            &character.progression,
                            &self.definition.catalog.rules.progression,
                        ),
                    },
                    physical_attribute_adds: (&character.physical_attribute_adds).into(),
                    promotion_history: character.promotion_history.iter().map(Into::into).collect(),
                    known_spells: character
                        .known_spells
                        .iter()
                        .map(|spell| KnownSpellViewV1 {
                            spell_id: spell.spell_id.clone(),
                            lane: spell.lane.clone(),
                            learned_at_level: spell.learned_at_level,
                        })
                        .collect(),
                    skill_ledger: character
                        .skill_ledger
                        .iter()
                        .map(|entry| SkillEntryViewV1 {
                            track_id: entry.track_id.clone(),
                            level: entry.level,
                            critique_rank: entry.critique_rank,
                            practice_points: entry.practice_points,
                            learning_rate: entry.learning_rate,
                            track_display: None,
                            level_title: None,
                        })
                        .collect(),
                };
                for entry in &mut view.skill_ledger {
                    entry.track_display = self.skill_track_display(&entry.track_id);
                    entry.level_title = self.skill_level_title(&entry.track_id, entry.level);
                }
                view
            }),
            active_effects: actor
                .active_effects
                .iter()
                .map(ActiveEffectViewV1::from)
                .collect(),
            magic_resistance: crate::view::MagicResistanceViewV1 {
                natural_save_twentieths: actor.magic_resistance.natural_save_twentieths,
                evidence_state: actor.magic_resistance.evidence_state,
                boosts: self
                    .actor_resistance_boosts_for_index(index)
                    .into_iter()
                    .map(|boost| crate::view::MagicResistanceBoostViewV1 {
                        tag: boost.tag,
                        bonus_twentieths: boost.bonus_twentieths,
                        source_kind: boost.source_kind,
                        source_id: boost.source_id,
                    })
                    .collect(),
            },
            warmed_spell: actor
                .warmed_spell
                .as_ref()
                .map(crate::view::WarmedSpellViewV1::from),
            owner_id: actor
                .summoned
                .as_ref()
                .map(|summoned| summoned.owner_id.clone()),
            summoned: actor
                .summoned
                .as_ref()
                .map(crate::view::SummonedActorViewV1::from),
        }
    }

    pub(in crate::engine) fn observed_actor_view(
        &self,
        observer_index: usize,
        target_index: usize,
    ) -> Result<ObservedActorViewV1, super::super::StepError> {
        let base = self.actor_view(target_index, false);
        Ok(ObservedActorViewV1 {
            id: base.id,
            kind: base.kind,
            creature_traits: base.creature_traits,
            social: self.observed_social_view(observer_index, target_index)?,
            name: base.name,
            location: base.location,
            hp: base.hp,
            max_hp: base.max_hp,
            wound_state: base.wound_state,
            armor_protection: base.armor_protection,
            life_state: base.life_state,
            ready_at: base.ready_at,
            last_resource_activity_at: base.last_resource_activity_at,
            attack_ready_at: base.attack_ready_at,
            physical_weapon: base.physical_weapon,
            carried: base.carried,
            burden: base.burden,
            npc: base.npc,
            character: (observer_index == target_index)
                .then_some(base.character)
                .flatten(),
            active_effects: base.active_effects,
            magic_resistance: base.magic_resistance,
            warmed_spell: base.warmed_spell,
            owner_id: base.owner_id,
            summoned: base.summoned,
        })
    }
}

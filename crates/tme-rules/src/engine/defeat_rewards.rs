use std::collections::BTreeMap;

use crate::events::Event;
use crate::model::{
    ActorId, ActorKind, CharacterId, DefeatContributionKey, DefeatContributionLedger,
    DefeatRewardClass, DefeatRewardUnitContribution, DefeatRewardUnitId,
    GROUP_DISCONNECT_GRACE_UNITS, GroupMembershipKey, SpellDamageRewardClass,
};

use super::death::DefeatContext;
use super::{Engine, PLAYER_OBSERVATION_RADIUS, StepError};

impl Engine {
    pub(super) fn record_defeat_contribution(
        &mut self,
        target_index: usize,
        applied_damage: i32,
        context: &DefeatContext,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        if applied_damage <= 0 {
            return Ok(());
        }
        let damage = u64::try_from(applied_damage)
            .map_err(|_| StepError::new("applied defeat contribution is invalid"))?;
        let target_id = self
            .world
            .actors
            .get(target_index)
            .ok_or_else(|| StepError::new("defeat contribution target disappeared"))?
            .id
            .clone();

        let credited = context
            .reward_source()
            .and_then(|(actor_id, reward_class)| {
                self.controlled_character_for_reward_source(actor_id)
                    .map(|character_id| (character_id, reward_class))
            });
        let contribution = credited.as_ref().and_then(|(character_id, reward_class)| {
            self.reward_unit_for_contributor(character_id).map(
                |(reward_unit_id, eligible_memberships)| {
                    (
                        reward_unit_id,
                        DefeatContributionKey {
                            contributor_character_id: character_id.clone(),
                            reward_class: *reward_class,
                            eligible_memberships,
                        },
                    )
                },
            )
        });

        let ledger = self
            .world
            .defeat_contributions
            .entry(target_id.clone())
            .or_default();
        ledger.total_actual_damage = ledger
            .total_actual_damage
            .checked_add(damage)
            .ok_or_else(|| StepError::new("defeat contribution damage overflow"))?;
        if let Some((reward_unit_id, key)) = contribution.as_ref() {
            let amount = ledger
                .reward_units
                .entry(reward_unit_id.clone())
                .or_default()
                .slices
                .entry(key.clone())
                .or_default();
            *amount = amount
                .checked_add(damage)
                .ok_or_else(|| StepError::new("defeat contribution slice overflow"))?;
        }
        events.push(Event::DefeatContributionRecorded {
            contributor_character_id: credited.map(|(character_id, _)| character_id),
            target_id,
            reward_unit_id: contribution.as_ref().map(|(unit_id, _)| unit_id.clone()),
            reward_class: contribution.as_ref().map(|(_, key)| key.reward_class),
            applied_damage: damage,
            total_actual_damage: ledger.total_actual_damage,
        });
        Ok(())
    }

    fn controlled_character_for_reward_source(
        &self,
        source_actor_id: &ActorId,
    ) -> Option<CharacterId> {
        let mut cursor = source_actor_id;
        for _ in 0..=self.world.actors.len() {
            let actor = self.world.actors.iter().find(|actor| &actor.id == cursor)?;
            if actor.kind == ActorKind::Player {
                return actor
                    .character_id
                    .clone()
                    .filter(|_| actor.character.is_some());
            }
            let owner_id = &actor.summoned.as_ref()?.owner_id;
            cursor = owner_id;
        }
        None
    }

    fn reward_unit_for_contributor(
        &self,
        character_id: &CharacterId,
    ) -> Option<(DefeatRewardUnitId, Vec<GroupMembershipKey>)> {
        if !self
            .world
            .actors
            .iter()
            .any(|actor| actor.character_id.as_ref() == Some(character_id))
        {
            return None;
        }
        if let Some(group_id) = self.group_id_for_character(character_id) {
            let group = self.world.groups.get(&group_id)?;
            let mut memberships = group
                .members
                .iter()
                .map(|member| member.membership_key())
                .collect::<Vec<_>>();
            memberships.sort();
            return Some((DefeatRewardUnitId::Group { group_id }, memberships));
        }
        Some((
            DefeatRewardUnitId::Solo {
                character_id: character_id.clone(),
            },
            Vec::new(),
        ))
    }

    pub(super) fn award_defeat_rewards(
        &mut self,
        defeated_index: usize,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let defeated = self
            .world
            .actors
            .get(defeated_index)
            .ok_or_else(|| StepError::new("defeat reward target disappeared"))?;
        let target_id = defeated.id.clone();
        let target_name = defeated.name.clone();
        let target_location = defeated.location.clone();
        let authored_experience = defeated.xp_value;
        let ineligible_reason = if defeated.kind == ActorKind::Player {
            Some("player_target")
        } else if defeated.summoned.is_some() {
            Some("owned_summon_target")
        } else if authored_experience <= 0 {
            Some("zero_authored_experience")
        } else {
            None
        };
        let ledger = self
            .world
            .defeat_contributions
            .remove(&target_id)
            .unwrap_or_default();
        let denominator = self.reward_weight_denominator()?;
        let weighted_units = self.weighted_reward_units(&ledger, denominator)?;
        let weighted_total = weighted_units
            .iter()
            .try_fold(0_u128, |total, (_, weight)| total.checked_add(*weight))
            .ok_or_else(|| StepError::new("defeat reward weight overflow"))?;

        let available = if ineligible_reason.is_some()
            || ledger.total_actual_damage == 0
            || weighted_total == 0
        {
            0
        } else {
            let numerator = u128::try_from(authored_experience)
                .map_err(|_| StepError::new("authored defeat reward is invalid"))?
                .checked_mul(weighted_total)
                .ok_or_else(|| StepError::new("defeat reward pool overflow"))?;
            let divisor = u128::from(ledger.total_actual_damage)
                .checked_mul(u128::from(denominator))
                .ok_or_else(|| StepError::new("defeat reward divisor overflow"))?;
            i32::try_from(numerator / divisor)
                .map_err(|_| StepError::new("defeat reward exceeds supported range"))?
        };

        let unit_allocations = largest_remainder(available, &weighted_units)?;
        let mut shares = BTreeMap::<(DefeatRewardUnitId, CharacterId), i32>::new();
        for ((unit_id, _), unit_amount) in weighted_units.iter().zip(unit_allocations) {
            if unit_amount == 0 {
                continue;
            }
            let Some(unit) = ledger.reward_units.get(unit_id) else {
                continue;
            };
            match unit_id {
                DefeatRewardUnitId::Solo { character_id } => {
                    if self
                        .reward_recipient_actor(character_id, &target_location)
                        .is_some()
                    {
                        shares.insert((unit_id.clone(), character_id.clone()), unit_amount);
                    }
                }
                DefeatRewardUnitId::Group { group_id } => {
                    let slice_weights = unit
                        .slices
                        .iter()
                        .map(|(key, damage)| {
                            Ok((
                                key.clone(),
                                self.scaled_damage_weight(*damage, key.reward_class, denominator)?,
                            ))
                        })
                        .collect::<Result<Vec<_>, StepError>>()?;
                    let slice_allocations = largest_remainder(unit_amount, &slice_weights)?;
                    for ((slice, _), slice_amount) in slice_weights.iter().zip(slice_allocations) {
                        if slice_amount == 0 {
                            continue;
                        }
                        let eligible = self.eligible_group_reward_members(
                            *group_id,
                            &slice.eligible_memberships,
                            &target_location,
                        );
                        if eligible.is_empty() {
                            continue;
                        }
                        let equal_weights = eligible
                            .iter()
                            .map(|(_, character_id)| (character_id.clone(), 1_u128))
                            .collect::<Vec<_>>();
                        let member_allocations = largest_remainder(slice_amount, &equal_weights)?;
                        for ((_, character_id), amount) in eligible.iter().zip(member_allocations) {
                            let share = shares
                                .entry((unit_id.clone(), character_id.clone()))
                                .or_default();
                            *share = share
                                .checked_add(amount)
                                .ok_or_else(|| StepError::new("defeat reward share overflow"))?;
                        }
                    }
                }
            }
        }
        let awarded = shares.values().try_fold(0_i32, |total, amount| {
            total
                .checked_add(*amount)
                .ok_or_else(|| StepError::new("defeat reward awarded total overflow"))
        })?;
        let reason = ineligible_reason.unwrap_or(if ledger.total_actual_damage == 0 {
            "no_damage"
        } else if weighted_total == 0 {
            "no_eligible_player_contribution"
        } else {
            "contribution_shared"
        });
        events.push(Event::DefeatRewardEvaluated {
            target_id,
            target: target_name,
            authored_experience,
            actual_damage: ledger.total_actual_damage,
            weighted_damage_numerator: u64::try_from(weighted_total)
                .map_err(|_| StepError::new("defeat reward event weight exceeds u64"))?,
            weighted_damage_denominator: denominator,
            available_experience: available,
            awarded_experience: awarded,
            reason: reason.to_string(),
        });
        for ((reward_unit_id, character_id), amount) in shares {
            if amount <= 0 {
                continue;
            }
            let Some(actor_index) = self.reward_recipient_actor(&character_id, &target_location)
            else {
                continue;
            };
            let actor = &self.world.actors[actor_index];
            events.push(Event::DefeatRewardShareAwarded {
                character_id,
                actor_id: actor.id.clone(),
                actor: actor.name.clone(),
                reward_unit_id,
                amount,
            });
            events.extend(super::progression::award_character_experience(
                self,
                actor_index,
                amount,
            )?);
        }
        Ok(())
    }

    fn reward_weight_denominator(&self) -> Result<u64, StepError> {
        let rules = &self.definition.catalog.rules.magic.kill_experience;
        let directed = u64::from(rules.directed.denominator);
        let area = u64::from(rules.area_or_illusion.denominator);
        if directed == 0 || area == 0 {
            return Err(StepError::new("defeat reward fraction denominator is zero"));
        }
        directed
            .checked_div(gcd(directed, area))
            .and_then(|value| value.checked_mul(area))
            .ok_or_else(|| StepError::new("defeat reward common denominator overflow"))
    }

    fn scaled_damage_weight(
        &self,
        damage: u64,
        class: DefeatRewardClass,
        denominator: u64,
    ) -> Result<u128, StepError> {
        let (numerator, class_denominator) = match class {
            DefeatRewardClass::Physical => (1_u64, 1_u64),
            DefeatRewardClass::DirectedSpell => {
                let fraction = self.definition.catalog.rules.magic.kill_experience.directed;
                (
                    u64::from(fraction.numerator),
                    u64::from(fraction.denominator),
                )
            }
            DefeatRewardClass::AreaOrIllusionSpell => {
                let fraction = self
                    .definition
                    .catalog
                    .rules
                    .magic
                    .kill_experience
                    .area_or_illusion;
                (
                    u64::from(fraction.numerator),
                    u64::from(fraction.denominator),
                )
            }
        };
        if class_denominator == 0 || !denominator.is_multiple_of(class_denominator) {
            return Err(StepError::new("defeat reward fraction is invalid"));
        }
        u128::from(damage)
            .checked_mul(u128::from(numerator))
            .and_then(|value| value.checked_mul(u128::from(denominator / class_denominator)))
            .ok_or_else(|| StepError::new("defeat contribution weight overflow"))
    }

    fn weighted_reward_units(
        &self,
        ledger: &DefeatContributionLedger,
        denominator: u64,
    ) -> Result<Vec<(DefeatRewardUnitId, u128)>, StepError> {
        ledger
            .reward_units
            .iter()
            .map(|(unit_id, unit)| {
                let weight = unit
                    .slices
                    .iter()
                    .try_fold(0_u128, |total, (key, damage)| {
                        total
                            .checked_add(self.scaled_damage_weight(
                                *damage,
                                key.reward_class,
                                denominator,
                            )?)
                            .ok_or_else(|| StepError::new("defeat reward unit weight overflow"))
                    })?;
                Ok((unit_id.clone(), weight))
            })
            .collect()
    }

    fn reward_recipient_actor(
        &self,
        character_id: &CharacterId,
        target_location: &crate::model::WorldPosition,
    ) -> Option<usize> {
        if !self.presence_is_reward_eligible(character_id) {
            return None;
        }
        self.world.actors.iter().position(|actor| {
            actor.character_id.as_ref() == Some(character_id)
                && actor.location.same_site(target_location)
                && actor
                    .location
                    .position
                    .chebyshev_distance(target_location.position)
                    <= PLAYER_OBSERVATION_RADIUS as i32
        })
    }

    fn presence_is_reward_eligible(&self, character_id: &CharacterId) -> bool {
        self.world
            .character_presence
            .get(character_id)
            .is_some_and(|presence| {
                presence.connected
                    || presence.absent_since.is_some_and(|absent_since| {
                        self.current_time()
                            .value()
                            .saturating_sub(absent_since.value())
                            < GROUP_DISCONNECT_GRACE_UNITS
                    })
            })
    }

    fn eligible_group_reward_members(
        &self,
        group_id: crate::model::GroupId,
        cohort: &[GroupMembershipKey],
        target_location: &crate::model::WorldPosition,
    ) -> Vec<(u64, CharacterId)> {
        let Some(group) = self.world.groups.get(&group_id) else {
            return Vec::new();
        };
        let mut eligible = group
            .members
            .iter()
            .filter(|member| cohort.binary_search(&member.membership_key()).is_ok())
            .filter(|member| {
                self.reward_recipient_actor(&member.character_id, target_location)
                    .is_some()
            })
            .map(|member| (member.joined_order, member.character_id.clone()))
            .collect::<Vec<_>>();
        eligible.sort();
        eligible
    }

    pub(super) fn reattribute_dissolved_group_contributions(
        &mut self,
        group_id: crate::model::GroupId,
    ) -> Result<(), StepError> {
        for ledger in self.world.defeat_contributions.values_mut() {
            let Some(group_unit) = ledger
                .reward_units
                .remove(&DefeatRewardUnitId::Group { group_id })
            else {
                continue;
            };
            for (key, damage) in group_unit.slices {
                let solo_id = DefeatRewardUnitId::Solo {
                    character_id: key.contributor_character_id.clone(),
                };
                let solo_key = DefeatContributionKey {
                    contributor_character_id: key.contributor_character_id,
                    reward_class: key.reward_class,
                    eligible_memberships: Vec::new(),
                };
                let amount = ledger
                    .reward_units
                    .entry(solo_id)
                    .or_insert_with(DefeatRewardUnitContribution::default)
                    .slices
                    .entry(solo_key)
                    .or_default();
                *amount = amount
                    .checked_add(damage)
                    .ok_or_else(|| StepError::new("dissolved group contribution overflow"))?;
            }
        }
        Ok(())
    }
}

impl DefeatContext {
    fn reward_source(&self) -> Option<(&ActorId, DefeatRewardClass)> {
        if let Some(credit) = self.spell_damage_credit.as_ref() {
            let class = match credit.reward_class {
                SpellDamageRewardClass::Directed => DefeatRewardClass::DirectedSpell,
                SpellDamageRewardClass::AreaOrIllusion => DefeatRewardClass::AreaOrIllusionSpell,
            };
            return Some((&credit.caster_actor_id, class));
        }
        self.credited_actor_id
            .as_ref()
            .map(|actor_id| (actor_id, DefeatRewardClass::Physical))
    }
}

fn gcd(mut left: u64, mut right: u64) -> u64 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left
}

fn largest_remainder<K: Ord + Clone>(
    total: i32,
    weighted: &[(K, u128)],
) -> Result<Vec<i32>, StepError> {
    if total <= 0 || weighted.is_empty() {
        return Ok(vec![0; weighted.len()]);
    }
    let denominator = weighted
        .iter()
        .try_fold(0_u128, |sum, (_, weight)| sum.checked_add(*weight))
        .ok_or_else(|| StepError::new("largest-remainder weight overflow"))?;
    if denominator == 0 {
        return Ok(vec![0; weighted.len()]);
    }
    let total =
        u128::try_from(total).map_err(|_| StepError::new("largest-remainder total is invalid"))?;
    let mut allocations = Vec::with_capacity(weighted.len());
    let mut remainders = Vec::with_capacity(weighted.len());
    let mut assigned = 0_u128;
    for (index, (key, weight)) in weighted.iter().enumerate() {
        let scaled = total
            .checked_mul(*weight)
            .ok_or_else(|| StepError::new("largest-remainder multiplication overflow"))?;
        let floor = scaled / denominator;
        assigned = assigned
            .checked_add(floor)
            .ok_or_else(|| StepError::new("largest-remainder assigned overflow"))?;
        allocations.push(floor);
        remainders.push((scaled % denominator, key.clone(), index));
    }
    remainders.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    let remainder_count = usize::try_from(total.saturating_sub(assigned))
        .map_err(|_| StepError::new("largest-remainder count exceeds usize"))?;
    for (_, _, index) in remainders.into_iter().take(remainder_count) {
        allocations[index] = allocations[index]
            .checked_add(1)
            .ok_or_else(|| StepError::new("largest-remainder allocation overflow"))?;
    }
    allocations
        .into_iter()
        .map(|amount| {
            i32::try_from(amount)
                .map_err(|_| StepError::new("largest-remainder allocation exceeds i32"))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        CharacterPresenceState, CommunicationPreferences, DeathCause, SocialIntent,
        SpellDamageCredit,
    };

    fn reward_engine(two_players: bool) -> Engine {
        let engine = crate::engine::setup::test_engine("skill_progression");
        let mut engine = engine
            .prepare_character_id_rekey(&ActorId::new("player"), CharacterId::new("character:0"))
            .expect("rekey player");
        if two_players {
            let mut second = engine.world.actors[0].clone();
            second.id = ActorId::new("player1");
            second.name = "Player 1".to_string();
            second.character_id = Some(CharacterId::new("character:1"));
            second.timing.tie_break_order += 100;
            second.carried.items.clear();
            second.carried.gold = Default::default();
            engine.world.actors.push(second);
            engine.world.communication_preferences.insert(
                CharacterId::new("character:1"),
                CommunicationPreferences::default(),
            );
            engine.world.character_presence.insert(
                CharacterId::new("character:1"),
                CharacterPresenceState {
                    connected: true,
                    control_epoch: 1,
                    absent_since: None,
                },
            );
        }
        let target_index = engine
            .world
            .actors
            .iter()
            .position(|actor| actor.id == "mireling")
            .expect("reward target");
        let player_location = engine.world.actors[0].location.clone();
        engine.world.actors[target_index].location = player_location.clone();
        engine.world.actors[target_index].xp_value = 100;
        if two_players {
            let second_index = engine
                .world
                .actors
                .iter()
                .position(|actor| actor.id == "player1")
                .unwrap();
            engine.world.actors[second_index].location = player_location;
        }
        engine
    }

    fn target_index(engine: &Engine) -> usize {
        engine
            .world
            .actors
            .iter()
            .position(|actor| actor.id == "mireling")
            .unwrap()
    }

    fn physical(actor_id: &str) -> DefeatContext {
        DefeatContext {
            cause: DeathCause::Physical,
            credited_actor_id: Some(ActorId::new(actor_id)),
            direct_social_actor_id: None,
            spell_damage_credit: None,
            hostile_authority: None,
        }
    }

    fn area(actor_id: &str) -> DefeatContext {
        DefeatContext {
            cause: DeathCause::OtherMagic,
            credited_actor_id: None,
            direct_social_actor_id: None,
            spell_damage_credit: Some(SpellDamageCredit {
                caster_actor_id: ActorId::new(actor_id),
                spell_id: "area_test".to_string(),
                reward_class: SpellDamageRewardClass::AreaOrIllusion,
            }),
            hostile_authority: None,
        }
    }

    fn create_group(engine: &mut Engine) -> crate::model::GroupId {
        let invite = engine
            .apply_social_intent(
                &ActorId::new("player"),
                SocialIntent::Invite {
                    target_character_id: CharacterId::new("character:1"),
                },
            )
            .unwrap();
        let invitation_id = invite
            .events
            .iter()
            .find_map(|event| match event {
                Event::GroupInvitationCreated { invitation_id, .. } => Some(*invitation_id),
                _ => None,
            })
            .unwrap();
        engine
            .apply_social_intent(
                &ActorId::new("player1"),
                SocialIntent::AcceptInvite { invitation_id },
            )
            .unwrap();
        engine
            .group_id_for_character(&CharacterId::new("character:0"))
            .unwrap()
    }

    fn awarded(events: &[Event], character_id: &str) -> i32 {
        events
            .iter()
            .filter_map(|event| match event {
                Event::DefeatRewardShareAwarded {
                    character_id: awarded,
                    amount,
                    ..
                } if awarded.as_str() == character_id => Some(*amount),
                _ => None,
            })
            .sum()
    }

    #[test]
    fn mixed_group_damage_is_one_unit_and_shares_with_support_member() {
        let mut engine = reward_engine(true);
        create_group(&mut engine);
        let target = target_index(&engine);
        let mut events = Vec::new();
        engine
            .record_defeat_contribution(target, 6, &physical("player"), &mut events)
            .unwrap();
        engine
            .record_defeat_contribution(target, 4, &area("player1"), &mut events)
            .unwrap();
        engine.award_defeat_rewards(target, &mut events).unwrap();
        assert_eq!(awarded(&events, "character:0"), 38);
        assert_eq!(awarded(&events, "character:1"), 38);
        assert!(events.iter().any(|event| matches!(
            event,
            Event::DefeatRewardEvaluated {
                available_experience: 76,
                awarded_experience: 76,
                ..
            }
        )));
    }

    #[test]
    fn environmental_damage_reduces_pool_without_stealing_prior_credit() {
        let mut engine = reward_engine(false);
        let target = target_index(&engine);
        let environment = DefeatContext {
            cause: DeathCause::Hazard,
            credited_actor_id: None,
            direct_social_actor_id: None,
            spell_damage_credit: None,
            hostile_authority: None,
        };
        let mut events = Vec::new();
        engine
            .record_defeat_contribution(target, 5, &physical("player"), &mut events)
            .unwrap();
        engine
            .record_defeat_contribution(target, 5, &environment, &mut events)
            .unwrap();
        engine.award_defeat_rewards(target, &mut events).unwrap();
        assert_eq!(awarded(&events, "character:0"), 50);
    }

    #[test]
    fn joining_after_damage_cannot_inherit_the_old_solo_unit() {
        let mut engine = reward_engine(true);
        let target = target_index(&engine);
        let mut events = Vec::new();
        engine
            .record_defeat_contribution(target, 10, &physical("player"), &mut events)
            .unwrap();
        create_group(&mut engine);
        engine.award_defeat_rewards(target, &mut events).unwrap();
        assert_eq!(awarded(&events, "character:0"), 100);
        assert_eq!(awarded(&events, "character:1"), 0);
    }

    #[test]
    fn dissolving_group_reattributes_open_damage_to_actual_contributor() {
        let mut engine = reward_engine(true);
        create_group(&mut engine);
        let target = target_index(&engine);
        let mut events = Vec::new();
        engine
            .record_defeat_contribution(target, 10, &physical("player"), &mut events)
            .unwrap();
        engine
            .apply_social_intent(&ActorId::new("player1"), SocialIntent::LeaveGroup)
            .unwrap();
        engine.award_defeat_rewards(target, &mut events).unwrap();
        assert_eq!(awarded(&events, "character:0"), 100);
        assert_eq!(awarded(&events, "character:1"), 0);
    }

    #[test]
    fn equal_group_share_uses_stable_largest_remainder_order() {
        let mut engine = reward_engine(true);
        create_group(&mut engine);
        let target = target_index(&engine);
        engine.world.actors[target].xp_value = 101;
        let mut events = Vec::new();
        engine
            .record_defeat_contribution(target, 10, &physical("player"), &mut events)
            .unwrap();
        engine.award_defeat_rewards(target, &mut events).unwrap();
        assert_eq!(awarded(&events, "character:0"), 51);
        assert_eq!(awarded(&events, "character:1"), 50);
    }
}

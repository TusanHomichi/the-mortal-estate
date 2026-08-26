use std::collections::BTreeMap;

use crate::model::{
    ActorId, ActorKind, CharacterId, DefeatContributionKey, DefeatRewardUnitContribution,
    DefeatRewardUnitId, GroupMembershipKey, ItemBindingState, LootClaim, LootOwnerId,
};

use super::{Engine, StepError};

impl Engine {
    pub fn prepare_character_id_rekey(
        &self,
        actor_id: &ActorId,
        server_character_id: CharacterId,
    ) -> Result<Self, StepError> {
        let actor = self
            .world
            .actor(actor_id)
            .ok_or_else(|| StepError::new("character rekey actor is missing"))?;
        if actor.kind != ActorKind::Player {
            return Err(StepError::new("character rekey actor is not a player"));
        }
        let old = actor
            .character_id
            .clone()
            .ok_or_else(|| StepError::new("character rekey actor has no character ID"))?;
        if old == server_character_id {
            return Err(StepError::new(
                "character rekey must replace the current ID",
            ));
        }
        if self.character_id_occurs(&server_character_id) {
            return Err(StepError::new("server character ID already exists"));
        }
        if self
            .world
            .actors
            .iter()
            .filter(|candidate| candidate.character_id.as_ref() == Some(&old))
            .count()
            != 1
        {
            return Err(StepError::new(
                "authored character ID is not uniquely owned",
            ));
        }

        let mut candidate = self.clone();
        rekey_world(&mut candidate, &old, &server_character_id);
        if candidate.character_id_occurs(&old)
            || !candidate.character_id_occurs(&server_character_id)
        {
            return Err(StepError::new("character rekey was not exhaustive"));
        }
        candidate.validate_world_item_locations()?;
        Ok(candidate)
    }

    fn character_id_occurs(&self, character_id: &CharacterId) -> bool {
        self.world
            .actors
            .iter()
            .any(|actor| actor.character_id.as_ref() == Some(character_id))
            || self.world.banks.values().any(|bank| bank.balances.contains_key(character_id))
            || self
                .world
                .locker_vaults
                .values()
                .any(|vault| vault.lockers.contains_key(character_id))
            || self.world.quest_states.contains_key(character_id)
            || self
                .world
                .communication_preferences
                .contains_key(character_id)
            || self.world.character_presence.contains_key(character_id)
            || self.world.groups.values().any(|group| {
                &group.leader_character_id == character_id
                    || group
                        .members
                        .iter()
                        .any(|member| &member.character_id == character_id)
            })
            || self.world.group_invitations.values().any(|invitation| {
                &invitation.issuer_character_id == character_id
                    || &invitation.target_character_id == character_id
            })
            || self.world.player_follow_targets.iter().any(|(follower, target)| {
                follower == character_id || target == character_id
            })
            || self.world.defeat_contributions.values().any(|ledger| {
                ledger.reward_units.iter().any(|(unit_id, unit)| {
                    matches!(unit_id, DefeatRewardUnitId::Solo { character_id: owner } if owner == character_id)
                        || unit.slices.keys().any(|slice| {
                            &slice.contributor_character_id == character_id
                                || slice
                                    .eligible_memberships
                                    .iter()
                                    .any(|membership| &membership.character_id == character_id)
                        })
                })
            })
            || self.world.item_instances.values().any(|item| {
                matches!(&item.binding, ItemBindingState::Bound { character_id: bound } if bound == character_id)
            })
            || self.world.item_offers.values().any(|offer| {
                &offer.sender_character_id == character_id
                    || &offer.recipient_character_id == character_id
            })
            || self.world.corpses.values().any(|corpse| {
                corpse.origin_character_id.as_ref() == Some(character_id)
                    || corpse
                        .loot_claim
                        .as_ref()
                        .is_some_and(|claim| claim_owned_by(claim, character_id))
            })
            || self.world.ground_items.iter().any(|item| {
                item.loot_claim
                    .as_ref()
                    .is_some_and(|claim| claim_owned_by(claim, character_id))
            })
            || self.world.ground_gold.values().any(|gold| {
                gold.loot_claim
                    .as_ref()
                    .is_some_and(|claim| claim_owned_by(claim, character_id))
            })
            || self.world.actors.iter().any(|actor| {
                actor
                    .npc
                    .as_ref()
                    .and_then(|npc| npc.following_character_id.as_ref())
                    == Some(character_id)
            })
            || self.world.social_relations.self_defense.iter().any(|(key, relation)| {
                key == character_id
                    || &relation.victim_character_id == character_id
                    || &relation.attacker_character_id == character_id
            })
    }
}

fn rekey_world(engine: &mut Engine, old: &CharacterId, new: &CharacterId) {
    for actor in &mut engine.world.actors {
        replace_option(&mut actor.character_id, old, new);
        if let Some(npc) = &mut actor.npc {
            replace_option(&mut npc.following_character_id, old, new);
        }
    }
    for item in engine.world.item_instances.values_mut() {
        if let ItemBindingState::Bound { character_id } = &mut item.binding
            && character_id == old
        {
            *character_id = new.clone();
        }
    }
    for bank in engine.world.banks.values_mut() {
        rekey_map(&mut bank.balances, old, new);
    }
    for vault in engine.world.locker_vaults.values_mut() {
        rekey_map(&mut vault.lockers, old, new);
    }
    rekey_map(&mut engine.world.quest_states, old, new);
    for offer in engine.world.item_offers.values_mut() {
        replace_value(&mut offer.sender_character_id, old, new);
        replace_value(&mut offer.recipient_character_id, old, new);
    }
    for corpse in engine.world.corpses.values_mut() {
        replace_option(&mut corpse.origin_character_id, old, new);
        if let Some(claim) = &mut corpse.loot_claim {
            rekey_claim(claim, old, new);
        }
    }
    for item in &mut engine.world.ground_items {
        if let Some(claim) = &mut item.loot_claim {
            rekey_claim(claim, old, new);
        }
    }
    for gold in engine.world.ground_gold.values_mut() {
        if let Some(claim) = &mut gold.loot_claim {
            rekey_claim(claim, old, new);
        }
    }
    for group in engine.world.groups.values_mut() {
        replace_value(&mut group.leader_character_id, old, new);
        for member in &mut group.members {
            replace_value(&mut member.character_id, old, new);
        }
    }
    for invitation in engine.world.group_invitations.values_mut() {
        replace_value(&mut invitation.issuer_character_id, old, new);
        replace_value(&mut invitation.target_character_id, old, new);
    }
    let follows = std::mem::take(&mut engine.world.player_follow_targets);
    for (mut follower, mut target) in follows {
        replace_value(&mut follower, old, new);
        replace_value(&mut target, old, new);
        engine.world.player_follow_targets.insert(follower, target);
    }
    rekey_map(&mut engine.world.communication_preferences, old, new);
    for preferences in engine.world.communication_preferences.values_mut() {
        if preferences.blocked_character_ids.remove(old) {
            preferences.blocked_character_ids.insert(new.clone());
        }
    }
    rekey_map(&mut engine.world.character_presence, old, new);
    for ledger in engine.world.defeat_contributions.values_mut() {
        let reward_units = std::mem::take(&mut ledger.reward_units);
        for (unit_id, unit) in reward_units {
            let unit_id = match unit_id {
                DefeatRewardUnitId::Solo { mut character_id } => {
                    replace_value(&mut character_id, old, new);
                    DefeatRewardUnitId::Solo { character_id }
                }
                group @ DefeatRewardUnitId::Group { .. } => group,
            };
            let mut rekeyed = DefeatRewardUnitContribution::default();
            for (key, damage) in unit.slices {
                let mut contributor_character_id = key.contributor_character_id;
                replace_value(&mut contributor_character_id, old, new);
                let eligible_memberships = key
                    .eligible_memberships
                    .into_iter()
                    .map(|membership| GroupMembershipKey {
                        character_id: if &membership.character_id == old {
                            new.clone()
                        } else {
                            membership.character_id
                        },
                        membership_epoch: membership.membership_epoch,
                    })
                    .collect();
                let key = DefeatContributionKey {
                    contributor_character_id,
                    reward_class: key.reward_class,
                    eligible_memberships,
                };
                *rekeyed.slices.entry(key).or_default() += damage;
            }
            ledger.reward_units.insert(unit_id, rekeyed);
        }
    }
    let relations = std::mem::take(&mut engine.world.social_relations.self_defense);
    for (mut key, mut relation) in relations {
        replace_value(&mut key, old, new);
        replace_value(&mut relation.victim_character_id, old, new);
        replace_value(&mut relation.attacker_character_id, old, new);
        engine
            .world
            .social_relations
            .self_defense
            .insert(key, relation);
    }
    for link in &mut engine.world.linked_player_kill_karma {
        replace_value(&mut link.killer_character_id, old, new);
        replace_value(&mut link.victim_character_id, old, new);
    }
    engine.world.linked_player_kill_karma.sort();
}

fn replace_option(value: &mut Option<CharacterId>, old: &CharacterId, new: &CharacterId) {
    if value.as_ref() == Some(old) {
        *value = Some(new.clone());
    }
}

fn replace_value(value: &mut CharacterId, old: &CharacterId, new: &CharacterId) {
    if value == old {
        *value = new.clone();
    }
}

fn rekey_map<T>(map: &mut BTreeMap<CharacterId, T>, old: &CharacterId, new: &CharacterId) {
    if let Some(value) = map.remove(old) {
        map.insert(new.clone(), value);
    }
}

fn rekey_claim(claim: &mut LootClaim, old: &CharacterId, new: &CharacterId) {
    if let LootOwnerId::Character(character_id) = &mut claim.owner {
        replace_value(character_id, old, new);
    }
}

fn claim_owned_by(claim: &LootClaim, character_id: &CharacterId) -> bool {
    matches!(&claim.owner, LootOwnerId::Character(owner) if owner == character_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn character_transfer_rekey_is_exhaustive_and_non_mutating() {
        let engine = crate::engine::setup::test_engine("world_topology_gallery");
        let actor_id = ActorId::from("player");
        let old_character_id = engine
            .world()
            .actor(&actor_id)
            .and_then(|actor| actor.character_id.clone())
            .expect("fixture character ID");
        let server_character_id = CharacterId::new("019abcdef-server-character");

        let rekeyed = engine
            .prepare_character_id_rekey(&actor_id, server_character_id.clone())
            .expect("server rekey");

        assert!(engine.character_id_occurs(&old_character_id));
        assert!(!engine.character_id_occurs(&server_character_id));
        assert!(!rekeyed.character_id_occurs(&old_character_id));
        assert!(rekeyed.character_id_occurs(&server_character_id));
    }
}

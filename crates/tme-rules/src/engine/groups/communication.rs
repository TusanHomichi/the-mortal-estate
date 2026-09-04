use super::*;

impl Engine {
    pub(in crate::engine) fn process_player_follow_opportunities(
        &mut self,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let mut followers = self
            .world
            .player_follow_targets
            .keys()
            .filter_map(|character_id| {
                let actor_index = self.character_actor_index(character_id).ok()?;
                let actor = &self.world.actors[actor_index];
                (actor.timing.ready_at <= self.current_time())
                    .then_some((actor.timing.tie_break_order, character_id.clone()))
            })
            .collect::<Vec<_>>();
        followers.sort();
        for (_, follower_character_id) in followers {
            let Some(target_character_id) = self
                .world
                .player_follow_targets
                .get(&follower_character_id)
                .cloned()
            else {
                continue;
            };
            let Ok(follower_index) = self.character_actor_index(&follower_character_id) else {
                self.remove_follow_edges_for_character(
                    &follower_character_id,
                    PlayerFollowChangeReasonV1::TargetLost,
                    events,
                );
                continue;
            };
            let Ok(target_index) = self.character_actor_index(&target_character_id) else {
                self.remove_follow_edges_for_character(
                    &target_character_id,
                    PlayerFollowChangeReasonV1::TargetLost,
                    events,
                );
                continue;
            };
            if !self.world.actors[follower_index].is_alive()
                || !self.world.actors[target_index].is_alive()
                || !self.presence_is_connected(&follower_character_id)
                || !self.presence_is_connected(&target_character_id)
                || self.group_id_for_character(&follower_character_id)
                    != self.group_id_for_character(&target_character_id)
            {
                self.remove_follow_edges_for_character(
                    &follower_character_id,
                    PlayerFollowChangeReasonV1::TargetLost,
                    events,
                );
                continue;
            }
            if !self.actor_can_see(follower_index, &self.world.actors[target_index].location) {
                self.remove_follow_edges_for_character(
                    &follower_character_id,
                    PlayerFollowChangeReasonV1::ObservationLost,
                    events,
                );
                continue;
            }
            let from = self.world.actors[follower_index].location.position;
            let target = self.world.actors[target_index].location.position;
            let Some(direction) = direction_toward(from, target) else {
                continue;
            };
            self.try_actor_move(follower_index, direction, events)?;
            self.schedule_actor(follower_index, ActionCost::STANDARD, events)?;
        }
        Ok(())
    }

    pub(in crate::engine) fn presence_is_connected(&self, character_id: &CharacterId) -> bool {
        self.world
            .character_presence
            .get(character_id)
            .is_some_and(|presence| presence.connected)
    }

    pub(in crate::engine) fn character_actor_index(
        &self,
        character_id: &CharacterId,
    ) -> Result<usize, StepError> {
        self.world
            .actors
            .iter()
            .position(|actor| actor.character_id.as_ref() == Some(character_id))
            .ok_or_else(|| StepError::new("unknown local character"))
    }

    pub(in crate::engine) fn group_id_for_character(
        &self,
        character_id: &CharacterId,
    ) -> Option<GroupId> {
        self.world.groups.iter().find_map(|(group_id, group)| {
            group
                .members
                .iter()
                .any(|member| &member.character_id == character_id)
                .then_some(*group_id)
        })
    }

    pub(in crate::engine) fn characters_block_each_other(
        &self,
        left: &CharacterId,
        right: &CharacterId,
    ) -> bool {
        self.world
            .communication_preferences
            .get(left)
            .is_some_and(|preferences| preferences.blocked_character_ids.contains(right))
            || self
                .world
                .communication_preferences
                .get(right)
                .is_some_and(|preferences| preferences.blocked_character_ids.contains(left))
    }

    pub fn social_broadcast_recipients(
        &self,
        sender_character_id: &CharacterId,
        scope: SocialBroadcastScope,
    ) -> Result<Vec<CharacterId>, StepError> {
        let sender_index = self.character_actor_index(sender_character_id)?;
        let sender = &self.world.actors[sender_index];
        let sender_group = self.group_id_for_character(sender_character_id);
        if scope == SocialBroadcastScope::Group && sender_group.is_none() {
            return Err(StepError::new("group message requires a group"));
        }
        let mut recipients = self
            .world
            .actors
            .iter()
            .enumerate()
            .filter_map(|(recipient_index, recipient)| {
                let recipient_character_id = recipient.character_id.as_ref()?;
                if recipient_character_id == sender_character_id
                    || !self.presence_is_connected(recipient_character_id)
                    || self.characters_block_each_other(sender_character_id, recipient_character_id)
                {
                    return None;
                }
                let included = match scope {
                    SocialBroadcastScope::Say => {
                        self.actor_can_see(recipient_index, &sender.location)
                    }
                    SocialBroadcastScope::Shout => {
                        recipient.location.same_site(&sender.location)
                            && recipient
                                .location
                                .position
                                .chebyshev_distance(sender.location.position)
                                <= 6
                    }
                    SocialBroadcastScope::Group => {
                        self.group_id_for_character(recipient_character_id) == sender_group
                    }
                };
                included.then(|| recipient_character_id.clone())
            })
            .collect::<Vec<_>>();
        recipients.sort();
        Ok(recipients)
    }

    pub fn page_source_allows(
        &self,
        sender_character_id: &CharacterId,
        target_character_id: &CharacterId,
    ) -> bool {
        self.character_actor_index(sender_character_id).is_ok()
            && self
                .world
                .communication_preferences
                .get(sender_character_id)
                .is_some_and(|preferences| {
                    !preferences
                        .blocked_character_ids
                        .contains(target_character_id)
                })
    }

    pub fn page_target_allows(
        &self,
        target_character_id: &CharacterId,
        sender_character_id: &CharacterId,
    ) -> bool {
        self.character_actor_index(target_character_id).is_ok()
            && self.presence_is_connected(target_character_id)
            && self
                .world
                .communication_preferences
                .get(target_character_id)
                .is_some_and(|preferences| {
                    preferences.pages_enabled
                        && !preferences
                            .blocked_character_ids
                            .contains(sender_character_id)
                })
    }
}

use std::collections::BTreeSet;

use crate::events::{
    Event, GroupChangeReasonV1, GroupInvitationResolutionV1, PlayerFollowChangeReasonV1,
};
use crate::model::{
    ActionCost, CharacterId, CommunicationPreferences, Direction, GROUP_DISCONNECT_GRACE_UNITS,
    GROUP_INVITATION_LIFETIME_UNITS, GroupId, GroupInvitationState, GroupInviteId,
    GroupMemberState, GroupState, MAX_BLOCKED_CHARACTERS, MAX_GROUP_MEMBERS,
    MAX_INCOMING_GROUP_INVITATIONS, MAX_OUTGOING_GROUP_INVITATIONS, SocialBroadcastScope,
    SocialIntent,
};

use super::{Engine, RulesOutcomeV1, StepError};

impl Engine {
    pub fn apply_social_intent(
        &mut self,
        actor_id: &crate::model::ActorId,
        intent: SocialIntent,
    ) -> Result<RulesOutcomeV1, StepError> {
        let before = self.clone();
        match self.apply_social_intent_transaction(actor_id, intent) {
            Ok(events) => Ok(RulesOutcomeV1 {
                events,
                state_changed: true,
                durable_effects: Vec::new(),
            }),
            Err(error) => {
                *self = before;
                Err(error)
            }
        }
    }

    fn apply_social_intent_transaction(
        &mut self,
        actor_id: &crate::model::ActorId,
        intent: SocialIntent,
    ) -> Result<Vec<Event>, StepError> {
        let actor_index = self.player_actor_index(actor_id)?;
        let character_id = self.world.actors[actor_index]
            .character_id
            .clone()
            .ok_or_else(|| StepError::new("social commands require a stable character"))?;
        if !self
            .world
            .communication_preferences
            .contains_key(&character_id)
        {
            return Err(StepError::new(
                "social command character has no communication preferences",
            ));
        }
        match intent {
            SocialIntent::Invite {
                target_character_id,
            } => self.invite_to_group(&character_id, &target_character_id),
            SocialIntent::AcceptInvite { invitation_id } => {
                self.accept_group_invitation(&character_id, invitation_id)
            }
            SocialIntent::DeclineInvite { invitation_id } => self.resolve_group_invitation(
                &character_id,
                invitation_id,
                false,
                GroupInvitationResolutionV1::Declined,
            ),
            SocialIntent::CancelInvite { invitation_id } => self.resolve_group_invitation(
                &character_id,
                invitation_id,
                true,
                GroupInvitationResolutionV1::Cancelled,
            ),
            SocialIntent::LeaveGroup => self.leave_group(&character_id),
            SocialIntent::RemoveMember {
                member_character_id,
            } => self.remove_group_member(&character_id, &member_character_id),
            SocialIntent::DisbandGroup => self.disband_group(&character_id),
            SocialIntent::TransferLeadership {
                member_character_id,
            } => self.transfer_group_leadership(&character_id, &member_character_id),
            SocialIntent::BeginFollow {
                target_character_id,
            } => self.begin_player_follow(&character_id, &target_character_id),
            SocialIntent::EndFollow => {
                self.end_player_follow(&character_id, PlayerFollowChangeReasonV1::Ended)
            }
            SocialIntent::SetPagesEnabled { enabled } => {
                let preferences = self
                    .world
                    .communication_preferences
                    .get_mut(&character_id)
                    .expect("validated communication preferences");
                if preferences.pages_enabled == enabled {
                    return Err(StepError::new("page preference already has that value"));
                }
                preferences.pages_enabled = enabled;
                Ok(vec![Event::CommunicationPreferenceChanged {
                    character_id,
                    pages_enabled: enabled,
                }])
            }
            SocialIntent::Block {
                target_character_id,
            } => self.change_character_block(&character_id, &target_character_id, true),
            SocialIntent::Unblock {
                target_character_id,
            } => self.change_character_block(&character_id, &target_character_id, false),
        }
    }

    pub fn apply_connection_presence(
        &mut self,
        character_id: &CharacterId,
        control_epoch: u64,
        connected: bool,
    ) -> Result<RulesOutcomeV1, StepError> {
        self.character_actor_index(character_id)?;
        let before = self.clone();
        let before_presence = self.world.character_presence.get(character_id).copied();
        match self.apply_connection_presence_transaction(character_id, control_epoch, connected) {
            Ok(events) => {
                let state_changed =
                    self.world.character_presence.get(character_id).copied() != before_presence;
                Ok(RulesOutcomeV1 {
                    state_changed,
                    events,
                    durable_effects: Vec::new(),
                })
            }
            Err(error) => {
                *self = before;
                Err(error)
            }
        }
    }

    fn apply_connection_presence_transaction(
        &mut self,
        character_id: &CharacterId,
        control_epoch: u64,
        connected: bool,
    ) -> Result<Vec<Event>, StepError> {
        let now = self.world.timing.now;
        let presence = self
            .world
            .character_presence
            .get_mut(character_id)
            .ok_or_else(|| StepError::new("presence character is unknown"))?;
        if control_epoch < presence.control_epoch {
            return Ok(Vec::new());
        }
        if control_epoch == presence.control_epoch && presence.connected == connected {
            return Ok(Vec::new());
        }
        presence.control_epoch = control_epoch;
        presence.connected = connected;
        presence.absent_since = (!connected).then_some(now);

        let Some(group_id) = self.group_id_for_character(character_id) else {
            return Ok(Vec::new());
        };
        let mut events = vec![Event::GroupPresenceChanged {
            group_id,
            character_id: character_id.clone(),
            connected,
            absent_since: (!connected).then_some(now),
        }];
        if connected {
            self.reconcile_group_leadership(group_id, &mut events)?;
        } else {
            self.remove_follow_edges_for_character(
                character_id,
                PlayerFollowChangeReasonV1::TargetLost,
                &mut events,
            );
        }
        Ok(events)
    }

    pub fn mark_all_characters_disconnected(&mut self) -> Result<RulesOutcomeV1, StepError> {
        let now = self.world.timing.now;
        let mut events = Vec::new();
        let mut changed = false;
        let group_by_character = self
            .world
            .groups
            .iter()
            .flat_map(|(group_id, group)| {
                group
                    .members
                    .iter()
                    .map(|member| (member.character_id.clone(), *group_id))
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        for (character_id, presence) in &mut self.world.character_presence {
            if presence.connected || presence.absent_since != Some(now) {
                changed = true;
                presence.connected = false;
                presence.absent_since = Some(now);
                if let Some(group_id) = group_by_character.get(character_id) {
                    events.push(Event::GroupPresenceChanged {
                        group_id: *group_id,
                        character_id: character_id.clone(),
                        connected: false,
                        absent_since: Some(now),
                    });
                }
            }
        }
        Ok(RulesOutcomeV1 {
            state_changed: changed,
            events,
            durable_effects: Vec::new(),
        })
    }

    pub(super) fn expire_group_state(&mut self, events: &mut Vec<Event>) -> Result<(), StepError> {
        let now = self.world.timing.now;
        let expired = self
            .world
            .group_invitations
            .iter()
            .filter_map(|(id, invitation)| (invitation.expires_at <= now).then_some(*id))
            .collect::<Vec<_>>();
        for invitation_id in expired {
            let invitation = self
                .world
                .group_invitations
                .remove(&invitation_id)
                .expect("selected invitation exists");
            events.push(Self::invitation_resolved_event(
                &invitation,
                GroupInvitationResolutionV1::Expired,
            ));
        }
        let group_ids = self.world.groups.keys().copied().collect::<Vec<_>>();
        for group_id in group_ids {
            self.reconcile_group_leadership(group_id, events)?;
        }
        Ok(())
    }

    pub(super) fn process_player_follow_opportunities(
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

    fn presence_is_connected(&self, character_id: &CharacterId) -> bool {
        self.world
            .character_presence
            .get(character_id)
            .is_some_and(|presence| presence.connected)
    }

    pub(super) fn character_actor_index(
        &self,
        character_id: &CharacterId,
    ) -> Result<usize, StepError> {
        self.world
            .actors
            .iter()
            .position(|actor| actor.character_id.as_ref() == Some(character_id))
            .ok_or_else(|| StepError::new("unknown local character"))
    }

    pub(super) fn group_id_for_character(&self, character_id: &CharacterId) -> Option<GroupId> {
        self.world.groups.iter().find_map(|(group_id, group)| {
            group
                .members
                .iter()
                .any(|member| &member.character_id == character_id)
                .then_some(*group_id)
        })
    }

    pub(super) fn characters_block_each_other(
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

    fn invite_to_group(
        &mut self,
        issuer_character_id: &CharacterId,
        target_character_id: &CharacterId,
    ) -> Result<Vec<Event>, StepError> {
        if issuer_character_id == target_character_id {
            return Err(StepError::new("cannot invite self"));
        }
        self.character_actor_index(target_character_id)?;
        if self.group_id_for_character(target_character_id).is_some() {
            return Err(StepError::new("target is already grouped"));
        }
        if self.characters_block_each_other(issuer_character_id, target_character_id) {
            return Err(StepError::new("group invitation is unavailable"));
        }

        let group_id = self.group_id_for_character(issuer_character_id);
        let issuer_membership_epoch = group_id.and_then(|id| {
            self.world.groups[&id]
                .members
                .iter()
                .find(|member| &member.character_id == issuer_character_id)
                .map(|member| member.membership_epoch)
        });
        if let Some(group_id) = group_id
            && self.world.groups[&group_id].members.len() >= MAX_GROUP_MEMBERS
        {
            return Err(StepError::new("group is full"));
        }
        let incoming = self
            .world
            .group_invitations
            .values()
            .filter(|invite| &invite.target_character_id == target_character_id)
            .count();
        if incoming >= MAX_INCOMING_GROUP_INVITATIONS {
            return Err(StepError::new("target has too many pending invitations"));
        }
        let outgoing = self
            .world
            .group_invitations
            .values()
            .filter(|invite| match group_id {
                Some(group_id) => invite.group_id == Some(group_id),
                None => {
                    invite.group_id.is_none() && &invite.issuer_character_id == issuer_character_id
                }
            })
            .count();
        if outgoing >= MAX_OUTGOING_GROUP_INVITATIONS {
            return Err(StepError::new("too many outgoing group invitations"));
        }
        if self.world.group_invitations.values().any(|invite| {
            invite.target_character_id == *target_character_id
                && match group_id {
                    Some(group_id) => invite.group_id == Some(group_id),
                    None => {
                        invite.group_id.is_none()
                            && invite.issuer_character_id == *issuer_character_id
                    }
                }
        }) {
            return Err(StepError::new("duplicate group invitation"));
        }

        let invitation_id = GroupInviteId::new(self.world.next_group_invite_sequence);
        self.world.next_group_invite_sequence = self
            .world
            .next_group_invite_sequence
            .checked_add(1)
            .ok_or_else(|| StepError::new("group invitation sequence overflow"))?;
        let expires_at = crate::model::LogicalTime::new(
            self.world
                .timing
                .now
                .value()
                .saturating_add(GROUP_INVITATION_LIFETIME_UNITS),
        );
        let invitation = GroupInvitationState {
            id: invitation_id,
            issuer_character_id: issuer_character_id.clone(),
            issuer_membership_epoch,
            group_id,
            target_character_id: target_character_id.clone(),
            expires_at,
        };
        self.world
            .group_invitations
            .insert(invitation_id, invitation.clone());
        Ok(vec![Event::GroupInvitationCreated {
            invitation_id,
            issuer_character_id: issuer_character_id.clone(),
            target_character_id: target_character_id.clone(),
            group_id,
            expires_at,
        }])
    }

    fn accept_group_invitation(
        &mut self,
        target_character_id: &CharacterId,
        invitation_id: GroupInviteId,
    ) -> Result<Vec<Event>, StepError> {
        let invitation = self
            .world
            .group_invitations
            .get(&invitation_id)
            .cloned()
            .ok_or_else(|| StepError::new("unknown group invitation"))?;
        if &invitation.target_character_id != target_character_id {
            return Err(StepError::new(
                "group invitation is not addressed to character",
            ));
        }
        if invitation.expires_at <= self.world.timing.now {
            return Err(StepError::new("group invitation has expired"));
        }
        if self.group_id_for_character(target_character_id).is_some() {
            return Err(StepError::new("character is already grouped"));
        }
        if self.characters_block_each_other(&invitation.issuer_character_id, target_character_id) {
            return Err(StepError::new("group invitation is unavailable"));
        }

        let mut events = Vec::new();
        let (group_id, reason) = if let Some(group_id) = invitation.group_id {
            let group = self
                .world
                .groups
                .get(&group_id)
                .ok_or_else(|| StepError::new("invitation group no longer exists"))?;
            let issuer_is_current = group.members.iter().any(|member| {
                member.character_id == invitation.issuer_character_id
                    && Some(member.membership_epoch) == invitation.issuer_membership_epoch
            });
            if !issuer_is_current {
                return Err(StepError::new("invitation issuer left the group"));
            }
            if group.members.len() >= MAX_GROUP_MEMBERS {
                return Err(StepError::new("group is full"));
            }
            self.add_group_member(group_id, target_character_id)?;
            (group_id, GroupChangeReasonV1::Joined)
        } else {
            if self
                .group_id_for_character(&invitation.issuer_character_id)
                .is_some()
            {
                return Err(StepError::new("solo invitation issuer is now grouped"));
            }
            let group_id = GroupId::new(self.world.next_group_sequence);
            self.world.next_group_sequence = self
                .world
                .next_group_sequence
                .checked_add(1)
                .ok_or_else(|| StepError::new("group sequence overflow"))?;
            let issuer_member = self.allocate_group_member(&invitation.issuer_character_id, 1)?;
            let target_member = self.allocate_group_member(target_character_id, 2)?;
            self.world.groups.insert(
                group_id,
                GroupState {
                    id: group_id,
                    leader_character_id: invitation.issuer_character_id.clone(),
                    members: vec![issuer_member, target_member],
                    next_join_order: 3,
                },
            );
            (group_id, GroupChangeReasonV1::Created)
        };

        let resolved = self
            .world
            .group_invitations
            .remove(&invitation_id)
            .expect("validated invitation exists");
        events.push(Self::invitation_resolved_event(
            &resolved,
            GroupInvitationResolutionV1::Accepted,
        ));
        self.invalidate_target_invitations(target_character_id, &mut events);
        events.push(self.group_changed_event(
            group_id,
            reason,
            Some(target_character_id.clone()),
        )?);
        Ok(events)
    }

    fn resolve_group_invitation(
        &mut self,
        actor_character_id: &CharacterId,
        invitation_id: GroupInviteId,
        issuer_action: bool,
        resolution: GroupInvitationResolutionV1,
    ) -> Result<Vec<Event>, StepError> {
        let invitation = self
            .world
            .group_invitations
            .get(&invitation_id)
            .ok_or_else(|| StepError::new("unknown group invitation"))?;
        let authorized = if issuer_action {
            &invitation.issuer_character_id == actor_character_id
        } else {
            &invitation.target_character_id == actor_character_id
        };
        if !authorized {
            return Err(StepError::new("group invitation action is unauthorized"));
        }
        let invitation = self
            .world
            .group_invitations
            .remove(&invitation_id)
            .expect("validated invitation exists");
        Ok(vec![Self::invitation_resolved_event(
            &invitation,
            resolution,
        )])
    }

    fn leave_group(&mut self, character_id: &CharacterId) -> Result<Vec<Event>, StepError> {
        let group_id = self
            .group_id_for_character(character_id)
            .ok_or_else(|| StepError::new("character is not grouped"))?;
        self.remove_member(group_id, character_id, GroupChangeReasonV1::Left)
    }

    fn remove_group_member(
        &mut self,
        leader_character_id: &CharacterId,
        member_character_id: &CharacterId,
    ) -> Result<Vec<Event>, StepError> {
        let group_id = self
            .group_id_for_character(leader_character_id)
            .ok_or_else(|| StepError::new("character is not grouped"))?;
        if self.world.groups[&group_id].leader_character_id != *leader_character_id {
            return Err(StepError::new("only the group leader may remove a member"));
        }
        if leader_character_id == member_character_id {
            return Err(StepError::new("leader must leave or disband explicitly"));
        }
        self.remove_member(group_id, member_character_id, GroupChangeReasonV1::Removed)
    }

    pub(super) fn remove_member(
        &mut self,
        group_id: GroupId,
        character_id: &CharacterId,
        reason: GroupChangeReasonV1,
    ) -> Result<Vec<Event>, StepError> {
        let group = self
            .world
            .groups
            .get_mut(&group_id)
            .ok_or_else(|| StepError::new("group no longer exists"))?;
        let index = group
            .members
            .iter()
            .position(|member| &member.character_id == character_id)
            .ok_or_else(|| StepError::new("character is not in group"))?;
        let was_leader = group.leader_character_id == *character_id;
        group.members.remove(index);
        if was_leader && !group.members.is_empty() {
            group
                .members
                .sort_by_key(|member| (member.joined_order, member.character_id.clone()));
            group.leader_character_id = group.members[0].character_id.clone();
        }
        let mut events = Vec::new();
        self.cancel_invitations_by_issuer(character_id, &mut events);
        self.remove_follow_edges_for_character(
            character_id,
            PlayerFollowChangeReasonV1::MembershipLost,
            &mut events,
        );
        if self.world.groups[&group_id].members.len() < 2 {
            self.dissolve_group(group_id, GroupChangeReasonV1::Dissolved, &mut events)?;
        } else {
            events.push(self.group_changed_event(group_id, reason, Some(character_id.clone()))?);
            if was_leader {
                events.push(self.group_changed_event(
                    group_id,
                    GroupChangeReasonV1::LeadershipFallback,
                    None,
                )?);
            }
        }
        Ok(events)
    }

    fn disband_group(
        &mut self,
        leader_character_id: &CharacterId,
    ) -> Result<Vec<Event>, StepError> {
        let group_id = self
            .group_id_for_character(leader_character_id)
            .ok_or_else(|| StepError::new("character is not grouped"))?;
        if self.world.groups[&group_id].leader_character_id != *leader_character_id {
            return Err(StepError::new("only the group leader may disband"));
        }
        let mut events = Vec::new();
        self.dissolve_group(group_id, GroupChangeReasonV1::Disbanded, &mut events)?;
        Ok(events)
    }

    fn transfer_group_leadership(
        &mut self,
        leader_character_id: &CharacterId,
        member_character_id: &CharacterId,
    ) -> Result<Vec<Event>, StepError> {
        let group_id = self
            .group_id_for_character(leader_character_id)
            .ok_or_else(|| StepError::new("character is not grouped"))?;
        let group = self
            .world
            .groups
            .get_mut(&group_id)
            .expect("selected group exists");
        if group.leader_character_id != *leader_character_id {
            return Err(StepError::new(
                "only the group leader may transfer leadership",
            ));
        }
        if leader_character_id == member_character_id {
            return Err(StepError::new("character is already group leader"));
        }
        if !group
            .members
            .iter()
            .any(|member| &member.character_id == member_character_id)
        {
            return Err(StepError::new("leadership target is not a group member"));
        }
        group.leader_character_id = member_character_id.clone();
        Ok(vec![self.group_changed_event(
            group_id,
            GroupChangeReasonV1::LeadershipTransferred,
            Some(member_character_id.clone()),
        )?])
    }

    fn begin_player_follow(
        &mut self,
        follower_character_id: &CharacterId,
        target_character_id: &CharacterId,
    ) -> Result<Vec<Event>, StepError> {
        if follower_character_id == target_character_id {
            return Err(StepError::new("cannot follow self"));
        }
        let follower_group = self
            .group_id_for_character(follower_character_id)
            .ok_or_else(|| StepError::new("follow requires a group"))?;
        if self.group_id_for_character(target_character_id) != Some(follower_group) {
            return Err(StepError::new("follow target is not in the same group"));
        }
        let mut cursor = target_character_id;
        let mut visited = BTreeSet::new();
        while let Some(next) = self.world.player_follow_targets.get(cursor) {
            if next == follower_character_id {
                return Err(StepError::new("follow cycle is not allowed"));
            }
            if !visited.insert(cursor.clone()) {
                return Err(StepError::new("existing follow graph contains a cycle"));
            }
            cursor = next;
        }
        if self.world.player_follow_targets.get(follower_character_id) == Some(target_character_id)
        {
            return Err(StepError::new("follow target is unchanged"));
        }
        self.world
            .player_follow_targets
            .insert(follower_character_id.clone(), target_character_id.clone());
        Ok(vec![Event::PlayerFollowChanged {
            follower_character_id: follower_character_id.clone(),
            target_character_id: Some(target_character_id.clone()),
            reason: PlayerFollowChangeReasonV1::Began,
        }])
    }

    fn end_player_follow(
        &mut self,
        follower_character_id: &CharacterId,
        reason: PlayerFollowChangeReasonV1,
    ) -> Result<Vec<Event>, StepError> {
        let target = self
            .world
            .player_follow_targets
            .remove(follower_character_id)
            .ok_or_else(|| StepError::new("character is not following"))?;
        Ok(vec![Event::PlayerFollowChanged {
            follower_character_id: follower_character_id.clone(),
            target_character_id: Some(target),
            reason,
        }])
    }

    fn change_character_block(
        &mut self,
        character_id: &CharacterId,
        target_character_id: &CharacterId,
        blocked: bool,
    ) -> Result<Vec<Event>, StepError> {
        if character_id == target_character_id {
            return Err(StepError::new("cannot block self"));
        }
        if let Some(group_id) = self.group_id_for_character(character_id)
            && self.group_id_for_character(target_character_id) == Some(group_id)
        {
            return Err(StepError::new(
                "current group members cannot block each other",
            ));
        }
        let preferences = self
            .world
            .communication_preferences
            .get_mut(character_id)
            .expect("validated communication preferences");
        if blocked {
            if preferences
                .blocked_character_ids
                .contains(target_character_id)
            {
                return Err(StepError::new("character is already blocked"));
            }
            if preferences.blocked_character_ids.len() >= MAX_BLOCKED_CHARACTERS {
                return Err(StepError::new("block list is full"));
            }
            preferences
                .blocked_character_ids
                .insert(target_character_id.clone());
        } else if !preferences
            .blocked_character_ids
            .remove(target_character_id)
        {
            return Err(StepError::new("character is not blocked"));
        }
        let mut events = vec![Event::CharacterBlockChanged {
            character_id: character_id.clone(),
            target_character_id: target_character_id.clone(),
            blocked,
        }];
        if blocked {
            let affected = self
                .world
                .group_invitations
                .iter()
                .filter_map(|(id, invitation)| {
                    ((invitation.issuer_character_id == *character_id
                        && invitation.target_character_id == *target_character_id)
                        || (invitation.issuer_character_id == *target_character_id
                            && invitation.target_character_id == *character_id))
                        .then_some(*id)
                })
                .collect::<Vec<_>>();
            for invitation_id in affected {
                let invitation = self
                    .world
                    .group_invitations
                    .remove(&invitation_id)
                    .expect("selected invitation exists");
                events.push(Self::invitation_resolved_event(
                    &invitation,
                    GroupInvitationResolutionV1::Invalidated,
                ));
            }
        }
        Ok(events)
    }

    fn allocate_group_member(
        &mut self,
        character_id: &CharacterId,
        joined_order: u64,
    ) -> Result<GroupMemberState, StepError> {
        let membership_epoch = self.world.next_membership_epoch;
        self.world.next_membership_epoch = self
            .world
            .next_membership_epoch
            .checked_add(1)
            .ok_or_else(|| StepError::new("group membership epoch overflow"))?;
        Ok(GroupMemberState {
            character_id: character_id.clone(),
            joined_order,
            membership_epoch,
        })
    }

    fn add_group_member(
        &mut self,
        group_id: GroupId,
        character_id: &CharacterId,
    ) -> Result<(), StepError> {
        let joined_order = self.world.groups[&group_id].next_join_order;
        let member = self.allocate_group_member(character_id, joined_order)?;
        let group = self
            .world
            .groups
            .get_mut(&group_id)
            .expect("selected group exists");
        group.next_join_order = group
            .next_join_order
            .checked_add(1)
            .ok_or_else(|| StepError::new("group join order overflow"))?;
        group.members.push(member);
        Ok(())
    }

    fn reconcile_group_leadership(
        &mut self,
        group_id: GroupId,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let group = self
            .world
            .groups
            .get(&group_id)
            .ok_or_else(|| StepError::new("group no longer exists"))?;
        let leader = group.leader_character_id.clone();
        let leader_presence = self
            .world
            .character_presence
            .get(&leader)
            .ok_or_else(|| StepError::new("group leader has no presence state"))?;
        if leader_presence.connected {
            return Ok(());
        }
        let expired = leader_presence.absent_since.is_some_and(|absent_since| {
            self.world.timing.now.value()
                >= absent_since
                    .value()
                    .saturating_add(GROUP_DISCONNECT_GRACE_UNITS)
        });
        if !expired {
            return Ok(());
        }
        let replacement = group
            .members
            .iter()
            .filter(|member| {
                self.world
                    .character_presence
                    .get(&member.character_id)
                    .is_some_and(|presence| presence.connected)
            })
            .min_by_key(|member| (member.joined_order, member.character_id.clone()))
            .map(|member| member.character_id.clone());
        let Some(replacement) = replacement else {
            return Ok(());
        };
        if replacement == leader {
            return Ok(());
        }
        self.world
            .groups
            .get_mut(&group_id)
            .expect("selected group exists")
            .leader_character_id = replacement.clone();
        events.push(self.group_changed_event(
            group_id,
            GroupChangeReasonV1::LeadershipFallback,
            Some(replacement),
        )?);
        Ok(())
    }

    fn dissolve_group(
        &mut self,
        group_id: GroupId,
        reason: GroupChangeReasonV1,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let group = self
            .world
            .groups
            .remove(&group_id)
            .ok_or_else(|| StepError::new("group no longer exists"))?;
        self.reattribute_dissolved_group_contributions(group_id)?;
        let members = group
            .members
            .iter()
            .map(|member| member.character_id.clone())
            .collect::<Vec<_>>();
        let invitation_ids = self
            .world
            .group_invitations
            .iter()
            .filter_map(|(id, invitation)| (invitation.group_id == Some(group_id)).then_some(*id))
            .collect::<Vec<_>>();
        for invitation_id in invitation_ids {
            let invitation = self
                .world
                .group_invitations
                .remove(&invitation_id)
                .expect("selected invitation exists");
            events.push(Self::invitation_resolved_event(
                &invitation,
                GroupInvitationResolutionV1::Invalidated,
            ));
        }
        for member in &members {
            self.remove_follow_edges_for_character(
                member,
                PlayerFollowChangeReasonV1::MembershipLost,
                events,
            );
        }
        events.push(Event::GroupChanged {
            group_id,
            reason,
            leader_character_id: None,
            member_character_ids: members,
            subject_character_id: None,
        });
        Ok(())
    }

    pub(super) fn remove_follow_edges_for_character(
        &mut self,
        character_id: &CharacterId,
        reason: PlayerFollowChangeReasonV1,
        events: &mut Vec<Event>,
    ) {
        let followers = self
            .world
            .player_follow_targets
            .iter()
            .filter_map(|(follower, target)| {
                (follower == character_id || target == character_id).then_some(follower.clone())
            })
            .collect::<Vec<_>>();
        for follower in followers {
            if let Some(target) = self.world.player_follow_targets.remove(&follower) {
                events.push(Event::PlayerFollowChanged {
                    follower_character_id: follower,
                    target_character_id: Some(target),
                    reason,
                });
            }
        }
    }

    fn cancel_invitations_by_issuer(
        &mut self,
        character_id: &CharacterId,
        events: &mut Vec<Event>,
    ) {
        let invitation_ids = self
            .world
            .group_invitations
            .iter()
            .filter_map(|(id, invitation)| {
                (&invitation.issuer_character_id == character_id).then_some(*id)
            })
            .collect::<Vec<_>>();
        for invitation_id in invitation_ids {
            let invitation = self
                .world
                .group_invitations
                .remove(&invitation_id)
                .expect("selected invitation exists");
            events.push(Self::invitation_resolved_event(
                &invitation,
                GroupInvitationResolutionV1::Invalidated,
            ));
        }
    }

    fn invalidate_target_invitations(
        &mut self,
        target_character_id: &CharacterId,
        events: &mut Vec<Event>,
    ) {
        let invitation_ids = self
            .world
            .group_invitations
            .iter()
            .filter_map(|(id, invitation)| {
                (&invitation.target_character_id == target_character_id).then_some(*id)
            })
            .collect::<Vec<_>>();
        for invitation_id in invitation_ids {
            let invitation = self
                .world
                .group_invitations
                .remove(&invitation_id)
                .expect("selected invitation exists");
            events.push(Self::invitation_resolved_event(
                &invitation,
                GroupInvitationResolutionV1::Invalidated,
            ));
        }
    }

    fn group_changed_event(
        &self,
        group_id: GroupId,
        reason: GroupChangeReasonV1,
        subject_character_id: Option<CharacterId>,
    ) -> Result<Event, StepError> {
        let group = self
            .world
            .groups
            .get(&group_id)
            .ok_or_else(|| StepError::new("group no longer exists"))?;
        Ok(Event::GroupChanged {
            group_id,
            reason,
            leader_character_id: Some(group.leader_character_id.clone()),
            member_character_ids: group
                .members
                .iter()
                .map(|member| member.character_id.clone())
                .collect(),
            subject_character_id,
        })
    }

    fn invitation_resolved_event(
        invitation: &GroupInvitationState,
        resolution: GroupInvitationResolutionV1,
    ) -> Event {
        Event::GroupInvitationResolved {
            invitation_id: invitation.id,
            issuer_character_id: invitation.issuer_character_id.clone(),
            target_character_id: invitation.target_character_id.clone(),
            group_id: invitation.group_id,
            resolution,
        }
    }

    pub fn communication_preferences(
        &self,
        character_id: &CharacterId,
    ) -> Option<&CommunicationPreferences> {
        self.world.communication_preferences.get(character_id)
    }
}

fn direction_toward(from: crate::model::Coord, target: crate::model::Coord) -> Option<Direction> {
    match ((target.x - from.x).signum(), (target.y - from.y).signum()) {
        (0, 0) => None,
        (0, -1) => Some(Direction::North),
        (1, -1) => Some(Direction::Northeast),
        (1, 0) => Some(Direction::East),
        (1, 1) => Some(Direction::Southeast),
        (0, 1) => Some(Direction::South),
        (-1, 1) => Some(Direction::Southwest),
        (-1, 0) => Some(Direction::West),
        (-1, -1) => Some(Direction::Northwest),
        _ => unreachable!("signum pair is bounded"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ActorId, ActorKind, CharacterPresenceState, LogicalTime};

    fn social_engine(character_count: usize) -> Engine {
        let mut engine = crate::engine::setup::test_engine("character_sheet");
        engine
            .world
            .actors
            .retain(|actor| actor.kind == ActorKind::Player);
        let original_character_id = engine.world.actors[0]
            .character_id
            .clone()
            .expect("fixture character");
        engine.world.communication_preferences.clear();
        engine.world.character_presence.clear();
        for index in 0..character_count {
            let character_id = CharacterId::new(format!("character:{index}"));
            if index == 0 {
                engine.world.actors[0].character_id = Some(character_id.clone());
                engine.world.actors[0].id = ActorId::new("player0");
                engine.world.actors[0].name = "Player 0".to_string();
            } else {
                let mut actor = engine.world.actors[0].clone();
                actor.id = ActorId::new(format!("player{index}"));
                actor.name = format!("Player {index}");
                actor.character_id = Some(character_id.clone());
                actor.timing.tie_break_order = u64::try_from(index).unwrap() + 10;
                actor.carried.items.clear();
                actor.carried.gold = Default::default();
                engine.world.actors.push(actor);
            }
            engine
                .world
                .communication_preferences
                .insert(character_id.clone(), CommunicationPreferences::default());
            engine.world.character_presence.insert(
                character_id,
                CharacterPresenceState {
                    connected: true,
                    control_epoch: 1,
                    absent_since: None,
                },
            );
        }
        engine.world.quest_states.remove(&original_character_id);
        engine
    }

    fn group(engine: &mut Engine, count: usize) -> GroupId {
        let first = CharacterId::new("character:0");
        for index in 1..count {
            let target = CharacterId::new(format!("character:{index}"));
            let invite = engine
                .apply_social_intent(
                    &ActorId::new("player0"),
                    SocialIntent::Invite {
                        target_character_id: target.clone(),
                    },
                )
                .expect("invite");
            let invitation_id = invite
                .events
                .iter()
                .find_map(|event| match event {
                    Event::GroupInvitationCreated { invitation_id, .. } => Some(*invitation_id),
                    _ => None,
                })
                .expect("invitation ID");
            engine
                .apply_social_intent(
                    &ActorId::new(format!("player{index}")),
                    SocialIntent::AcceptInvite { invitation_id },
                )
                .expect("accept");
        }
        engine.group_id_for_character(&first).expect("group")
    }

    #[test]
    fn group_contract_caps_six_and_social_commands_are_free() {
        let mut engine = social_engine(7);
        let ready_before = engine.world.actors[0].timing.ready_at;
        let now_before = engine.world.timing.now;
        let group_id = group(&mut engine, MAX_GROUP_MEMBERS);
        assert_eq!(engine.world.groups[&group_id].members.len(), 6);
        assert_eq!(engine.world.actors[0].timing.ready_at, ready_before);
        assert_eq!(engine.world.timing.now, now_before);
        let error = engine
            .apply_social_intent(
                &ActorId::new("player0"),
                SocialIntent::Invite {
                    target_character_id: CharacterId::new("character:6"),
                },
            )
            .expect_err("full group rejects invitation");
        assert_eq!(error.message(), "group is full");
    }

    #[test]
    fn leader_disconnect_grace_falls_back_once_and_reconnect_does_not_reclaim() {
        let mut engine = social_engine(3);
        let group_id = group(&mut engine, 3);
        let leader = CharacterId::new("character:0");
        engine
            .apply_connection_presence(&leader, 2, false)
            .expect("disconnect");
        let disconnected_at = engine.world.timing.now.value();
        engine.world.timing.now =
            LogicalTime::new(disconnected_at + GROUP_DISCONNECT_GRACE_UNITS - 1);
        engine.expire_group_state(&mut Vec::new()).unwrap();
        assert_eq!(engine.world.groups[&group_id].leader_character_id, leader);
        engine.world.timing.now = LogicalTime::new(disconnected_at + GROUP_DISCONNECT_GRACE_UNITS);
        let mut events = Vec::new();
        engine.expire_group_state(&mut events).unwrap();
        assert_eq!(
            engine.world.groups[&group_id].leader_character_id,
            CharacterId::new("character:1")
        );
        engine
            .apply_connection_presence(&leader, 3, true)
            .expect("reconnect");
        assert_eq!(
            engine.world.groups[&group_id].leader_character_id,
            CharacterId::new("character:1")
        );
        assert!(events.iter().any(|event| matches!(
            event,
            Event::GroupChanged {
                reason: GroupChangeReasonV1::LeadershipFallback,
                ..
            }
        )));
    }

    #[test]
    fn dormant_group_promotes_the_first_member_who_reconnects() {
        let mut engine = social_engine(3);
        let group_id = group(&mut engine, 3);
        for index in 0..3 {
            engine
                .apply_connection_presence(
                    &CharacterId::new(format!("character:{index}")),
                    2,
                    false,
                )
                .expect("disconnect");
        }
        let disconnected_at = engine.world.timing.now.value();
        engine.world.timing.now = LogicalTime::new(disconnected_at + GROUP_DISCONNECT_GRACE_UNITS);
        engine.expire_group_state(&mut Vec::new()).unwrap();
        assert_eq!(
            engine.world.groups[&group_id].leader_character_id,
            CharacterId::new("character:0")
        );
        engine
            .apply_connection_presence(&CharacterId::new("character:2"), 3, true)
            .expect("first reconnect");
        assert_eq!(
            engine.world.groups[&group_id].leader_character_id,
            CharacterId::new("character:2")
        );
    }

    #[test]
    fn follow_rejects_cycles_and_target_disconnect_clears_the_edge() {
        let mut engine = social_engine(3);
        group(&mut engine, 3);
        engine
            .apply_social_intent(
                &ActorId::new("player1"),
                SocialIntent::BeginFollow {
                    target_character_id: CharacterId::new("character:0"),
                },
            )
            .expect("first follow");
        engine
            .apply_social_intent(
                &ActorId::new("player2"),
                SocialIntent::BeginFollow {
                    target_character_id: CharacterId::new("character:1"),
                },
            )
            .expect("second follow");
        assert!(
            engine
                .apply_social_intent(
                    &ActorId::new("player0"),
                    SocialIntent::BeginFollow {
                        target_character_id: CharacterId::new("character:2"),
                    },
                )
                .is_err()
        );
        engine
            .apply_connection_presence(&CharacterId::new("character:0"), 2, false)
            .expect("target disconnect");
        assert!(
            !engine
                .world
                .player_follow_targets
                .contains_key(&CharacterId::new("character:1"))
        );
        assert_eq!(
            engine
                .world
                .player_follow_targets
                .get(&CharacterId::new("character:2")),
            Some(&CharacterId::new("character:1"))
        );
    }

    #[test]
    fn leadership_can_be_transferred_and_voluntary_leave_uses_tenure() {
        let mut engine = social_engine(3);
        let group_id = group(&mut engine, 3);
        engine
            .apply_social_intent(
                &ActorId::new("player0"),
                SocialIntent::TransferLeadership {
                    member_character_id: CharacterId::new("character:2"),
                },
            )
            .expect("handoff");
        assert_eq!(
            engine.world.groups[&group_id].leader_character_id,
            CharacterId::new("character:2")
        );
        engine
            .apply_social_intent(&ActorId::new("player2"), SocialIntent::LeaveGroup)
            .expect("leader leaves");
        assert_eq!(
            engine.world.groups[&group_id].leader_character_id,
            CharacterId::new("character:0")
        );
    }

    #[test]
    fn social_checkpoint_round_trip_is_canonical() {
        let mut engine = social_engine(3);
        group(&mut engine, 3);
        engine
            .apply_social_intent(
                &ActorId::new("player1"),
                SocialIntent::BeginFollow {
                    target_character_id: CharacterId::new("character:0"),
                },
            )
            .expect("follow");
        let checkpoint = engine.export_checkpoint().expect("checkpoint");
        let hydrated =
            Engine::hydrate_checkpoint(engine.definition.clone(), &checkpoint).expect("hydrate");
        assert_eq!(
            hydrated.export_checkpoint().unwrap().as_bytes(),
            checkpoint.as_bytes()
        );
    }
}

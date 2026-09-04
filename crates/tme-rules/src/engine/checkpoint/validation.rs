use super::*;

pub(super) fn validate_checkpoint_references(engine: &Engine) -> Result<(), CheckpointError> {
    let definition = engine.definition.as_ref();
    let actor_ids = engine
        .world
        .actors
        .iter()
        .map(|actor| actor.id.clone())
        .collect::<BTreeSet<_>>();
    let character_ids = engine
        .world
        .actors
        .iter()
        .filter_map(|actor| actor.character_id.clone())
        .collect::<BTreeSet<_>>();

    for actor in &engine.world.actors {
        let authored = definition
            .catalog
            .actor_definitions
            .get(&actor.definition_id)
            .ok_or_else(|| CheckpointError::new("checkpoint actor definition is unknown"))?;
        if authored.kind != actor.kind {
            return Err(CheckpointError::new(
                "checkpoint actor kind differs from its definition",
            ));
        }
        validate_position(definition, &actor.location)?;
        validate_position(definition, &actor.home_location)?;
        if let Some(origin) = &actor.ecology_origin {
            let site = engine
                .world
                .ecology_sites
                .get(&origin.site_id)
                .ok_or_else(|| CheckpointError::new("checkpoint ecology origin site is unknown"))?;
            let slot = site.member_slots.get(&origin.member_id).ok_or_else(|| {
                CheckpointError::new("checkpoint ecology origin member slot is unknown")
            })?;
            if origin.generation > site.generation
                || (actor.is_alive() && slot.actor_id.as_ref() != Some(&actor.id))
            {
                return Err(CheckpointError::new(
                    "checkpoint ecology actor origin disagrees with its site",
                ));
            }
        }
    }

    for (site_id, site) in &engine.world.ecology_sites {
        if site_id != &site.id
            || !definition
                .catalog
                .spawn_groups
                .contains_key(&site.spawn_group_id)
        {
            return Err(CheckpointError::new(
                "checkpoint ecology site identity is invalid",
            ));
        }
        let group = &definition.catalog.spawn_groups[&site.spawn_group_id];
        let expected_members = group
            .members
            .iter()
            .map(|member| member.member_id.as_str())
            .collect::<BTreeSet<_>>();
        let actual_members = site
            .member_slots
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        if actual_members != expected_members {
            return Err(CheckpointError::new(
                "checkpoint ecology member slots disagree with the spawn group",
            ));
        }
        if site.full_clear_due_at.is_some()
            && site
                .member_slots
                .values()
                .any(|slot| slot.actor_id.is_some())
        {
            return Err(CheckpointError::new(
                "checkpoint ecology full-clear due state has a living slot",
            ));
        }
        for (member_id, slot) in &site.member_slots {
            if member_id != &slot.member_id || (slot.actor_id.is_some() && slot.due_at.is_some()) {
                return Err(CheckpointError::new(
                    "checkpoint ecology member slot state is invalid",
                ));
            }
            validate_position(definition, &slot.location)?;
            let Some(actor_id) = &slot.actor_id else {
                continue;
            };
            let actor = engine
                .world
                .actor(actor_id)
                .ok_or_else(|| CheckpointError::new("checkpoint ecology actor is unknown"))?;
            if !actor.is_alive()
                || actor.ecology_origin.as_ref().is_none_or(|origin| {
                    origin.site_id != *site_id
                        || origin.member_id != *member_id
                        || origin.generation > site.generation
                })
            {
                return Err(CheckpointError::new(
                    "checkpoint ecology site disagrees with its actor origin",
                ));
            }
        }
    }

    let mut service_ids = BTreeSet::new();
    for service in &engine.world.service_instances {
        if !service_ids.insert(&service.id)
            || !definition
                .catalog
                .service_definitions
                .iter()
                .any(|candidate| candidate.id == service.definition_id)
        {
            return Err(CheckpointError::new(
                "checkpoint service identity is invalid",
            ));
        }
        validate_position(definition, &service.position)?;
    }

    for (instance_id, item) in &engine.world.item_instances {
        if instance_id.is_empty()
            || item.quantity == 0
            || !definition
                .catalog
                .item_catalog
                .contains_key(&item.definition_id)
        {
            return Err(CheckpointError::new("checkpoint item instance is invalid"));
        }
    }
    for item in &engine.world.ground_items {
        validate_position(definition, &item.location)?;
    }

    for (bank_id, bank) in &engine.world.banks {
        if !definition.catalog.bank_definitions.contains_key(bank_id)
            || bank.balances.iter().any(|(character_id, balance)| {
                !character_ids.contains(character_id) || *balance < 0
            })
        {
            return Err(CheckpointError::new("checkpoint bank state is invalid"));
        }
    }
    for (vault_id, vault) in &engine.world.locker_vaults {
        let definition = definition
            .catalog
            .locker_vault_definitions
            .get(vault_id)
            .ok_or_else(|| CheckpointError::new("checkpoint locker definition is unknown"))?;
        if vault.lockers.iter().any(|(character_id, items)| {
            !character_ids.contains(character_id)
                || items.len() > definition.capacity as usize
                || items
                    .iter()
                    .any(|item_id| !engine.world.item_instances.contains_key(item_id))
        }) {
            return Err(CheckpointError::new("checkpoint locker state is invalid"));
        }
    }
    for (item_id, offer) in &engine.world.item_offers {
        if !engine.world.item_instances.contains_key(item_id)
            || !character_ids.contains(&offer.sender_character_id)
            || !character_ids.contains(&offer.recipient_character_id)
            || offer.sender_character_id == offer.recipient_character_id
        {
            return Err(CheckpointError::new("checkpoint item offer is invalid"));
        }
    }
    for (character_id, quests) in &engine.world.quest_states {
        if !character_ids.contains(character_id) {
            return Err(CheckpointError::new("checkpoint quest owner is unknown"));
        }
        for (quest_id, stage_id) in quests {
            let quest = definition
                .catalog
                .quests
                .get(quest_id)
                .ok_or_else(|| CheckpointError::new("checkpoint quest is unknown"))?;
            if !quest.stages.contains_key(stage_id) {
                return Err(CheckpointError::new("checkpoint quest stage is unknown"));
            }
        }
    }

    let mut max_corpse_sequence = 0;
    for (corpse_id, corpse) in &engine.world.corpses {
        if corpse_id != &corpse.id {
            return Err(CheckpointError::new(
                "checkpoint corpse key differs from its identity",
            ));
        }
        validate_position(definition, &corpse.location)?;
        max_corpse_sequence = max_corpse_sequence.max(corpse.sequence);
    }
    if engine.world.next_corpse_sequence == 0
        || engine.world.next_corpse_sequence <= max_corpse_sequence
    {
        return Err(CheckpointError::new(
            "checkpoint corpse allocation sequence is invalid",
        ));
    }
    let mut max_gold_sequence = 0;
    for (gold_id, pile) in &engine.world.ground_gold {
        if gold_id != &pile.id || pile.amount <= 0 {
            return Err(CheckpointError::new(
                "checkpoint ground-gold state is invalid",
            ));
        }
        validate_position(definition, &pile.location)?;
        max_gold_sequence = max_gold_sequence.max(sequence_suffix(gold_id.as_str(), "gold:")?);
    }
    if engine.world.next_gold_sequence == 0 || engine.world.next_gold_sequence <= max_gold_sequence
    {
        return Err(CheckpointError::new(
            "checkpoint gold allocation sequence is invalid",
        ));
    }

    for effect in &engine.world.tile_effects {
        validate_position(definition, &effect.location)?;
    }
    for enchantment in &engine.world.item_enchantments {
        if !engine
            .world
            .item_instances
            .contains_key(&enchantment.item_instance_id)
        {
            return Err(CheckpointError::new(
                "checkpoint enchantment item is unknown",
            ));
        }
    }
    for portal in &engine.world.portal_transitions {
        validate_position(definition, &portal.location)?;
        validate_position(definition, &portal.target)?;
    }
    for concealed in &engine.world.concealed_transitions {
        validate_position(definition, &concealed.location)?;
    }
    for position in engine
        .world
        .hidden_transition_revealed
        .keys()
        .chain(engine.world.door_states.keys())
    {
        validate_position(definition, position)?;
    }

    for (victim, relation) in &engine.world.social_relations.self_defense {
        if victim != &relation.victim_character_id
            || !character_ids.contains(victim)
            || relation.victim_character_id == relation.attacker_character_id
        {
            return Err(CheckpointError::new(
                "checkpoint self-defense relation is invalid",
            ));
        }
    }
    let mut previous_link = None;
    for link in &engine.world.linked_player_kill_karma {
        if link.facet_kill_sequence == 0
            || link.killer_character_id == link.victim_character_id
            || !character_ids.contains(&link.killer_character_id)
        {
            return Err(CheckpointError::new(
                "checkpoint linked player-kill karma is invalid",
            ));
        }
        if previous_link
            .as_ref()
            .is_some_and(|previous| previous >= link)
        {
            return Err(CheckpointError::new(
                "checkpoint linked player-kill karma is not strictly ordered",
            ));
        }
        previous_link = Some(link.clone());
    }
    if engine
        .world
        .social_relations
        .npc_grudges
        .iter()
        .any(|relation| {
            !actor_ids.contains(&relation.npc_actor_id)
                || !actor_ids.contains(&relation.attacker_actor_id)
        })
    {
        return Err(CheckpointError::new(
            "checkpoint NPC grudge relation is invalid",
        ));
    }
    Ok(())
}

pub(super) fn validate_position(
    definition: &GameDefinition,
    position: &WorldPosition,
) -> Result<(), CheckpointError> {
    match definition.world_position_status(position) {
        Some(SeedWorldPositionStatus::Passable | SeedWorldPositionStatus::Blocked) => Ok(()),
        Some(SeedWorldPositionStatus::OutOfBounds) | None => Err(CheckpointError::new(
            "checkpoint world position is outside immutable content",
        )),
    }
}

pub(super) fn sequence_suffix(value: &str, prefix: &str) -> Result<u64, CheckpointError> {
    value
        .strip_prefix(prefix)
        .and_then(|value| value.parse().ok())
        .ok_or_else(|| CheckpointError::new("checkpoint sequence identity is invalid"))
}

pub(super) fn validate_social_checkpoint_state(engine: &Engine) -> Result<(), CheckpointError> {
    let character_ids = engine
        .world
        .actors
        .iter()
        .filter_map(|actor| actor.character_id.clone())
        .collect::<BTreeSet<_>>();
    let preference_ids = engine
        .world
        .communication_preferences
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    let presence_ids = engine
        .world
        .character_presence
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    if preference_ids != character_ids || presence_ids != character_ids {
        return Err(CheckpointError::new(
            "checkpoint communication/presence ownership differs from local characters",
        ));
    }
    for (character_id, preferences) in &engine.world.communication_preferences {
        if preferences.blocked_character_ids.len() > MAX_BLOCKED_CHARACTERS
            || preferences.blocked_character_ids.contains(character_id)
        {
            return Err(CheckpointError::new(
                "checkpoint communication preferences are invalid",
            ));
        }
    }
    for presence in engine.world.character_presence.values() {
        if presence.connected == presence.absent_since.is_some() {
            return Err(CheckpointError::new(
                "checkpoint character presence is internally inconsistent",
            ));
        }
    }
    if engine.world.next_group_sequence == 0
        || engine.world.next_group_invite_sequence == 0
        || engine.world.next_membership_epoch == 0
    {
        return Err(CheckpointError::new(
            "checkpoint social allocation sequence is zero",
        ));
    }

    let mut grouped_characters = BTreeSet::new();
    let mut membership_epochs = BTreeSet::new();
    let mut greatest_group_id = 0;
    for (group_id, group) in &engine.world.groups {
        greatest_group_id = greatest_group_id.max(group_id.value());
        if group.id != *group_id
            || group.members.len() < 2
            || group.members.len() > MAX_GROUP_MEMBERS
            || !group
                .members
                .iter()
                .any(|member| member.character_id == group.leader_character_id)
        {
            return Err(CheckpointError::new("checkpoint group shape is invalid"));
        }
        let mut joined_orders = BTreeSet::new();
        let mut greatest_join_order = 0;
        for member in &group.members {
            greatest_join_order = greatest_join_order.max(member.joined_order);
            if !character_ids.contains(&member.character_id)
                || !grouped_characters.insert(member.character_id.clone())
                || !joined_orders.insert(member.joined_order)
                || !membership_epochs.insert(member.membership_epoch)
                || member.membership_epoch >= engine.world.next_membership_epoch
            {
                return Err(CheckpointError::new(
                    "checkpoint group membership is invalid",
                ));
            }
        }
        if group.next_join_order <= greatest_join_order {
            return Err(CheckpointError::new(
                "checkpoint group join sequence is invalid",
            ));
        }
    }
    if engine.world.next_group_sequence <= greatest_group_id {
        return Err(CheckpointError::new(
            "checkpoint group allocation sequence is invalid",
        ));
    }

    let mut incoming = BTreeMap::<CharacterId, usize>::new();
    let mut outgoing = BTreeMap::<(Option<GroupId>, Option<CharacterId>), usize>::new();
    let mut greatest_invitation_id = 0;
    for (invitation_id, invitation) in &engine.world.group_invitations {
        greatest_invitation_id = greatest_invitation_id.max(invitation_id.value());
        if invitation.id != *invitation_id
            || invitation.issuer_character_id == invitation.target_character_id
            || !character_ids.contains(&invitation.issuer_character_id)
            || !character_ids.contains(&invitation.target_character_id)
            || grouped_characters.contains(&invitation.target_character_id)
        {
            return Err(CheckpointError::new(
                "checkpoint group invitation shape is invalid",
            ));
        }
        if let Some(group_id) = invitation.group_id {
            let group =
                engine.world.groups.get(&group_id).ok_or_else(|| {
                    CheckpointError::new("checkpoint invitation group is unknown")
                })?;
            if !group.members.iter().any(|member| {
                member.character_id == invitation.issuer_character_id
                    && Some(member.membership_epoch) == invitation.issuer_membership_epoch
            }) {
                return Err(CheckpointError::new(
                    "checkpoint invitation issuer membership is stale",
                ));
            }
        } else if invitation.issuer_membership_epoch.is_some()
            || grouped_characters.contains(&invitation.issuer_character_id)
        {
            return Err(CheckpointError::new(
                "checkpoint solo invitation issuer is grouped",
            ));
        }
        *incoming
            .entry(invitation.target_character_id.clone())
            .or_default() += 1;
        let outgoing_key = match invitation.group_id {
            Some(group_id) => (Some(group_id), None),
            None => (None, Some(invitation.issuer_character_id.clone())),
        };
        *outgoing.entry(outgoing_key).or_default() += 1;
    }
    if incoming
        .values()
        .any(|count| *count > MAX_INCOMING_GROUP_INVITATIONS)
        || outgoing
            .values()
            .any(|count| *count > MAX_OUTGOING_GROUP_INVITATIONS)
        || engine.world.next_group_invite_sequence <= greatest_invitation_id
    {
        return Err(CheckpointError::new(
            "checkpoint group invitation bounds or sequence are invalid",
        ));
    }

    for (follower, target) in &engine.world.player_follow_targets {
        if follower == target
            || !character_ids.contains(follower)
            || !character_ids.contains(target)
            || engine.group_id_for_character(follower) != engine.group_id_for_character(target)
            || engine.group_id_for_character(follower).is_none()
        {
            return Err(CheckpointError::new(
                "checkpoint player follow edge is invalid",
            ));
        }
        let mut cursor = target;
        let mut visited = BTreeSet::new();
        while let Some(next) = engine.world.player_follow_targets.get(cursor) {
            if next == follower || !visited.insert(cursor.clone()) {
                return Err(CheckpointError::new(
                    "checkpoint player follow graph contains a cycle",
                ));
            }
            cursor = next;
        }
    }

    for (target_actor_id, ledger) in &engine.world.defeat_contributions {
        if !engine
            .world
            .actors
            .iter()
            .any(|actor| &actor.id == target_actor_id)
            || ledger.total_actual_damage == 0
        {
            return Err(CheckpointError::new(
                "checkpoint defeat contribution target is invalid",
            ));
        }
        let mut recorded_damage = 0_u64;
        for (unit_id, unit) in &ledger.reward_units {
            if unit.slices.is_empty() {
                return Err(CheckpointError::new(
                    "checkpoint defeat reward unit has no contribution slices",
                ));
            }
            for (slice, amount) in &unit.slices {
                if *amount == 0 {
                    return Err(CheckpointError::new(
                        "checkpoint defeat contribution slice is invalid",
                    ));
                }
                recorded_damage = recorded_damage.checked_add(*amount).ok_or_else(|| {
                    CheckpointError::new("checkpoint defeat contribution sum overflow")
                })?;
                if slice
                    .eligible_memberships
                    .windows(2)
                    .any(|pair| pair[0] >= pair[1])
                    || slice
                        .eligible_memberships
                        .iter()
                        .any(|membership| membership.membership_epoch == 0)
                {
                    return Err(CheckpointError::new(
                        "checkpoint defeat membership cohort is invalid",
                    ));
                }
            }
            match unit_id {
                DefeatRewardUnitId::Solo { character_id } => {
                    if unit.slices.keys().any(|slice| {
                        &slice.contributor_character_id != character_id
                            || !slice.eligible_memberships.is_empty()
                    }) {
                        return Err(CheckpointError::new(
                            "checkpoint solo defeat reward unit is invalid",
                        ));
                    }
                }
                DefeatRewardUnitId::Group { group_id } => {
                    engine.world.groups.get(group_id).ok_or_else(|| {
                        CheckpointError::new("checkpoint contribution group is unknown")
                    })?;
                    if unit
                        .slices
                        .keys()
                        .any(|slice| slice.eligible_memberships.is_empty())
                    {
                        return Err(CheckpointError::new(
                            "checkpoint group reward membership is invalid",
                        ));
                    }
                }
            }
        }
        if recorded_damage > ledger.total_actual_damage {
            return Err(CheckpointError::new(
                "checkpoint rewarded damage exceeds total actual damage",
            ));
        }
    }
    Ok(())
}

pub(super) fn validate_character_ownership(engine: &Engine) -> Result<(), CheckpointError> {
    let mut actors = BTreeSet::new();
    let mut characters = BTreeSet::new();
    for actor in &engine.world.actors {
        if !actors.insert(actor.id.clone()) {
            return Err(CheckpointError::new(
                "checkpoint contains duplicate actor IDs",
            ));
        }
        if let Some(character_id) = &actor.character_id
            && !characters.insert(character_id.clone())
        {
            return Err(CheckpointError::new(
                "checkpoint contains duplicate character ownership",
            ));
        }
        if actor.character_id.is_some() != actor.character.is_some() {
            return Err(CheckpointError::new(
                "checkpoint character ID/sheet presence disagrees",
            ));
        }
    }
    Ok(())
}

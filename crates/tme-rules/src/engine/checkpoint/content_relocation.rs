//! Authored map expansion: translate every typed live spatial field, then
//! reconcile keyed topology state against the destination definition.
use super::*;

/// One authored member that keeps its identity while its coordinates move.
///
/// `realm` and `level` name the same member in both definitions; only the
/// offset changes. Every live position in that member is translated by it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContentRelocation {
    pub realm: String,
    pub level: String,
    pub offset: Coord,
}

type MemberOffsets = BTreeMap<(String, String), Coord>;

pub(super) fn apply_content_relocation(
    before: &GameDefinition,
    after: &GameDefinition,
    plan: &CheckpointContentMigration,
    world: &mut World,
) -> Result<(), CheckpointError> {
    let offsets = relocation_offsets(before, after, plan)?;
    translate_world(after, world, &offsets)?;
    reconcile_topology(before, after, plan.initialize_new_topology, world, &offsets)
}

fn relocation_offsets(
    before: &GameDefinition,
    after: &GameDefinition,
    plan: &CheckpointContentMigration,
) -> Result<MemberOffsets, CheckpointError> {
    let mut offsets = MemberOffsets::new();
    for relocation in &plan.relocations {
        if relocation.offset.x == 0 && relocation.offset.y == 0 {
            return Err(CheckpointError::new("content relocation offset is zero"));
        }
        let member = (relocation.realm.clone(), relocation.level.clone());
        let source = level_extent(before, &member)
            .ok_or_else(|| CheckpointError::new("content relocation source member is absent"))?;
        let destination = level_extent(after, &member).ok_or_else(|| {
            CheckpointError::new("content relocation destination member is absent")
        })?;
        if source.0 <= 0 || source.1 <= 0 {
            return Err(CheckpointError::new(
                "content relocation source member has no extent",
            ));
        }
        for (x, y) in [
            (0, 0),
            (source.0 - 1, 0),
            (0, source.1 - 1),
            (source.0 - 1, source.1 - 1),
        ] {
            let translated_x = x
                .checked_add(relocation.offset.x)
                .ok_or_else(overflow_error)?;
            let translated_y = y
                .checked_add(relocation.offset.y)
                .ok_or_else(overflow_error)?;
            if translated_x < 0
                || translated_y < 0
                || translated_x >= destination.0
                || translated_y >= destination.1
            {
                return Err(CheckpointError::new(
                    "content relocation moves a level corner out of bounds",
                ));
            }
        }
        if offsets.insert(member, relocation.offset).is_some() {
            return Err(CheckpointError::new(
                "content relocation duplicates a member transform",
            ));
        }
    }
    Ok(offsets)
}

fn level_extent(definition: &GameDefinition, member: &(String, String)) -> Option<(i32, i32)> {
    let level = definition
        .world_template
        .realms
        .get(&member.0)?
        .levels
        .get(&member.1)?;
    Some((level.width, level.height))
}

fn overflow_error() -> CheckpointError {
    CheckpointError::new("content relocation overflows a coordinate")
}

fn translate_coord(
    position: &mut WorldPosition,
    offsets: &MemberOffsets,
) -> Result<(), CheckpointError> {
    let member = (position.realm.clone(), position.level.clone());
    if let Some(offset) = offsets.get(&member) {
        position.position.x = position
            .position
            .x
            .checked_add(offset.x)
            .ok_or_else(overflow_error)?;
        position.position.y = position
            .position
            .y
            .checked_add(offset.y)
            .ok_or_else(overflow_error)?;
    }
    Ok(())
}

fn translated_position(
    position: &WorldPosition,
    offsets: &MemberOffsets,
) -> Result<WorldPosition, CheckpointError> {
    let mut translated = position.clone();
    translate_coord(&mut translated, offsets)?;
    Ok(translated)
}

fn translate_position(
    after: &GameDefinition,
    position: &mut WorldPosition,
    offsets: &MemberOffsets,
) -> Result<(), CheckpointError> {
    let member = (position.realm.clone(), position.level.clone());
    if !offsets.contains_key(&member) {
        return Ok(());
    }
    translate_coord(position, offsets)?;
    let (width, height) = level_extent(after, &member)
        .ok_or_else(|| CheckpointError::new("content relocation destination member is absent"))?;
    if position.position.x < 0
        || position.position.y < 0
        || position.position.x >= width
        || position.position.y >= height
    {
        return Err(CheckpointError::new(
            "content relocation moves live state out of bounds",
        ));
    }
    Ok(())
}

fn translate_world(
    after: &GameDefinition,
    world: &mut World,
    offsets: &MemberOffsets,
) -> Result<(), CheckpointError> {
    for actor in &mut world.actors {
        // Patrol coordinates carry no member of their own: they belong to the
        // actor's original home member, captured before the home is translated.
        let home_member = (
            actor.home_location.realm.clone(),
            actor.home_location.level.clone(),
        );
        translate_position(after, &mut actor.location, offsets)?;
        translate_position(after, &mut actor.home_location, offsets)?;
        if let Some(offset) = offsets.get(&home_member)
            && let Some(npc) = actor.npc.as_mut()
        {
            for coordinate in &mut npc.patrol {
                coordinate.x = coordinate
                    .x
                    .checked_add(offset.x)
                    .ok_or_else(overflow_error)?;
                coordinate.y = coordinate
                    .y
                    .checked_add(offset.y)
                    .ok_or_else(overflow_error)?;
            }
        }
        if let Some(remembered) = actor
            .ai
            .as_mut()
            .and_then(|ai| ai.awareness.remembered.as_mut())
        {
            translate_position(after, &mut remembered.last_seen, offsets)?;
        }
    }
    for site in world.ecology_sites.values_mut() {
        for slot in site.member_slots.values_mut() {
            translate_position(after, &mut slot.location, offsets)?;
        }
    }
    for service in &mut world.service_instances {
        if let ServicePlacement::Fixed { location } = &mut service.placement {
            translate_position(after, location, offsets)?;
        }
    }
    for item in &mut world.ground_items {
        translate_position(after, &mut item.location, offsets)?;
    }
    for corpse in world.corpses.values_mut() {
        translate_position(after, &mut corpse.location, offsets)?;
    }
    for pile in world.ground_gold.values_mut() {
        translate_position(after, &mut pile.location, offsets)?;
    }
    for effect in &mut world.tile_effects {
        translate_position(after, &mut effect.location, offsets)?;
    }
    for portal in &mut world.portal_transitions {
        translate_position(after, &mut portal.location, offsets)?;
        translate_position(after, &mut portal.target, offsets)?;
    }
    for concealed in &mut world.concealed_transitions {
        translate_position(after, &mut concealed.location, offsets)?;
    }
    Ok(())
}

fn reconcile_topology(
    before: &GameDefinition,
    after: &GameDefinition,
    initialize_new_topology: bool,
    world: &mut World,
    offsets: &MemberOffsets,
) -> Result<(), CheckpointError> {
    let mut door_states = translate_keys(&world.door_states, offsets)?;
    let mut hidden_transition_revealed =
        translate_keys(&world.hidden_transition_revealed, offsets)?;

    verify_retained_navigation(before, after, offsets)?;

    for position in door_states.keys() {
        if authored_door_state(after, position).is_none() {
            return Err(CheckpointError::new(
                "retained door state no longer matches authored door topology",
            ));
        }
    }
    for position in hidden_transition_revealed.keys() {
        if !authored_hidden_key(after, position) {
            return Err(CheckpointError::new(
                "retained hidden state no longer matches authored hidden topology",
            ));
        }
    }

    for (position, initial_state) in authored_door_states(after) {
        if door_states.contains_key(&position) {
            continue;
        }
        if !initialize_new_topology {
            return Err(CheckpointError::new(
                "content migration introduces an uninitialized authored door",
            ));
        }
        door_states.insert(position, initial_state);
    }
    for position in authored_hidden_keys(after) {
        if hidden_transition_revealed.contains_key(&position) {
            continue;
        }
        if !initialize_new_topology {
            return Err(CheckpointError::new(
                "content migration introduces uninitialized hidden topology",
            ));
        }
        hidden_transition_revealed.insert(position, false);
    }

    world.door_states = door_states;
    world.hidden_transition_revealed = hidden_transition_revealed;
    Ok(())
}

fn translate_keys(
    values: &std::collections::HashMap<WorldPosition, bool>,
    offsets: &MemberOffsets,
) -> Result<std::collections::HashMap<WorldPosition, bool>, CheckpointError> {
    let mut translated = std::collections::HashMap::new();
    for (position, value) in values {
        let mut position = position.clone();
        translate_coord(&mut position, offsets)?;
        if translated.insert(position, *value).is_some() {
            return Err(CheckpointError::new(
                "content relocation collapses distinct topology keys",
            ));
        }
    }
    Ok(translated)
}

/// Every retained navigation location must still carry the same edges, with
/// targets translated by the same mapping. Authored initial door state may
/// change; live mutated state is what the keyed maps preserve.
fn verify_retained_navigation(
    before: &GameDefinition,
    after: &GameDefinition,
    offsets: &MemberOffsets,
) -> Result<(), CheckpointError> {
    for (location, edges) in &before.world_template.navigation {
        let translated_location = translated_position(location, offsets)?;
        let candidates = after
            .world_template
            .navigation
            .get(&translated_location)
            .ok_or_else(|| CheckpointError::new("content migration removes retained navigation"))?;
        let mut matched = vec![false; candidates.len()];
        for edge in edges {
            let translated_target = translated_position(&edge.target, offsets)?;
            let index = candidates
                .iter()
                .enumerate()
                .position(|(index, candidate)| {
                    !matched[index]
                        && candidate.kind == edge.kind
                        && candidate.hidden == edge.hidden
                        && candidate.target == translated_target
                })
                .ok_or_else(|| {
                    CheckpointError::new("content migration changes retained navigation")
                })?;
            matched[index] = true;
        }
    }
    Ok(())
}

fn authored_door_state(definition: &GameDefinition, position: &WorldPosition) -> Option<bool> {
    definition
        .world_template
        .navigation
        .get(position)?
        .iter()
        .find(|edge| edge.kind == NavigationKind::Door)?
        .initial_state
        .map(|state| matches!(state, DoorState::Open))
}

fn authored_door_states(definition: &GameDefinition) -> Vec<(WorldPosition, bool)> {
    definition
        .world_template
        .navigation
        .iter()
        .filter_map(|(position, edges)| {
            let edge = edges
                .iter()
                .find(|edge| edge.kind == NavigationKind::Door)?;
            Some((
                position.clone(),
                matches!(edge.initial_state?, DoorState::Open),
            ))
        })
        .collect()
}

fn authored_hidden_key(definition: &GameDefinition, position: &WorldPosition) -> bool {
    definition
        .world_template
        .navigation
        .get(position)
        .is_some_and(|edges| edges.iter().any(|edge| edge.hidden))
}

fn authored_hidden_keys(definition: &GameDefinition) -> Vec<WorldPosition> {
    definition
        .world_template
        .navigation
        .iter()
        .filter(|(_, edges)| edges.iter().any(|edge| edge.hidden))
        .map(|(position, _)| position.clone())
        .collect()
}

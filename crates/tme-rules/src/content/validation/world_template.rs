use std::collections::BTreeMap;

use crate::model::WorldPosition;

use super::super::{
    TerrainDef, TerrainNavigationDef, TopologyKindDef, TopologyTargetDef, WORLD_TEMPLATE_KIND,
    WORLD_TEMPLATE_SCHEMA_VERSION, WorldTemplateV3,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WorldPositionStatus {
    Passable,
    Blocked,
    OutOfBounds,
}

pub(super) fn validate_envelope(template: &WorldTemplateV3, errors: &mut Vec<String>) {
    if template.schema_version != WORLD_TEMPLATE_SCHEMA_VERSION {
        errors.push(format!(
            "world_template.schema_version must be {WORLD_TEMPLATE_SCHEMA_VERSION}"
        ));
    }
    if template.kind != WORLD_TEMPLATE_KIND {
        errors.push(format!(
            "world_template.kind must be {WORLD_TEMPLATE_KIND:?}"
        ));
    }
    if template.id.trim().is_empty() {
        errors.push("world_template.id must be non-empty".to_string());
    }
    if template.visual_manifest_digest.len() != 64
        || !template
            .visual_manifest_digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        errors.push(
            "world_template.visual_manifest_digest must be 64 lowercase hexadecimal characters"
                .to_string(),
        );
    }
}

pub(super) fn validate_template(
    template: &WorldTemplateV3,
    terrains: &[TerrainDef],
    clean_content: bool,
    errors: &mut Vec<String>,
) {
    let terrain_map = validate_terrains(terrains, clean_content, errors);
    validate_world(template, &terrain_map, errors);
}

fn validate_terrains<'a>(
    terrains: &'a [TerrainDef],
    clean_content: bool,
    errors: &mut Vec<String>,
) -> BTreeMap<&'a str, &'a TerrainDef> {
    let mut ids = BTreeMap::new();
    for (index, terrain) in terrains.iter().enumerate() {
        let prefix = format!("terrains[{index}]");
        if terrain.id.trim().is_empty() {
            errors.push(format!("{prefix}.id must be non-empty"));
        } else if let Some(previous) = ids.insert(terrain.id.as_str(), terrain) {
            errors.push(format!(
                "{prefix}.id duplicates selected terrain runtime id {:?}",
                previous.id
            ));
        }
        if terrain.name.trim().is_empty() {
            errors.push(format!("{prefix}.name must be non-empty"));
        }
        match &terrain.navigation {
            TerrainNavigationDef::Walk { move_cost, .. }
            | TerrainNavigationDef::Swim { move_cost, .. } => {
                if *move_cost <= 0 {
                    errors.push(format!("{prefix}.navigation.move_cost must be positive"));
                }
            }
            TerrainNavigationDef::Blocked { .. } => {}
            TerrainNavigationDef::Unresolved {
                source_code: _,
                question_id,
            } => {
                if clean_content {
                    errors.push(format!(
                        "{prefix}.navigation unresolved terrain is invalid in clean content"
                    ));
                }
                if question_id.trim().is_empty() {
                    errors.push(format!("{prefix}.navigation.question_id must be non-empty"));
                }
            }
        }
    }
    ids
}

fn validate_world(
    template: &WorldTemplateV3,
    terrains: &BTreeMap<&str, &TerrainDef>,
    errors: &mut Vec<String>,
) {
    if template.realms.is_empty() {
        errors.push("world_template.realms must be non-empty".to_string());
    }
    for (realm_id, realm) in &template.realms {
        let prefix = format!("world_template.realms[{realm_id:?}]");
        if realm_id.trim().is_empty() {
            errors.push("world_template realm ids must be non-empty".to_string());
        }
        if realm.name.trim().is_empty() {
            errors.push(format!("{prefix}.name must be non-empty"));
        }
        if realm.levels.is_empty() {
            errors.push(format!("{prefix}.levels must be non-empty"));
        }
        for (level_id, level) in &realm.levels {
            let level_prefix = format!("{prefix}.levels[{level_id:?}]");
            if level_id.trim().is_empty() {
                errors.push(format!("{prefix} level ids must be non-empty"));
            }
            if level.width <= 0 {
                errors.push(format!("{level_prefix}.width must be positive"));
            }
            if level.height <= 0 {
                errors.push(format!("{level_prefix}.height must be positive"));
            }
            if level.world_zoom.screen_cell_pitch[0] == 0
                || level.world_zoom.screen_cell_pitch[1] == 0
            {
                errors.push(format!(
                    "{level_prefix}.world_zoom.screen_cell_pitch values must be positive"
                ));
            }
            if level.maximum_clear_sightline == 0 || level.maximum_clear_sightline > 12 {
                errors.push(format!(
                    "{level_prefix}.maximum_clear_sightline must be in 1..=12"
                ));
            }
            match (level.scene_role, level.presentation_mode) {
                (
                    super::super::SceneRoleDef::Overworld,
                    super::super::PresentationModeDef::OverworldTown,
                )
                | (
                    super::super::SceneRoleDef::CombatSpace,
                    super::super::PresentationModeDef::CombatSpace,
                )
                | (super::super::SceneRoleDef::Interior, _) => {}
                _ => errors.push(format!(
                    "{level_prefix}.scene_role and presentation_mode disagree"
                )),
            }
            match (level.scene_role, level.staged_viewport) {
                (super::super::SceneRoleDef::Interior, Some(viewport)) => {
                    if !viewport.fit_whole_level {
                        errors.push(format!(
                            "{level_prefix}.staged_viewport.fit_whole_level must be true"
                        ));
                    }
                    let required_width = i64::from(level.width.max(0))
                        * i64::from(level.world_zoom.screen_cell_pitch[0]);
                    let required_height = i64::from(level.height.max(0))
                        * i64::from(level.world_zoom.screen_cell_pitch[1])
                        + 152;
                    if required_width > i64::from(viewport.frame_size[0])
                        || required_height > i64::from(viewport.frame_size[1])
                    {
                        errors.push(format!(
                            "{level_prefix} whole-level composition requires {required_width}x{required_height} but staged viewport is {}x{}",
                            viewport.frame_size[0], viewport.frame_size[1]
                        ));
                    }
                }
                (super::super::SceneRoleDef::Interior, None) => errors.push(format!(
                    "{level_prefix}.staged_viewport is required for an interior"
                )),
                (_, Some(_)) => errors.push(format!(
                    "{level_prefix}.staged_viewport is only valid for an interior"
                )),
                (_, None) => {}
            }
            if level.cells.len() != level.height.max(0) as usize {
                errors.push(format!(
                    "{level_prefix}.cells has {} rows but height is {}",
                    level.cells.len(),
                    level.height
                ));
            }
            for (y, row) in level.cells.iter().enumerate() {
                if row.len() != level.width.max(0) as usize {
                    errors.push(format!(
                        "{level_prefix}.cells[{y}] has {} cells but width is {}",
                        row.len(),
                        level.width
                    ));
                }
                for (x, cell) in row.iter().enumerate() {
                    let cell_prefix = format!("{level_prefix}.cells[{y}][{x}]");
                    if cell.is_empty() {
                        errors.push(format!("{cell_prefix} must be a non-empty layer list"));
                    }
                    if !cell.iter().any(Option::is_some) {
                        errors.push(format!(
                            "{cell_prefix} must contain at least one non-null terrain layer"
                        ));
                    }
                    for (layer, terrain_id) in cell.iter().enumerate() {
                        if let Some(terrain_id) = terrain_id
                            && !terrains.contains_key(terrain_id.as_str())
                        {
                            errors.push(format!(
                                "{cell_prefix}[{layer}] references unselected terrain {terrain_id:?}"
                            ));
                        }
                    }
                }
            }
            validate_level_doctrine(level, terrains, &level_prefix, errors);
        }
    }

    for (arrival_id, location) in &template.arrivals {
        if arrival_id.trim().is_empty() {
            errors.push("world_template arrival ids must be non-empty".to_string());
        }
        validate_passable_position(
            template,
            terrains,
            location,
            &format!("world_template.arrivals[{arrival_id:?}]"),
            errors,
        );
    }

    let mut automatic = BTreeMap::<&WorldPosition, &str>::new();
    let mut explicit = BTreeMap::<(&WorldPosition, String), &str>::new();
    let mut door_bindings = BTreeMap::<&str, Vec<(&str, &super::super::TopologyEdgeDef)>>::new();
    for (edge_id, edge) in &template.topology {
        let prefix = format!("world_template.topology[{edge_id:?}]");
        if edge_id.trim().is_empty() {
            errors.push("world_template topology edge ids must be non-empty".to_string());
        }
        validate_passable_position(
            template,
            terrains,
            &edge.at,
            &format!("{prefix}.at"),
            errors,
        );
        let resolved = match &edge.target {
            TopologyTargetDef::Position { location } => {
                if location.realm != edge.at.realm {
                    errors.push(format!(
                        "{prefix}.target must use an arrival for a cross-realm destination"
                    ));
                }
                Some(location)
            }
            TopologyTargetDef::Arrival { arrival_id } => {
                if arrival_id.trim().is_empty() {
                    errors.push(format!("{prefix}.target.arrival_id must be non-empty"));
                }
                match template.arrivals.get(arrival_id) {
                    Some(location) => Some(location),
                    None => {
                        errors.push(format!(
                            "{prefix}.target.arrival_id {arrival_id:?} does not exist"
                        ));
                        None
                    }
                }
            }
        };
        if let Some(location) = resolved {
            validate_passable_position(
                template,
                terrains,
                location,
                &format!("{prefix}.target"),
                errors,
            );
        }

        let key = explicit_key(&edge.kind);
        if let Some(key) = key {
            if let Some(previous) = explicit.insert((&edge.at, key.clone()), edge_id.as_str()) {
                errors.push(format!(
                    "{prefix}.kind conflicts with explicit topology edge {previous:?} at {}",
                    edge.at.label()
                ));
            }
        } else if let Some(previous) = automatic.insert(&edge.at, edge_id.as_str()) {
            errors.push(format!(
                "{prefix}.kind conflicts with automatic topology edge {previous:?} at {}",
                edge.at.label()
            ));
        }
        if let super::super::TopologyKindDef::Door {
            binding_id,
            endpoint_id,
            reciprocal_endpoint_id,
            ..
        } = &edge.kind
        {
            if binding_id.trim().is_empty()
                || endpoint_id.trim().is_empty()
                || reciprocal_endpoint_id.trim().is_empty()
                || endpoint_id == reciprocal_endpoint_id
            {
                errors.push(format!(
                    "{prefix}.kind door binding and distinct endpoint IDs must be non-empty"
                ));
            }
            door_bindings
                .entry(binding_id.as_str())
                .or_default()
                .push((edge_id.as_str(), edge));
        }
    }
    for (binding_id, endpoints) in door_bindings {
        if endpoints.len() != 2 {
            errors.push(format!(
                "world_template door binding {binding_id:?} has {} endpoints; expected exactly 2",
                endpoints.len()
            ));
            continue;
        }
        let (left_id, left) = endpoints[0];
        let (right_id, right) = endpoints[1];
        let (
            super::super::TopologyKindDef::Door {
                endpoint_id: left_endpoint,
                reciprocal_endpoint_id: left_reciprocal,
                ..
            },
            super::super::TopologyKindDef::Door {
                endpoint_id: right_endpoint,
                reciprocal_endpoint_id: right_reciprocal,
                ..
            },
        ) = (&left.kind, &right.kind)
        else {
            unreachable!("door binding registry contains only doors");
        };
        let left_target = resolve_target(template, &left.target);
        let right_target = resolve_target(template, &right.target);
        if left_reciprocal != right_endpoint
            || right_reciprocal != left_endpoint
            || left_target != Some(&right.at)
            || right_target != Some(&left.at)
        {
            errors.push(format!(
                "world_template door binding {binding_id:?} endpoints {left_id:?}/{right_id:?} are not exact reciprocals"
            ));
        }
    }
}

fn resolve_target<'a>(
    template: &'a WorldTemplateV3,
    target: &'a TopologyTargetDef,
) -> Option<&'a WorldPosition> {
    match target {
        TopologyTargetDef::Position { location } => Some(location),
        TopologyTargetDef::Arrival { arrival_id } => template.arrivals.get(arrival_id),
    }
}

fn validate_level_doctrine(
    level: &super::super::LevelDef,
    terrains: &BTreeMap<&str, &TerrainDef>,
    prefix: &str,
    errors: &mut Vec<String>,
) {
    // Shape errors are reported by the caller. Doctrine projection indexes by
    // coordinate, so malformed grids must fail closed without panicking.
    if level.width <= 0
        || level.height <= 0
        || level.cells.len() != level.height as usize
        || level
            .cells
            .iter()
            .any(|row| row.len() != level.width as usize)
    {
        return;
    }

    let mut wall_ids = std::collections::BTreeSet::new();
    for (index, terrain_id) in level.wall_terrain_ids.iter().enumerate() {
        let Some(terrain) = terrains.get(terrain_id.as_str()) else {
            errors.push(format!(
                "{prefix}.wall_terrain_ids[{index}] references unselected terrain {terrain_id:?}"
            ));
            continue;
        };
        if matches!(
            terrain.navigation,
            TerrainNavigationDef::Walk { .. } | TerrainNavigationDef::Swim { .. }
        ) {
            errors.push(format!(
                "{prefix}.wall_terrain_ids[{index}] must name blocked solid mass"
            ));
        }
        if !wall_ids.insert(terrain_id.as_str()) {
            errors.push(format!(
                "{prefix}.wall_terrain_ids[{index}] duplicates {terrain_id:?}"
            ));
        }
    }

    let is_wall = |x: i32, y: i32| -> bool {
        x >= 0
            && y >= 0
            && x < level.width
            && y < level.height
            && level.cells[y as usize][x as usize]
                .iter()
                .flatten()
                .any(|terrain_id| wall_ids.contains(terrain_id.as_str()))
    };
    let is_walkable = |x: i32, y: i32| -> bool {
        if x < 0 || y < 0 || x >= level.width || y >= level.height {
            return false;
        }
        let mut passable = false;
        let mut blocked = false;
        for terrain_id in level.cells[y as usize][x as usize].iter().flatten() {
            match terrains
                .get(terrain_id.as_str())
                .map(|terrain| &terrain.navigation)
            {
                Some(TerrainNavigationDef::Walk { .. } | TerrainNavigationDef::Swim { .. }) => {
                    passable = true;
                }
                Some(
                    TerrainNavigationDef::Blocked { .. } | TerrainNavigationDef::Unresolved { .. },
                )
                | None => blocked = true,
            }
        }
        passable && !blocked
    };

    for y in 0..level.height {
        for x in 0..level.width {
            if is_wall(x, y) && is_walkable(x, y + 1) && !is_wall(x, y - 1) {
                errors.push(format!(
                    "{prefix}.wall_run[{x},{y}] projected shadow requires solid mass at [{x},{}]",
                    y - 1
                ));
            }
        }
    }

    let mut actual_max = 0_u32;
    for y in 0..level.height {
        let mut run = 0_u32;
        for x in 0..level.width {
            if is_walkable(x, y) {
                run += 1;
                actual_max = actual_max.max(run);
            } else {
                run = 0;
            }
        }
    }
    for x in 0..level.width {
        let mut run = 0_u32;
        for y in 0..level.height {
            if is_walkable(x, y) {
                run += 1;
                actual_max = actual_max.max(run);
            } else {
                run = 0;
            }
        }
    }
    if matches!(
        level.presentation_mode,
        super::super::PresentationModeDef::CombatSpace
    ) && !level.wall_terrain_ids.is_empty()
        && actual_max > level.maximum_clear_sightline
    {
        errors.push(format!(
            "{prefix}.maximum_clear_sightline declares {} but static topology contains a cardinal run of {actual_max}",
            level.maximum_clear_sightline
        ));
    }

    let mut prop_ids = std::collections::BTreeSet::new();
    for (index, prop) in level.static_props.iter().enumerate() {
        if prop.id.trim().is_empty()
            || prop.visual_family.trim().is_empty()
            || !prop_ids.insert(prop.id.as_str())
        {
            errors.push(format!(
                "{prefix}.static_props[{index}] must have unique non-empty id and visual_family"
            ));
        }
        if prop.anchor.x < 0
            || prop.anchor.y < 0
            || prop.anchor.x >= level.width
            || prop.anchor.y >= level.height
        {
            errors.push(format!(
                "{prefix}.static_props[{index}].anchor is out of bounds at {},{}",
                prop.anchor.x, prop.anchor.y
            ));
        }
    }
}

fn explicit_key(kind: &TopologyKindDef) -> Option<String> {
    match kind {
        TopologyKindDef::Stairs { direction } => Some(format!("stairs:{direction:?}")),
        TopologyKindDef::Climb { direction } => Some(format!("climb:{direction:?}")),
        TopologyKindDef::Door { .. }
        | TopologyKindDef::Pit
        | TopologyKindDef::Passage
        | TopologyKindDef::Portal => None,
    }
}

fn validate_passable_position(
    template: &WorldTemplateV3,
    terrains: &BTreeMap<&str, &TerrainDef>,
    location: &WorldPosition,
    label: &str,
    errors: &mut Vec<String>,
) {
    match position_status(template, terrains, location) {
        None => errors.push(format!(
            "{label} references missing realm/level {}/{}",
            location.realm, location.level
        )),
        Some(WorldPositionStatus::OutOfBounds) => {
            errors.push(format!("{label} is out of bounds at {}", location.label()))
        }
        Some(WorldPositionStatus::Blocked) => errors.push(format!(
            "{label} is not traversable at {}",
            location.label()
        )),
        Some(WorldPositionStatus::Passable) => {}
    }
}

pub(super) fn position_status(
    template: &WorldTemplateV3,
    terrains: &BTreeMap<&str, &TerrainDef>,
    location: &WorldPosition,
) -> Option<WorldPositionStatus> {
    if location.realm.trim().is_empty() || location.level.trim().is_empty() {
        return None;
    }
    let level = template
        .realms
        .get(&location.realm)?
        .levels
        .get(&location.level)?;
    let position = location.position;
    if position.x < 0 || position.y < 0 || position.x >= level.width || position.y >= level.height {
        return Some(WorldPositionStatus::OutOfBounds);
    }
    let Some(cell) = level
        .cells
        .get(position.y as usize)
        .and_then(|row| row.get(position.x as usize))
    else {
        return Some(WorldPositionStatus::OutOfBounds);
    };
    let mut blocked = false;
    let mut has_traversable = false;
    for terrain_id in cell.iter().flatten() {
        match terrains
            .get(terrain_id.as_str())
            .map(|terrain| &terrain.navigation)
        {
            Some(TerrainNavigationDef::Walk { .. } | TerrainNavigationDef::Swim { .. }) => {
                has_traversable = true;
            }
            Some(
                TerrainNavigationDef::Blocked { .. } | TerrainNavigationDef::Unresolved { .. },
            )
            | None => blocked = true,
        }
    }
    Some(if blocked || !has_traversable {
        WorldPositionStatus::Blocked
    } else {
        WorldPositionStatus::Passable
    })
}

pub(super) fn terrain_map(terrains: &[TerrainDef]) -> BTreeMap<&str, &TerrainDef> {
    terrains
        .iter()
        .map(|terrain| (terrain.id.as_str(), terrain))
        .collect()
}

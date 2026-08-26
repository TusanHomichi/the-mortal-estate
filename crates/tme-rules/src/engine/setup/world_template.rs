use crate::content::{
    DoorStateDef, LawZoneDef, PresentationModeDef, SceneRoleDef, TopologyKindDef,
    TopologyTargetDef, WorldTemplateV3,
};
use crate::engine::WorldTemplate;
use crate::model::{DoorState, LawZone, LevelState, NavigationDef, NavigationKind, RealmState};

pub(super) fn compile(source: &WorldTemplateV3) -> WorldTemplate {
    let realms = source
        .realms
        .iter()
        .map(|(realm_id, realm)| {
            let levels = realm
                .levels
                .iter()
                .map(|(level_id, level)| {
                    (
                        level_id.clone(),
                        LevelState {
                            law_zone: match level.law_zone {
                                LawZoneDef::None => LawZone::None,
                                LawZoneDef::Town => LawZone::Town,
                            },
                            scene_role: match level.scene_role {
                                SceneRoleDef::Overworld => crate::model::SceneRole::Overworld,
                                SceneRoleDef::CombatSpace => crate::model::SceneRole::CombatSpace,
                                SceneRoleDef::Interior => crate::model::SceneRole::Interior,
                            },
                            presentation_mode: match level.presentation_mode {
                                PresentationModeDef::OverworldTown => {
                                    crate::model::PresentationMode::OverworldTown
                                }
                                PresentationModeDef::CombatSpace => {
                                    crate::model::PresentationMode::CombatSpace
                                }
                            },
                            world_zoom: level.world_zoom.screen_cell_pitch,
                            maximum_clear_sightline: level.maximum_clear_sightline,
                            staged_viewport: level.staged_viewport,
                            wall_terrain_ids: level.wall_terrain_ids.clone(),
                            static_props: level.static_props.clone(),
                            width: level.width,
                            height: level.height,
                            cells: level.cells.clone(),
                        },
                    )
                })
                .collect();
            (
                realm_id.clone(),
                RealmState {
                    name: realm.name.clone(),
                    levels,
                },
            )
        })
        .collect();

    let arrivals = source.arrivals.clone().into_iter().collect();
    let mut navigation = std::collections::HashMap::new();
    for edge in source.topology.values() {
        let target = match &edge.target {
            TopologyTargetDef::Position { location } => location.clone(),
            TopologyTargetDef::Arrival { arrival_id } => source.arrivals[arrival_id].clone(),
        };
        let (kind, initial_state) = match edge.kind {
            TopologyKindDef::Door { initial_state, .. } => (
                NavigationKind::Door,
                Some(match initial_state {
                    DoorStateDef::Open => DoorState::Open,
                    DoorStateDef::Closed => DoorState::Closed,
                }),
            ),
            TopologyKindDef::Stairs { direction } => (NavigationKind::Stairs { direction }, None),
            TopologyKindDef::Pit => (NavigationKind::Pit, None),
            TopologyKindDef::Climb { direction } => (NavigationKind::Climb { direction }, None),
            TopologyKindDef::Passage => (NavigationKind::Passage, None),
            TopologyKindDef::Portal => (NavigationKind::Portal, None),
        };
        navigation
            .entry(edge.at.clone())
            .or_insert_with(Vec::new)
            .push(NavigationDef {
                kind,
                target,
                initial_state,
                hidden: edge.hidden,
            });
    }

    WorldTemplate {
        visual_manifest_digest: source.visual_manifest_digest.clone(),
        realms,
        arrivals,
        navigation,
    }
}

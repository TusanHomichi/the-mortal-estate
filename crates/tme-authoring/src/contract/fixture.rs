//! The authoring fixture's accepted contract.
//!
//! Two members, synthetic identifiers, zero content authority, and no runtime
//! that loads it. The fixture exists so the compiler and the Workbench have an
//! honest logical target; see `content/authoring-fixture/README.md`.

use super::{
    LandContract, LandmarkContract, LevelPresentation, MemberContract, PropertyValue,
    ReceiptAuthority, StructureContract, TileClass, TileRole, TransitionContract, base, class,
};
use tme_rules::{LawZoneDef, PresentationModeDef, SceneRoleDef, StagedViewportDef, WorldZoomDef};

const SURFACE_CLASSES: &[TileClass] = &[
    base("testland_deep_water", true),
    base("testland_grass", false),
    base("testland_forest", false),
    base("testland_marsh", false),
    base("testland_rock_coast", true),
    base("testland_town_ground", false),
    class("testland_path", TileRole::Route),
    class("testland_bridge", TileRole::Route),
    class("testland_structure_footprint", TileRole::Footprint),
    class("testland_ruin_ground", TileRole::Mark),
    class("testland_shaft", TileRole::Mark),
    class("fixture_blocked", TileRole::Passability { walkable: false }),
    class("fixture_walkable", TileRole::Passability { walkable: true }),
];

const INTERIOR_CLASSES: &[TileClass] = &[
    base("testland_wall", true),
    base("testland_dungeon_floor", false),
    base("testland_stairs_up", false),
    class("fixture_blocked", TileRole::Passability { walkable: false }),
    class("fixture_walkable", TileRole::Passability { walkable: true }),
];

/// The fixture's standing self-description: synthetic, original, carrying no
/// content authority, and never loaded by a runtime (P8).
const MAP_PROPERTIES: &[(&str, PropertyValue)] = &[
    (
        "artifact_status",
        PropertyValue::Text("tracked_synthetic_authoring_fixture"),
    ),
    (
        "authoring_origin",
        PropertyValue::Text("original_synthetic_fixture"),
    ),
    ("content_authority", PropertyValue::Text("none")),
    ("envelope_locked", PropertyValue::Flag(true)),
    ("runtime_consumable", PropertyValue::Flag(false)),
];

const WORLD_ZOOM: WorldZoomDef = WorldZoomDef {
    screen_cell_pitch: [156, 104],
};

pub static LAND: LandContract = LandContract {
    id: "authoring_fixture",
    realm_id: "testland",
    realm_name: "Testland",
    arrival_id: "fixture_dock",
    members: &[
        MemberContract {
            id: "surface",
            document: "content/authoring-fixture/fixture-surface.tmj",
            width: 24,
            height: 16,
            classes: SURFACE_CLASSES,
            tile_layers: &[
                "base_terrain",
                "routes",
                "structure_footprints",
                "landmark_marks",
                "passability",
            ],
            object_layers: &["structures", "transitions", "landmarks"],
            map_properties: MAP_PROPERTIES,
            structures: &[
                StructureContract {
                    id: "fixture_structure_north",
                    scope: "clustered",
                },
                StructureContract {
                    id: "fixture_structure_outland",
                    scope: "isolated",
                },
                StructureContract {
                    id: "fixture_structure_south",
                    scope: "clustered",
                },
            ],
            landmarks: &[
                LandmarkContract {
                    id: "fixture_dock_arrival",
                    role: "arrival",
                    marker_class: "",
                },
                LandmarkContract {
                    id: "fixture_ruin_marker",
                    role: "ruin",
                    marker_class: "testland_ruin_ground",
                },
            ],
            transitions: &[TransitionContract {
                id: "fixture_descent",
                target_member: "interior",
                paired_transition: "fixture_ascent",
                direction: "down",
                marker_class: "testland_shaft",
            }],
            clustered_ground_class: Some("testland_town_ground"),
            wall_terrain_ids: &[],
            presentation: LevelPresentation {
                scene_role: SceneRoleDef::Overworld,
                presentation_mode: PresentationModeDef::OverworldTown,
                law_zone: LawZoneDef::None,
                world_zoom: WORLD_ZOOM,
                staged_viewport: None,
            },
            candidate_entry: true,
        },
        MemberContract {
            id: "interior",
            document: "content/authoring-fixture/fixture-interior.tmj",
            width: 10,
            height: 8,
            classes: INTERIOR_CLASSES,
            tile_layers: &["base_terrain", "passability"],
            object_layers: &["transitions"],
            map_properties: MAP_PROPERTIES,
            structures: &[],
            landmarks: &[],
            transitions: &[TransitionContract {
                id: "fixture_ascent",
                target_member: "surface",
                paired_transition: "fixture_descent",
                direction: "up",
                marker_class: "testland_stairs_up",
            }],
            clustered_ground_class: None,
            wall_terrain_ids: &["testland_wall"],
            presentation: LevelPresentation {
                scene_role: SceneRoleDef::Interior,
                presentation_mode: PresentationModeDef::OverworldTown,
                law_zone: LawZoneDef::None,
                world_zoom: WORLD_ZOOM,
                staged_viewport: Some(StagedViewportDef {
                    frame_size: [1920, 1080],
                    fit_whole_level: true,
                }),
            },
            candidate_entry: false,
        },
    ],
    receipt_path: "content/authoring-fixture/promotion.json",
    receipt_kind: "authoring_fixture_promotion",
    receipt_status: "owner_accepted_at_g4x",
    receipt_attested_by: "peter",
    receipt_attested_on: "2026-08-19",
    master_digest: "3ffd01beec1db6df3218c0fb5870335dffe42128ec53cbbdfe8fdb832b5925da",
    authority: ReceiptAuthority {
        coordinates: true,
        terrain_and_passability: true,
        structures_and_landmarks: true,
        member_transition_endpoints: true,
        // The fixture is not runtime content and never becomes it. The land a
        // runtime loads is the identity proof's, whose receipt says so.
        runtime_loads_authoring_source: false,
        presentation_art: false,
        gameplay_tuning: false,
        content_canon: false,
    },
    world_template_output: "content/authoring-fixture/generated/world_template.json",
    report_output: "content/authoring-fixture/generated/compile_report.json",
    workbench_projection_output: "content/authoring-fixture/generated/workbench_projection.json",
    // The successor has no production content registry yet; the only registry
    // it carries is the test corpus catalog. Both lands bind to it deliberately
    // so that an unmapped terrain class is a compile failure rather than a
    // discovery at load time. Re-point when a production registry lands.
    terrain_registry_catalog: "content/test-corpus/catalogs/prototype_catalog_v6.json",
    terrain_registry_profile: "profile/first_land_structure",
};

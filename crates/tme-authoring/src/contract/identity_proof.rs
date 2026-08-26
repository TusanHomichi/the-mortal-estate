//! The identity proof's land: the first authored land a runtime loads.
//!
//! One member today — `settlement`, 48 x 32 — because the slice that authors
//! the layer beneath is the next one. The member count lives in this
//! declaration and nowhere else, so growing it is an edit here plus a
//! re-attestation, which is exactly the ceremony the promotion gate exists to
//! require.
//!
//! **The identifiers are placeholder role labels.** `identity_proof`,
//! `settlement`, `settlement_ruin_mouth` and the structure ids claim nothing
//! about the settlement's name, the dead world's name, or the vocabulary for
//! death, return, succession, ancestors, and departure. Owner ruling D2
//! reopened every name; the design packet
//! (`docs/plans/2026-08-21-identity-proof-packet.md`) labels these as
//! placeholders and this contract keeps them that way.

use super::{
    LandContract, LandmarkContract, LevelPresentation, MemberContract, PropertyValue,
    ReceiptAuthority, StructureContract, TileClass, TileRole, base, class,
};
use tme_rules::{LawZoneDef, PresentationModeDef, SceneRoleDef, WorldZoomDef};

/// The settlement's tile vocabulary. Every base, route, footprint, and mark
/// class resolves in the runtime terrain registry the land binds to; the two
/// passability classes are annotations and carry no terrain identity.
const SETTLEMENT_CLASSES: &[TileClass] = &[
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
    class("proof_blocked", TileRole::Passability { walkable: false }),
    class("proof_walkable", TileRole::Passability { walkable: true }),
];

pub static LAND: LandContract = LandContract {
    id: "identity_proof",
    realm_id: "identity_proof",
    realm_name: "Identity Proof",
    arrival_id: "settlement_arrival",
    members: &[MemberContract {
        id: "settlement",
        document: "content/lands/identity-proof/settlement.tmj",
        width: 48,
        height: 32,
        classes: SETTLEMENT_CLASSES,
        tile_layers: &[
            "base_terrain",
            "routes",
            "structure_footprints",
            "landmark_marks",
            "passability",
        ],
        object_layers: &["structures", "transitions", "landmarks"],
        map_properties: &[
            (
                "artifact_status",
                PropertyValue::Text("tracked_authored_land"),
            ),
            (
                "authoring_origin",
                PropertyValue::Text("original_authored_land"),
            ),
            (
                "content_authority",
                PropertyValue::Text("authored_geography"),
            ),
            ("envelope_locked", PropertyValue::Flag(true)),
            ("runtime_consumable", PropertyValue::Flag(true)),
        ],
        structures: &[
            StructureContract {
                id: "proof_common_hall",
                scope: "clustered",
            },
            StructureContract {
                id: "proof_keeper_hall",
                scope: "clustered",
            },
            StructureContract {
                id: "proof_waymark_shelter",
                scope: "isolated",
            },
        ],
        landmarks: &[
            LandmarkContract {
                id: "settlement_arrival",
                role: "arrival",
                marker_class: "",
            },
            LandmarkContract {
                id: "settlement_ruin_mouth",
                role: "ruin",
                marker_class: "testland_ruin_ground",
            },
        ],
        // No transition: this land has one member. The descent to the layer
        // beneath is authored by the slice that authors the layer.
        transitions: &[],
        clustered_ground_class: Some("testland_town_ground"),
        wall_terrain_ids: &[],
        presentation: LevelPresentation {
            scene_role: SceneRoleDef::Overworld,
            presentation_mode: PresentationModeDef::OverworldTown,
            law_zone: LawZoneDef::None,
            world_zoom: WorldZoomDef {
                screen_cell_pitch: [156, 104],
            },
            staged_viewport: None,
        },
        candidate_entry: true,
    }],
    receipt_path: "content/lands/identity-proof/promotion.json",
    receipt_kind: "authored_land_promotion",
    // The owner accepted this geography on 2026-08-21, after a shape pass on
    // the first authored version. It was lane-attested and pending until then:
    // fabricating an owner approval would have made the strongest check in this
    // crate a lie. Status, attestor and reviewed digest move together, which is
    // the ceremony the double anchor exists to require.
    receipt_status: "owner_accepted_at_s1",
    receipt_attested_by: "peter",
    receipt_attested_on: "2026-08-21",
    master_digest: "320ecf5553a110ca349272db2ddbb3de005021f648b8b397d8d6bbf9f98b0b0a",
    authority: ReceiptAuthority {
        coordinates: true,
        terrain_and_passability: true,
        structures_and_landmarks: true,
        member_transition_endpoints: true,
        // Owner ruling R1 (2026-08-21): the proof's land is authoring-compiled
        // and the server loads the compiler's output. This is the receipt that
        // carries that authority, and the only one that does.
        runtime_loads_authoring_source: true,
        presentation_art: false,
        gameplay_tuning: false,
        content_canon: false,
    },
    world_template_output: "content/lands/identity-proof/generated/world_template.json",
    report_output: "content/lands/identity-proof/generated/compile_report.json",
    workbench_projection_output: "content/lands/identity-proof/generated/workbench_projection.json",
    terrain_registry_catalog: "content/test-corpus/catalogs/prototype_catalog_v6.json",
    terrain_registry_profile: "profile/first_land_structure",
};

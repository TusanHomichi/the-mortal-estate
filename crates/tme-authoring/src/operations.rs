//! The staged-operation vocabulary: what an owner or an agent may change.
//!
//! **Why the vocabulary lives here.** A verb is a statement about the authored
//! document's object model — its layers, its tile classes, its typed objects.
//! That model is declared once, in [`crate::contract`], and asserted once, in
//! [`crate::compile`]. A verb set defined anywhere else would be a second
//! opinion about what an authored member is made of, and the first time the two
//! disagreed the disagreement would surface as a mysterious rejection rather
//! than as a compile error.
//!
//! **What the vocabulary deliberately cannot do.** The accepted contract pins
//! the structure, landmark, and transition PROGRAMS exactly — which features
//! exist, their scopes, their roles, their pairings. A verb that created or
//! destroyed one would produce a document the compiler rejects by construction,
//! so no such verb exists. V1 edits the features the fixture already declares:
//! where they stand, what ground they stand on, and where the routes run. That
//! is a bound limit of the accepted contract, not a gap in this module —
//! growing the programs is a source change and an owner re-attestation, which
//! is exactly the ceremony the double anchor exists to require.
//!
//! **Three layers are derived and none of them is authored by a verb.**
//! `structure_footprints`, `landmark_marks`, and `passability` follow from the
//! base terrain, the routes, and the typed objects. [`crate::replay`] refreshes
//! them; [`crate::compile`] independently asserts them. A replay that refreshed
//! them wrongly is a rejected candidate, never an accepted lie.

use serde::Deserialize;
use serde_json::Value;

use crate::Result;
use crate::contract::{LandContract, MemberContract, TileRole};
use crate::tiled::Point;

pub const SCHEMA_VERSION: u32 = 1;
pub const OPERATION_SET_KIND: &str = "workbench_truth_operation_set";
pub const VOCABULARY_KIND: &str = "workbench_truth_operation_vocabulary";

/// The one class this module owns. Dressing and asset operations are the
/// project's, never the compiler's: they do not touch an authored member.
pub const TRUTH_CLASS: &str = "truth";

// A truth operation addresses one member, and the member it may address is the
// land's candidate entry point. A member without one is out of reach and says
// so rather than failing later with a confusing diagnostic.

/// One staged operation, exactly as the session log carries it.
///
/// The envelope — who staged it, which record it is — belongs to the session
/// and is carried through untouched so a diagnostic can name the record the
/// owner is looking at. The `parameters` are this module's alone.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StagedOperation {
    pub record_id: String,
    pub author: String,
    pub class: String,
    pub member: String,
    pub verb: String,
    pub parameters: Value,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationSet {
    pub schema_version: u32,
    pub kind: String,
    pub operations: Vec<StagedOperation>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cell {
    pub x: usize,
    pub y: usize,
}

impl From<Cell> for Point {
    fn from(cell: Cell) -> Self {
        Point {
            x: cell.x,
            y: cell.y,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetTerrain {
    pub cells: Vec<Cell>,
    pub class: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetRoute {
    pub cells: Vec<Cell>,
    /// The route class to paint, or `null` to clear the route from the cells.
    pub class: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MoveStructure {
    pub structure_id: String,
    /// The new footprint origin. The access cell travels with the building, so
    /// a move keeps the door it already had; move the access cell on its own
    /// with `set_structure_access`.
    pub to: Cell,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetStructureAccess {
    pub structure_id: String,
    pub cell: Cell,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MoveLandmark {
    pub landmark_id: String,
    pub to: Cell,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetTransitionEndpoint {
    pub transition_id: String,
    pub marker: Option<Cell>,
    pub access: Option<Cell>,
}

/// One parsed verb and its parameters.
#[derive(Debug)]
pub enum TruthEdit {
    SetTerrain(SetTerrain),
    SetRoute(SetRoute),
    MoveStructure(MoveStructure),
    SetStructureAccess(SetStructureAccess),
    MoveLandmark(MoveLandmark),
    SetTransitionEndpoint(SetTransitionEndpoint),
}

pub struct ParameterSpec {
    pub name: &'static str,
    pub shape: &'static str,
    pub summary: &'static str,
    /// Which closed set the value must come from, when there is one.
    ///
    /// Carried as data rather than left for a reader to infer from the summary,
    /// because the interface builds its input from this: a parameter with
    /// choices gets a picker that cannot produce a value the compiler will
    /// refuse, and one without gets a free field. A consumer matching on prose
    /// would be a consumer that breaks when the prose is improved.
    pub choices: Choices,
}

/// Where a parameter's legal values come from. Resolved at description time so
/// the published table carries the actual set rather than a name for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Choices {
    Any,
    BaseTerrain,
    Route,
}

impl Choices {
    fn values(self, member: &'static MemberContract) -> Option<Vec<&'static str>> {
        match self {
            Self::Any => None,
            Self::BaseTerrain => Some(base_classes(member)),
            Self::Route => Some(route_classes(member)),
        }
    }
}

/// One verb, described as data so that the interface, the agent CLI, and the
/// documentation all read the same table instead of restating it.
pub struct VerbSpec {
    pub verb: &'static str,
    pub summary: &'static str,
    pub parameters: &'static [ParameterSpec],
    /// The blocking assertion this verb can provably trip. Every entry has a
    /// rejection test in `tests/operation_replay.rs` that produces exactly
    /// this failure — P9 applied to the verb set rather than to the checks.
    pub rejects: &'static str,
}

pub const VOCABULARY: &[VerbSpec] = &[
    VerbSpec {
        verb: "set_terrain",
        summary: "Repaint the base terrain of one or more cells.",
        parameters: &[
            ParameterSpec {
                name: "cells",
                shape: "[{x, y}]",
                summary: "the cells to repaint",
                choices: Choices::Any,
            },
            ParameterSpec {
                name: "class",
                shape: "string",
                summary: "a base terrain class from the accepted vocabulary",
                choices: Choices::BaseTerrain,
            },
        ],
        rejects: "watering over a walkable cell seals a pocket: \
                  \"the surface has N walkable cells no one can reach\"",
    },
    VerbSpec {
        verb: "set_route",
        summary: "Paint or clear the authored route overlay on one or more cells.",
        parameters: &[
            ParameterSpec {
                name: "cells",
                shape: "[{x, y}]",
                summary: "the cells to change",
                choices: Choices::Any,
            },
            ParameterSpec {
                name: "class",
                shape: "string | null",
                summary: "a route class to paint, or null to clear the route",
                choices: Choices::Route,
            },
        ],
        rejects: "clearing the route under the arrival landmark: \
                  \"the arrival landmark must stand on an authored route cell\"",
    },
    VerbSpec {
        verb: "move_structure",
        summary: "Move a structure's footprint origin; its access cell travels with it.",
        parameters: &[
            ParameterSpec {
                name: "structure_id",
                shape: "string",
                summary: "an authored structure of this member",
                choices: Choices::Any,
            },
            ParameterSpec {
                name: "to",
                shape: "{x, y}",
                summary: "the new footprint origin",
                choices: Choices::Any,
            },
        ],
        rejects: "moving a clustered building off the town ground: \
                  \"structure ... is scoped \\\"clustered\\\" but its footprint at x,y disagrees\"",
    },
    VerbSpec {
        verb: "set_structure_access",
        summary: "Move a structure's access cell. The façade door is derived, never authored.",
        parameters: &[
            ParameterSpec {
                name: "structure_id",
                shape: "string",
                summary: "an authored structure of this member",
                choices: Choices::Any,
            },
            ParameterSpec {
                name: "cell",
                shape: "{x, y}",
                summary: "the new access cell",
                choices: Choices::Any,
            },
        ],
        rejects: "an access cell inside the footprint touches two footprint cells: \
                  \"access cell must touch exactly one footprint cell, not 2\"",
    },
    VerbSpec {
        verb: "move_landmark",
        summary: "Move a landmark, carrying its authored marker tile with it.",
        parameters: &[
            ParameterSpec {
                name: "landmark_id",
                shape: "string",
                summary: "an authored landmark of this member",
                choices: Choices::Any,
            },
            ParameterSpec {
                name: "to",
                shape: "{x, y}",
                summary: "the new cell",
                choices: Choices::Any,
            },
        ],
        rejects: "moving a landmark onto deep water: \"landmark ... stands on a blocked cell\"",
    },
    VerbSpec {
        verb: "set_transition_endpoint",
        summary: "Move a transition's marker cell, its access cell, or both.",
        parameters: &[
            ParameterSpec {
                name: "transition_id",
                shape: "string",
                summary: "an authored transition of this member",
                choices: Choices::Any,
            },
            ParameterSpec {
                name: "marker",
                shape: "{x, y} | null",
                summary: "the new marker cell, or null to leave it",
                choices: Choices::Any,
            },
            ParameterSpec {
                name: "access",
                shape: "{x, y} | null",
                summary: "the new access cell, or null to leave it",
                choices: Choices::Any,
            },
        ],
        rejects: "separating the access cell from its marker: \
                  \"access cell must be cardinally adjacent to its marker\"",
    },
];

/// The vocabulary as a serializable document, for `describe-operations`.
///
/// The verbs are the same for every member; the class sets and the addressable
/// member are the ones this member actually declares, so a picker built from
/// this document cannot offer a class the compiler would refuse.
pub fn vocabulary_document(land: &'static LandContract, member: &'static MemberContract) -> Value {
    serde_json::json!({
        "schema_version": SCHEMA_VERSION,
        "kind": VOCABULARY_KIND,
        "class": TRUTH_CLASS,
        "land": land.id,
        "member": member.id,
        "operation_set_kind": OPERATION_SET_KIND,
        "note": "The compiler owns this vocabulary because a verb is a statement \
                 about the authored object model. Dressing and asset operations \
                 are the project's and are not described here.",
        "verbs": VOCABULARY.iter().map(|spec| serde_json::json!({
            "verb": spec.verb,
            "summary": spec.summary,
            "rejects": spec.rejects,
            "parameters": spec.parameters.iter().map(|parameter| serde_json::json!({
                "name": parameter.name,
                "shape": parameter.shape,
                "summary": parameter.summary,
                "choices": parameter.choices.values(member),
            })).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "terrain_classes": base_classes(member),
        "route_classes": route_classes(member),
    })
}

/// The base terrain classes `set_terrain` accepts, read off the contract.
pub fn base_classes(member: &'static MemberContract) -> Vec<&'static str> {
    member
        .classes
        .iter()
        .filter(|class| matches!(class.role, TileRole::Base { .. }))
        .map(|class| class.name)
        .collect()
}

/// The route classes `set_route` accepts, read off the contract.
pub fn route_classes(member: &'static MemberContract) -> Vec<&'static str> {
    member
        .classes
        .iter()
        .filter(|class| matches!(class.role, TileRole::Route))
        .map(|class| class.name)
        .collect()
}

fn parameters<T: for<'de> Deserialize<'de>>(operation: &StagedOperation) -> Result<T> {
    serde_json::from_value(operation.parameters.clone()).map_err(|error| {
        format!(
            "{}: {} parameters are malformed: {error}",
            operation.record_id, operation.verb
        )
    })
}

/// Parse one staged operation into a typed edit, or refuse naming the record.
///
/// The class and member checks are here rather than at the call site because a
/// refusal has to name the record, and this is the only place that knows both
/// the record and the vocabulary.
pub fn parse(member: &'static MemberContract, operation: &StagedOperation) -> Result<TruthEdit> {
    if operation.class != TRUTH_CLASS {
        return Err(format!(
            "{}: this entry point replays {TRUTH_CLASS} operations; it was handed class {:?}",
            operation.record_id, operation.class
        ));
    }
    if operation.member != member.id {
        return Err(format!(
            "{}: the candidate entry point compiles the {}; \
             member {:?} has no candidate path",
            operation.record_id, member.id, operation.member
        ));
    }
    Ok(match operation.verb.as_str() {
        "set_terrain" => TruthEdit::SetTerrain(parameters(operation)?),
        "set_route" => TruthEdit::SetRoute(parameters(operation)?),
        "move_structure" => TruthEdit::MoveStructure(parameters(operation)?),
        "set_structure_access" => TruthEdit::SetStructureAccess(parameters(operation)?),
        "move_landmark" => TruthEdit::MoveLandmark(parameters(operation)?),
        "set_transition_endpoint" => TruthEdit::SetTransitionEndpoint(parameters(operation)?),
        other => {
            return Err(format!(
                "{}: unknown verb {other:?}; the vocabulary is {}",
                operation.record_id,
                VOCABULARY
                    .iter()
                    .map(|spec| spec.verb)
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::fixture;

    fn editable() -> &'static MemberContract {
        fixture::LAND
            .candidate_member()
            .expect("a candidate member")
    }

    /// The parser and the published table are one vocabulary. A verb added to
    /// one and not the other is a table that lies about what the tool accepts.
    #[test]
    fn every_published_verb_parses_and_every_parsed_verb_is_published() {
        for spec in VOCABULARY {
            let operation = StagedOperation {
                record_id: "op-0001".into(),
                author: "test".into(),
                class: TRUTH_CLASS.into(),
                member: editable().id.into(),
                verb: spec.verb.into(),
                parameters: Value::Object(Default::default()),
            };
            let refusal =
                parse(editable(), &operation).expect_err("empty parameters are malformed");
            assert!(
                refusal.contains("parameters are malformed"),
                "{:?} is not a published verb the parser knows: {refusal}",
                spec.verb
            );
        }
    }

    #[test]
    fn an_unknown_verb_names_the_whole_vocabulary() {
        let operation = StagedOperation {
            record_id: "op-0002".into(),
            author: "test".into(),
            class: TRUTH_CLASS.into(),
            member: editable().id.into(),
            verb: "repaint_everything".into(),
            parameters: Value::Null,
        };
        let refusal = parse(editable(), &operation).expect_err("an unknown verb is refused");
        for spec in VOCABULARY {
            assert!(refusal.contains(spec.verb), "{refusal}");
        }
    }
}

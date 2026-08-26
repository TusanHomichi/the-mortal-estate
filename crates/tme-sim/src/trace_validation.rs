use serde::de::DeserializeOwned;
use serde_json::Value;
use tme_rules::{
    ACTION_CONTEXT_CONTRACT_VERSION, ActorId, ActorKind, COMMAND_CONTRACT_VERSION,
    EVENT_CONTRACT_VERSION, Event, MovementPace, OBSERVED_SNAPSHOT_CONTRACT_VERSION,
    PATH_PREVIEW_CONTRACT_VERSION, PathPreviewV1, PlayerActionContextV2, PlayerCommandV1,
    PlayerIntentPayloadV1, SNAPSHOT_CONTRACT_VERSION, TRACE_CONTRACT_VERSION,
    TRACE_V2_CONTRACT_VERSION, TraceV1, TraceV2, WorldSnapshotV1, WorldSnapshotV2,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceValidationError {
    pub path: String,
    pub message: String,
}

impl TraceValidationError {
    fn new(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            message: message.into(),
        }
    }
}

impl std::fmt::Display for TraceValidationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.path, self.message)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TraceValidationReport {
    pub contract_version: u32,
    pub step_count: usize,
}

pub fn validate_trace_json(
    input: &str,
) -> Result<TraceValidationReport, Vec<TraceValidationError>> {
    let value: Value = serde_json::from_str(input).map_err(|error| {
        vec![TraceValidationError::new(
            "$",
            format!("invalid JSON: {error}"),
        )]
    })?;
    let version = value
        .get("header")
        .and_then(|header| header.get("contract_version"))
        .and_then(Value::as_u64)
        .ok_or_else(|| {
            vec![TraceValidationError::new(
                "$.header.contract_version",
                "missing unsigned trace contract version",
            )]
        })?;

    match version {
        version if version == u64::from(TRACE_CONTRACT_VERSION) => {
            let trace: TraceV1 = strict_deserialize(&value)?;
            canonical_round_trip(&value, &trace)?;
            let errors = validate_trace_v1(&trace);
            finish(TRACE_CONTRACT_VERSION, trace.steps.len(), errors)
        }
        version if version == u64::from(TRACE_V2_CONTRACT_VERSION) => {
            let trace: TraceV2 = strict_deserialize(&value)?;
            canonical_round_trip(&value, &trace)?;
            let errors = validate_trace_v2(&trace);
            finish(TRACE_V2_CONTRACT_VERSION, trace.steps.len(), errors)
        }
        _ => Err(vec![TraceValidationError::new(
            "$.header.contract_version",
            format!("unsupported trace contract version {version}"),
        )]),
    }
}

fn finish(
    contract_version: u32,
    step_count: usize,
    errors: Vec<TraceValidationError>,
) -> Result<TraceValidationReport, Vec<TraceValidationError>> {
    if errors.is_empty() {
        Ok(TraceValidationReport {
            contract_version,
            step_count,
        })
    } else {
        Err(errors)
    }
}

fn strict_deserialize<T: DeserializeOwned>(value: &Value) -> Result<T, Vec<TraceValidationError>> {
    let encoded = serde_json::to_string(value).expect("JSON Value must serialize");
    let mut deserializer = serde_json::Deserializer::from_str(&encoded);
    serde_path_to_error::deserialize(&mut deserializer).map_err(|error| {
        vec![TraceValidationError::new(
            format!("$.{}", error.path()),
            stable_serde_message(error.inner()),
        )]
    })
}

fn stable_serde_message(error: &serde_json::Error) -> String {
    error
        .to_string()
        .split_once(" at line ")
        .map_or_else(|| error.to_string(), |(message, _)| message.to_string())
}

fn canonical_round_trip<T: serde::Serialize>(
    original: &Value,
    typed: &T,
) -> Result<(), Vec<TraceValidationError>> {
    let canonical = serde_json::to_value(typed).expect("typed trace must serialize");
    if let Some((path, message)) = first_difference(original, &canonical, "$") {
        Err(vec![TraceValidationError::new(
            path,
            format!("trace is not canonical: {message}"),
        )])
    } else {
        Ok(())
    }
}

fn first_difference(left: &Value, right: &Value, path: &str) -> Option<(String, String)> {
    match (left, right) {
        (Value::Object(left), Value::Object(right)) => {
            for key in left.keys() {
                if !right.contains_key(key) {
                    return Some((
                        format!("{path}.{key}"),
                        "field is not emitted by the current Rust DTO".to_string(),
                    ));
                }
            }
            for key in right.keys() {
                if !left.contains_key(key) {
                    return Some((
                        format!("{path}.{key}"),
                        "required canonical field is omitted".to_string(),
                    ));
                }
            }
            for (key, left_value) in left {
                if let Some(difference) =
                    first_difference(left_value, &right[key], &format!("{path}.{key}"))
                {
                    return Some(difference);
                }
            }
            None
        }
        (Value::Array(left), Value::Array(right)) => {
            if left.len() != right.len() {
                return Some((
                    path.to_string(),
                    format!(
                        "array length {} differs from canonical length {}",
                        left.len(),
                        right.len()
                    ),
                ));
            }
            left.iter()
                .zip(right)
                .enumerate()
                .find_map(|(index, (left, right))| {
                    first_difference(left, right, &format!("{path}[{index}]"))
                })
        }
        _ if left != right => Some((
            path.to_string(),
            format!("value {left} differs from canonical value {right}"),
        )),
        _ => None,
    }
}

fn validate_trace_v1(trace: &TraceV1) -> Vec<TraceValidationError> {
    let mut errors = Vec::new();
    check_version(
        &mut errors,
        "$.header.contract_version",
        trace.header.contract_version,
        TRACE_CONTRACT_VERSION,
    );
    check_non_empty(
        &mut errors,
        "$.header.scenario_id",
        &trace.header.scenario_id,
    );
    validate_debug_snapshot(
        &mut errors,
        "$.header.initial_snapshot",
        &trace.header.initial_snapshot,
    );

    let mut before = &trace.header.initial_snapshot;
    for (index, step) in trace.steps.iter().enumerate() {
        let path = format!("$.steps[{index}]");
        check_version(
            &mut errors,
            &format!("{path}.contract_version"),
            step.contract_version,
            TRACE_CONTRACT_VERSION,
        );
        if step.step_index != index {
            errors.push(TraceValidationError::new(
                format!("{path}.step_index"),
                format!("expected {index}, found {}", step.step_index),
            ));
        }
        check_non_empty(
            &mut errors,
            &format!("{path}.intent_label"),
            &step.intent_label,
        );
        if step.events.is_empty() {
            errors.push(TraceValidationError::new(
                format!("{path}.events"),
                "must contain the player prelude",
            ));
        }
        let actor_id = single_controlled_actor(before);
        validate_step_prelude(
            &mut errors,
            &format!("{path}.events"),
            &step.events,
            actor_id,
            &step.intent_label,
        );
        validate_debug_snapshot(
            &mut errors,
            &format!("{path}.after_snapshot"),
            &step.after_snapshot,
        );
        if step.after_snapshot.logical_time < before.logical_time {
            errors.push(TraceValidationError::new(
                format!("{path}.after_snapshot.logical_time"),
                "regresses from the previous snapshot",
            ));
        }
        if let Some(preview) = &step.preview {
            validate_preview(
                &mut errors,
                &format!("{path}.preview"),
                preview,
                actor_id,
                None,
            );
            if let Some(actor) = step
                .after_snapshot
                .actors
                .iter()
                .find(|actor| actor.id == preview.actor_id)
                && actor.location != preview.final_position
            {
                errors.push(TraceValidationError::new(
                    format!("{path}.preview.final_position"),
                    "does not match the addressed actor after the step",
                ));
            }
        }
        before = &step.after_snapshot;
    }

    check_version(
        &mut errors,
        "$.final.contract_version",
        trace.r#final.contract_version,
        TRACE_CONTRACT_VERSION,
    );
    validate_debug_snapshot(
        &mut errors,
        "$.final.final_snapshot",
        &trace.r#final.final_snapshot,
    );
    if trace.r#final.final_snapshot != *before {
        errors.push(TraceValidationError::new(
            "$.final.final_snapshot",
            "does not equal the terminal step surface",
        ));
    }
    errors
}

fn validate_trace_v2(trace: &TraceV2) -> Vec<TraceValidationError> {
    let mut errors = Vec::new();
    let header = &trace.header;
    check_version(
        &mut errors,
        "$.header.contract_version",
        header.contract_version,
        TRACE_V2_CONTRACT_VERSION,
    );
    check_version(
        &mut errors,
        "$.header.event_contract_version",
        header.event_contract_version,
        EVENT_CONTRACT_VERSION,
    );
    check_version(
        &mut errors,
        "$.header.snapshot_contract_version",
        header.snapshot_contract_version,
        SNAPSHOT_CONTRACT_VERSION,
    );
    check_version(
        &mut errors,
        "$.header.observed_snapshot_contract_version",
        header.observed_snapshot_contract_version,
        OBSERVED_SNAPSHOT_CONTRACT_VERSION,
    );
    check_version(
        &mut errors,
        "$.header.action_context_contract_version",
        header.action_context_contract_version,
        ACTION_CONTEXT_CONTRACT_VERSION,
    );
    check_version(
        &mut errors,
        "$.header.intent_contract_version",
        header.intent_contract_version,
        COMMAND_CONTRACT_VERSION,
    );
    check_non_empty(&mut errors, "$.header.scenario_id", &header.scenario_id);
    validate_v2_surfaces(
        &mut errors,
        "$.header",
        &header.initial_debug_snapshot,
        &header.initial_observed_snapshot,
        &header.initial_action_context,
    );

    let mut before_debug = &header.initial_debug_snapshot;
    let mut before_context = &header.initial_action_context;
    let mut terminal_observed = &header.initial_observed_snapshot;
    for (index, step) in trace.steps.iter().enumerate() {
        let path = format!("$.steps[{index}]");
        if step.step_index != index {
            errors.push(TraceValidationError::new(
                format!("{path}.step_index"),
                format!("expected {index}, found {}", step.step_index),
            ));
        }
        validate_command(&mut errors, &format!("{path}.command"), &step.command);
        check_non_empty(
            &mut errors,
            &format!("{path}.intent_label"),
            &step.intent_label,
        );
        if step.events.is_empty() {
            errors.push(TraceValidationError::new(
                format!("{path}.events"),
                "must contain the player prelude",
            ));
        }
        validate_step_prelude(
            &mut errors,
            &format!("{path}.events"),
            &step.events,
            Some(&step.command.actor_id),
            &step.intent_label,
        );
        validate_v2_surfaces(
            &mut errors,
            &path,
            &step.after_debug_snapshot,
            &step.after_observed_snapshot,
            &step.after_action_context,
        );
        if step.after_debug_snapshot.logical_time < before_debug.logical_time {
            errors.push(TraceValidationError::new(
                format!("{path}.after_debug_snapshot.logical_time"),
                "regresses from the previous snapshot",
            ));
        }
        if step.command.actor_id != step.after_action_context.actor_id {
            errors.push(TraceValidationError::new(
                format!("{path}.after_action_context.actor_id"),
                "does not match the command actor",
            ));
        }

        let requested_path = match &step.command.intent {
            PlayerIntentPayloadV1::MovePath { path: requested } => Some(requested.as_slice()),
            _ => None,
        };
        match (&step.preview, requested_path) {
            (Some(preview), Some(requested)) => {
                validate_preview(
                    &mut errors,
                    &format!("{path}.preview"),
                    preview,
                    Some(&step.command.actor_id),
                    Some(requested),
                );
                if preview.start != before_context.position {
                    errors.push(TraceValidationError::new(
                        format!("{path}.preview.start"),
                        "does not match the actor position before the step",
                    ));
                }
                if preview.final_position != step.after_action_context.position {
                    errors.push(TraceValidationError::new(
                        format!("{path}.preview.final_position"),
                        "does not match the actor position after the step",
                    ));
                }
            }
            (None, Some(_)) => errors.push(TraceValidationError::new(
                format!("{path}.preview"),
                "missing for move_path command",
            )),
            (Some(_), None) => errors.push(TraceValidationError::new(
                format!("{path}.preview"),
                "present for a non-movement command",
            )),
            (None, None) => {}
        }

        before_debug = &step.after_debug_snapshot;
        before_context = &step.after_action_context;
        terminal_observed = &step.after_observed_snapshot;
    }

    check_version(
        &mut errors,
        "$.final.contract_version",
        trace.r#final.contract_version,
        TRACE_V2_CONTRACT_VERSION,
    );
    validate_v2_surfaces(
        &mut errors,
        "$.final",
        &trace.r#final.final_debug_snapshot,
        &trace.r#final.final_observed_snapshot,
        &trace.r#final.final_action_context,
    );
    if trace.r#final.final_debug_snapshot != *before_debug {
        errors.push(TraceValidationError::new(
            "$.final.final_debug_snapshot",
            "does not equal the terminal step surface",
        ));
    }
    if trace.r#final.final_observed_snapshot != *terminal_observed {
        errors.push(TraceValidationError::new(
            "$.final.final_observed_snapshot",
            "does not equal the terminal step surface",
        ));
    }
    if trace.r#final.final_action_context != *before_context {
        errors.push(TraceValidationError::new(
            "$.final.final_action_context",
            "does not equal the terminal step surface",
        ));
    }
    errors
}

fn validate_v2_surfaces(
    errors: &mut Vec<TraceValidationError>,
    path: &str,
    debug: &WorldSnapshotV1,
    observed: &WorldSnapshotV2,
    context: &PlayerActionContextV2,
) {
    let debug_name = if path.ends_with("header") {
        "initial_debug_snapshot"
    } else if path.ends_with("final") {
        "final_debug_snapshot"
    } else {
        "after_debug_snapshot"
    };
    let observed_name = if path.ends_with("header") {
        "initial_observed_snapshot"
    } else if path.ends_with("final") {
        "final_observed_snapshot"
    } else {
        "after_observed_snapshot"
    };
    let context_name = if path.ends_with("header") {
        "initial_action_context"
    } else if path.ends_with("final") {
        "final_action_context"
    } else {
        "after_action_context"
    };
    validate_debug_snapshot(errors, &format!("{path}.{debug_name}"), debug);
    check_version(
        errors,
        &format!("{path}.{observed_name}.contract_version"),
        observed.contract_version,
        OBSERVED_SNAPSHOT_CONTRACT_VERSION,
    );
    check_version(
        errors,
        &format!("{path}.{context_name}.contract_version"),
        context.contract_version,
        ACTION_CONTEXT_CONTRACT_VERSION,
    );
    validate_nested_command_versions(errors, &format!("{path}.{context_name}"), context);
    if debug.controlled_actor_ids.as_slice() != std::slice::from_ref(&context.actor_id) {
        errors.push(TraceValidationError::new(
            format!("{path}.{debug_name}.controlled_actor_ids"),
            "must contain exactly the action-context actor",
        ));
    }
    if observed.observer_actor_id != context.actor_id {
        errors.push(TraceValidationError::new(
            format!("{path}.{observed_name}.observer_actor_id"),
            "does not match the action-context actor",
        ));
    }
    if observed.logical_time != debug.logical_time || context.logical_time != debug.logical_time {
        errors.push(TraceValidationError::new(
            path,
            "debug, observed, and action-context logical times disagree",
        ));
    }
    if observed.observation_center != context.position {
        errors.push(TraceValidationError::new(
            format!("{path}.{observed_name}.observation_center"),
            "does not match the action-context position",
        ));
    }
}

fn validate_debug_snapshot(
    errors: &mut Vec<TraceValidationError>,
    path: &str,
    snapshot: &WorldSnapshotV1,
) {
    check_version(
        errors,
        &format!("{path}.contract_version"),
        snapshot.contract_version,
        SNAPSHOT_CONTRACT_VERSION,
    );
}

fn validate_command(errors: &mut Vec<TraceValidationError>, path: &str, command: &PlayerCommandV1) {
    check_version(
        errors,
        &format!("{path}.contract_version"),
        command.contract_version,
        COMMAND_CONTRACT_VERSION,
    );
    check_non_empty(
        errors,
        &format!("{path}.actor_id"),
        command.actor_id.as_str(),
    );
}

fn validate_nested_command_versions<T: serde::Serialize>(
    errors: &mut Vec<TraceValidationError>,
    path: &str,
    value: &T,
) {
    let value = serde_json::to_value(value).expect("action context must serialize");
    walk_nested_commands(errors, path, &value);
}

fn walk_nested_commands(errors: &mut Vec<TraceValidationError>, path: &str, value: &Value) {
    match value {
        Value::Object(object) => {
            if object.contains_key("actor_id")
                && object.contains_key("intent")
                && object.contains_key("contract_version")
                && object.get("contract_version").and_then(Value::as_u64)
                    != Some(u64::from(COMMAND_CONTRACT_VERSION))
            {
                errors.push(TraceValidationError::new(
                    format!("{path}.contract_version"),
                    format!("expected {COMMAND_CONTRACT_VERSION}"),
                ));
            }
            for (key, child) in object {
                walk_nested_commands(errors, &format!("{path}.{key}"), child);
            }
        }
        Value::Array(array) => {
            for (index, child) in array.iter().enumerate() {
                walk_nested_commands(errors, &format!("{path}[{index}]"), child);
            }
        }
        _ => {}
    }
}

fn validate_step_prelude(
    errors: &mut Vec<TraceValidationError>,
    path: &str,
    events: &[Event],
    expected_actor: Option<&ActorId>,
    intent_label: &str,
) {
    let (ready_actor, ready_time) = match events.first() {
        Some(Event::ActorReady {
            actor_id,
            kind: ActorKind::Player,
            logical_time,
            ..
        }) => (Some(actor_id), Some(*logical_time)),
        _ => {
            errors.push(TraceValidationError::new(
                format!("{path}[0]"),
                "must be the player ActorReady event",
            ));
            (None, None)
        }
    };
    let (intent_actor, intent_time, event_label) = match events.get(1) {
        Some(Event::PlayerIntent {
            actor_id,
            logical_time,
            intent,
            ..
        }) => (Some(actor_id), Some(*logical_time), Some(intent.as_str())),
        _ => {
            errors.push(TraceValidationError::new(
                format!("{path}[1]"),
                "must be the PlayerIntent event",
            ));
            (None, None, None)
        }
    };
    if ready_actor != expected_actor || intent_actor != expected_actor {
        errors.push(TraceValidationError::new(
            path,
            "player prelude does not name the addressed actor",
        ));
    }
    if ready_time != intent_time {
        errors.push(TraceValidationError::new(
            path,
            "player prelude logical times disagree",
        ));
    }
    if event_label != Some(intent_label) {
        errors.push(TraceValidationError::new(
            format!("{path}[1].player_intent.intent"),
            "does not match intent_label",
        ));
    }
}

fn validate_preview(
    errors: &mut Vec<TraceValidationError>,
    path: &str,
    preview: &PathPreviewV1,
    expected_actor: Option<&ActorId>,
    requested_path: Option<&[tme_rules::Direction]>,
) {
    check_version(
        errors,
        &format!("{path}.contract_version"),
        preview.contract_version,
        PATH_PREVIEW_CONTRACT_VERSION,
    );
    if expected_actor.is_some_and(|actor| actor != &preview.actor_id) {
        errors.push(TraceValidationError::new(
            format!("{path}.actor_id"),
            "does not match the addressed actor",
        ));
    }
    let count = preview.requested_path.len();
    if !(1..=tme_rules::MAX_CONTROLLED_PATH_STEPS).contains(&count) {
        errors.push(TraceValidationError::new(
            format!("{path}.requested_path"),
            "must contain one through three directions",
        ));
    }
    if requested_path.is_some_and(|requested| requested != preview.requested_path) {
        errors.push(TraceValidationError::new(
            format!("{path}.requested_path"),
            "does not match the command",
        ));
    }
    if MovementPace::from_step_count(count) != Some(preview.pace) {
        errors.push(TraceValidationError::new(
            format!("{path}.pace"),
            "does not match requested cardinality",
        ));
    }
    if preview.accepted_steps > count {
        errors.push(TraceValidationError::new(
            format!("{path}.accepted_steps"),
            "exceeds the requested path",
        ));
    }
    for (index, step) in preview.steps.iter().enumerate() {
        if step.index != index {
            errors.push(TraceValidationError::new(
                format!("{path}.steps[{index}].index"),
                format!("expected {index}, found {}", step.index),
            ));
        }
        if preview.requested_path.get(index) != Some(&step.direction) {
            errors.push(TraceValidationError::new(
                format!("{path}.steps[{index}].direction"),
                "does not match requested_path",
            ));
        }
    }
    match (
        preview.stamina_before,
        preview.stamina_cost,
        preview.stamina_after,
    ) {
        (None, None, None) => {}
        (Some(before), Some(cost), Some(after)) if before >= 0 && cost >= 0 && after >= 0 => {
            if before.saturating_sub(cost) != after {
                errors.push(TraceValidationError::new(
                    format!("{path}.stamina_after"),
                    "does not match the one-action stamina charge",
                ));
            }
        }
        (Some(_), Some(_), Some(_)) => errors.push(TraceValidationError::new(
            path,
            "stamina decision contains a negative value",
        )),
        _ => errors.push(TraceValidationError::new(
            path,
            "stamina decision is partial",
        )),
    }
}

fn single_controlled_actor(snapshot: &WorldSnapshotV1) -> Option<&ActorId> {
    (snapshot.controlled_actor_ids.len() == 1).then(|| &snapshot.controlled_actor_ids[0])
}

fn check_version(errors: &mut Vec<TraceValidationError>, path: &str, actual: u32, expected: u32) {
    if actual != expected {
        errors.push(TraceValidationError::new(
            path,
            format!("expected {expected}, found {actual}"),
        ));
    }
}

fn check_non_empty(errors: &mut Vec<TraceValidationError>, path: &str, value: &str) {
    if value.is_empty() {
        errors.push(TraceValidationError::new(path, "must be non-empty"));
    }
}

#[cfg(test)]
mod tests {
    use super::validate_trace_json;
    use serde_json::Value;

    const TRACE_V1: &str =
        include_str!("../tests/golden/trace_v1_creature_ecology_gallery_seed_7.json");
    const TRACE_V2: &str = include_str!("../tests/golden/trace_v2_first_room_seed_7.json");

    fn trace_v2() -> Value {
        serde_json::from_str(TRACE_V2).expect("tracked Trace V2 golden must parse")
    }

    fn rejected(mutant: Value, expected_fragment: &str) {
        let encoded = serde_json::to_string(&mutant).expect("mutant must encode");
        let errors = validate_trace_json(&encoded).expect_err("mutant must be rejected");
        assert!(
            errors
                .iter()
                .any(|error| error.to_string().contains(expected_fragment)),
            "expected an error containing {expected_fragment:?}, got {errors:#?}"
        );
    }

    #[test]
    fn tracked_v1_and_v2_goldens_are_consistent() {
        let v1 = validate_trace_json(TRACE_V1).expect("Trace V1 golden must validate");
        let v2 = validate_trace_json(TRACE_V2).expect("Trace V2 golden must validate");
        assert_eq!((v1.contract_version, v1.step_count), (1, 5));
        assert_eq!((v2.contract_version, v2.step_count), (2, 2));
    }

    #[test]
    fn unknown_and_noncanonical_fields_are_rejected() {
        let mut trace = trace_v2();
        trace["header"]["initial_action_context"]["active_effects"] = Value::Null;
        rejected(trace, "active_effects");

        let mut trace = trace_v2();
        trace["header"]["private_alias"] = Value::Bool(true);
        rejected(trace, "private_alias");
    }

    #[test]
    fn every_advertised_surface_version_is_current() {
        for pointer in [
            "/header/contract_version",
            "/header/event_contract_version",
            "/header/snapshot_contract_version",
            "/header/observed_snapshot_contract_version",
            "/header/action_context_contract_version",
            "/header/intent_contract_version",
            "/header/initial_debug_snapshot/contract_version",
            "/header/initial_observed_snapshot/contract_version",
            "/header/initial_action_context/contract_version",
            "/steps/0/command/contract_version",
            "/steps/0/preview/contract_version",
            "/steps/0/after_action_context/attack_targets/0/physical_attacks/0/command/contract_version",
            "/final/contract_version",
        ] {
            let mut trace = trace_v2();
            *trace
                .pointer_mut(pointer)
                .expect("version pointer must exist") = Value::from(0);
            rejected(trace, "version");
        }
    }

    #[test]
    fn step_order_intent_and_event_prelude_are_bound() {
        let mut trace = trace_v2();
        trace["steps"][0]["step_index"] = Value::from(3);
        rejected(trace, "step_index");

        let mut trace = trace_v2();
        trace["steps"][0]["intent_label"] = Value::String(String::new());
        rejected(trace, "must be non-empty");

        let mut trace = trace_v2();
        trace["steps"][0]["events"] = Value::Array(Vec::new());
        rejected(trace, "player prelude");
    }

    #[test]
    fn logical_time_cannot_regress_or_disagree_across_surfaces() {
        let mut trace = trace_v2();
        trace["steps"][0]["after_debug_snapshot"]["logical_time"] = Value::from(0);
        rejected(trace, "logical_time");

        let mut trace = trace_v2();
        trace["steps"][0]["after_observed_snapshot"]["logical_time"] = Value::from(999);
        rejected(trace, "logical times disagree");
    }

    #[test]
    fn observer_context_command_and_preview_identities_are_bound() {
        let mut trace = trace_v2();
        trace["steps"][0]["after_observed_snapshot"]["observer_actor_id"] =
            Value::String("someone_else".to_string());
        rejected(trace, "observer_actor_id");

        let mut trace = trace_v2();
        trace["steps"][0]["after_debug_snapshot"]["controlled_actor_ids"] =
            serde_json::json!(["someone_else"]);
        rejected(trace, "controlled_actor_ids");

        let mut trace = trace_v2();
        trace["steps"][0]["preview"]["actor_id"] = Value::String("someone_else".to_string());
        rejected(trace, "preview.actor_id");
    }

    #[test]
    fn move_path_preview_cardinality_steps_and_stamina_are_coherent() {
        let mut trace = trace_v2();
        trace["steps"][0]["preview"]["requested_path"] =
            serde_json::json!(["east", "east", "east", "east"]);
        rejected(trace, "one through three");

        let mut trace = trace_v2();
        trace["steps"][0]["preview"]["steps"][0]["index"] = Value::from(4);
        rejected(trace, "steps[0].index");

        let mut trace = trace_v2();
        trace["steps"][0]["preview"]["stamina_after"] = Value::from(99);
        rejected(trace, "stamina decision");
    }

    #[test]
    fn move_preview_and_final_surfaces_bind_exactly() {
        let mut trace = trace_v2();
        trace["steps"][0]["preview"] = Value::Null;
        rejected(trace, "missing for move_path");

        let mut trace = trace_v2();
        trace["steps"][0]["preview"]["final_position"]["position"]["x"] = Value::from(99);
        rejected(trace, "final_position");

        let mut trace = trace_v2();
        trace["final"]["final_debug_snapshot"]["logical_time"] = Value::from(99);
        rejected(trace, "terminal step surface");
    }
}

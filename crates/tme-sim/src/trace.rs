use tme_rules::{ActorId, Engine, PlayerIntent};

use crate::session::{self, IntentSource, SessionObserver, StepErrorPolicy, TranscriptWriter};

struct TraceV1PreparedStep {
    intent_label: String,
    preview: Option<tme_rules::PathPreviewV1>,
}

struct TraceV1Observer {
    steps: Vec<tme_rules::TraceStepV1>,
}

impl SessionObserver for TraceV1Observer {
    type PreparedStep = TraceV1PreparedStep;

    fn prepare_step(
        &mut self,
        engine: &Engine,
        actor_id: &ActorId,
        intent: &PlayerIntent,
    ) -> Result<Self::PreparedStep, String> {
        Ok(TraceV1PreparedStep {
            intent_label: intent.label(),
            preview: intent
                .movement_path()
                .and_then(|path| engine.preview_actor_path(actor_id, &path).ok()),
        })
    }

    fn step_committed<W: std::io::Write>(
        &mut self,
        engine: &Engine,
        _intent: &PlayerIntent,
        prepared: Self::PreparedStep,
        events: Vec<tme_rules::Event>,
        _transcript: &mut TranscriptWriter<W>,
    ) -> Result<(), String> {
        self.steps.push(tme_rules::TraceStepV1 {
            contract_version: tme_rules::TRACE_CONTRACT_VERSION,
            step_index: self.steps.len(),
            intent_label: prepared.intent_label,
            preview: prepared.preview,
            events,
            after_snapshot: engine.snapshot(),
        });
        Ok(())
    }
}

struct TraceV2PreparedStep {
    intent_label: String,
    command: tme_rules::PlayerCommandV1,
    preview: Option<tme_rules::PathPreviewV1>,
}

struct TraceV2Observer {
    steps: Vec<tme_rules::TraceStepV2>,
}

impl SessionObserver for TraceV2Observer {
    type PreparedStep = TraceV2PreparedStep;

    fn prepare_step(
        &mut self,
        engine: &Engine,
        actor_id: &ActorId,
        intent: &PlayerIntent,
    ) -> Result<Self::PreparedStep, String> {
        let command = engine
            .actor_command_for_intent(actor_id, intent)
            .map_err(|error| error.to_string())?;
        debug_assert_eq!(
            command.contract_version,
            tme_rules::COMMAND_CONTRACT_VERSION
        );
        Ok(TraceV2PreparedStep {
            intent_label: intent.label(),
            command,
            preview: intent
                .movement_path()
                .and_then(|path| engine.preview_actor_path(actor_id, &path).ok()),
        })
    }

    fn step_committed<W: std::io::Write>(
        &mut self,
        engine: &Engine,
        _intent: &PlayerIntent,
        prepared: Self::PreparedStep,
        events: Vec<tme_rules::Event>,
        _transcript: &mut TranscriptWriter<W>,
    ) -> Result<(), String> {
        let after_debug_snapshot = engine.snapshot();
        let after_frame = engine
            .actor_observed_frame(&prepared.command.actor_id)
            .map_err(|error| error.to_string())?;
        let after_observed_snapshot = after_frame.observed_snapshot;
        let after_action_context = after_frame.action_context;
        debug_assert_eq!(
            after_debug_snapshot.contract_version,
            tme_rules::SNAPSHOT_CONTRACT_VERSION
        );
        debug_assert_eq!(
            after_observed_snapshot.contract_version,
            tme_rules::OBSERVED_SNAPSHOT_CONTRACT_VERSION
        );
        debug_assert_eq!(
            after_action_context.contract_version,
            tme_rules::ACTION_CONTEXT_CONTRACT_VERSION
        );
        self.steps.push(tme_rules::TraceStepV2 {
            step_index: self.steps.len(),
            command: prepared.command,
            intent_label: prepared.intent_label,
            preview: prepared.preview,
            events,
            after_debug_snapshot,
            after_observed_snapshot,
            after_action_context,
        });
        Ok(())
    }
}

pub(crate) fn run_trace_json<S>(
    engine: Engine,
    scenario_id: String,
    seed: u64,
    mut source: S,
    mut step_error_policy: StepErrorPolicy,
) -> Result<tme_rules::TraceV1, String>
where
    S: IntentSource,
{
    session::scenario_player_actor_id(&engine)?;
    let initial_snapshot = engine.snapshot();
    let header = tme_rules::TraceHeaderV1 {
        contract_version: tme_rules::TRACE_CONTRACT_VERSION,
        scenario_id,
        seed,
        initial_snapshot,
    };

    // Trace mode discards rendered text; the IntentSource still requires a writer.
    let mut dummy_sink = Vec::new();
    let mut dummy_transcript = TranscriptWriter::new(&mut dummy_sink);
    let mut observer = TraceV1Observer { steps: Vec::new() };
    let engine = session::drive_session(
        engine,
        &mut source,
        &mut dummy_transcript,
        &mut step_error_policy,
        &mut observer,
    )?;

    let final_snapshot = engine.snapshot();
    let r#final = tme_rules::TraceFinalV1 {
        contract_version: tme_rules::TRACE_CONTRACT_VERSION,
        final_snapshot,
    };

    Ok(tme_rules::TraceV1 {
        header,
        steps: observer.steps,
        r#final,
    })
}

pub(crate) fn run_trace_json_v2<S>(
    engine: Engine,
    scenario_id: String,
    seed: u64,
    mut source: S,
    mut step_error_policy: StepErrorPolicy,
) -> Result<tme_rules::TraceV2, String>
where
    S: IntentSource,
{
    let actor_id = session::scenario_player_actor_id(&engine)?;
    let initial_debug_snapshot = engine.snapshot();
    let initial_frame = engine
        .actor_observed_frame(&actor_id)
        .map_err(|e| e.to_string())?;
    let initial_observed_snapshot = initial_frame.observed_snapshot;
    let initial_action_context = initial_frame.action_context;
    debug_assert_eq!(
        initial_debug_snapshot.contract_version,
        tme_rules::SNAPSHOT_CONTRACT_VERSION
    );
    debug_assert_eq!(
        initial_observed_snapshot.contract_version,
        tme_rules::OBSERVED_SNAPSHOT_CONTRACT_VERSION
    );
    debug_assert_eq!(
        initial_action_context.contract_version,
        tme_rules::ACTION_CONTEXT_CONTRACT_VERSION
    );

    let header = tme_rules::TraceHeaderV2 {
        contract_version: tme_rules::TRACE_V2_CONTRACT_VERSION,
        scenario_id,
        seed,
        event_contract_version: tme_rules::EVENT_CONTRACT_VERSION,
        snapshot_contract_version: tme_rules::SNAPSHOT_CONTRACT_VERSION,
        observed_snapshot_contract_version: tme_rules::OBSERVED_SNAPSHOT_CONTRACT_VERSION,
        action_context_contract_version: tme_rules::ACTION_CONTEXT_CONTRACT_VERSION,
        intent_contract_version: tme_rules::COMMAND_CONTRACT_VERSION,
        initial_debug_snapshot,
        initial_observed_snapshot,
        initial_action_context,
    };

    let mut dummy_sink = Vec::new();
    let mut dummy_transcript = TranscriptWriter::new(&mut dummy_sink);
    let mut observer = TraceV2Observer { steps: Vec::new() };
    let engine = session::drive_session(
        engine,
        &mut source,
        &mut dummy_transcript,
        &mut step_error_policy,
        &mut observer,
    )?;

    let final_debug_snapshot = engine.snapshot();
    let final_frame = engine
        .actor_observed_frame(&actor_id)
        .map_err(|e| e.to_string())?;
    let final_observed_snapshot = final_frame.observed_snapshot;
    let final_action_context = final_frame.action_context;
    debug_assert_eq!(
        final_debug_snapshot.contract_version,
        tme_rules::SNAPSHOT_CONTRACT_VERSION
    );
    debug_assert_eq!(
        final_observed_snapshot.contract_version,
        tme_rules::OBSERVED_SNAPSHOT_CONTRACT_VERSION
    );
    debug_assert_eq!(
        final_action_context.contract_version,
        tme_rules::ACTION_CONTEXT_CONTRACT_VERSION
    );

    let r#final = tme_rules::TraceFinalV2 {
        contract_version: tme_rules::TRACE_V2_CONTRACT_VERSION,
        final_debug_snapshot,
        final_observed_snapshot,
        final_action_context,
    };

    Ok(tme_rules::TraceV2 {
        header,
        steps: observer.steps,
        r#final,
    })
}

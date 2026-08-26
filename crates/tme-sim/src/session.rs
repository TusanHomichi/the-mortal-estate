use std::io::{self, Write};

use tme_rules::{ActorId, ActorKind, Engine, PlayerIntent, ValidatedWorldSeed};

pub(crate) fn scenario_player_actor_id(engine: &Engine) -> Result<ActorId, String> {
    let controlled = engine
        .world()
        .actors
        .iter()
        .filter(|actor| actor.kind == ActorKind::Player)
        .map(|actor| actor.id.clone())
        .collect::<Vec<_>>();
    match controlled.as_slice() {
        [actor_id] => Ok(actor_id.clone()),
        _ => Err(format!(
            "Simulation Seed 3 requires exactly one player actor, found {}",
            controlled.len()
        )),
    }
}

pub(crate) fn step_scenario_player(
    engine: &mut Engine,
    actor_id: &ActorId,
    intent: PlayerIntent,
) -> Result<Vec<tme_rules::Event>, tme_rules::StepError> {
    engine
        .apply_actor_intent(actor_id, intent)
        .map(|outcome| outcome.events)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum IntentAction {
    Step(PlayerIntent),
    Continue,
    Stop,
}

pub(crate) trait IntentSource {
    fn next_intent<W: Write>(
        &mut self,
        engine: &Engine,
        transcript: &mut TranscriptWriter<W>,
    ) -> Result<IntentAction, String>;
}

pub(crate) trait SessionObserver {
    type PreparedStep;

    fn prepare_step(
        &mut self,
        engine: &Engine,
        actor_id: &ActorId,
        intent: &PlayerIntent,
    ) -> Result<Self::PreparedStep, String>;

    fn step_committed<W: Write>(
        &mut self,
        engine: &Engine,
        intent: &PlayerIntent,
        prepared: Self::PreparedStep,
        events: Vec<tme_rules::Event>,
        transcript: &mut TranscriptWriter<W>,
    ) -> Result<(), String>;

    fn step_rejected<W: Write>(
        &mut self,
        _error: &tme_rules::StepError,
        _transcript: &mut TranscriptWriter<W>,
    ) -> Result<(), String> {
        Ok(())
    }
}

pub(crate) enum StepErrorPolicy {
    Fatal,
    RePrompt {
        world_seed: ValidatedWorldSeed,
        seed: u64,
        accepted_intents: Vec<PlayerIntent>,
    },
}

impl StepErrorPolicy {
    pub(crate) fn record_success(&mut self, intent: PlayerIntent) {
        if let Self::RePrompt {
            accepted_intents, ..
        } = self
        {
            accepted_intents.push(intent);
        }
    }

    pub(crate) fn rebuild_engine(&self, actor_id: &ActorId) -> Result<Option<Engine>, String> {
        let Self::RePrompt {
            world_seed,
            seed,
            accepted_intents,
        } = self
        else {
            return Ok(None);
        };

        let mut engine =
            Engine::new(world_seed.clone(), *seed).map_err(|error| error.to_string())?;
        for intent in accepted_intents {
            step_scenario_player(&mut engine, actor_id, intent.clone())
                .map_err(|error| format!("failed to replay accepted intent: {error}"))?;
        }
        Ok(Some(engine))
    }
}

pub(crate) struct TranscriptWriter<W: Write> {
    sink: W,
    wrote_block: bool,
}

impl<W: Write> TranscriptWriter<W> {
    pub(crate) fn new(sink: W) -> Self {
        Self {
            sink,
            wrote_block: false,
        }
    }

    pub(crate) fn write_block(&mut self, lines: Vec<String>) -> io::Result<()> {
        if self.wrote_block {
            writeln!(self.sink)?;
        }
        for line in lines {
            writeln!(self.sink, "{line}")?;
        }
        self.wrote_block = true;
        Ok(())
    }

    pub(crate) fn write_raw(&mut self, text: &str) -> io::Result<()> {
        self.sink.write_all(text.as_bytes())
    }

    pub(crate) fn flush(&mut self) -> io::Result<()> {
        self.sink.flush()
    }

    pub(crate) fn into_inner(self) -> W {
        self.sink
    }
}

struct TranscriptSessionObserver;

impl SessionObserver for TranscriptSessionObserver {
    type PreparedStep = ();

    fn prepare_step(
        &mut self,
        _engine: &Engine,
        _actor_id: &ActorId,
        _intent: &PlayerIntent,
    ) -> Result<Self::PreparedStep, String> {
        Ok(())
    }

    fn step_committed<W: Write>(
        &mut self,
        _engine: &Engine,
        _intent: &PlayerIntent,
        (): Self::PreparedStep,
        events: Vec<tme_rules::Event>,
        transcript: &mut TranscriptWriter<W>,
    ) -> Result<(), String> {
        transcript
            .write_block(crate::render_events(&events))
            .map_err(|error| error.to_string())
    }

    fn step_rejected<W: Write>(
        &mut self,
        error: &tme_rules::StepError,
        transcript: &mut TranscriptWriter<W>,
    ) -> Result<(), String> {
        transcript
            .write_raw(&format!("error: {error}\n"))
            .map_err(|write_error| write_error.to_string())
    }
}

pub(crate) fn drive_session<S, W, O>(
    mut engine: Engine,
    source: &mut S,
    transcript: &mut TranscriptWriter<W>,
    step_error_policy: &mut StepErrorPolicy,
    observer: &mut O,
) -> Result<Engine, String>
where
    S: IntentSource,
    W: Write,
    O: SessionObserver,
{
    let actor_id = scenario_player_actor_id(&engine)?;
    while engine
        .world()
        .actors
        .iter()
        .find(|actor| actor.id == actor_id)
        .is_some_and(|actor| actor.is_alive())
    {
        match source.next_intent(&engine, transcript)? {
            IntentAction::Step(intent) => {
                let prepared = observer.prepare_step(&engine, &actor_id, &intent)?;
                match step_scenario_player(&mut engine, &actor_id, intent.clone()) {
                    Ok(events) => {
                        observer.step_committed(&engine, &intent, prepared, events, transcript)?;
                        step_error_policy.record_success(intent);
                    }
                    Err(error) => match step_error_policy.rebuild_engine(&actor_id)? {
                        Some(rebuilt) => {
                            observer.step_rejected(&error, transcript)?;
                            engine = rebuilt;
                        }
                        None => return Err(error.to_string()),
                    },
                }
            }
            IntentAction::Continue => {}
            IntentAction::Stop => break,
        }
    }
    Ok(engine)
}

pub(crate) struct SessionHeader {
    pub(crate) scenario_id: String,
    pub(crate) seed: u64,
    pub(crate) scenario_loaded_event: tme_rules::Event,
    pub(crate) mode: Option<&'static str>,
}

pub(crate) fn run_simulation_loop<S, W>(
    engine: Engine,
    header: SessionHeader,
    mut source: S,
    sink: W,
    mut step_error_policy: StepErrorPolicy,
) -> Result<W, String>
where
    S: IntentSource,
    W: Write,
{
    let mut transcript = TranscriptWriter::new(sink);
    let mut header_lines = vec![
        "The Mortal Estate local simulation".to_string(),
        format!("scenario: {}", header.scenario_id),
        format!("seed: {}", header.seed),
    ];
    if let Some(mode) = header.mode {
        header_lines.push(format!("mode: {mode}"));
    }
    transcript
        .write_block(header_lines)
        .map_err(|error| error.to_string())?;
    let mut initial_events = vec![header.scenario_loaded_event];
    initial_events.extend(engine.initial_events());
    transcript
        .write_block(crate::render_events(&initial_events))
        .map_err(|error| error.to_string())?;

    let mut observer = TranscriptSessionObserver;
    let engine = drive_session(
        engine,
        &mut source,
        &mut transcript,
        &mut step_error_policy,
        &mut observer,
    )?;

    transcript
        .write_block(crate::render_events(&engine.final_events()))
        .map_err(|error| error.to_string())?;
    Ok(transcript.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tme_rules::{Direction, PlayerIntent};

    use crate::fixture::script::{ScriptPhysicalAttackStep, ScriptStep, ScriptedIntentSource};

    fn empty_step() -> ScriptStep {
        ScriptStep {
            move_path: None,
            traverse: None,
            hide: None,
            hide_field_present: false,
            nock: None,
            unload_bow: None,
            physical_attack: None,
            search_corpse: None,
            move_item: None,
            move_gold: None,
            deposit_bank_gold: None,
            withdraw_bank_gold: None,
            deposit_locker_item: None,
            withdraw_locker_item: None,
            drink: None,
            open: None,
            close: None,
            show_sack: None,
            wait: None,
            inspect: None,
            train: None,
            critique: None,
            promote: None,
            learn_spell: None,
            commit_service_transaction: None,
            buy_from_merchant: None,
            sell_to_merchant: None,
            use_item_service: None,
            use_restoration_service: None,
            interact_with_npc: None,
            cast_spell: None,
            warm_spell: None,
            cast_warmed_spell: None,
            fizzle_warmed_spell: None,
            rest: None,
        }
    }

    #[test]
    fn scripted_intent_source_yields_script_steps_then_stops() {
        let mut move_step = empty_step();
        move_step.move_path = Some(vec![Direction::East]);
        let mut attack_step = empty_step();
        attack_step.physical_attack = Some(ScriptPhysicalAttackStep {
            mode: tme_rules::PhysicalAttackMode::Fight,
            target_actor_id: "mireling".into(),
            authorization: tme_rules::HostilityAuthorization::Safe,
        });

        let mut source = ScriptedIntentSource::new(vec![move_step, attack_step]);
        let mut transcript = TranscriptWriter::new(Vec::new());
        let loaded = crate::loading::load_simulation(
            &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../content/test-corpus/first_room.json"),
        )
        .expect("fixture loads");
        let engine = Engine::new(loaded.world_seed, 7).expect("engine starts");

        assert_eq!(
            source
                .next_intent(&engine, &mut transcript)
                .expect("first intent"),
            IntentAction::Step(PlayerIntent::MovePath(vec![Direction::East]))
        );
        assert_eq!(
            source
                .next_intent(&engine, &mut transcript)
                .expect("second intent"),
            IntentAction::Step(PlayerIntent::PhysicalAttack {
                mode: tme_rules::PhysicalAttackMode::Fight,
                target_actor_id: "mireling".into(),
                authorization: tme_rules::HostilityAuthorization::Safe,
            })
        );
        assert_eq!(
            source
                .next_intent(&engine, &mut transcript)
                .expect("script end"),
            IntentAction::Stop
        );
    }
}

//! Recovery preserves the one corpse-bound location and the original death deadline.
use super::*;

pub(super) fn validate(engine: &Engine) -> Result<(), CheckpointError> {
    for actor in &engine.world.actors {
        let ActorLifeState::Ghost {
            corpse_id,
            defeated_at,
        } = &actor.life_state
        else {
            continue;
        };
        let corpse = engine
            .world
            .corpses
            .get(corpse_id)
            .ok_or_else(|| CheckpointError::new("checkpoint ghost corpse is missing"))?;
        if actor.kind != ActorKind::Player
            || corpse.origin_actor_id != actor.id
            || corpse.origin_kind != ActorKind::Player
            || corpse.origin_character_id != actor.character_id
            || corpse.location != actor.location
            || corpse.created_at != *defeated_at
            || *defeated_at > engine.world.timing.now
        {
            return Err(CheckpointError::new(
                "checkpoint ghost differs from its corpse or death time",
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ghost() -> Engine {
        let mut engine = crate::engine::setup::test_engine("death_corpse");
        let player = ActorId::from("player");
        for target in ["scavenger", "lookout"] {
            engine
                .apply_actor_intent(
                    &player,
                    PlayerIntent::PhysicalAttack {
                        mode: PhysicalAttackMode::Fight,
                        target_actor_id: target.into(),
                        authorization: HostilityAuthorization::Safe,
                    },
                )
                .unwrap();
        }
        for id in [2, 1] {
            engine
                .apply_actor_intent(
                    &player,
                    PlayerIntent::SearchCorpse(CorpseId::from_sequence(id)),
                )
                .unwrap();
        }
        engine
            .apply_actor_intent(&player, PlayerIntent::Wait)
            .unwrap();
        assert!(matches!(
            engine.world.actors[0].life_state,
            ActorLifeState::Ghost { .. }
        ));
        engine
    }

    #[test]
    fn recovery_refuses_displaced_missing_or_mistimed_ghost_corpses() {
        let engine = ghost();
        let recover = |engine: &Engine| {
            Engine::hydrate_checkpoint(
                engine.definition().clone(),
                &engine.export_checkpoint().unwrap(),
            )
        };
        assert!(recover(&engine).is_ok());
        let mut displaced = engine.clone();
        displaced.world.actors[0].location.position.x += 1;
        assert!(
            recover(&displaced)
                .err()
                .unwrap()
                .to_string()
                .contains("checkpoint ghost")
        );
        let mut missing = engine.clone();
        missing.world.corpses.remove(&CorpseId::from_sequence(3));
        assert!(recover(&missing).is_err());
        let mut mistimed = engine.clone();
        if let ActorLifeState::Ghost { defeated_at, .. } = &mut mistimed.world.actors[0].life_state
        {
            *defeated_at = defeated_at.saturating_add_millis(1);
        }
        assert!(
            recover(&mistimed)
                .err()
                .unwrap()
                .to_string()
                .contains("checkpoint ghost")
        );
    }
}

use crate::model::{ActorId, PlayerIntent};

#[test]
fn failed_change_materialization_rolls_back_payment_training_xp_and_rng() {
    let mut engine = crate::engine::setup::test_engine("gold_training");
    engine.world.next_gold_sequence = u64::MAX;
    let before = engine.world.clone();
    let before_rng = engine.rng.clone();
    let error = engine
        .apply_actor_intent(
            &ActorId::from("player"),
            PlayerIntent::Train {
                service_id: "sword_trainer".to_string(),
                offered_gold: 40,
            },
        )
        .expect_err("the change pile cannot allocate a sequence");
    assert!(error.message().contains("ground gold sequence overflow"));
    assert_eq!(engine.world, before);
    assert_eq!(engine.rng, before_rng);
}

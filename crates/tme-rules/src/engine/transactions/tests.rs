use super::*;

#[test]
fn returned_gold_is_coordinated_with_payment_and_cannot_move_to_another_square() {
    let engine = crate::engine::setup::test_engine("gold_training");
    let before = engine.world.clone();
    let here = engine.world.actors[0].location.clone();
    let mut elsewhere = here.clone();
    elsewhere.position.x += 1;
    let transaction = Transaction {
        id: "return_test".into(),
        label: "Return test".into(),
        requirements: vec![],
        costs: vec![TransactionCost::CarriedGold { amount: 10 }],
        rewards: vec![],
    };
    for (amounts, location) in [
        (vec![11], here.clone()),
        (vec![6, 5], here.clone()),
        (vec![0], here.clone()),
        (vec![-1], here.clone()),
        (vec![1], elsewhere),
    ] {
        let result = engine.plan_transaction(
            0,
            TransactionSource::SkillTraining {
                service_id: "sword_trainer".into(),
                capability_id: "training".into(),
                track_id: "sword".into(),
            },
            &transaction,
            None,
            amounts
                .into_iter()
                .map(|amount| PlannedReward::ReturnedGold {
                    amount,
                    location: location.clone(),
                })
                .collect(),
        );
        assert_eq!(
            result
                .expect_err("invalid return must fail preflight")
                .reason(),
            ActionBlockedReasonV1::InvalidGoldAmount
        );
        assert_eq!(engine.world, before);
    }
}

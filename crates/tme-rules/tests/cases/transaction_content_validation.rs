use crate::support::content_parts::ContentParts;

fn value() -> ContentParts {
    ContentParts::tracked("service_transactions", "profile/service_transactions")
}

fn error(mutator: impl FnOnce(&mut ContentParts)) -> String {
    let mut fixture = value();
    mutator(&mut fixture);
    match fixture.validated_seed() {
        Ok(_) => panic!("mutated transaction must fail"),
        Err(error) => error,
    }
}

#[test]
fn transaction_definition_is_strict_and_reusable() {
    value()
        .validated_seed()
        .expect("transaction fixture validates");
    let unknown = error(|fixture| {
        fixture.selected_mut("service_definitions", 0)["capabilities"][0]["transactions"][0]["mystery"] =
            serde_json::json!(true);
    });
    assert!(unknown.contains("unknown field `mystery`"));
    let illegal = error(|fixture| {
        fixture.selected_mut("service_definitions", 0)["capabilities"][0]["transactions"][0]["rewards"]
            [0] = serde_json::json!({
            "kind": "class",
            "to_class_id": "knight",
            "to_class_display": "Knight"
        });
    });
    assert!(illegal.contains("class is legal only for class_promotion"));
    let duplicate = error(|fixture| {
        let transaction =
            fixture.selected_mut("service_definitions", 0)["capabilities"][0]["transactions"][0]
                .clone();
        fixture.selected_mut("service_definitions", 0)["capabilities"][0]["transactions"]
            .as_array_mut()
            .expect("transactions")
            .push(transaction);
    });
    assert!(
        duplicate.contains("transactions[1].id duplicates"),
        "{duplicate}"
    );
}

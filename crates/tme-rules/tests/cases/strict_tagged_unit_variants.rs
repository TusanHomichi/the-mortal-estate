use serde::de::DeserializeOwned;
use tme_rules::content::RestorationOutcomeDef;
use tme_rules::{
    GoldMoveDestination, GoldMoveQuantity, ItemBindingState, ItemMoveDestination,
    SpellResistanceMitigation,
};

fn assert_unknown_field_rejected<T: DeserializeOwned + std::fmt::Debug>(json: &str) {
    let error =
        serde_json::from_str::<T>(json).expect_err("unknown sibling field must be rejected");
    assert!(
        error.to_string().contains("unknown field `legacy`"),
        "unexpected strict-shape diagnostic: {error}"
    );
}

#[test]
fn catalog_tagged_unit_variants_reject_unknown_sibling_fields() {
    assert_unknown_field_rejected::<RestorationOutcomeDef>(
        r#"{"kind":"priest_resurrection","legacy":true}"#,
    );
    assert_unknown_field_rejected::<SpellResistanceMitigation>(
        r#"{"mode":"negate","legacy":true}"#,
    );
}

#[test]
fn simulation_script_tagged_unit_variants_reject_unknown_sibling_fields() {
    assert_unknown_field_rejected::<ItemMoveDestination>(r#"{"kind":"ground_here","legacy":true}"#);
    assert_unknown_field_rejected::<GoldMoveDestination>(r#"{"kind":"ground_here","legacy":true}"#);
    assert_unknown_field_rejected::<GoldMoveQuantity>(r#"{"kind":"all","legacy":true}"#);
}

#[test]
fn simulation_seed_binding_unit_variants_reject_unknown_sibling_fields() {
    assert_unknown_field_rejected::<ItemBindingState>(r#"{"state":"unrestricted","legacy":true}"#);
    assert_unknown_field_rejected::<ItemBindingState>(
        r#"{"state":"bind_on_first_character_touch","legacy":true}"#,
    );
}

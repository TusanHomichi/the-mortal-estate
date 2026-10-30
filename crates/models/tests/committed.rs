//! The generated models in the repository are the ones the code makes.

use std::path::Path;

#[test]
fn the_committed_models_are_what_the_code_makes() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/models/generated");
    for (name, text) in tme_models::all() {
        let committed = std::fs::read_to_string(dir.join(name)).unwrap_or_default();
        assert!(
            committed == text,
            "assets/models/generated/{name} isn't what the code makes: run `cargo run -p tme-models`"
        );
    }
}

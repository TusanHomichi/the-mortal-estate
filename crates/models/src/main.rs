//! `cargo run -p tme-models`: rewrite `assets/models/generated/` from the code.

use std::path::Path;

fn main() -> std::io::Result<()> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/models/generated");
    std::fs::create_dir_all(&dir)?;
    for (name, text) in tme_models::all() {
        std::fs::write(dir.join(name), text)?;
        println!("wrote assets/models/generated/{name}");
    }
    Ok(())
}

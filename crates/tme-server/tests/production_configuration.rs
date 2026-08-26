//! What the production runtime refuses to start without.
//!
//! The server serves exactly one world and is told which one by
//! `TME_BOOTSTRAP_MANIFEST`. There is no default manifest, no built-in land,
//! and no fallback to whatever content the tree happens to carry — so the
//! proof of that is a real process, started without the variable, failing and
//! saying which variable it wanted.
//!
//! This needs no database and no display: the manifest is read before any
//! credential, and a missing variable is refused before any of it.

use std::process::Command;

fn serve_without(variable: &str) -> String {
    let mut command = Command::new(env!("CARGO_BIN_EXE_tme-server"));
    command
        .arg("serve")
        .env("TME_PUBLIC_LISTEN", "127.0.0.1:0")
        .env("TME_OPS_LISTEN", "127.0.0.1:1")
        .env("TME_PUBLIC_HOST", "127.0.0.1")
        .env("TME_PUBLIC_ORIGIN", "http://127.0.0.1")
        .env("TME_BOOTSTRAP_MANIFEST", "/nonexistent/bootstrap.json")
        .env_remove(variable);
    let output = command.output().expect("the server binary runs");
    assert!(
        !output.status.success(),
        "the server started without {variable}"
    );
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn the_production_runtime_refuses_to_serve_without_a_bootstrap_manifest() {
    let stderr = serve_without("TME_BOOTSTRAP_MANIFEST");
    assert!(
        stderr.contains("TME_BOOTSTRAP_MANIFEST is required"),
        "{stderr}"
    );
}

#[test]
fn a_bootstrap_manifest_that_is_not_there_is_refused_rather_than_replaced() {
    let mut command = Command::new(env!("CARGO_BIN_EXE_tme-server"));
    let output = command
        .arg("serve")
        .env("TME_PUBLIC_LISTEN", "127.0.0.1:0")
        .env("TME_OPS_LISTEN", "127.0.0.1:1")
        .env("TME_PUBLIC_HOST", "127.0.0.1")
        .env("TME_PUBLIC_ORIGIN", "http://127.0.0.1")
        .env("TME_BOOTSTRAP_MANIFEST", "/nonexistent/bootstrap.json")
        .output()
        .expect("the server binary runs");
    assert!(!output.status.success(), "the server started with no world");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("production input is unavailable"),
        "{stderr}"
    );
}

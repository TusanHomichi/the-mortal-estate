//! What `tme-server` prints: the invite code only when the host asks for
//! the invite, never in what a service manager keeps as logs.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn server() -> Command {
    Command::new(env!("CARGO_BIN_EXE_tme-server"))
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tme-cli-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn printed(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

/// Make a world in `dir`; what was printed, and its invite code.
fn init(dir: &Path) -> (String, String) {
    let content = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
    let out = server()
        .arg("init")
        .arg(dir)
        .args(["--name", "Lattermead", "--bind", "127.0.0.1:0"])
        .arg("--content")
        .arg(content)
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", printed(&out));
    let config: toml::Table = std::fs::read_to_string(dir.join("world.toml"))
        .unwrap()
        .parse()
        .expect("world.toml is TOML any tool can read");
    (printed(&out), config["invite"].as_str().unwrap().to_owned())
}

#[test]
fn the_invite_code_is_printed_only_when_asked_for() {
    let dir = scratch("quiet");
    let (made, code) = init(&dir);
    assert!(!made.contains(&code), "init printed it: {made}");

    // Running: everything it prints until it's listening.
    let mut run = server()
        .arg("run")
        .arg(&dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut said = String::new();
    for line in BufReader::new(run.stdout.take().unwrap()).lines() {
        let line = line.unwrap();
        said.push_str(&line);
        if line.starts_with("listening") {
            break;
        }
    }
    run.kill().unwrap();
    run.wait().unwrap();
    assert!(!said.contains(&code), "run printed it: {said}");

    let asked = server().arg("invite").arg(&dir).output().unwrap();
    assert!(printed(&asked).contains(&code), "invite prints it");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_damaged_world_toml_is_reported_without_its_invite_code() {
    let dir = scratch("damaged");
    let (_, code) = init(&dir);
    let path = dir.join("world.toml");
    let text = std::fs::read_to_string(&path).unwrap();
    let broken = text.replace(&format!("\"{code}\""), &format!("\"{code}"));
    std::fs::write(&path, broken).unwrap();
    let out = server().arg("run").arg(&dir).output().unwrap();
    assert!(!out.status.success());
    let said = printed(&out);
    assert!(said.contains("world.toml"), "says where: {said}");
    assert!(!said.contains(&code), "the error quoted it: {said}");
    std::fs::remove_dir_all(&dir).unwrap();
}

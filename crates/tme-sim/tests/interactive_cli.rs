use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

#[test]
fn help_lists_interactive_flag() {
    let output = Command::new(env!("CARGO_BIN_EXE_tme-sim"))
        .arg("--help")
        .output()
        .expect("binary should run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("usage: tme-sim [--scenario <path>] [--seed <u64>] [--interactive]"));
    assert!(stdout.contains("--interactive"));
}

#[test]
fn interactive_flag_streams_prompted_session_from_stdin() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_tme-sim"))
        .arg("--scenario")
        .arg(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../content/test-corpus/first_room.json"),
        )
        .arg("--seed")
        .arg("7")
        .arg("--interactive")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("binary should spawn");

    child
        .stdin
        .as_mut()
        .expect("stdin should be piped")
        .write_all(b"quit\n")
        .expect("stdin write should succeed");

    let output = child.wait_with_output().expect("binary should finish");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("mode: interactive\n"));
    assert!(stdout.contains("> quit\n\nfinal state\n"));
    assert!(stdout.contains("Mireling at realm_0/room_0:3,1 hp=7 alive\n"));
}

use std::process::Command;

fn coherence() -> Command {
    Command::new(env!("CARGO_BIN_EXE_coherence"))
}

#[test]
fn help_lists_the_new_surface() {
    let output = coherence()
        .arg("help")
        .output()
        .expect("run coherence help");
    assert!(output.status.success());

    let text = String::from_utf8_lossy(&output.stdout);
    for command in ["init", "db", "tui", "verify", "author", "explain", "code"] {
        assert!(text.contains(command), "help is missing {command}");
    }
}

#[test]
fn unknown_command_exits_with_usage_error() {
    let status = coherence()
        .arg("not-a-command")
        .output()
        .expect("run coherence")
        .status;
    assert_eq!(status.code(), Some(2));
}

#[test]
fn legacy_version_still_works() {
    let output = coherence()
        .arg("version")
        .output()
        .expect("run coherence version");
    assert!(output.status.success());
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("coherence"));
}

#[test]
fn legacy_command_still_dispatches() {
    // Old commands keep their exit codes until the removal release.
    let status = coherence()
        .arg("ac-tests")
        .output()
        .expect("run coherence ac-tests")
        .status;
    assert_eq!(status.code(), Some(1));
}

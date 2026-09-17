use std::process::Command;

#[test]
fn administration_commands_do_not_simulate_live_trading_or_expose_operations() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("private/journal.sqlite");
    let run = |command: &str| {
        Command::new(env!("CARGO_BIN_EXE_cinder-operator"))
            .arg(command)
            .arg(&path)
            .output()
            .unwrap()
    };
    assert!(!run("status").status.success());
    assert!(!path.exists());
    assert!(!run("run").status.success());
    assert!(!path.exists());
    assert!(run("init").status.success());
    let output = run("status");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains(&format!(
        "Journal schema: {}",
        cinder_operator::SCHEMA_VERSION
    )));
    assert!(stdout.contains("Live trading: not implemented"));
    assert!(!stdout.contains("user_pubkey"));
    assert!(!stdout.contains("client_oid"));
}

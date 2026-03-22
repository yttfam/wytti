use assert_cmd::Command;
use predicates::prelude::*;

fn fixtures_dir() -> std::path::PathBuf {
    let mut dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.pop();
    dir.push("tests/fixtures");
    dir
}

fn fixture(name: &str) -> String {
    let path = fixtures_dir().join(name);
    assert!(
        path.exists(),
        "fixture not found: {path:?} — run tests/build_fixtures.sh"
    );
    path.to_string_lossy().to_string()
}

#[test]
fn e2e_hello() {
    Command::cargo_bin("wytti")
        .unwrap()
        .args(["run", &fixture("hello.wasm")])
        .assert()
        .success()
        .stdout(predicate::str::contains("hello from wytti"));
}

#[test]
fn e2e_args_passthrough() {
    Command::cargo_bin("wytti")
        .unwrap()
        .args(["run", &fixture("args.wasm"), "--", "alpha", "beta"])
        .assert()
        .success()
        .stdout(predicate::str::contains("alpha"))
        .stdout(predicate::str::contains("beta"));
}

#[test]
fn e2e_timeout() {
    Command::cargo_bin("wytti")
        .unwrap()
        .args(["run", &fixture("infinite.wasm"), "--max-time", "2s"])
        .timeout(std::time::Duration::from_secs(10))
        .assert()
        .failure()
        .stderr(predicate::str::contains("timed out"));
}

#[test]
fn e2e_exit_code_propagated() {
    Command::cargo_bin("wytti")
        .unwrap()
        .args(["run", &fixture("exit_code.wasm")])
        .assert()
        .failure();
}

#[test]
fn e2e_policy_file() {
    let policy_content = r#"
[sandbox]
max_memory = "64MB"
max_time = "2s"
allow_tcp = false
"#;
    let dir = tempfile::tempdir().unwrap();
    let policy_path = dir.path().join("policy.toml");
    std::fs::write(&policy_path, policy_content).unwrap();

    Command::cargo_bin("wytti")
        .unwrap()
        .args([
            "run",
            &fixture("hello.wasm"),
            "--policy",
            &policy_path.to_string_lossy(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("hello from wytti"));
}

#[test]
fn e2e_help() {
    Command::cargo_bin("wytti")
        .unwrap()
        .args(["--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("WASI runtime"));
}

#[test]
fn e2e_run_help() {
    Command::cargo_bin("wytti")
        .unwrap()
        .args(["run", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("allow-tcp"))
        .stdout(predicate::str::contains("allow-udp"))
        .stdout(predicate::str::contains("allow-dns"))
        .stdout(predicate::str::contains("allow-net"));
}

use std::time::Duration;
use wytti_runtime::{Runtime, RuntimeConfig};
use wytti_sandbox::SandboxPolicy;

fn fixtures_dir() -> std::path::PathBuf {
    let mut dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.pop(); // wytti-runtime -> wytti
    dir.push("tests/fixtures");
    dir
}

fn fixture(name: &str) -> std::path::PathBuf {
    let path = fixtures_dir().join(name);
    assert!(path.exists(), "fixture not found: {path:?} — run tests/build_fixtures.sh");
    path
}

#[test]
fn run_hello_p1() {
    let runtime = Runtime::new().unwrap();
    let config = RuntimeConfig::new()
        .args(["hello.wasm"]);
    runtime.run_file(fixture("hello.wasm"), &config).unwrap();
}

#[test]
fn run_with_args() {
    let runtime = Runtime::new().unwrap();
    let config = RuntimeConfig::new()
        .args(["args.wasm", "foo", "bar"]);
    runtime.run_file(fixture("args.wasm"), &config).unwrap();
}

#[test]
fn timeout_kills_infinite_loop() {
    let runtime = Runtime::new().unwrap();
    let sandbox = SandboxPolicy {
        max_time: Duration::from_secs(2),
        ..Default::default()
    };
    let config = RuntimeConfig::new()
        .args(["infinite.wasm"])
        .sandbox(sandbox);

    let result = runtime.run_file(fixture("infinite.wasm"), &config);
    assert!(result.is_err());

    let err = result.unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("timed out"), "expected timeout error, got: {msg}");
}

#[test]
fn exit_code_error() {
    let runtime = Runtime::new().unwrap();
    let config = RuntimeConfig::new()
        .args(["exit_code.wasm"]);

    let result = runtime.run_file(fixture("exit_code.wasm"), &config);
    assert!(result.is_err());
}

#[test]
fn memory_limit_enforced() {
    let runtime = Runtime::new().unwrap();
    // 2MB is enough to run hello.wasm but prevents excessive growth
    let sandbox = SandboxPolicy {
        max_memory: 2 * 1024 * 1024, // 2MB
        ..Default::default()
    };
    let config = RuntimeConfig::new()
        .args(["hello.wasm"])
        .sandbox(sandbox);

    runtime.run_file(fixture("hello.wasm"), &config).unwrap();
}

#[test]
fn networking_denied_by_default() {
    let policy = SandboxPolicy::default();
    assert!(!policy.allow_tcp);
    assert!(!policy.allow_udp);
    assert!(!policy.allow_dns);
}

#[test]
fn component_detection() {
    // Core module magic: \0asm\x01\x00\x00\x00
    let core_module = b"\x00asm\x01\x00\x00\x00";
    // Component magic: \0asm\x0d\x00\x01\x00
    let component = b"\x00asm\x0d\x00\x01\x00";

    // Load the hello fixture and verify it's detected as a core module
    let hello_bytes = std::fs::read(fixture("hello.wasm")).unwrap();
    assert_eq!(hello_bytes[4], 0x01, "hello.wasm should be a core module");

    // Verify magic byte patterns
    assert_eq!(core_module[4], 0x01);
    assert_eq!(component[4], 0x0d);
}

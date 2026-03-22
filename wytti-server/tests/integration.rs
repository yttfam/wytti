use std::net::TcpListener;

/// Find a free port for testing.
fn free_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap().port()
}

fn fixtures_dir() -> std::path::PathBuf {
    let mut dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.pop();
    dir.push("tests/fixtures");
    dir
}

#[tokio::test]
async fn health_check() {
    let port = free_port();
    let config = wytti_server::ServerConfig {
        port,
        ..Default::default()
    };

    // Start server in background
    tokio::spawn(async move {
        wytti_server::start_server(config).await.unwrap();
    });

    // Give server time to bind
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let resp = reqwest::get(format!("http://127.0.0.1:{port}/health"))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.text().await.unwrap(), "wytti ok");
}

#[tokio::test]
async fn exec_wasm_by_path() {
    let fixture = fixtures_dir().join("hello.wasm");
    if !fixture.exists() {
        eprintln!("skipping exec_wasm_by_path: run tests/build_fixtures.sh first");
        return;
    }

    let port = free_port();
    let config = wytti_server::ServerConfig {
        port,
        ..Default::default()
    };

    tokio::spawn(async move {
        wytti_server::start_server(config).await.unwrap();
    });
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://127.0.0.1:{port}/exec"))
        .json(&serde_json::json!({
            "wasm_path": fixture.to_string_lossy(),
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["exit_code"], 0);
}

#[tokio::test]
async fn exec_missing_wasm() {
    let port = free_port();
    let config = wytti_server::ServerConfig {
        port,
        ..Default::default()
    };

    tokio::spawn(async move {
        wytti_server::start_server(config).await.unwrap();
    });
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://127.0.0.1:{port}/exec"))
        .json(&serde_json::json!({}))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 500);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["error"].as_str().unwrap().contains("must provide"));
}

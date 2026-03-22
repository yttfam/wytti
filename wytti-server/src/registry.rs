use serde::Serialize;
use std::time::Duration;

/// Announce Wytti to Hermytt's service registry.
pub async fn announce_loop(hermytt_url: String, wytti_port: u16, token: Option<String>) {
    let client = reqwest::Client::new();
    let endpoint = format!("http://localhost:{wytti_port}");

    loop {
        let body = AnnounceBody {
            name: "wytti".to_string(),
            role: "sandbox".to_string(),
            endpoint: endpoint.clone(),
            meta: AnnounceMeta {
                runtime: "wasmtime".to_string(),
            },
        };

        let mut req = client
            .post(format!("{hermytt_url}/registry/announce"))
            .json(&body);

        if let Some(ref t) = token {
            req = req.header("X-Hermytt-Key", t);
        }

        match req.send().await {
            Ok(resp) if resp.status().is_success() => {}
            Ok(resp) => {
                eprintln!("registry announce failed: {}", resp.status());
            }
            Err(e) => {
                eprintln!("registry announce error: {e}");
            }
        }

        tokio::time::sleep(Duration::from_secs(15)).await;
    }
}

#[derive(Debug, Serialize)]
struct AnnounceBody {
    name: String,
    role: String,
    endpoint: String,
    meta: AnnounceMeta,
}

#[derive(Debug, Serialize)]
struct AnnounceMeta {
    runtime: String,
}

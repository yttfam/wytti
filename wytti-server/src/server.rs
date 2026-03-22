use axum::routing::{get, post};
use axum::Router;
use std::sync::Arc;
use wytti_runtime::Runtime;
use wytti_sandbox::SandboxPolicy;

use crate::exec::{handle_exec, AppState};
use crate::registry;

/// Server configuration.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Port to listen on.
    pub port: u16,
    /// Default sandbox policy for exec requests.
    pub default_policy: SandboxPolicy,
    /// Hermytt registry URL (e.g. "http://localhost:7777").
    /// If set, Wytti will announce itself to the service registry.
    pub hermytt_url: Option<String>,
    /// Hermytt auth token.
    pub hermytt_token: Option<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: 9001,
            default_policy: SandboxPolicy::default(),
            hermytt_url: None,
            hermytt_token: None,
        }
    }
}

/// Health check endpoint.
async fn health() -> &'static str {
    "wytti ok"
}

/// Start the Wytti HTTP server.
pub async fn start_server(config: ServerConfig) -> anyhow::Result<()> {
    let runtime = Runtime::new()?;

    let state = Arc::new(AppState {
        runtime,
        default_policy: config.default_policy,
    });

    let app = Router::new()
        .route("/health", get(health))
        .route("/exec", post(handle_exec))
        .with_state(state);

    // Start registry announcer if hermytt_url is configured
    if let Some(hermytt_url) = config.hermytt_url.clone() {
        let token = config.hermytt_token.clone();
        let port = config.port;
        tokio::spawn(async move {
            registry::announce_loop(hermytt_url, port, token).await;
        });
    }

    let addr = format!("0.0.0.0:{}", config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    eprintln!("wytti server listening on {addr}");
    axum::serve(listener, app).await?;

    Ok(())
}

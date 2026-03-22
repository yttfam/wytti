use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use wytti_runtime::{Runtime, RuntimeConfig};
use wytti_sandbox::SandboxPolicy;

/// Shared server state.
pub struct AppState {
    pub runtime: Runtime,
    pub default_policy: SandboxPolicy,
}

/// Request body for POST /exec.
#[derive(Debug, Deserialize)]
pub struct ExecRequest {
    /// Base64-encoded WASM binary, or a path to a .wasm file on disk.
    #[serde(default)]
    pub wasm_base64: Option<String>,
    #[serde(default)]
    pub wasm_path: Option<String>,
    /// Arguments passed to the WASM program.
    #[serde(default)]
    pub args: Vec<String>,
    /// Environment variables.
    #[serde(default)]
    pub env: Vec<(String, String)>,
    /// Override sandbox policy fields.
    #[serde(default)]
    pub max_memory: Option<String>,
    #[serde(default)]
    pub max_time: Option<String>,
}

/// Response body for POST /exec.
#[derive(Debug, Serialize)]
pub struct ExecResponse {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// POST /exec — run a WASM binary and return stdout/stderr/exit_code.
pub async fn handle_exec(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ExecRequest>,
) -> (StatusCode, Json<ExecResponse>) {
    let result = tokio::task::spawn_blocking(move || exec_wasm(&state, &req)).await;

    match result {
        Ok(Ok(resp)) => (StatusCode::OK, Json(resp)),
        Ok(Err(e)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ExecResponse {
                exit_code: 1,
                stdout: String::new(),
                stderr: String::new(),
                error: Some(e.to_string()),
            }),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ExecResponse {
                exit_code: 1,
                stdout: String::new(),
                stderr: String::new(),
                error: Some(format!("task join error: {e}")),
            }),
        ),
    }
}

fn exec_wasm(state: &AppState, req: &ExecRequest) -> anyhow::Result<ExecResponse> {
    // Load WASM bytes
    let wasm = if let Some(ref b64) = req.wasm_base64 {
        use base64_decode;
        base64_decode(b64)?
    } else if let Some(ref path) = req.wasm_path {
        std::fs::read(path)?
    } else {
        anyhow::bail!("must provide wasm_base64 or wasm_path");
    };

    // Build config
    let mut config = RuntimeConfig::new()
        .sandbox(state.default_policy.clone())
        .inherit_stdio(false); // capture instead of inherit

    if !req.args.is_empty() {
        config = config.args(req.args.iter().map(|s| s.as_str()));
    }
    for (k, v) in &req.env {
        config = config.env(k, v);
    }

    // Run
    match state.runtime.run(&wasm, &config) {
        Ok(()) => Ok(ExecResponse {
            exit_code: 0,
            stdout: String::new(), // TODO: capture stdout when inherit_stdio=false
            stderr: String::new(),
            error: None,
        }),
        Err(e) => Ok(ExecResponse {
            exit_code: 1,
            stdout: String::new(),
            stderr: String::new(),
            error: Some(e.to_string()),
        }),
    }
}

/// Simple base64 decoder (no extra dependency).
fn base64_decode(input: &str) -> anyhow::Result<Vec<u8>> {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    fn val(c: u8) -> anyhow::Result<u8> {
        match c {
            b'A'..=b'Z' => Ok(c - b'A'),
            b'a'..=b'z' => Ok(c - b'a' + 26),
            b'0'..=b'9' => Ok(c - b'0' + 52),
            b'+' => Ok(62),
            b'/' => Ok(63),
            _ => anyhow::bail!("invalid base64 char: {c}"),
        }
    }
    let _ = TABLE; // silence unused

    let input: Vec<u8> = input.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    let mut out = Vec::with_capacity(input.len() * 3 / 4);
    let chunks = input.chunks(4);

    for chunk in chunks {
        let len = chunk.iter().filter(|&&b| b != b'=').count();
        let mut buf = [0u8; 4];
        for (i, &b) in chunk.iter().enumerate() {
            if b != b'=' {
                buf[i] = val(b)?;
            }
        }
        let combined = (buf[0] as u32) << 18 | (buf[1] as u32) << 12 | (buf[2] as u32) << 6 | buf[3] as u32;

        if len >= 2 { out.push((combined >> 16) as u8); }
        if len >= 3 { out.push((combined >> 8) as u8); }
        if len >= 4 { out.push(combined as u8); }
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_roundtrip() {
        let decoded = base64_decode("SGVsbG8gV29ybGQ=").unwrap();
        assert_eq!(decoded, b"Hello World");
    }

    #[test]
    fn base64_empty() {
        let decoded = base64_decode("").unwrap();
        assert!(decoded.is_empty());
    }
}

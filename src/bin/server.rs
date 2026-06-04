//! AuraVM REST API Daemon
//!
//! Exposes the AuraVM sandbox as a high-performance HTTP service.
//!
//! Endpoints:
//!   POST /execute       - Execute a WebAssembly binary (base64-encoded)
//!   POST /execute-js    - Execute a raw JavaScript string via embedded QuickJS
//!   GET  /health        - Health check

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use aura_vm::{AuraSandbox, ExecutionLimits, ExecutionReport};
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::net::TcpListener;

/// Shared application state — one sandbox engine, used across all requests.
struct AppState {
    sandbox: AuraSandbox,
}

/// Request body for POST /execute.
#[derive(Deserialize)]
struct ExecuteRequest {
    /// WebAssembly binary encoded as base64.
    wasm_b64: String,
    /// Name of the exported function to invoke.
    function: String,
    /// Optional resource limits (fuel, memory).
    limits: Option<ExecutionLimits>,
    /// Optional list of domain names the Wasm module is allowed to reach via HTTP.
    whitelisted_domains: Option<Vec<String>>,
}

/// Request body for POST /execute-js.
#[derive(Deserialize)]
struct ExecuteJsRequest {
    /// Raw JavaScript code string.
    code: String,
}

/// Successful API response wrapper.
#[derive(Serialize)]
struct ApiResponse {
    ok: bool,
    report: ExecutionReport,
}

/// Error response wrapper.
#[derive(Serialize)]
struct ApiError {
    ok: bool,
    error: String,
}

/// POST /execute — Run a base64-encoded WebAssembly binary.
async fn execute_wasm(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ExecuteRequest>,
) -> impl IntoResponse {
    // Decode the base64 Wasm bytes
    let wasm_bytes = match base64::engine::general_purpose::STANDARD.decode(&req.wasm_b64) {
        Ok(b) => b,
        Err(e) => {
            let body = Json(ApiError {
                ok: false,
                error: format!("Invalid base64: {}", e),
            });
            return (StatusCode::BAD_REQUEST, body.into_response());
        }
    };

    let domains = req.whitelisted_domains.unwrap_or_default();

    // Run inside a blocking thread (wasmtime is synchronous)
    let sandbox = Arc::clone(&state);
    let function = req.function.clone();
    let limits = req.limits;

    let result = tokio::task::spawn_blocking(move || {
        sandbox.sandbox.execute_agent_code_with_limits(&wasm_bytes, &function, limits, domains)
    })
    .await;

    match result {
        Ok(Ok(report)) => {
            let body = Json(ApiResponse { ok: true, report });
            (StatusCode::OK, body.into_response())
        }
        Ok(Err(e)) => {
            let body = Json(ApiError {
                ok: false,
                error: e.to_string(),
            });
            (StatusCode::INTERNAL_SERVER_ERROR, body.into_response())
        }
        Err(e) => {
            let body = Json(ApiError {
                ok: false,
                error: format!("Task panic: {}", e),
            });
            (StatusCode::INTERNAL_SERVER_ERROR, body.into_response())
        }
    }
}

/// POST /execute-js — Run a raw JavaScript code string inside embedded QuickJS.
async fn execute_js(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ExecuteJsRequest>,
) -> impl IntoResponse {
    let sandbox = Arc::clone(&state);
    let code = req.code.clone();

    let result = tokio::task::spawn_blocking(move || sandbox.sandbox.execute_js(&code)).await;

    match result {
        Ok(Ok(report)) => {
            let body = Json(ApiResponse { ok: true, report });
            (StatusCode::OK, body.into_response())
        }
        Ok(Err(e)) => {
            let body = Json(ApiError {
                ok: false,
                error: e.to_string(),
            });
            (StatusCode::INTERNAL_SERVER_ERROR, body.into_response())
        }
        Err(e) => {
            let body = Json(ApiError {
                ok: false,
                error: format!("Task panic: {}", e),
            });
            (StatusCode::INTERNAL_SERVER_ERROR, body.into_response())
        }
    }
}

/// GET /health — Lightweight liveness probe.
async fn health() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok", "version": env!("CARGO_PKG_VERSION") }))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let port: u16 = std::env::var("AURA_PORT")
        .unwrap_or_else(|_| "8080".into())
        .parse()
        .expect("AURA_PORT must be a valid port number");

    let sandbox = AuraSandbox::new()?;
    let state = Arc::new(AppState { sandbox });

    let app = Router::new()
        .route("/execute", post(execute_wasm))
        .route("/execute-js", post(execute_js))
        .route("/health", get(health))
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    println!("[AuraVM] REST API Server starting on http://{}", addr);
    println!("[AuraVM] Endpoints:");
    println!("  POST /execute     — Run a base64-encoded .wasm binary");
    println!("  POST /execute-js  — Run a raw JavaScript string via QuickJS");
    println!("  GET  /health      — Liveness check");

    let listener = TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

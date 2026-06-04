# AuraVM: The MicroVM for AI Agents

![Status](https://img.shields.io/badge/Status-Beta-orange) ![Rust](https://img.shields.io/badge/Language-Rust-black) ![License](https://img.shields.io/badge/License-MIT-blue)

> **AuraVM** is an ultra-fast, highly secure WebAssembly (WASM) Micro-Virtual Machine designed specifically for AI startups that need to execute untrusted AI-generated code safely — without the 3-second cold start times of Docker.

---

## Why AuraVM?

When an AI Agent (like Devin or AutoGPT) writes code, it needs to test it. Running untrusted AI code on your host server is a massive security risk. Running it in Docker is too slow.

**AuraVM boots a completely isolated execution environment in under 5 milliseconds.**

---

## Features

### Core Security
| Feature | Description |
|---|---|
| **Fuel Limits** | Every WASM instruction costs 1 unit of fuel. If the AI writes an infinite loop, the VM kills the process instantly without blocking the CPU. |
| **Memory Hard Caps** | Each sandbox is capped at a configurable RAM limit (default 10 MB). Overflowing allocations trap immediately. |
| **Zero-State Ephemerality** | The moment execution ends, the `Store` drops — wiping all memory instantly. No lingering processes. |
| **WASI Glass Wall** | AI code can print to stdout/stderr but never touch the host OS, filesystem, or network without explicit permission. |

### New in v0.2.0
| Feature | Description |
|---|---|
| **Detailed JSON Telemetry** | Every execution returns a structured `ExecutionReport` with fuel consumed, peak memory, exit code, and captured stdout/stderr. |
| **JavaScript Execution** | Run raw JS strings directly via an embedded [QuickJS](https://bellard.org/quickjs/) WebAssembly engine — no compiler needed. |
| **Domain-Whitelisted Networking** | Sandboxed code can make outbound HTTP calls only to domains you explicitly allow. All other domains are blocked. |
| **State Snapshotting** | Capture a point-in-time snapshot of a VM's memory and globals, save it to disk, and resume it on any machine. |
| **REST API Daemon** | A standalone HTTP server (axum + tokio) exposing `POST /execute`, `POST /execute-js`, and `GET /health`. |

---

## Quick Start

### CLI

```bash
# Run a WASM binary
cargo run -- --file agent.wasm --function _start

# Start the REST API daemon (default port 8080)
cargo run --bin server
AURA_PORT=9000 cargo run --bin server
```

### Embedded Library

```rust
use aura_vm::{AuraSandbox, ExecutionLimits};

let sandbox = AuraSandbox::new()?;

// Run WASM bytes
let report = sandbox.execute_agent_code(&wasm_bytes, "my_function")?;
println!("{}", serde_json::to_string_pretty(&report)?);

// Run raw JavaScript
let js_report = sandbox.execute_js("console.log(40 + 2);")?;
println!("stdout: {}", js_report.stdout); // "42"

// Run with custom limits and whitelisted domains
let report = sandbox.execute_agent_code_with_limits(
    &wasm_bytes,
    "fetch_data",
    Some(ExecutionLimits { fuel: Some(50_000), memory_limit_bytes: Some(32 * 1024 * 1024) }),
    vec!["api.github.com".to_string()],
)?;
```

### REST API

```bash
# Start the server
cargo run --bin server

# Execute a WASM binary (base64 encoded)
curl -X POST http://localhost:8080/execute \
  -H "Content-Type: application/json" \
  -d '{
    "wasm_b64": "<base64-encoded-wasm>",
    "function": "run",
    "limits": { "fuel": 50000, "memory_limit_bytes": 10485760 },
    "whitelisted_domains": ["api.openai.com"]
  }'

# Execute raw JavaScript
curl -X POST http://localhost:8080/execute-js \
  -H "Content-Type: application/json" \
  -d '{ "code": "const x = 6 * 7; console.log(x);" }'

# Health check
curl http://localhost:8080/health
```

### Execution Report

Every execution returns a structured JSON report:

```json
{
  "status": "Success",
  "exit_code": 0,
  "fuel_consumed": 312,
  "peak_memory": 65536,
  "stdout": "Hello from AI: 42\n",
  "stderr": ""
}
```

Status values: `Success` | `OutOfFuel` | `MemoryLimitExceeded` | `Trap("<reason>")`

---

## Snapshotting & Resumption

```rust
// Run a counter module until it reaches 5
let snapshot = sandbox.take_snapshot(&mut store, &instance)?;

// Serialize snapshot to disk
let bytes = bincode::serialize(&snapshot)?;
std::fs::write("vm_state.snap", bytes)?;

// Later — restore on any machine
let snapshot: VmSnapshot = bincode::deserialize(&std::fs::read("vm_state.snap")?)?;
sandbox.restore_snapshot(&mut store, &instance, &snapshot)?;
```

---

## vs. the Competition

| | AuraVM | Docker | AWS Lambda |
|---|---|---|---|
| **Cold start** | ~5ms | ~2,800ms | ~100–500ms |
| **Memory isolation** | ✅ WASM linear memory | ✅ cgroups | ✅ |
| **Fuel / CPU limits** | ✅ per-instruction | ❌ | ❌ |
| **JS without compile** | ✅ via QuickJS | ❌ | ❌ |
| **Snapshot & resume** | ✅ | ❌ | ❌ |
| **Network whitelist** | ✅ per-call | ✅ iptables | ✅ VPC |
| **REST API** | ✅ built-in | ❌ | ✅ |

---

## Building from Source

```bash
git clone https://github.com/peteradeojo/aura-vm
cd aura-vm
cargo build --release
cargo test
```

---

## License

MIT — See [LICENSE](LICENSE) for details.

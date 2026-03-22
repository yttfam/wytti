You are building Wytti — a lightweight WASI runtime in Rust. The fourth child of the YTT dynasty. The family executes, displays, prettifies, and now runs arbitrary code in a sandbox.

Wytti runs WASM binaries with WASI system interface. In the browser, on the server, sandboxed, safe. The missing piece that turns the YTT family from a terminal suite into a compute platform.

## The YTT Family

- **Hermytt** — transport-agnostic terminal multiplexer (PTY ↔ REST/MQTT/WS/Telegram)
- **Crytter** — WASM terminal emulator (37KB, Canvas2D, xterm.js killer)
- **Prytty** — syntax highlighting and pretty-printing
- **Wytti** — WASI runtime (you are here)

## The Full Stack

```
User types command
  → Hermytt (transport: WS/MQTT/Telegram/REST)
    → Wytti (execute WASI binary in sandbox)
      → stdout → Prytty (colorize)
        → Crytter (render in browser)
          → User sees output
```

Run arbitrary WASM binaries through a terminal, from anywhere, sandboxed. A cloud IDE without the cloud.

## What is WASI

WebAssembly System Interface — a standard for WASM programs to access system resources (files, env, args, stdin/stdout, clocks, random) in a capability-based sandboxed way.

- WASI Preview 1: stable, widely supported (fd_read, fd_write, path_open, etc.)
- WASI Preview 2: component model, typed interfaces (emerging)

## Architecture

```
wytti/
├── wytti-runtime/        # Core WASI runtime: instantiate, execute, capability management
├── wytti-vfs/            # Virtual filesystem: in-memory, mapped, or sandboxed real FS
├── wytti-io/             # stdin/stdout/stderr bridging to Hermytt sessions
├── wytti-sandbox/        # Capability policies: what can a WASM module access?
├── wytti-server/         # HTTP API: upload WASM, execute, stream output
├── wytti-cli/            # CLI: wytti run program.wasm, wytti serve
└── wytti-web/            # Browser runtime: run WASI in the browser via Crytter
```

## Core Design

### Runtime Engine
Two options:
- **wasmtime** — Bytecode Alliance, mature, fast JIT/AOT, full WASI support, but heavy (~20MB)
- **wasmer** — alternative, plugin-based backends, slightly smaller
- **wasm3** — interpreter only, tiny (~100KB), slow but embeddable
- **custom** — minimal WASI P1 interpreter, maximum control, smallest binary

Recommendation: start with **wasmtime** for correctness, consider **wasm3** for the browser build.

### Virtual Filesystem
WASI programs expect a filesystem. Wytti provides:
- In-memory VFS (default, fully sandboxed)
- Mapped directories (explicit: `--dir /host/path:/guest/path`)
- Read-only mounts for shared data
- Deny-by-default: nothing accessible unless explicitly granted

### I/O Bridge to Hermytt
- Wytti's stdin reads from Hermytt's session input
- Wytti's stdout/stderr writes to Hermytt's session output
- This means: run a WASI program and interact with it through Telegram. Or MQTT. Or a browser.

### Capability Policies
```toml
[sandbox]
allow_net = false           # no network by default
allow_fs = ["/data:ro"]     # read-only /data mount
allow_env = ["PATH", "HOME"] # filtered env vars
max_memory = "256MB"
max_time = "30s"
```

### Use Cases
1. **Run CLI tools in browser**: compile coreutils to WASM, run `ls`, `grep`, `cat` in Crytter
2. **Sandboxed code execution**: upload a WASM binary, run it, get output — no escape
3. **Agent code execution**: AI agents compile Rust to WASM, Wytti runs it safely
4. **Education**: run student code in sandbox, stream output to terminal
5. **Edge compute**: run WASM functions at the edge (on Hermytt endpoints)

## Integration Points

### With Crytter (browser)
```
Browser loads Crytter (terminal) + Wytti (runtime)
User types → Crytter captures → Wytti executes → stdout → Crytter renders
No server needed for simple programs — fully client-side terminal + execution
```

### With Hermytt (server)
```
Hermytt session → instead of spawning bash, spawn Wytti
Wytti loads a WASM binary → executes with WASI
stdin/stdout bridged to Hermytt transports
Result: run WASM programs over MQTT/Telegram
```

### With Prytty
Wytti pipes stdout through Prytty before sending to the terminal. Automatic syntax detection and colorization of program output.

## Tech Stack

- `wasmtime` — WASI runtime (or `wasm3` for minimal builds)
- `tokio` — async I/O
- `cap-std` — capability-based std library (pairs with wasmtime)
- `axum` — HTTP API for the server mode
- `clap` — CLI
- `serde` + `toml` — config and sandbox policies

## Build Targets

- Native binary (Linux, macOS): full runtime with JIT
- WASM build: Wytti itself compiled to WASM (meta!), interpreter mode via wasm3
  - This enables: browser runs Crytter → which runs Wytti → which runs WASI programs
  - WASM inception

## Cali's Preferences

- Start with wasmtime, get it working, optimize later
- CLI first: `wytti run hello.wasm` must just work
- Hermytt integration as the killer feature
- Browser build is stretch goal but architecturally planned from day one
- Capability-based security, deny by default
- Part of the YTT family — same quality, same release profile, same energy
- The name is Wytti. The wise one who runs things safely.

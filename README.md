# Wytti

WASI runtime — sandboxed WASM execution for the [YTT family](https://github.com/yttfam).

Wytti takes `.wasm` binaries and runs them. Sandboxed, capability-gated, time-limited. The execution engine that turns the YTT family from a terminal suite into a compute platform.

## Quick Start

```bash
# Run a WASM binary
wytti run hello.wasm

# With sandbox limits
wytti run app.wasm --max-time 5s --max-memory 64MB

# Allow networking
wytti run app.wasm -N

# Start the HTTP exec server
wytti serve --port 9001

# With Hermytt service registry
wytti serve --hermytt-url http://localhost:7777
```

## Crates

| Crate | Description |
|-------|-------------|
| **wytti-runtime** | Core engine — wasmtime, WASI P1 + P2, epoch timeouts, memory limits |
| **wytti-sandbox** | Capability policies — TOML config, deny-by-default |
| **wytti-vfs** | Filesystem mount config — ro/rw preopens |
| **wytti-host** | `HostBackend` trait + `fytti_*` WASM host function linker |
| **wytti-manifest** | `.fytti.toml` app manifest parser |
| **wytti-server** | HTTP exec API + Hermytt service registry |
| **wytti-cli** | `wytti run` and `wytti serve` commands |

## Sandbox

Everything denied by default. Grant capabilities explicitly:

```toml
[sandbox]
max_memory = "256MB"
max_time = "30s"
allow_tcp = false
allow_udp = false
allow_dns = false

[[sandbox.fs]]
guest = "/data"
host = "/tmp/sandbox"
permission = "ro"
```

## Host API

WASM apps can import the `"fytti"` module to draw, handle input, and manage resources when hosted by [Fytti](https://github.com/yttfam/fytti):

- **Rendering** — `clear`, `fill_rect`, `stroke_rect`, `draw_line`, `draw_text`, `gradient_rect`, `fill_ellipse`, `present`
- **Viewport** — `get_width`, `get_height`
- **Input** — `poll_event`, `poll_mouse`
- **Resources** — `load_font`, `load_image`
- **Lifecycle** — `set_title`, `request_frame`

## The YTT Family

| Sibling | Role |
|---------|------|
| [Hermytt](https://github.com/yttfam/hermytt) | Transport — WS/MQTT/REST/Telegram |
| [Crytter](https://github.com/yttfam/crytter) | Terminal emulator — Canvas2D, 37KB |
| [Prytty](https://github.com/yttfam/prytty) | Syntax highlighting |
| **Wytti** | WASI runtime (you are here) |
| [Fytti](https://github.com/yttfam/fytti) | Browser engine — wgpu rendering |
| [Shytti](https://github.com/yttfam/shytti) | Shell orchestrator daemon |

## License

MIT

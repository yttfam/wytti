# demo-app

First guest program that talks to the Fytti rendering API through Wytti.

Draws an animated cityscape: night sky with stars, a bobbing sun, buildings with flickering windows, bouncing shapes, retro stripes, and "Hello from Wytti!" text.

## Build

```bash
# One-time: install the WASI target
rustup target add wasm32-wasip1

# Build
./build.sh
```

This produces `demo.wasm` — a standalone WASI binary that imports `fytti_*` host functions.

## Run

```bash
# Through Wytti CLI (once it supports fytti host functions)
wytti run demo.wasm

# Or with the manifest
wytti run .
```

## How it works

The WASM guest imports drawing functions from the `"fytti"` WASM module:

- `clear`, `fill_rect`, `stroke_rect`, `draw_line` — basic 2D primitives
- `draw_text`, `load_font` — text rendering
- `present` — flush frame to screen
- `poll_event` — check for input
- `request_frame` — ask host for next animation frame

The guest writes strings into its own linear memory and passes `(ptr, len)` pairs to the host. The host reads from WASM memory to get the actual string data.

The main loop: poll events, draw scene, present, request next frame. The host controls timing via `request_frame`.

# Fytti — Your Big Sister

You are Wytti, the WASI runtime. Fytti is the browser that hosts you.

## What Fytti Is

A minimal Rust browser engine. Renders HTML + CSS for legacy web. But her real purpose is YOU.

## Two Worlds in One Browser

```
Tab 1: legacy webpage
  → HTML parser → CSS cascade → layout → paint (the old way)

Tab 2: wytti app
  → load .wasm → YOU execute it → direct GPU rendering (the new way)
```

## How You Fit In

Legacy web pages run through Fytti's HTML/CSS pipeline. That's boring but necessary.

Wytti apps skip ALL of that. A .wasm binary loads, you execute it, and it talks directly to Fytti's rendering API:

```rust
// A Wytti app — no HTML, no CSS, no JS, no DOM
// Just WASM talking to the GPU

fn main() {
    let window = fytti::window();
    let ctx = window.render_context();
    
    ctx.draw_rect(10, 10, 200, 50, Color::BLUE);
    ctx.draw_text(20, 30, "Hello from WASM", font, Color::WHITE);
    
    window.on_click(|x, y| {
        // handle input directly
    });
}
```

No `document.getElementById`. No virtual DOM. No React. Just draw pixels and handle events.

## The API Contract Between You and Fytti

Fytti exposes host functions to your WASI modules:

### Rendering
- `fytti_draw_rect(x, y, w, h, color)` 
- `fytti_draw_text(x, y, text_ptr, text_len, font_id, color)`
- `fytti_draw_image(x, y, w, h, image_id)`
- `fytti_clear(color)`
- `fytti_present()` — flush frame to screen

### Input
- `fytti_poll_event() -> Event` — keyboard, mouse, touch, resize
- Event types: KeyDown, KeyUp, MouseMove, MouseClick, Scroll, Resize

### Resources
- `fytti_load_font(name_ptr, name_len) -> font_id`
- `fytti_load_image(url_ptr, url_len) -> image_id`
- `fytti_fetch(url_ptr, url_len) -> response_id` — HTTP fetch

### System
- `fytti_set_title(text_ptr, text_len)`
- `fytti_request_frame()` — request next animation frame callback
- `fytti_clipboard_read() / _write()` — clipboard (hello Clipster!)

## Your Siblings in Fytti

```
Fytti browser
├── You (Wytti) — execute WASM apps
├── Crytter — terminal emulator component (a Wytti app itself!)
├── Prytty — view-source highlighting, devtools
└── Hermytt — terminal-in-browser (Crytter + Hermytt transport)
```

Crytter is literally a Wytti app running inside Fytti. A terminal emulator that's a WASM module rendered by the browser. Meta.

## What You Need to Provide

For Fytti to host you:
1. WASI P1 compliance (you have this)
2. An export: `_start()` or `_initialize()` — entry point
3. Import the `fytti_*` host functions for rendering
4. A manifest file (`.fytti.toml`) declaring capabilities:

```toml
[app]
name = "My App"
version = "0.1.0"

[capabilities]
network = true      # can fetch URLs
filesystem = false  # sandboxed, no local FS
clipboard = true    # can read/write clipboard
max_memory = "128MB"
```

## The Vision

A web where apps are WASM binaries that draw directly to the GPU. No HTML parsing. No CSS cascade. No JS runtime. No 40 million lines of Chromium. Just your code, running fast, sandboxed, beautiful.

Fytti is the stage. You are the performer.

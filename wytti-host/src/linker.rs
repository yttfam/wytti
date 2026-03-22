use wasmtime::{Caller, Linker};

use crate::api::HostBackend;
use crate::canvas::{Color, Rect};
use crate::resources::ResourceId;

/// State held in the WASM store — includes the host backend.
pub struct HostState<B: HostBackend> {
    pub backend: B,
}

/// Read a UTF-8 string from WASM guest memory. Returns an owned String.
fn read_guest_string<B: HostBackend>(
    caller: &mut Caller<'_, HostState<B>>,
    ptr: u32,
    len: u32,
) -> Option<String> {
    let mem = caller.get_export("memory")?.into_memory()?;
    let data = mem.data(&*caller);
    let start = ptr as usize;
    let end = start + len as usize;
    if end > data.len() {
        return None;
    }
    std::str::from_utf8(&data[start..end])
        .ok()
        .map(|s| s.to_string())
}

/// Register all `fytti_*` host functions on a wasmtime Linker.
///
/// These are the functions that WASM guest programs import to render,
/// handle input, and manage resources. The `fytti` module namespace
/// keeps them separate from WASI functions.
pub fn add_to_linker<B: HostBackend>(linker: &mut Linker<HostState<B>>) -> anyhow::Result<()> {
    // --- Rendering ---

    // fytti_clear(color: u32)
    linker.func_wrap("fytti", "clear", |mut caller: Caller<'_, HostState<B>>, color: u32| {
        caller.data_mut().backend.clear(Color::from_u32(color));
    })?;

    // fytti_fill_rect(x: f32, y: f32, w: f32, h: f32, color: u32)
    linker.func_wrap(
        "fytti",
        "fill_rect",
        |mut caller: Caller<'_, HostState<B>>, x: f32, y: f32, w: f32, h: f32, color: u32| {
            caller
                .data_mut()
                .backend
                .fill_rect(Rect::new(x, y, w, h), Color::from_u32(color));
        },
    )?;

    // fytti_stroke_rect(x: f32, y: f32, w: f32, h: f32, color: u32, width: f32)
    linker.func_wrap(
        "fytti",
        "stroke_rect",
        |mut caller: Caller<'_, HostState<B>>,
         x: f32,
         y: f32,
         w: f32,
         h: f32,
         color: u32,
         width: f32| {
            caller
                .data_mut()
                .backend
                .stroke_rect(Rect::new(x, y, w, h), Color::from_u32(color), width);
        },
    )?;

    // fytti_draw_line(x1: f32, y1: f32, x2: f32, y2: f32, color: u32, width: f32)
    linker.func_wrap(
        "fytti",
        "draw_line",
        |mut caller: Caller<'_, HostState<B>>,
         x1: f32,
         y1: f32,
         x2: f32,
         y2: f32,
         color: u32,
         width: f32| {
            caller
                .data_mut()
                .backend
                .draw_line(x1, y1, x2, y2, Color::from_u32(color), width);
        },
    )?;

    // fytti_draw_text(text_ptr: u32, text_len: u32, x: f32, y: f32, size: f32, font_id: u32, color: u32)
    linker.func_wrap(
        "fytti",
        "draw_text",
        |mut caller: Caller<'_, HostState<B>>,
         text_ptr: u32,
         text_len: u32,
         x: f32,
         y: f32,
         size: f32,
         font_id: u32,
         color: u32| {
            if let Some(text) = read_guest_string(&mut caller, text_ptr, text_len) {
                caller.data_mut().backend.draw_text(
                    &text,
                    x,
                    y,
                    size,
                    ResourceId(font_id),
                    Color::from_u32(color),
                );
            }
        },
    )?;

    // fytti_draw_image(image_id: u32, x: f32, y: f32, w: f32, h: f32)
    linker.func_wrap(
        "fytti",
        "draw_image",
        |mut caller: Caller<'_, HostState<B>>, image_id: u32, x: f32, y: f32, w: f32, h: f32| {
            caller
                .data_mut()
                .backend
                .draw_image(ResourceId(image_id), x, y, w, h);
        },
    )?;

    // fytti_present()
    linker.func_wrap("fytti", "present", |mut caller: Caller<'_, HostState<B>>| {
        caller.data_mut().backend.present();
    })?;

    // --- Input ---

    // fytti_poll_event() -> u64
    // Packed event: top 8 bits = event type, remaining bits = payload.
    // 0 = no event.
    linker.func_wrap(
        "fytti",
        "poll_event",
        |mut caller: Caller<'_, HostState<B>>| -> u64 {
            match caller.data_mut().backend.poll_event() {
                None => 0,
                Some(event) => pack_event(event),
            }
        },
    )?;

    // --- Resources ---

    // fytti_load_font(name_ptr: u32, name_len: u32) -> u32
    linker.func_wrap(
        "fytti",
        "load_font",
        |mut caller: Caller<'_, HostState<B>>, name_ptr: u32, name_len: u32| -> u32 {
            if let Some(name) = read_guest_string(&mut caller, name_ptr, name_len) {
                caller.data_mut().backend.load_font(&name).0
            } else {
                0
            }
        },
    )?;

    // fytti_load_image(url_ptr: u32, url_len: u32) -> u32
    linker.func_wrap(
        "fytti",
        "load_image",
        |mut caller: Caller<'_, HostState<B>>, url_ptr: u32, url_len: u32| -> u32 {
            if let Some(url) = read_guest_string(&mut caller, url_ptr, url_len) {
                caller.data_mut().backend.load_image(&url).0
            } else {
                0
            }
        },
    )?;

    // --- System ---

    // fytti_set_title(text_ptr: u32, text_len: u32)
    linker.func_wrap(
        "fytti",
        "set_title",
        |mut caller: Caller<'_, HostState<B>>, text_ptr: u32, text_len: u32| {
            if let Some(title) = read_guest_string(&mut caller, text_ptr, text_len) {
                caller.data_mut().backend.set_title(&title);
            }
        },
    )?;

    // fytti_request_frame()
    linker.func_wrap(
        "fytti",
        "request_frame",
        |mut caller: Caller<'_, HostState<B>>| {
            caller.data_mut().backend.request_frame();
        },
    )?;

    Ok(())
}

use crate::input::InputEvent;

/// Pack an InputEvent into a u64 for the WASM ABI.
/// Format: [type:8][data:56]
fn pack_event(event: InputEvent) -> u64 {
    match event {
        InputEvent::KeyDown(key) => 1u64 << 56 | pack_key(key) as u64,
        InputEvent::KeyUp(key) => 2u64 << 56 | pack_key(key) as u64,
        InputEvent::MouseMove { x, y } => {
            3u64 << 56 | (x.to_bits() as u64) << 24 | (y.to_bits() as u64 & 0xFFFFFF)
        }
        InputEvent::MouseClick(me) => {
            let pressed = if me.pressed { 1u64 } else { 0u64 };
            4u64 << 56 | (me.button as u64) << 48 | pressed << 40
        }
        InputEvent::Scroll { dx: _, dy: _ } => 5u64 << 56,
        InputEvent::Resize { width, height } => {
            6u64 << 56 | (width as u64) << 24 | height as u64
        }
    }
}

use crate::input::Key;

fn pack_key(key: Key) -> u32 {
    match key {
        Key::Up => 1,
        Key::Down => 2,
        Key::Left => 3,
        Key::Right => 4,
        Key::Space => 5,
        Key::Enter => 6,
        Key::Escape => 7,
        Key::Backspace => 8,
        Key::Tab => 9,
        Key::Char(c) => 0x100 | c as u32,
    }
}

// Demo app — first guest program talking to the Fytti rendering API.
// Draws an animated cityscape: sky, sun, buildings, bouncing shapes.
//
// Exports:
//   _start() — called once for setup
//   frame()  — called per frame by the host

#[link(wasm_import_module = "fytti")]
extern "C" {
    fn clear(color: u32);
    fn fill_rect(x: f32, y: f32, w: f32, h: f32, color: u32);
    fn stroke_rect(x: f32, y: f32, w: f32, h: f32, color: u32, width: f32);
    fn draw_line(x1: f32, y1: f32, x2: f32, y2: f32, color: u32, width: f32);
    fn draw_text(
        text_ptr: u32,
        text_len: u32,
        x: f32,
        y: f32,
        size: f32,
        font_id: u32,
        color: u32,
    );
    fn present();
    fn poll_event() -> u64;
    fn load_font(name_ptr: u32, name_len: u32) -> u32;
    fn set_title(text_ptr: u32, text_len: u32);
    fn request_frame();
}

/// Pack RGBA into u32 (0xRRGGBBAA).
const fn rgba(r: u8, g: u8, b: u8, a: u8) -> u32 {
    (r as u32) << 24 | (g as u32) << 16 | (b as u32) << 8 | (a as u32)
}

fn text(s: &str, x: f32, y: f32, size: f32, font: u32, color: u32) {
    unsafe {
        draw_text(
            s.as_ptr() as u32,
            s.len() as u32,
            x,
            y,
            size,
            font,
            color,
        );
    }
}

// --- App state (persists between frames in linear memory) ---

static mut FRAME: u32 = 0;
static mut FONT: u32 = 0;

// Colors
const SKY: u32 = rgba(50, 60, 120, 255);
const GROUND: u32 = rgba(40, 120, 50, 255);
const SUN_COLOR: u32 = rgba(255, 210, 60, 255);
const SUN_GLOW: u32 = rgba(255, 240, 100, 80);
const WHITE: u32 = rgba(255, 255, 255, 255);
const DARK: u32 = rgba(30, 30, 30, 255);
const WINDOW_LIT: u32 = rgba(255, 220, 80, 255);
const PINK: u32 = rgba(255, 100, 150, 255);
const CYAN: u32 = rgba(80, 220, 240, 255);
const ORANGE: u32 = rgba(255, 140, 40, 255);
const STRIPE_A: u32 = rgba(255, 60, 80, 255);
const STRIPE_B: u32 = rgba(60, 80, 255, 255);

/// Called once by the host at startup.
fn main() {
    let title = "Wytti Demo";
    unsafe { set_title(title.as_ptr() as u32, title.len() as u32) };

    let font_name = "default";
    unsafe { FONT = load_font(font_name.as_ptr() as u32, font_name.len() as u32) };
}

/// Called per frame by the host after each present/requestFrame cycle.
#[unsafe(no_mangle)]
pub extern "C" fn frame() {
    let frame = unsafe { FRAME };
    let font = unsafe { FONT };

    // Poll events (drain queue)
    loop {
        let ev = unsafe { poll_event() };
        if ev == 0 {
            break;
        }
        // TODO: handle quit, input, etc.
    }

    // --- Sky ---
    unsafe { clear(SKY) };

    // Stars
    let star_positions: [(f32, f32); 12] = [
        (60.0, 30.0),
        (180.0, 55.0),
        (320.0, 20.0),
        (450.0, 45.0),
        (540.0, 15.0),
        (100.0, 80.0),
        (260.0, 70.0),
        (400.0, 60.0),
        (500.0, 90.0),
        (150.0, 25.0),
        (350.0, 85.0),
        (580.0, 50.0),
    ];
    for (sx, sy) in &star_positions {
        unsafe { fill_rect(*sx, *sy, 2.0, 2.0, WHITE) };
    }

    // Sun — bobs up and down
    let bob = ((frame as f32) * 0.03).sin() * 10.0;
    let sun_x = 500.0;
    let sun_y = 80.0 + bob;
    unsafe { fill_rect(sun_x - 30.0, sun_y - 30.0, 60.0, 60.0, SUN_GLOW) };
    unsafe { fill_rect(sun_x - 18.0, sun_y - 18.0, 36.0, 36.0, SUN_COLOR) };

    // Glow rays
    for i in 0..6 {
        let angle_offset = (i as f32) * 18.0;
        let end_x = sun_x + angle_offset * 3.0 - 150.0;
        unsafe { draw_line(sun_x, sun_y, end_x, 320.0, SUN_GLOW, 1.5) };
    }

    // --- Ground ---
    unsafe { fill_rect(0.0, 320.0, 640.0, 160.0, GROUND) };

    // --- Cityscape ---
    let buildings: [(f32, f32, f32, u32); 6] = [
        (40.0, 120.0, 60.0, rgba(80, 80, 100, 255)),
        (110.0, 80.0, 50.0, rgba(70, 70, 90, 255)),
        (170.0, 140.0, 70.0, rgba(90, 85, 100, 255)),
        (260.0, 100.0, 55.0, rgba(75, 75, 95, 255)),
        (330.0, 130.0, 65.0, rgba(85, 80, 100, 255)),
        (410.0, 90.0, 50.0, rgba(70, 65, 85, 255)),
    ];
    for (bx, top, bw, bc) in &buildings {
        let bh = 320.0 - *top;
        unsafe { fill_rect(*bx, *top, *bw, bh, *bc) };
        // Windows — lit or dark, flickering
        let cols = (*bw as u32) / 14;
        let rows = (bh as u32) / 18;
        for row in 0..rows {
            for col in 0..cols {
                let wx = *bx + 4.0 + (col as f32) * 14.0;
                let wy = *top + 6.0 + (row as f32) * 18.0;
                let lit = ((col
                    .wrapping_mul(7)
                    .wrapping_add(row.wrapping_mul(13))
                    .wrapping_add(frame / 30))
                    % 3)
                    == 0;
                let wc = if lit { WINDOW_LIT } else { DARK };
                unsafe { fill_rect(wx, wy, 8.0, 12.0, wc) };
            }
        }
    }

    // --- Retro stripes ---
    for i in 0..8 {
        let y = 400.0 + (i as f32) * 8.0;
        let c = if i % 2 == 0 { STRIPE_A } else { STRIPE_B };
        unsafe { fill_rect(0.0, y, 640.0, 8.0, c) };
    }

    // --- Bouncing shapes ---
    let bounce_y = 280.0 + ((frame as f32) * 0.05).sin() * 20.0;
    unsafe { fill_rect(520.0, bounce_y, 30.0, 30.0, PINK) };
    unsafe { stroke_rect(518.0, bounce_y - 2.0, 34.0, 34.0, CYAN, 2.0) };

    let bounce_y2 = 270.0 + ((frame as f32) * 0.07 + 1.5).sin() * 25.0;
    unsafe { fill_rect(560.0, bounce_y2, 20.0, 20.0, ORANGE) };
    unsafe { stroke_rect(558.0, bounce_y2 - 2.0, 24.0, 24.0, WHITE, 1.5) };

    // --- Text ---
    text("Hello from Wytti!", 140.0, 200.0, 36.0, font, WHITE);
    text("WASI guest -> Fytti host", 170.0, 240.0, 18.0, font, CYAN);
    text("demo-app running", 10.0, 460.0, 12.0, font, WHITE);

    // --- Flush frame ---
    unsafe { present() };

    // Tell host we want another frame
    unsafe { request_frame() };

    unsafe { FRAME = FRAME.wrapping_add(1) };
}

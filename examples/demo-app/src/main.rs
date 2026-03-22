// Demo app — first guest program talking to the Fytti rendering API.
// Draws an animated cityscape: sky, sun, buildings, bouncing shapes.
// All coordinates scale to the viewport size via get_width/get_height.
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
    fn get_width() -> u32;
    fn get_height() -> u32;
}

const fn rgba(r: u8, g: u8, b: u8, a: u8) -> u32 {
    (r as u32) << 24 | (g as u32) << 16 | (b as u32) << 8 | (a as u32)
}

fn text(s: &str, x: f32, y: f32, size: f32, font: u32, color: u32) {
    unsafe {
        draw_text(s.as_ptr() as u32, s.len() as u32, x, y, size, font, color);
    }
}

// --- App state ---

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

/// Called per frame by the host.
#[unsafe(no_mangle)]
pub extern "C" fn frame() {
    let f = unsafe { FRAME };
    let font = unsafe { FONT };

    // Query viewport — scale everything relative to this
    let w = unsafe { get_width() } as f32;
    let h = unsafe { get_height() } as f32;

    // Scale factors relative to a 640x480 design canvas
    let sx = w / 640.0;
    let sy = h / 480.0;

    // Drain events
    loop {
        if unsafe { poll_event() } == 0 {
            break;
        }
    }

    // --- Sky ---
    unsafe { clear(SKY) };

    // Stars
    let stars: [(f32, f32); 12] = [
        (60.0, 30.0), (180.0, 55.0), (320.0, 20.0), (450.0, 45.0),
        (540.0, 15.0), (100.0, 80.0), (260.0, 70.0), (400.0, 60.0),
        (500.0, 90.0), (150.0, 25.0), (350.0, 85.0), (580.0, 50.0),
    ];
    for (px, py) in &stars {
        unsafe { fill_rect(px * sx, py * sy, 2.0 * sx, 2.0 * sy, WHITE) };
    }

    // Sun — bobs up and down
    let bob = ((f as f32) * 0.03).sin() * 10.0 * sy;
    let sun_x = 500.0 * sx;
    let sun_y = 80.0 * sy + bob;
    unsafe { fill_rect(sun_x - 30.0 * sx, sun_y - 30.0 * sy, 60.0 * sx, 60.0 * sy, SUN_GLOW) };
    unsafe { fill_rect(sun_x - 18.0 * sx, sun_y - 18.0 * sy, 36.0 * sx, 36.0 * sy, SUN_COLOR) };

    // Glow rays
    let ground_y = 320.0 * sy;
    for i in 0..6 {
        let offset = (i as f32) * 18.0 * sx;
        let end_x = sun_x + offset * 3.0 - 150.0 * sx;
        unsafe { draw_line(sun_x, sun_y, end_x, ground_y, SUN_GLOW, 1.5) };
    }

    // --- Ground ---
    unsafe { fill_rect(0.0, ground_y, w, h - ground_y, GROUND) };

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
        let bx = bx * sx;
        let top = top * sy;
        let bw = bw * sx;
        let bh = ground_y - top;
        unsafe { fill_rect(bx, top, bw, bh, *bc) };
        // Windows
        let cols = (bw as u32) / (14 * sx as u32).max(1);
        let rows = (bh as u32) / (18 * sy as u32).max(1);
        for row in 0..rows {
            for col in 0..cols {
                let wx = bx + 4.0 * sx + (col as f32) * 14.0 * sx;
                let wy = top + 6.0 * sy + (row as f32) * 18.0 * sy;
                let lit = ((col.wrapping_mul(7).wrapping_add(row.wrapping_mul(13)).wrapping_add(f / 30)) % 3) == 0;
                let wc = if lit { WINDOW_LIT } else { DARK };
                unsafe { fill_rect(wx, wy, 8.0 * sx, 12.0 * sy, wc) };
            }
        }
    }

    // --- Retro stripes at bottom ---
    let stripe_y = h * 0.833; // ~400/480
    let stripe_h = (h - stripe_y) / 8.0;
    for i in 0..8 {
        let y = stripe_y + (i as f32) * stripe_h;
        let c = if i % 2 == 0 { STRIPE_A } else { STRIPE_B };
        unsafe { fill_rect(0.0, y, w, stripe_h, c) };
    }

    // --- Bouncing shapes ---
    let bounce_y = 280.0 * sy + ((f as f32) * 0.05).sin() * 20.0 * sy;
    unsafe { fill_rect(520.0 * sx, bounce_y, 30.0 * sx, 30.0 * sy, PINK) };
    unsafe { stroke_rect(518.0 * sx, bounce_y - 2.0 * sy, 34.0 * sx, 34.0 * sy, CYAN, 2.0) };

    let bounce_y2 = 270.0 * sy + ((f as f32) * 0.07 + 1.5).sin() * 25.0 * sy;
    unsafe { fill_rect(560.0 * sx, bounce_y2, 20.0 * sx, 20.0 * sy, ORANGE) };
    unsafe { stroke_rect(558.0 * sx, bounce_y2 - 2.0 * sy, 24.0 * sx, 24.0 * sy, WHITE, 1.5) };

    // --- Text (scale font size to viewport) ---
    text("Hello from Wytti!", 0.22 * w, 0.42 * h, 36.0 * sy.min(sx), font, WHITE);
    text("WASI guest -> Fytti host", 0.27 * w, 0.50 * h, 18.0 * sy.min(sx), font, CYAN);
    text("demo-app running", 10.0, h - 20.0, 12.0 * sy.min(sx), font, WHITE);

    // --- Flush ---
    unsafe { present() };
    unsafe { request_frame() };
    unsafe { FRAME = FRAME.wrapping_add(1) };
}

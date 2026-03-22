// Wytti Demo — the full showcase.
// Gradient skies, ellipse sun, interactive cityscape, mouse tracking.
// Proves every host function: gradients, ellipses, poll_mouse, the lot.
//
// Controls:
//   Arrow keys — move player
//   Space      — cycle sky palette
//   Escape     — pause/unpause
//   Mouse      — cursor light follows mouse, click to drop marker

#[link(wasm_import_module = "fytti")]
#[allow(dead_code)]
extern "C" {
    fn clear(color: u32);
    fn fill_rect(x: f32, y: f32, w: f32, h: f32, color: u32);
    fn stroke_rect(x: f32, y: f32, w: f32, h: f32, color: u32, width: f32);
    fn draw_line(x1: f32, y1: f32, x2: f32, y2: f32, color: u32, width: f32);
    fn draw_text(
        text_ptr: u32, text_len: u32,
        x: f32, y: f32, size: f32, font_id: u32, color: u32,
    );
    fn present();
    fn poll_event() -> u64;
    fn poll_mouse() -> u64;
    fn load_font(name_ptr: u32, name_len: u32) -> u32;
    fn set_title(text_ptr: u32, text_len: u32);
    fn request_frame();
    fn get_width() -> u32;
    fn get_height() -> u32;
    fn gradient_rect(x: f32, y: f32, w: f32, h: f32, c1: u32, c2: u32, vertical: u32);
    fn fill_ellipse(cx: f32, cy: f32, rx: f32, ry: f32, color: u32);
}

const fn rgba(r: u8, g: u8, b: u8, a: u8) -> u32 {
    (r as u32) << 24 | (g as u32) << 16 | (b as u32) << 8 | a as u32
}

fn text(s: &str, x: f32, y: f32, size: f32, font: u32, color: u32) {
    unsafe { draw_text(s.as_ptr() as u32, s.len() as u32, x, y, size, font, color) }
}

// --- Event constants ---
const EV_KEY_DOWN: u64 = 1;
const EV_KEY_UP: u64 = 2;
const EV_MOUSE_CLICK: u64 = 4;
const KEY_UP: u32 = 1;
const KEY_DOWN: u32 = 2;
const KEY_LEFT: u32 = 3;
const KEY_RIGHT: u32 = 4;
const KEY_SPACE: u32 = 5;
const KEY_ESCAPE: u32 = 7;

// --- App state ---
static mut FRAME: u32 = 0;
static mut FONT: u32 = 0;
static mut PAUSED: bool = false;

// Player
static mut PX: f32 = 300.0;
static mut PY: f32 = 250.0;
const SPEED: f32 = 4.0;

// Input
static mut K_UP: bool = false;
static mut K_DOWN: bool = false;
static mut K_LEFT: bool = false;
static mut K_RIGHT: bool = false;

// Mouse
static mut MOUSE_X: f32 = 320.0;
static mut MOUSE_Y: f32 = 240.0;

// Sky
static mut SKY_IDX: u32 = 0;
// Each sky: (top_color, bottom_color)
const SKIES: [(u32, u32); 5] = [
    (rgba(10, 10, 40, 255), rgba(50, 60, 120, 255)),     // night
    (rgba(5, 0, 20, 255), rgba(20, 10, 50, 255)),        // deep night
    (rgba(60, 20, 60, 255), rgba(180, 80, 60, 255)),     // dusk
    (rgba(20, 50, 80, 255), rgba(80, 140, 180, 255)),    // twilight
    (rgba(255, 120, 30, 255), rgba(255, 200, 80, 255)),  // sunset
];

// Markers
static mut MARKS: [(f32, f32, u32); 24] = [(0.0, 0.0, 0); 24]; // x, y, frame_born
static mut MARK_N: u32 = 0;
static mut MARK_I: u32 = 0;

// Colors
const WHITE: u32 = rgba(255, 255, 255, 255);
const DARK: u32 = rgba(30, 30, 30, 255);
const WIN_LIT: u32 = rgba(255, 220, 80, 255);
const CYAN: u32 = rgba(80, 220, 240, 255);
const STRIPE_A: u32 = rgba(255, 60, 80, 255);
const STRIPE_B: u32 = rgba(60, 80, 255, 255);

fn main() {
    let t = "Wytti Demo";
    unsafe { set_title(t.as_ptr() as u32, t.len() as u32) };
    let f = "default";
    unsafe { FONT = load_font(f.as_ptr() as u32, f.len() as u32) };
}

#[unsafe(no_mangle)]
pub extern "C" fn frame() {
    let f = unsafe { FRAME };
    let font = unsafe { FONT };

    // --- Input ---
    loop {
        let ev = unsafe { poll_event() };
        if ev == 0 { break; }
        let et = ev >> 56;
        let pl = (ev & 0x00FFFFFFFFFFFFFF) as u32;
        match et {
            EV_KEY_DOWN => {
                if pl == KEY_ESCAPE { unsafe { PAUSED = !PAUSED; } }
                if !unsafe { PAUSED } {
                    match pl {
                        KEY_UP => unsafe { K_UP = true },
                        KEY_DOWN => unsafe { K_DOWN = true },
                        KEY_LEFT => unsafe { K_LEFT = true },
                        KEY_RIGHT => unsafe { K_RIGHT = true },
                        KEY_SPACE => unsafe { SKY_IDX = (SKY_IDX + 1) % SKIES.len() as u32 },
                        _ => {}
                    }
                }
            }
            EV_KEY_UP => match pl {
                KEY_UP => unsafe { K_UP = false },
                KEY_DOWN => unsafe { K_DOWN = false },
                KEY_LEFT => unsafe { K_LEFT = false },
                KEY_RIGHT => unsafe { K_RIGHT = false },
                _ => {}
            },
            EV_MOUSE_CLICK => {
                if !unsafe { PAUSED } {
                    unsafe {
                        let idx = MARK_I as usize % 24;
                        MARKS[idx] = (MOUSE_X, MOUSE_Y, f);
                        MARK_I += 1;
                        if MARK_N < 24 { MARK_N += 1; }
                    }
                }
            }
            _ => {}
        }
    }

    // Poll mouse position
    let mp = unsafe { poll_mouse() };
    if mp != 0 {
        unsafe {
            MOUSE_X = f32::from_bits((mp >> 32) as u32);
            MOUSE_Y = f32::from_bits(mp as u32);
        }
    }

    // Paused — just keep the frame loop alive
    if unsafe { PAUSED } {
        // Draw pause overlay
        let w = unsafe { get_width() } as f32;
        let h = unsafe { get_height() } as f32;
        unsafe { fill_rect(0.0, 0.0, w, h, rgba(0, 0, 0, 150)) };
        let s = (w / 640.0).min(h / 480.0);
        text("PAUSED", w * 0.38, h * 0.45, 40.0 * s, font, WHITE);
        text("press Escape to resume", w * 0.30, h * 0.55, 16.0 * s, font, CYAN);
        unsafe { present() };
        unsafe { request_frame() };
        return;
    }

    // --- Move player ---
    unsafe {
        if K_UP { PY -= SPEED; }
        if K_DOWN { PY += SPEED; }
        if K_LEFT { PX -= SPEED; }
        if K_RIGHT { PX += SPEED; }
        PX = PX.clamp(0.0, 620.0);
        PY = PY.clamp(0.0, 460.0);
    }

    // --- Draw ---
    let w = unsafe { get_width() } as f32;
    let h = unsafe { get_height() } as f32;
    let sx = w / 640.0;
    let sy = h / 480.0;
    let s = sx.min(sy);
    let ground_y = 0.667 * h;

    // Sky gradient
    let (sky_top, sky_bot) = SKIES[unsafe { SKY_IDX } as usize % SKIES.len()];
    unsafe { gradient_rect(0.0, 0.0, w, ground_y, sky_top, sky_bot, 1) };

    // Stars (twinkle)
    let stars: [(f32, f32); 14] = [
        (0.09, 0.06), (0.28, 0.11), (0.50, 0.04), (0.70, 0.09),
        (0.84, 0.03), (0.16, 0.17), (0.41, 0.15), (0.63, 0.13),
        (0.78, 0.19), (0.23, 0.05), (0.55, 0.18), (0.91, 0.10),
        (0.35, 0.02), (0.07, 0.14),
    ];
    for (i, (nx, ny)) in stars.iter().enumerate() {
        if ((f.wrapping_add(i as u32 * 19)) % 50) < 35 {
            unsafe { fill_rect(nx * w, ny * h, 2.0 * s, 2.0 * s, WHITE) };
        }
    }

    // Sun — ellipse with glow
    let bob = ((f as f32) * 0.025).sin() * 12.0 * sy;
    let sun_cx = 0.78 * w;
    let sun_cy = 0.17 * h + bob;
    let sun_rx = 28.0 * sx;
    let sun_ry = 28.0 * sy;
    // Glow layers
    unsafe {
        fill_ellipse(sun_cx, sun_cy, sun_rx * 2.5, sun_ry * 2.5, rgba(255, 240, 100, 25));
        fill_ellipse(sun_cx, sun_cy, sun_rx * 1.6, sun_ry * 1.6, rgba(255, 240, 100, 50));
        fill_ellipse(sun_cx, sun_cy, sun_rx, sun_ry, rgba(255, 210, 60, 255));
    }

    // Sun rays
    for i in 0..8 {
        let angle_off = (i as f32) * 20.0 * sx;
        let end_x = sun_cx + angle_off * 2.5 - 200.0 * sx;
        unsafe { draw_line(sun_cx, sun_cy, end_x, ground_y, rgba(255, 240, 100, 30), 1.5) };
    }

    // Ground gradient
    unsafe { gradient_rect(0.0, ground_y, w, h - ground_y,
        rgba(40, 120, 50, 255), rgba(20, 60, 25, 255), 1) };

    // --- Cityscape ---
    let buildings: [(f32, f32, f32, u32); 7] = [
        (0.06, 0.25, 0.09, rgba(80, 80, 100, 255)),
        (0.17, 0.17, 0.08, rgba(70, 70, 90, 255)),
        (0.27, 0.29, 0.11, rgba(90, 85, 100, 255)),
        (0.41, 0.21, 0.09, rgba(75, 75, 95, 255)),
        (0.52, 0.27, 0.10, rgba(85, 80, 100, 255)),
        (0.64, 0.19, 0.08, rgba(70, 65, 85, 255)),
        (0.74, 0.23, 0.12, rgba(78, 72, 92, 255)),
    ];
    for (nx, ny, nw, bc) in &buildings {
        let bx = nx * w;
        let top = ny * h;
        let bw = nw * w;
        let bh = ground_y - top;
        unsafe { fill_rect(bx, top, bw, bh, *bc) };
        // Windows
        let ww = 8.0 * sx;
        let wh = 12.0 * sy;
        let gap_x = 14.0 * sx;
        let gap_y = 18.0 * sy;
        let cols = ((bw - 4.0 * sx) / gap_x) as u32;
        let rows = ((bh - 6.0 * sy) / gap_y) as u32;
        for row in 0..rows {
            for col in 0..cols {
                let wx = bx + 4.0 * sx + (col as f32) * gap_x;
                let wy = top + 6.0 * sy + (row as f32) * gap_y;
                let lit = ((col.wrapping_mul(7).wrapping_add(row.wrapping_mul(13)).wrapping_add(f / 30)) % 3) == 0;
                unsafe { fill_rect(wx, wy, ww, wh, if lit { WIN_LIT } else { DARK }) };
            }
        }
    }

    // --- Retro stripes ---
    let stripe_y = h * 0.85;
    let stripe_h = (h - stripe_y) / 8.0;
    for i in 0..8 {
        let y = stripe_y + (i as f32) * stripe_h;
        unsafe { fill_rect(0.0, y, w, stripe_h, if i % 2 == 0 { STRIPE_A } else { STRIPE_B }) };
    }

    // --- Click markers ---
    let n = unsafe { MARK_N } as usize;
    let next = unsafe { MARK_I } as usize;
    for i in 0..n {
        let idx = if next >= n { next - n + i } else { i } % 24;
        let (mx, my, born) = unsafe { MARKS[idx] };
        if mx > 0.0 || my > 0.0 {
            let age = f.wrapping_sub(born) as f32;
            let fade = (1.0 - age / 600.0).max(0.2);
            let pulse = 1.0 + (age * 0.08).sin() * 0.4;
            let r = 5.0 * pulse * s;
            let alpha = (fade * 220.0) as u8;
            unsafe {
                fill_ellipse(mx, my, r, r, rgba(255, 200, 50, alpha));
                fill_ellipse(mx, my, r * 0.5, r * 0.5, rgba(255, 255, 200, alpha));
            }
        }
    }

    // --- Mouse cursor glow ---
    let mx = unsafe { MOUSE_X };
    let my = unsafe { MOUSE_Y };
    unsafe {
        fill_ellipse(mx, my, 30.0 * s, 30.0 * s, rgba(255, 255, 255, 15));
        fill_ellipse(mx, my, 12.0 * s, 12.0 * s, rgba(255, 255, 255, 30));
    }

    // --- Player ---
    let px = unsafe { PX } * sx;
    let py = unsafe { PY } * sy;
    let ps = 20.0 * s;
    // Shadow
    unsafe { fill_ellipse(px + ps * 0.5 + 3.0, py + ps + 2.0, ps * 0.5, ps * 0.15, rgba(0, 0, 0, 60)) };
    // Body
    unsafe { fill_rect(px, py, ps, ps, rgba(255, 80, 200, 255)) };
    // Border pulse
    let pw = 2.0 + ((f as f32) * 0.1).sin().abs();
    unsafe { stroke_rect(px - 1.0, py - 1.0, ps + 2.0, ps + 2.0, WHITE, pw) };

    // --- Text ---
    text("Hello from Wytti!", 0.22 * w, 0.42 * h, 36.0 * s, font, WHITE);
    text("arrows: move | space: sky | esc: pause | click: mark", 0.08 * w, 0.52 * h, 13.0 * s, font, CYAN);

    // --- Flush ---
    unsafe { present() };
    unsafe { request_frame() };
    unsafe { FRAME = f.wrapping_add(1) };
}

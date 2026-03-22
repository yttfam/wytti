use crate::canvas::{Color, Rect};
use crate::input::InputEvent;
use crate::resources::ResourceId;

/// The host backend trait. Fytti implements this with real GPU rendering.
/// CLI/test environments use StubBackend.
///
/// Every `fytti_*` host function maps 1:1 to a method here.
pub trait HostBackend: Send + 'static {
    // --- Rendering ---

    /// Clear the screen to a solid color.
    fn clear(&mut self, color: Color);

    /// Fill a rectangle.
    fn fill_rect(&mut self, rect: Rect, color: Color);

    /// Stroke a rectangle outline.
    fn stroke_rect(&mut self, rect: Rect, color: Color, width: f32);

    /// Draw a line.
    fn draw_line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, color: Color, width: f32);

    /// Draw text at a position.
    fn draw_text(&mut self, text: &str, x: f32, y: f32, size: f32, font: ResourceId, color: Color);

    /// Draw an image.
    fn draw_image(&mut self, image: ResourceId, x: f32, y: f32, w: f32, h: f32);

    /// Flush the current frame to the screen.
    fn present(&mut self);

    // --- Input ---

    /// Poll for the next input event. Returns None if no events pending.
    fn poll_event(&mut self) -> Option<InputEvent>;

    // --- Resources ---

    /// Load a font by name. Returns a resource handle.
    fn load_font(&mut self, name: &str) -> ResourceId;

    /// Load an image from a URL or path. Returns a resource handle.
    fn load_image(&mut self, url: &str) -> ResourceId;

    // --- System ---

    /// Set the window title.
    fn set_title(&mut self, title: &str);

    /// Request the next animation frame.
    fn request_frame(&mut self);

    /// Read from clipboard.
    fn clipboard_read(&mut self) -> Option<String>;

    /// Write to clipboard.
    fn clipboard_write(&mut self, text: &str);
}

/// A stub backend that does nothing. For CLI mode and tests.
pub struct StubBackend {
    pub title: String,
    pub frame_requested: bool,
    pub clipboard: String,
    pub draw_calls: Vec<DrawCall>,
}

/// Recorded draw call for testing/inspection.
#[derive(Debug, Clone)]
pub enum DrawCall {
    Clear(Color),
    FillRect(Rect, Color),
    StrokeRect(Rect, Color, f32),
    DrawLine(f32, f32, f32, f32, Color, f32),
    DrawText(String, f32, f32, f32, ResourceId, Color),
    DrawImage(ResourceId, f32, f32, f32, f32),
    Present,
}

impl StubBackend {
    pub fn new() -> Self {
        Self {
            title: String::new(),
            frame_requested: false,
            clipboard: String::new(),
            draw_calls: Vec::new(),
        }
    }
}

impl Default for StubBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl HostBackend for StubBackend {
    fn clear(&mut self, color: Color) {
        self.draw_calls.push(DrawCall::Clear(color));
    }

    fn fill_rect(&mut self, rect: Rect, color: Color) {
        self.draw_calls.push(DrawCall::FillRect(rect, color));
    }

    fn stroke_rect(&mut self, rect: Rect, color: Color, width: f32) {
        self.draw_calls.push(DrawCall::StrokeRect(rect, color, width));
    }

    fn draw_line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, color: Color, width: f32) {
        self.draw_calls
            .push(DrawCall::DrawLine(x1, y1, x2, y2, color, width));
    }

    fn draw_text(
        &mut self,
        text: &str,
        x: f32,
        y: f32,
        size: f32,
        font: ResourceId,
        color: Color,
    ) {
        self.draw_calls
            .push(DrawCall::DrawText(text.to_string(), x, y, size, font, color));
    }

    fn draw_image(&mut self, image: ResourceId, x: f32, y: f32, w: f32, h: f32) {
        self.draw_calls
            .push(DrawCall::DrawImage(image, x, y, w, h));
    }

    fn present(&mut self) {
        self.draw_calls.push(DrawCall::Present);
    }

    fn poll_event(&mut self) -> Option<InputEvent> {
        None
    }

    fn load_font(&mut self, _name: &str) -> ResourceId {
        ResourceId(1) // stub always returns a valid-looking ID
    }

    fn load_image(&mut self, _url: &str) -> ResourceId {
        ResourceId(2)
    }

    fn set_title(&mut self, title: &str) {
        self.title = title.to_string();
    }

    fn request_frame(&mut self) {
        self.frame_requested = true;
    }

    fn clipboard_read(&mut self) -> Option<String> {
        if self.clipboard.is_empty() {
            None
        } else {
            Some(self.clipboard.clone())
        }
    }

    fn clipboard_write(&mut self, text: &str) {
        self.clipboard = text.to_string();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_records_draw_calls() {
        let mut backend = StubBackend::new();
        backend.clear(Color::BLACK);
        backend.fill_rect(Rect::new(10.0, 20.0, 100.0, 50.0), Color::rgb(255, 0, 0));
        backend.draw_text("hello", 10.0, 30.0, 16.0, ResourceId(1), Color::WHITE);
        backend.present();

        assert_eq!(backend.draw_calls.len(), 4);
        assert!(matches!(backend.draw_calls[0], DrawCall::Clear(_)));
        assert!(matches!(backend.draw_calls[1], DrawCall::FillRect(_, _)));
        assert!(matches!(backend.draw_calls[2], DrawCall::DrawText(..)));
        assert!(matches!(backend.draw_calls[3], DrawCall::Present));
    }

    #[test]
    fn stub_clipboard() {
        let mut backend = StubBackend::new();
        assert!(backend.clipboard_read().is_none());

        backend.clipboard_write("copied text");
        assert_eq!(backend.clipboard_read().unwrap(), "copied text");
    }

    #[test]
    fn stub_title() {
        let mut backend = StubBackend::new();
        backend.set_title("My App");
        assert_eq!(backend.title, "My App");
    }

    #[test]
    fn stub_resources() {
        let mut backend = StubBackend::new();
        let font = backend.load_font("monospace");
        let image = backend.load_image("icon.png");
        assert!(font.is_valid());
        assert!(image.is_valid());
    }

    #[test]
    fn color_roundtrip() {
        let c = Color::rgb(100, 150, 200);
        let packed = c.to_u32();
        let unpacked = Color::from_u32(packed);
        assert_eq!(c, unpacked);
    }

    #[test]
    fn rect_contains() {
        let r = Rect::new(10.0, 10.0, 100.0, 50.0);
        assert!(r.contains(50.0, 30.0));
        assert!(!r.contains(5.0, 5.0));
        assert!(!r.contains(200.0, 200.0));
    }
}

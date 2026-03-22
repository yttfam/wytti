/// Keyboard key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Up,
    Down,
    Left,
    Right,
    Space,
    Enter,
    Escape,
    Backspace,
    Tab,
    Char(char),
}

/// Mouse button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// Mouse event.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MouseEvent {
    pub x: f32,
    pub y: f32,
    pub button: MouseButton,
    pub pressed: bool,
}

/// An input event from the host.
#[derive(Debug, Clone, Copy)]
pub enum InputEvent {
    KeyDown(Key),
    KeyUp(Key),
    MouseMove { x: f32, y: f32 },
    MouseClick(MouseEvent),
    Scroll { dx: f32, dy: f32 },
    Resize { width: u32, height: u32 },
}

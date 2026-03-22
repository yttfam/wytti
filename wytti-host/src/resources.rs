/// Opaque handle to a host resource (font, image, etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResourceId(pub u32);

impl ResourceId {
    pub const INVALID: Self = Self(0);

    pub fn is_valid(self) -> bool {
        self.0 != 0
    }
}

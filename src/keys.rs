//! Key types: crossterm's in a terminal, a minimal copy in the browser (crossterm does not build for wasm32).
#[cfg(not(target_arch = "wasm32"))]
pub use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[cfg(target_arch = "wasm32")]
pub use web::*;

#[cfg(target_arch = "wasm32")]
mod web {
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub enum KeyCode {
        Char(char), F(u8), Backspace, Enter, Up, Down, Left, Right, Tab, BackTab, Delete, Home, End, PageUp, PageDown, Esc,
    }

    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub struct KeyModifiers(u8);

    impl KeyModifiers {
        pub const NONE: Self = Self(0);
        pub const CONTROL: Self = Self(1);
        pub fn contains(self, other: Self) -> bool { self.0 & other.0 == other.0 }
    }

    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub struct KeyEvent { pub code: KeyCode, pub modifiers: KeyModifiers }
}

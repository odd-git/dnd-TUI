//! Browser entry point: the App lives here and the page (web/app.js) drives it.
//! The page asks for `view()` (JSON), draws it as HTML and sends back commands, answers and keys.
use std::cell::RefCell;

use wasm_bindgen::prelude::*;

use crate::app::{App, Cmd};
use crate::keys::{KeyCode, KeyEvent, KeyModifiers};
use crate::{store, view};

thread_local! { static APP: RefCell<Option<App>> = const { RefCell::new(None) }; }

fn with<T>(f: impl FnOnce(&mut App) -> T) -> T {
    APP.with(|a| {
        let mut a = a.borrow_mut();
        let app = a.get_or_insert_with(App::new);
        let out = f(app);
        app.save();
        out
    })
}

pub fn run() { console_error_panic_hook::set_once(); }

#[wasm_bindgen]
pub fn view() -> String {
    with(|a| {
        if store::take_imported() { a.reload_sheets(); }
        view::view(a).to_string()
    })
}

/// A command as the view listed it, e.g. `"Damage"` or `{"Roll":"DEX save"}`.
#[wasm_bindgen]
pub fn exec(cmd: &str) {
    if let Ok(c) = serde_json::from_str::<Cmd>(cmd) { with(|a| a.exec(c)); }
}

#[wasm_bindgen]
pub fn submit(text: &str) { with(|a| a.submit(text)); }

#[wasm_bindgen]
pub fn cancel() { with(|a| a.cancel()); }

#[wasm_bindgen]
pub fn set_query(q: &str) { with(|a| a.set_query(q)); }

/// A keydown from the page (desktop keyboards); false when the key means nothing here.
#[wasm_bindgen]
pub fn key(key: &str, ctrl: bool, shift: bool) -> bool {
    let mut chars = key.chars();
    let code = match (chars.next(), chars.next()) {
        (Some(c), None) => KeyCode::Char(c),
        _ => match key {
            "Backspace" => KeyCode::Backspace, "Enter" => KeyCode::Enter, "Escape" => KeyCode::Esc, "Delete" => KeyCode::Delete,
            "ArrowUp" => KeyCode::Up, "ArrowDown" => KeyCode::Down, "ArrowLeft" => KeyCode::Left, "ArrowRight" => KeyCode::Right,
            "Tab" if shift => KeyCode::BackTab, "Tab" => KeyCode::Tab,
            "Home" => KeyCode::Home, "End" => KeyCode::End, "PageUp" => KeyCode::PageUp, "PageDown" => KeyCode::PageDown,
            f if f.starts_with('F') => match f[1..].parse() { Ok(n) => KeyCode::F(n), Err(_) => return false },
            _ => return false,
        },
    };
    with(|a| a.on_key(KeyEvent { code, modifiers: if ctrl { KeyModifiers::CONTROL } else { KeyModifiers::NONE } }));
    true
}

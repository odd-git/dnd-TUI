//! Browser entry point: the same App drawn on a canvas by ratzilla, with keys and clicks forwarded to it.
use std::{cell::RefCell, rc::Rc};

use ratzilla::ratatui::Terminal;
use ratzilla::{CanvasBackend, WebRenderer};
use web_sys::wasm_bindgen::{convert::FromWasmAbi, prelude::Closure, JsCast};

use crate::app::App;
use crate::keys::{KeyCode, KeyEvent, KeyModifiers};
use crate::ui;

/// The canvas backend draws fixed 10x19 pixel cells.
const CELL: (i32, i32) = (10, 19);

fn key_code(e: &web_sys::KeyboardEvent) -> Option<KeyCode> {
    let k = e.key();
    let mut chars = k.chars();
    if let (Some(c), None) = (chars.next(), chars.next()) { return Some(KeyCode::Char(c)); }
    Some(match k.as_str() {
        "Backspace" => KeyCode::Backspace, "Enter" => KeyCode::Enter, "Escape" => KeyCode::Esc, "Delete" => KeyCode::Delete,
        "ArrowUp" => KeyCode::Up, "ArrowDown" => KeyCode::Down, "ArrowLeft" => KeyCode::Left, "ArrowRight" => KeyCode::Right,
        "Tab" if e.shift_key() => KeyCode::BackTab, "Tab" => KeyCode::Tab,
        "Home" => KeyCode::Home, "End" => KeyCode::End, "PageUp" => KeyCode::PageUp, "PageDown" => KeyCode::PageDown,
        f if f.starts_with('F') => KeyCode::F(f[1..].parse().ok()?),
        _ => return None,
    })
}

fn listen<E: FromWasmAbi + 'static>(name: &str, mut f: impl FnMut(E) + 'static) {
    let cb = Closure::<dyn FnMut(E)>::new(move |e: E| f(e));
    let doc = web_sys::window().and_then(|w| w.document()).expect("document");
    let _ = doc.add_event_listener_with_callback(name, cb.as_ref().unchecked_ref());
    cb.forget();
}

pub fn run() {
    console_error_panic_hook::set_once();
    let terminal = Terminal::new(CanvasBackend::new().expect("canvas")).expect("terminal");
    let app = Rc::new(RefCell::new(App::new()));

    listen("keydown", {
        let app = app.clone();
        move |e: web_sys::KeyboardEvent| {
            if e.ctrl_key() && e.key() != "c" || e.meta_key() || e.alt_key() { return; } // browser shortcuts stay the browser's
            let Some(code) = key_code(&e) else { return };
            if e.key() == "F5" || e.key() == "F12" { return; }
            e.prevent_default(); // Tab, "/" and the arrows would otherwise act on the page
            let modifiers = if e.ctrl_key() { KeyModifiers::CONTROL } else { KeyModifiers::NONE };
            let mut a = app.borrow_mut();
            a.on_key(KeyEvent { code, modifiers });
            a.save();
        }
    });
    listen("mousedown", {
        let app = app.clone();
        move |e: web_sys::MouseEvent| {
            if e.button() != 0 { return; }
            let mut a = app.borrow_mut();
            a.on_click((e.client_x() / CELL.0) as u16, (e.client_y() / CELL.1) as u16);
            a.save();
        }
    });
    listen("wheel", {
        let app = app.clone();
        move |e: web_sys::WheelEvent| app.borrow_mut().on_scroll(e.delta_y() < 0.0)
    });

    terminal.draw_web(move |f| {
        let mut a = app.borrow_mut();
        let hot = ui::draw(f, &a);
        a.hot = hot;
    });
}

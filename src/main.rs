mod app;
mod builder;
mod lists;
mod model;
mod pdf;
mod ui;

use std::collections::VecDeque;
use std::io::Write;
use std::sync::Mutex;

use ratatui::crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind, MouseButton, MouseEventKind};
use ratatui::crossterm::execute;

/// The last input events, written to crash.log so a crash says what led to it.
static RECENT: Mutex<VecDeque<String>> = Mutex::new(VecDeque::new());

fn remember(e: &Event) {
    let line = match e {
        Event::Key(k) => format!("key {:?}{}", k.code, if k.modifiers.is_empty() { String::new() } else { format!(" {:?}", k.modifiers) }),
        Event::Mouse(m) => format!("mouse {:?} at {},{}", m.kind, m.column, m.row),
        _ => return,
    };
    let mut r = RECENT.lock().unwrap_or_else(|p| p.into_inner());
    if r.len() == 40 { r.pop_front(); }
    r.push_back(line);
}

/// Append a crash report (message, backtrace, recent input) to crash.log in the data dir.
fn crash_log(what: &str) {
    let path = app::data_dir().join("crash.log");
    let _ = std::fs::create_dir_all(app::data_dir());
    let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) else { return };
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let recent = RECENT.lock().map(|r| r.iter().cloned().collect::<Vec<_>>().join("\n  ")).unwrap_or_default();
    let _ = writeln!(f, "=== crash at unix time {secs}, dnd-tui {}\n{what}\n{}\nlast input:\n  {recent}\n",
        env!("CARGO_PKG_VERSION"), std::backtrace::Backtrace::force_capture());
    eprintln!("dnd-tui crashed, details in {}", path.display());
}

fn main() -> std::io::Result<()> {
    let mut term = ratatui::init();
    let restore_terminal = std::panic::take_hook(); // ratatui's: puts the terminal back
    std::panic::set_hook(Box::new(move |info| {
        let _ = execute!(std::io::stdout(), DisableMouseCapture);
        restore_terminal(info);
        crash_log(&info.to_string());
    }));
    execute!(std::io::stdout(), EnableMouseCapture)?;
    let mut app = app::App::new();
    let result = (|| -> std::io::Result<()> {
        while !app.quit {
            let mut hot = vec![];
            term.draw(|f| hot = ui::draw(f, &app))?;
            app.hot = hot;
            let e = event::read()?;
            remember(&e);
            match e {
                Event::Key(k) if k.kind == KeyEventKind::Press => app.on_key(k),
                Event::Mouse(m) => match m.kind {
                    MouseEventKind::Down(MouseButton::Left) => app.on_click(m.column, m.row),
                    MouseEventKind::ScrollUp => app.on_scroll(true),
                    MouseEventKind::ScrollDown => app.on_scroll(false),
                    _ => {}
                },
                _ => {}
            }
            app.save();
        }
        Ok(())
    })();
    let _ = execute!(std::io::stdout(), DisableMouseCapture);
    ratatui::restore();
    if let Err(e) = &result { crash_log(&format!("terminal error: {e}")); }
    result
}

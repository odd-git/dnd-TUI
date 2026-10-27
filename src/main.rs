mod app;
mod model;
mod ui;

use ratatui::crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind, MouseButton, MouseEventKind};
use ratatui::crossterm::execute;

fn main() -> std::io::Result<()> {
    let mut term = ratatui::init();
    execute!(std::io::stdout(), EnableMouseCapture)?;
    let mut app = app::App::new();
    while !app.quit {
        let mut hot = vec![];
        term.draw(|f| hot = ui::draw(f, &app))?;
        app.hot = hot;
        match event::read()? {
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
    execute!(std::io::stdout(), DisableMouseCapture)?;
    ratatui::restore();
    Ok(())
}

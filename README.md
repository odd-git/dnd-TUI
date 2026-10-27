# dnd-TUI
Terminal character sheets for D&D 5e sessions, written in Rust ([ratatui](https://ratatui.rs)), inspired by [pythops](https://github.com/pythops/bluetui)' work.

## Run
```
cargo build --release
install -Dm755 target/release/dnd-tui ~/.local/bin/dnd-tui
dnd-tui
```
Pick a sheet at start. Keys are shown in the footer and every footer button, list row, check and tab is clickable.

## Sheets are data
Each character is one JSON file in `~/.local/share/dnd-tui/` (sheet and live state together, saved after every action). The bundled sheets in `data/characters/` are copied there on first run; delete a file to reset it, drop a new `.json` there to add a character. Class progression tables live in `data/progressions/` (Fighter / Battle Master so far, level up is disabled for sheets without one).

## Python version
The original Python scripts (`main.py`, `builder.py`, `combat_engine.py`, `sorlock.py`, `rowan.py`) are still here until the Rust version replaces them.

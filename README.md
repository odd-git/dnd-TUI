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
Each character is one JSON file in `~/.local/share/dnd-tui/` (sheet and live state together, saved after every action). The bundled sheets in `data/characters/` are copied there on first run; delete a file to reset it, drop a new `.json` there to add a character. Press `N` in the sheet picker to create a character with the wizard (`D` deletes a sheet, `E` in play opens the full sheet editor) (race, class, background, scores, skills, gear). Official lists (races, backgrounds, feats, weapons, armor, classes) live in `data/lists/`, generated from dnd5e.wikidot.com by `tools/fetch_lists.py`; Fighter, Barbarian, Rogue, Monk, Paladin and Ranger (without spells yet) are available so far (`tools/fetch_classes.py` builds the class files), level up is disabled for sheets without a class progression.

## Python version
The original Python scripts (`main.py`, `builder.py`, `combat_engine.py`, `sorlock.py`, `rowan.py`) are still here until the Rust version replaces them.

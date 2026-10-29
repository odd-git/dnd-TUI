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
Each character is one JSON file in `~/.local/share/dnd-tui/` (sheet and live state together, saved after every action). The bundled sheets in `data/characters/` are copied there on first run; delete a file to reset it, drop a new `.json` there to add a character. Press `N` in the sheet picker to create a character with the wizard (`D` deletes a sheet, `E` in play opens the full sheet editor) (race, class, background, scores, skills, gear). Official lists (races, backgrounds, feats, weapons, armor, classes) live in `data/lists/`, generated from dnd5e.wikidot.com by `tools/fetch_lists.py`; all 12 PHB classes are available, with Eldritch Knight and Arcane Trickster casting (`tools/fetch_classes.py` builds the class files, `tools/fetch_spells.py` the 510 official spells). Casters get a **Spells** tab (F2): slots, `Enter`/`c` casts (upcast, attack roll or save DC, damage), `p` prepares, `e` edits; the wizard and level up ask for cantrips, known/spellbook spells and prepared spells. Multiclassing: `U` asks which class to level (or a new one whose 13-point ability requirements you meet); slots pool as in the PHB, HP uses each class's hit die. A second class gets its features, resources, actions, toggles (Rage), multiclass proficiencies (plus the skill Bard/Ranger/Rogue grant), fighting style and expertise, and its own hit dice pool (a short rest asks each pool in turn); spell limits are only checked for the first casting class. Class mechanics: Metamagic, Eldritch Invocations (with prerequisites) and Pact Boons are picked when due (`tools/fetch_options.py`), Mystic Arcanum adds a once-a-day 6th-9th level spell, Magical Secrets offers any class's spells, Sorcerers convert slots and sorcery points, Arcane Recovery returns slots, Divine Smite spends a slot, Wild Shape takes the beast's HP and soaks damage with them. Opening a sheet adds class actions and choices it is missing., level up is disabled for sheets without a class progression.

## Python version
The original Python scripts (`main.py`, `builder.py`, `combat_engine.py`, `sorlock.py`, `rowan.py`) are still here until the Rust version replaces them.

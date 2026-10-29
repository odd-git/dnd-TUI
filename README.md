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
In combat `G` adds temporary HP (the higher value stays, a long rest clears them; damage takes them first). Casting a concentration spell marks it on the status box (a new one replaces it, `O` ends it); damage rolls the CON save against DC 10 or half the damage, and dropping to 0 HP ends it. If the program crashes it writes the error, a backtrace and the last 40 inputs to `~/.local/share/dnd-tui/crash.log`. Each character is one JSON file in `~/.local/share/dnd-tui/` (sheet and live state together, saved after every action). The bundled sheets in `data/characters/` are copied there on first run; delete a file to reset it, drop a new `.json` there to add a character. Press `N` in the sheet picker to create a character with the wizard (`D` deletes a sheet, `E` in play opens the full sheet editor) (race, class, background, scores, skills, gear). Official lists (races, backgrounds, feats, weapons, armor, classes) live in `data/lists/`, generated from dnd5e.wikidot.com by `tools/fetch_lists.py`; all 12 PHB classes are available, with Eldritch Knight and Arcane Trickster casting (`tools/fetch_classes.py` builds the class files, `tools/fetch_spells.py` the 510 official spells). Casters get a **Spells** tab (F2): slots, `Enter`/`c` casts (upcast, attack roll or save DC, damage), `p` prepares, `e` edits; the wizard and level up ask for cantrips, known/spellbook spells and prepared spells. Multiclassing: `U` asks which class to level (or a new one whose 13-point ability requirements you meet); slots pool as in the PHB, HP uses each class's hit die. A second class gets its features, resources, actions, toggles (Rage), multiclass proficiencies (plus the skill Bard/Ranger/Rogue grant), fighting style and expertise, and its own hit dice pool (a short rest asks each pool in turn); spell limits are only checked for the first casting class. Class mechanics: Metamagic, Eldritch Invocations (with prerequisites) and Pact Boons are picked when due (`tools/fetch_options.py`), Mystic Arcanum adds a once-a-day 6th-9th level spell, Magical Secrets offers any class's spells, Sorcerers convert slots and sorcery points, Arcane Recovery returns slots, Divine Smite spends a slot, Wild Shape offers the SRD beasts the druid can take (CR, flying and swimming limits by level, elementals for the Circle of the Moon at 10th level for two uses) and fills in their HP, AC, speeds and attacks; "Other beast" asks for the stats by hand (`Bite +4 2d4+2 piercing`): while it lasts the sheet uses the beast's AC and attacks, damage hits the beast's HP first and the leftover carries to the druid, ending the form. Opening a sheet adds class actions and choices it is missing. `Add spell` in the editor offers the class lists first and then every official spell (feats, races, DM gifts); spells from outside the class lists are always castable and never count against spells known, like the Pact of the Tome cantrips and the Pact of the Chain's Find Familiar, which are granted automatically., level up is disabled for sheets without a class progression.

## Python version
The original Python scripts (`main.py`, `builder.py`, `combat_engine.py`, `sorlock.py`, `rowan.py`) are still here until the Rust version replaces them.

## License

The code is licensed under the GNU General Public License v3.0 or later (see `LICENSE`).

The lists in `data/lists/` keep the licenses of their sources: the Creative Commons Attribution-ShareAlike 3.0 License for the data derived from dnd5e.wikidot.com and the Creative Commons Attribution 4.0 International License for the SRD 5.1 data (`beasts.json`), as detailed in Credits below.

This is an unofficial fan project, not affiliated with or endorsed by Wizards of the Coast. Dungeons & Dragons is a trademark of Wizards of the Coast LLC.

## Credits

Races, classes, subclasses, backgrounds, feats, weapons, armor, spells and class options (`data/lists/`, built by the scripts in `tools/`) are derived from the pages of the D&D 5th Edition wiki at https://dnd5e.wikidot.com, licensed under the Creative Commons Attribution-ShareAlike 3.0 License (https://creativecommons.org/licenses/by-sa/3.0/); these derived lists are shared under the same license. Each entry keeps the name of the official sourcebook it comes from. Descriptions are not copied from the books: spells in the SRD 5.1 carry their SRD text (from the Open5e API, marked `"srd": true`), every other entry carries a short mechanical summary written for this project (`data/lists/summaries.json`, merged in by the generators) and a link to its wiki page, shown as `More:` in the sheet. `tools/check_copy.py` checks that no description shares 8 or more consecutive words with the cached wiki pages.

Beast and elemental statistics (`data/lists/beasts.json`, built by `tools/fetch_beasts.py` from the Open5e API) come from the System Reference Document 5.1 by Wizards of the Coast LLC, available at https://dnd.wizards.com/resources/systems-reference-document and licensed under the Creative Commons Attribution 4.0 International License (https://creativecommons.org/licenses/by/4.0/legalcode).

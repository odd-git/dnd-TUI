# Changelog

All notable changes to this project are documented here.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added
- Artificer (Tasha's Cauldron of Everything): spell slots from 1st level, half caster rounded up when multiclassing,
  rituals when prepared, infusions chosen as they unlock (level prerequisites included), Flash of Genius from 7th level,
  Extra Attack for Armorer and Battle Smith, the four specialists with their always-prepared spells.

### Changed
- The web version is a touch-friendly page instead of the terminal drawn on a canvas: sticky HP bar, actions,
  abilities, saves, every skill and spells as buttons, questions as bottom sheets, a log you can open; works on phones.
  It looks like the terminal (green boxes with the title in the border, monospace, the same dark palette), one column
  on a phone and two on a wide screen. The terminal keys still work on a desktop keyboard. Same rules and flows as the
  terminal app, which is unchanged.

## [0.9.4] - 2026-10-30

### Changed
- The bundled Fighter / Battle Master sheet is a demo character (Rowan Ashford) instead of a real player's character.

## [0.9.3] - 2026-10-30

### Added
- Browser version: the same sheet runs in a web page (WebAssembly, drawn on a canvas), with sheets kept in the browser's
  local storage. Published on GitHub Pages at https://odd-git.github.io/dnd-TUI/.
- `E` on the sheet list saves a sheet as a JSON file; in the browser `I` loads one back.

### Changed
- All disk access goes through one module, so the terminal and browser builds share the rest of the code.

## [0.9.2] - 2026-10-30

### Added
- PDF export (`P` on the sheet list or the Character tab): an A4 character sheet to print, with abilities, saves, skills, attacks,
  resources and spell slots as boxes to tick, features, spell list, inventory and ruled notes.
- Racial spells: leveled spells of a lineage (Drow Magic, Infernal Legacy, Githyanki and Githzerai Psionics, Firbolg Magic...)
  are cast once per long rest from the character level that unlocks them, at their set slot level, with the lineage's ability.

### Changed
- CI uses actions/checkout v5.

## [0.9.1] - 2026-10-30

### Added
- Exhaustion (F): six levels, disadvantage on the rolls they affect, HP maximum halved from level 4, one level removed by a long rest.
- Heroic Inspiration (N): grant it, then spend it for advantage on the next roll.
- Ritual casting without a slot, under each class's rule (spellbook, prepared, known, Book of Ancient Secrets, Ritual Caster).
- Inventory: items with count and weight, attunement up to three, coins that make change when spent.
- Mechanics for every Player's Handbook subclass: Channel Divinity options, Divine Strike, Arcane Ward, Portent, Wild Magic surges,
  Hunter's choices, the Beast Master's companion, school restrictions for the Eldritch Knight and Arcane Trickster, and more.
  Subclasses from other books keep their features as text with a link.
- Racial cantrips and languages: the High Elf picks a wizard cantrip and an extra language; named racial cantrips
  (Light, Minor Illusion, Thaumaturgy...) are known and cast with the lineage's ability.
- Wizard: Spell Mastery casts its two spells at will, Signature Spells are always prepared and cast once per short rest without a slot.
- Jack of All Trades: half proficiency to checks without it, initiative included.
- Tough and Dwarven Toughness add their hit points per level.
- Spells that grow in steps (Shadow Blade, Spiritual Weapon, Spirit Shroud, Flame Blade); Prismatic Spray and Wall roll 10d6
  of the colour's type; Spiritual Weapon and Magic Stone add the spellcasting modifier.
- Tests of every class against the class tables at levels 1, 5, 11 and 20, single class and multiclass.
- `tools/audit_spells.py`: checks spell damage, saves and upcasting at every slot level against the Open5e API.

### Changed
- Descriptions are our own summaries with a link to the wiki page; SRD 5.1 spells keep their full text.
- The bundled Sorlock sheet is a Sorcerer 1 / Warlock 1 multiclass.
- Enter on the Spells tab casts the selected spell.
- The legacy Python scripts are gone: the Rust program replaces them.
- Code licensed under GPL-3.0-or-later; data licenses and the WotC disclaimer are in the README.

### Fixed
- Crash drawing the HP bar when HP is above the maximum.
- Crash creating a 1st-level Paladin or Ranger in the wizard.
- Level up no longer charges the Arcane Ward.
- Martial Arts and finesse picked the wrong ability on attacks added after creation.
- Paladins have one Channel Divinity use per rest at every level.
- Druids have unlimited Wild Shape at 20th level (Archdruid).
- Spells with damage or saves in bullet lists, several damage types, or upcasting worded differently now roll them.
- The blowgun deals 1 + DEX piercing damage.
- Finesse weapons use the better of STR and DEX when rolled, so they follow later ability score increases.
- Initiative counts as a DEX check, so Remarkable Athlete applies.

## [0.1.9] - 2026-10-29

### Added
- Character creation wizard, official data lists from dnd5e.wikidot.com, and search in every list.
- Sheet editor and sheet deletion.
- All twelve classes, spellcasting (slots, Spells tab, cast and prepare) and multiclassing with pooled slots.
- Class mechanics: Metamagic, Invocations, Pact Boon, Mystic Arcanum, Magical Secrets, Flexible Casting, Arcane Recovery,
  Divine Smite, Wild Shape with SRD 5.1 beasts.
- Temporary HP and concentration with automatic CON saves.
- `crash.log` with a backtrace and the last input on a panic or terminal error.

## [0.1.0] - 2026-10-27

### Added
- Rust rewrite of the sheet tracker (ratatui), with a Fighter / Battle Master sheet.

[Unreleased]: https://github.com/odd-git/dnd-TUI/compare/v0.9.4...HEAD
[0.9.4]: https://github.com/odd-git/dnd-TUI/compare/v0.9.3...v0.9.4
[0.9.3]: https://github.com/odd-git/dnd-TUI/compare/v0.9.2...v0.9.3
[0.9.2]: https://github.com/odd-git/dnd-TUI/compare/v0.9.1...v0.9.2
[0.9.1]: https://github.com/odd-git/dnd-TUI/compare/v0.1.9...v0.9.1
[0.1.9]: https://github.com/odd-git/dnd-TUI/compare/c22fc8a...v0.1.9
[0.1.0]: https://github.com/odd-git/dnd-TUI/commit/c22fc8a

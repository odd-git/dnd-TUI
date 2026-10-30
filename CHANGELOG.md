# Changelog

All notable changes to this project are documented here.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added
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
- Rust rewrite of the sheet tracker (ratatui), with the Fighter / Battle Master sheet of Rowan Ashford.

[Unreleased]: https://github.com/odd-git/dnd-TUI/compare/v0.9.1...HEAD
[0.9.1]: https://github.com/odd-git/dnd-TUI/compare/v0.1.9...v0.9.1
[0.1.9]: https://github.com/odd-git/dnd-TUI/compare/c22fc8a...v0.1.9
[0.1.0]: https://github.com/odd-git/dnd-TUI/commit/c22fc8a

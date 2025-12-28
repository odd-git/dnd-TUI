# dnd_data.py
from pathlib import Path

# Configurazione File
SAVE_FILE = Path(".dnd_character.json")

# Database Regole (SRD 5e)
ABILITIES = ["str", "dex", "con", "int", "wis", "cha"]

RACES_DB = {
    "1": {"name": "Human", "stats": {"str": 1, "dex": 1, "con": 1, "int": 1, "wis": 1, "cha": 1}, "speed": 30},
    "2": {"name": "High Elf", "stats": {"dex": 2, "int": 1}, "speed": 30},
    "3": {"name": "Mountain Dwarf", "stats": {"str": 2, "con": 2}, "speed": 25},
    "4": {"name": "Tiefling", "stats": {"cha": 2, "int": 1}, "speed": 30},
    "5": {"name": "Halfling", "stats": {"dex": 2}, "speed": 25}
}

CLASSES_DB = {
    "1": {"name": "Barbarian", "hd": 12, "primary": "str"},
    "2": {"name": "Bard", "hd": 8, "primary": "cha"},
    "3": {"name": "Cleric", "hd": 8, "primary": "wis"},
    "4": {"name": "Druid", "hd": 8, "primary": "wis"},
    "5": {"name": "Fighter", "hd": 10, "primary": "str"},
    "6": {"name": "Monk", "hd": 8, "primary": "dex"},
    "7": {"name": "Paladin", "hd": 10, "primary": "str"},
    "8": {"name": "Ranger", "hd": 10, "primary": "dex"},
    "9": {"name": "Rogue", "hd": 8, "primary": "dex"},
    "10": {"name": "Sorcerer", "hd": 6, "primary": "cha"},
    "11": {"name": "Warlock", "hd": 8, "primary": "cha"},
    "12": {"name": "Wizard", "hd": 6, "primary": "int"}
}

import json
import os
import math
import shutil
import re
from pathlib import Path

# ==============================================================================
# 1. GLOBAL DATABASE (SRD 5e)
# ==============================================================================

RACES_DB = {
    "1": {"name": "Human", "stats": {"str": 1, "dex": 1, "con": 1, "int": 1, "wis": 1, "cha": 1}, "speed": 30},
    "2": {"name": "High Elf", "stats": {"dex": 2, "int": 1}, "speed": 30},
    "3": {"name": "Mountain Dwarf", "stats": {"str": 2, "con": 2}, "speed": 25},
    "4": {"name": "Tiefling", "stats": {"cha": 2, "int": 1}, "speed": 30},
    "5": {"name": "Halfling", "stats": {"dex": 2}, "speed": 25}
}

CLASSES_DB = {
    "1": {"name": "Barbarian", "hd": 12, "saves": ["str", "con"]},
    "2": {"name": "Bard", "hd": 8, "saves": ["dex", "cha"]},
    "3": {"name": "Cleric", "hd": 8, "saves": ["wis", "cha"]},
    "4": {"name": "Druid", "hd": 8, "saves": ["int", "wis"]},
    "5": {"name": "Fighter", "hd": 10, "saves": ["str", "con"]},
    "6": {"name": "Monk", "hd": 8, "saves": ["str", "dex"]},
    "7": {"name": "Paladin", "hd": 10, "saves": ["wis", "cha"]},
    "8": {"name": "Ranger", "hd": 10, "saves": ["str", "dex"]},
    "9": {"name": "Rogue", "hd": 8, "saves": ["dex", "int"]},
    "10": {"name": "Sorcerer", "hd": 6, "saves": ["con", "cha"]},
    "11": {"name": "Warlock", "hd": 8, "saves": ["wis", "cha"]},
    "12": {"name": "Wizard", "hd": 6, "saves": ["int", "wis"]}
}

ABILITIES = ["str", "dex", "con", "int", "wis", "cha"]
SAVE_FILE = Path(".dnd_character.json")

# UI Colors
C_H = "\033[92m"  # Success/Header
C_Q = "\033[96m"  # Question/Cyan
C_W = "\033[93m"  # Warning/Yellow
C_B = "\033[90m"  # Borders/Grey
C_R = "\033[0m"   # Reset

# ==============================================================================
# 2. CORE UTILS
# ==============================================================================

def get_mod(score):
    return (score - 10) // 2

def get_prof(lvl):
    return (lvl - 1) // 4 + 2

def clear():
    os.system('cls' if os.name == 'nt' else 'clear')

# ==============================================================================
# 3. BUILDER CLASS
# ==============================================================================

class DndInternationalBuilder:
    def __init__(self):
        self.data = {
            "name": "",
            "race": "",
            "base_stats": {a: 10 for a in ABILITIES},
            "final_stats": {},
            "classes": [], # List of {name, level, hd, is_first}
            "total_level": 0,
            "hp_max": 0,
            "proficiency": 2,
            "inventory": []
        }

    def draw_header(self, title):
        width = shutil.get_terminal_size((80, 20)).columns
        print(f"{C_B}=" * width)
        print(f"{C_H}{title.upper()}{C_R}".center(width + 10))
        print(f"{C_B}=" * width + f"{C_R}\n")

    def run(self):
        clear()
        self.draw_header("D&D 5e Character Builder")
        
        # Step 1: Identity
        self.data["name"] = input(f"{C_Q}Enter Character Name: {C_R}").strip() or "Nameless Hero"
        
        # Step 2: Race
        clear()
        self.draw_header("Select Race")
        for k, v in RACES_DB.items():
            bonus = ", ".join([f"{s.upper()}+{n}" for s, n in v['stats'].items()])
            print(f"[{k}] {v['name']:<15} | Bonuses: {bonus}")
        
        rid = input(f"\n{C_Q}Select Race ID: {C_R}")
        race_obj = RACES_DB.get(rid, RACES_DB["1"])
        self.data["race"] = race_obj["name"]

        # Step 3: Attributes
        clear()
        self.draw_header("Ability Scores (Base)")
        print("Note: Enter scores before racial bonuses (e.g., 15, 14, 13, 12, 10, 8)\n")
        for stat in ABILITIES:
            while True:
                try:
                    val = int(input(f"{stat.upper()}: ") or 10)
                    self.data["base_stats"][stat] = val
                    break
                except ValueError: print(f"{C_W}Invalid number.{C_R}")

        # Step 4: Classes & Multiclassing
        is_first = True
        while True:
            clear()
            self.draw_header("Classes & Leveling")
            print(f"Current Level: {C_H}{self.data['total_level']}{C_R}")
            
            # Grid display
            items = list(CLASSES_DB.items())
            for i in range(0, len(items), 2):
                k1, v1 = items[i]
                line = f"[{k1}] {v1['name']:<12} (d{v1['hd']})"
                if i+1 < len(items):
                    k2, v2 = items[i+1]
                    line += f"  [{k2}] {v2['name']:<12} (d{v2['hd']})"
                print(line)

            cmd = input(f"\n{C_Q}Enter Class ID to add levels (or 'F' to finish): {C_R}").upper()
            if cmd == 'F' and self.data['total_level'] > 0: break
            if cmd in CLASSES_DB:
                tpl = CLASSES_DB[cmd]
                try:
                    lvls = int(input(f"How many levels of {tpl['name']}?: ") or 1)
                    self.data["classes"].append({
                        "name": tpl["name"], 
                        "level": lvls, 
                        "hd": tpl["hd"], 
                        "is_first": is_first
                    })
                    self.data["total_level"] += lvls
                    is_first = False
                except: pass

        # Step 5: Final Calculations
        self.finalize()
        self.save()

    def finalize(self):
        # Apply Racial Bonuses
        race_obj = next(r for r in RACES_DB.values() if r["name"] == self.data["race"])
        for s in ABILITIES:
            self.data["final_stats"][s] = self.data["base_stats"][s] + race_obj["stats"].get(s, 0)
        
        con_mod = get_mod(self.data["final_stats"]["con"])
        
        # HP Calculation (5e Rules)
        total_hp = 0
        for c in self.data["classes"]:
            if c["is_first"]:
                # First level of first class: Max HD + CON
                total_hp += c["hd"] + con_mod
                # Subsequent levels of same class: Average HD + CON
                if c["level"] > 1:
                    total_hp += (c["level"] - 1) * ((c["hd"] // 2 + 1) + con_mod)
            else:
                # Multiclass levels: Always Average HD + CON
                total_hp += c["level"] * ((c["hd"] // 2 + 1) + con_mod)
        
        self.data["hp_max"] = total_hp
        self.data["proficiency"] = get_prof(self.data["total_level"])

    def save(self):
        with open(SAVE_FILE, 'w', encoding='utf-8') as f:
            json.dump(self.data, f, indent=4)
        
        clear()
        self.draw_header("Character Created!")
        print(f"Name: {self.data['name']} ({self.data['race']})")
        print(f"Level: {self.data['total_level']} | HP: {self.data['hp_max']}")
        print(f"File saved to: {SAVE_FILE.absolute()}")
        input("\nPress Enter to exit...")

if __name__ == "__main__":
    try:
        DndInternationalBuilder().run()
    except KeyboardInterrupt:
        print("\nExiting...")

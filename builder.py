# builder.py
import json
import os
import shutil
from dnd_data import RACES_DB, CLASSES_DB, ABILITIES, SAVE_FILE

# Utility Functions
def get_mod(score): return (score - 10) // 2
def get_prof(lvl): return (lvl - 1) // 4 + 2
def clear(): os.system('cls' if os.name == 'nt' else 'clear')

class DndBuilder:
    def __init__(self):
        self.data = {
            "name": "",
            "race": "",
            "base_stats": {a: 10 for a in ABILITIES},
            "final_stats": {},
            "classes": [],
            "total_level": 0,
            "hp_max": 0,
            "proficiency": 2
        }
        # Colors
        self.C_H = "\033[92m"
        self.C_Q = "\033[96m"
        self.C_R = "\033[0m"

    def draw_header(self, title):
        w = shutil.get_terminal_size().columns
        print(f"\n{self.C_H}{title.upper()}{self.C_R}".center(w + 10))
        print("-" * w)

    def run(self):
        clear()
        self.draw_header("Character Creator")
        self.data["name"] = input(f"{self.C_Q}Enter Name: {self.C_R}").strip() or "Hero"

        # Race
        clear()
        self.draw_header("Select Race")
        for k, v in RACES_DB.items():
            print(f"[{k}] {v['name']}")
        rid = input("> ")
        self.data["race"] = RACES_DB.get(rid, RACES_DB["1"])["name"]

        # Stats
        clear()
        self.draw_header("Ability Scores (Base)")
        for stat in ABILITIES:
            try:
                val = int(input(f"{stat.upper()}: ") or 10)
                self.data["base_stats"][stat] = val
            except: pass

        # Classes
        is_first = True
        while True:
            clear()
            self.draw_header(f"Leveling (Current: {self.data['total_level']})")
            for k, v in CLASSES_DB.items():
                print(f"[{k}] {v['name']} (d{v['hd']})")
            
            cmd = input("\nClass ID to add level (F to finish): ").upper()
            if cmd == 'F' and self.data['total_level'] > 0: break
            
            if cmd in CLASSES_DB:
                tpl = CLASSES_DB[cmd]
                try:
                    qty = int(input(f"Levels of {tpl['name']}: ") or 1)
                    self.data["classes"].append({
                        "name": tpl["name"],
                        "level": qty,
                        "hd": tpl["hd"],
                        "primary_stat": tpl["primary"],
                        "is_first": is_first
                    })
                    self.data["total_level"] += qty
                    is_first = False
                except: pass

        self.finalize()
        self.save()

    def finalize(self):
        # Stats + Race Bonus
        race_data = next(v for v in RACES_DB.values() if v["name"] == self.data["race"])
        for s in ABILITIES:
            self.data["final_stats"][s] = self.data["base_stats"][s] + race_data["stats"].get(s, 0)
        
        # HP Calc
        con_mod = get_mod(self.data["final_stats"]["con"])
        hp = 0
        for c in self.data["classes"]:
            if c["is_first"]:
                hp += c["hd"] + con_mod
                if c["level"] > 1: hp += (c["level"] - 1) * ((c["hd"] // 2 + 1) + con_mod)
            else:
                hp += c["level"] * ((c["hd"] // 2 + 1) + con_mod)
        
        self.data["hp_max"] = hp
        self.data["proficiency"] = get_prof(self.data["total_level"])

    def save(self):
        with open(SAVE_FILE, 'w', encoding='utf-8') as f:
            json.dump(self.data, f, indent=4)
        print("Character Saved.")
        input("Press Enter...")

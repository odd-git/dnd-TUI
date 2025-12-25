import random
import os
import shutil
import re
import json

# ==============================================================================
# Configuration & Persistence
# ==============================================================================

DB_FILE = ".campaign_journal.json"

CHAR_CFG = {
    "name": "Sorlock",
    "total_level": 2, 
    "class": "Sorcerer 1 / Warlock 1",
    "stats": {"charisma": 18, "proficiency": 2},
    "max_resources": {
        "sorc_slots": 2,    
        "pact_slots": 1,    
        "balance": 2        
    }
}

SPELL_DB = {
    "1": {"name": "Eldritch Blast", "level": 0, "dices": (1, 10), "damage_type": "Force", "desc": "Crackling beam"},
    "2": {"name": "Fire Bolt", "level": 0, "dices": (1, 10), "damage_type": "Fire", "desc": "Fire dart"},
    "3": {"name": "Shocking Grasp", "level": 0, "dices": (1, 8), "damage_type": "Lightning", "desc": "Adv vs Metal"},
    "4": {"name": "Magic Missile", "level": 1, "dices": (3, 4), "fixed_bonus": 3, "damage_type": "Force", "auto_hit": True, "desc": "3 unerring darts"},
    "5": {"name": "Wrathful Smite", "level": 1, "dices": (1, 6), "damage_type": "Psychic", "desc": "Wis Save or Frightened"},
    "6": {"name": "Shield", "level": 1, "utility": True, "desc": "+5 AC (Reaction)"},
    "7": {"name": "Mage Armor", "level": 1, "utility": True, "desc": "AC 13 + Dex (8h)"},
    "8": {"name": "Alarm", "level": 1, "utility": True, "desc": "Wards 20ft area"}
}

# Colors
C_H = "\033[92m"  # Green
C_B = "\033[90m"  # Grey
C_W = "\033[93m"  # Yellow
C_R = "\033[0m"   # Reset

# ==============================================================================
# Main Application
# ==============================================================================

class SorlockTUI:
    def __init__(self):
        self.cfg = CHAR_CFG
        self.logs = ["System start.", "Data persistence active."]
        self.mod_car = (self.cfg["stats"]["charisma"] - 10) // 2
        self.prof = self.cfg["stats"]["proficiency"]
        self.atk_bonus = self.mod_car + self.prof
        self.cd_save = 8 + self.prof + self.mod_car
        self.res = self.cfg["max_resources"].copy()
        self.hex_curse = False
        self.campaign_notes = self.load_data()

    # --- Data Management ---

    def save_data(self):
        try:
            with open(DB_FILE, 'w', encoding='utf-8') as f:
                json.dump(self.campaign_notes, f, indent=4)
        except Exception as e:
            self.log(f"Disk Error: {str(e)}")

    def load_data(self):
        if os.path.exists(DB_FILE):
            try:
                with open(DB_FILE, 'r', encoding='utf-8') as f:
                    return json.load(f)
            except:
                return []
        return []

    # --- UI Helpers ---

    def get_terminal_width(self):
        return shutil.get_terminal_size((80, 20)).columns - 2

    def log(self, msg):
        self.logs.append(msg)
        if len(self.logs) > 6: self.logs.pop(0)

    def draw_box(self, title, lines, w):
        def clean_len(s): return len(re.sub(r'\033\[[0-9;]*m', '', s))
        title_str = f" {title} "
        print(f"{C_H}┌─{title_str}{'─'*(w - clean_len(title_str) - 2)}┐{C_R}")
        for l in lines:
            v_len = clean_len(l)
            if v_len > w - 4:
                l = l[:w-7] + "..."
                v_len = clean_len(l)
            print(f"{C_B}│{C_R} {l}{' '*(w - v_len - 2)}{C_B}│{C_R}")
        print(f"{C_H}└{'─'*(w)}┘{C_R}")

    # --- Combat Logic ---

    def execute_spell(self, spell_id):
        spell = SPELL_DB.get(spell_id)
        if not spell: return

        source = "Cantrip"
        if spell["level"] > 0:
            if self.res["pact_slots"] > 0:
                self.res["pact_slots"] -= 1
                source = "Pact Slot"
            elif self.res["sorc_slots"] > 0:
                self.res["sorc_slots"] -= 1
                source = "Sorc Slot"
            else:
                self.log(f"{C_W}No slots left!{C_R}")
                return

        self.log(f"Cast: {spell['name']} [{source}]")
        if spell.get("utility"):
            self.log(f"Effect: {spell['desc']}")
            return

        crit_min = 19 if self.hex_curse else 20
        beams = 2 if spell["name"] == "Eldritch Blast" and self.cfg["total_level"] >= 5 else 1
        
        for _ in range(beams):
            if not spell.get("auto_hit"):
                d20 = random.randint(1, 20)
                is_crit = d20 >= crit_min
                self.log(f"Atk: {d20}+{self.atk_bonus}={C_H}{d20+self.atk_bonus}{C_R}" + (f" {C_W}CRIT!{C_R}" if is_crit else ""))
                self.roll_damage(spell, is_crit)
            else:
                self.roll_damage(spell, False)

    def roll_damage(self, spell, is_crit):
        n, f = spell["dices"]
        if is_crit: n *= 2
        base = sum(random.randint(1, f) for _ in range(n))
        bonus = spell.get("fixed_bonus", 0) + (self.prof if self.hex_curse else 0)
        self.log(f" >> Damage: {C_H}{base + bonus}{C_R} {spell['damage_type']} ({base}+{bonus})")

    # --- Menus ---

    def campaign_menu(self):
        while True:
            os.system('cls' if os.name=='nt' else 'clear')
            w = self.get_terminal_width()
            display = self.campaign_notes if self.campaign_notes else ["Journal is empty."]
            
            self.draw_box("CAMPAIGN JOURNAL (Auto-Saving)", display, w)
            print(f" {C_H}A{C_R}: Add | {C_H}C{C_R}: Clear | {C_H}B{C_R}: Back")
            
            cmd = input(f"\n{C_H}journal_cmd> {C_R}").strip().upper()
            if cmd == "B": break
            elif cmd == "C":
                self.campaign_notes = []
                self.save_data()
            elif cmd == "A":
                note = input(f"{C_W}Note text: {C_R}").strip()
                if note:
                    self.campaign_notes.append(f"- {note}")
                    self.save_data()

    def run(self):
        while True:
            os.system('cls' if os.name=='nt' else 'clear')
            w = self.get_terminal_width()
            
            # 1. State Box
            stats = [
                f"{self.cfg['name']} | {self.cfg['class']}",
                f"Sorc Slots: {self.res['sorc_slots']}/{self.cfg['max_resources']['sorc_slots']} | Pact: {self.res['pact_slots']}/{self.cfg['max_resources']['pact_slots']}",
                f"Hex Curse: {'ON' if self.hex_curse else 'OFF'} | Bonus: +{self.atk_bonus}"
            ]
            self.draw_box("CHARACTER STATUS", stats, w)

            # 2. Spells Box
            menu = [f"{eid:<2} | {s['name']:<18} | Lv:{s['level']} | {s['desc']}" for eid, s in SPELL_DB.items()]
            self.draw_box("SPELLBOOK", menu, w)

            # 3. Logs Box
            self.draw_box("COMBAT LOG", self.logs, w)

            print(f" {C_H}H{C_R}: Hex | {C_H}I{C_R}: Journal | {C_H}R{C_R}: Long Rest | {C_H}Q{C_R}: Quit")
            cmd = input(f"\n{C_H}{self.cfg['name'].lower()}> {C_R}").strip().upper()
            
            if cmd == "Q": break
            elif cmd == "I": self.campaign_menu()
            elif cmd == "H":
                self.hex_curse = not self.hex_curse
                self.log(f"Hex state: {self.hex_curse}")
            elif cmd == "R":
                self.res = self.cfg["max_resources"].copy()
                self.log("Long Rest completed.")
            elif cmd in SPELL_DB:
                self.execute_spell(cmd)
            else:
                self.log("Unknown command.")

if __name__ == "__main__":
    try:
        SorlockTUI().run()
    except KeyboardInterrupt:
        print("\nExiting...")

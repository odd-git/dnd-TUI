# combat_engine.py
import os
import random
import shutil

class CombatEngine:
    def __init__(self, character_data):
        self.char = character_data
        self.current_hp = character_data["hp_max"]
        self.logs = ["Combat Initiated."]
        self.running = True
        
        # Colors
        self.C_H = "\033[92m" 
        self.C_W = "\033[93m" 
        self.C_R = "\033[0m"

    def log(self, msg):
        self.logs.append(msg)
        if len(self.logs) > 6: self.logs.pop(0)

    def draw_ui(self):
        os.system('cls' if os.name == 'nt' else 'clear')
        w = shutil.get_terminal_size().columns
        
        # Header
        name_str = f"{self.char['name']} | Lv.{self.char['total_level']} {self.char['race']}"
        print(f"{self.C_H}{name_str.center(w)}{self.C_R}")
        
        # HP Bar
        pct = self.current_hp / self.char["hp_max"]
        bar = "#" * int(20 * pct)
        print(f"HP: {self.current_hp}/{self.char['hp_max']} [{bar:<20}]".center(w))
        print("-" * w)

        # Main Stats Visualizer
        stats = []
        for k, v in self.char["final_stats"].items():
            mod = (v - 10) // 2
            sign = "+" if mod >= 0 else ""
            stats.append(f"{k.upper()}:{v}({sign}{mod})")
        print(" | ".join(stats).center(w))
        print("-" * w)

        # Log
        print("\n" + "[ LOG ]".center(w))
        for l in self.logs: print(f" > {l}")
        
        print("\n" + "-" * w)
        print(f"{self.C_H}[A] Attack  [H] Heal  [Q] Quit{self.C_R}")

    def run(self):
        while self.running:
            self.draw_ui()
            cmd = input("Cmd> ").lower().strip()
            
            if cmd == 'q': 
                self.running = False
            elif cmd == 'h':
                heal = random.randint(1, 8)
                self.current_hp = min(self.char["hp_max"], self.current_hp + heal)
                self.log(f"Healed for {heal} HP.")
            elif cmd == 'a':
                # Determine primary stat from first class
                first_class = self.char["classes"][0]
                stat_name = first_class.get("primary_stat", "str")
                stat_val = self.char["final_stats"][stat_name]
                mod = (stat_val - 10) // 2
                prof = self.char["proficiency"]
                
                roll = random.randint(1, 20)
                total = roll + mod + prof
                
                crit = f" {self.C_W}CRIT!{self.C_R}" if roll == 20 else ""
                self.log(f"Atk ({stat_name}): {roll}+{mod}+{prof} = {self.C_H}{total}{self.C_R}{crit}")

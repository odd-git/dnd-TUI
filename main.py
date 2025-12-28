# main.py
import json
import os
from pathlib import Path
from dnd_data import SAVE_FILE

# Import dei moduli
from builder import DndBuilder
from combat_engine import CombatEngine

class AppLauncher:
    def clear(self): os.system('cls' if os.name == 'nt' else 'clear')

    def main_menu(self):
        while True:
            self.clear()
            print("\033[92m=== D&D 5E TERMINAL SUITE ===\033[0m")
            
            exists = SAVE_FILE.exists()
            if exists:
                with open(SAVE_FILE, 'r') as f: d = json.load(f)
                print(f"1. Load Character ({d['name']})")
            else:
                print("1. [Empty] Load Character")
            
            print("2. Create New Character")
            print("3. Quit")

            ch = input("\nSelect> ")
            
            if ch == '1' and exists:
                with open(SAVE_FILE, 'r') as f: data = json.load(f)
                CombatEngine(data).run()
            elif ch == '2':
                DndBuilder().run()
            elif ch == '3':
                break

if __name__ == "__main__":
    AppLauncher().main_menu()

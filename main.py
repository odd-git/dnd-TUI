# main.py
import json
import os
from pathlib import Path
from dnd_data import SAVE_FILE

# Import dei moduli
from builder import DndBuilder
from combat_engine import CombatEngine
from rowan import RowanTUI
from sorlock import SorlockTUI

class AppLauncher:
    def clear(self): os.system('cls' if os.name == 'nt' else 'clear')

    def main_menu(self):
        while True:
            self.clear()
            print("\033[92m=== D&D 5E TERMINAL SUITE ===\033[0m")
            
            print("1. Rowan Ashford")
            print("2. Sorlock")

            exists = SAVE_FILE.exists()
            if exists:
                with open(SAVE_FILE, 'r') as f: d = json.load(f)
                print(f"3. Generic Character ({d['name']})")
            else:
                print("3. [Empty] Generic Character")

            print("4. Create New Character")
            print("5. Quit")

            ch = input("\nWhich sheet> ")

            if ch == '1':
                RowanTUI().run()
            elif ch == '2':
                SorlockTUI().run()
            elif ch == '3' and exists:
                with open(SAVE_FILE, 'r') as f: data = json.load(f)
                CombatEngine(data).run()
            elif ch == '4':
                DndBuilder().run()
            elif ch == '5':
                break

if __name__ == "__main__":
    AppLauncher().main_menu()

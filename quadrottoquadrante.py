import random
import os
import shutil

# ==============================================================================
#   AREA DI CONFIGURAZIONE
# ==============================================================================

CHAR_CFG = {
    "nome": "Quadrotto Quadrante",
    "livello_totale": 2, 
    "classi": "Stregone 1 / Warlock 1",
    "stats": {"carisma": 18, "competenza": 2},
    "risorse_max": {
        "sorc_slots": 2,    
        "pact_slots": 1,    
        "balance": 2        
    }
}

SPELL_DB = {
    "1": {"nome": "Eldritch Blast", "livello": 0, "dadi": (1, 10), "tipo_danno": "Forza", "desc": "Raggio crepitante"},
    "2": {"nome": "Fire Bolt", "livello": 0, "dadi": (1, 10), "tipo_danno": "Fuoco", "desc": "Dardo di fuoco"},
    "3": {"nome": "Shocking Grasp", "livello": 0, "dadi": (1, 8), "tipo_danno": "Fulmine", "desc": "Vantaggio vs Metallo"},
    "4": {"nome": "Magic Missile", "livello": 1, "dadi": (3, 4), "bonus_fisso": 3, "tipo_danno": "Forza", "auto_hit": True, "desc": "3 dardi infallibili"},
    "5": {"nome": "Wrathful Smite", "livello": 1, "dadi": (1, 6), "tipo_danno": "Psichico", "desc": "TS Saggezza o Spaventato"},
    "6": {"nome": "Shield", "livello": 1, "utility": True, "desc": "+5 CA (Reazione)"},
    "7": {"nome": "Mage Armor", "livello": 1, "utility": True, "desc": "CA 13 + Dex (8h)"},
    "8": {"nome": "Alarm", "livello": 1, "utility": True, "desc": "Sorveglia area 20ft"}
}

# ==============================================================================
#   MOTORE DI GIOCO ADATTIVO
# ==============================================================================

C_H = "\033[92m"  # Verde
C_B = "\033[90m"  # Grigio
C_W = "\033[93m"  # Giallo
C_R = "\033[0m"   # Reset

class QuadrottoTUI:
    def __init__(self):
        self.logs = ["Sistema avviato.", "Layout dinamico attivo."]
        self.cfg = CHAR_CFG
        self.mod_car = (self.cfg["stats"]["carisma"] - 10) // 2
        self.prof = self.cfg["stats"]["competenza"]
        self.atk_bonus = self.mod_car + self.prof
        self.cd_save = 8 + self.prof + self.mod_car
        self.res = self.cfg["risorse_max"].copy()
        self.hex_curse = False

    def get_terminal_width(self):
        # Rileva la larghezza del terminale, default a 80 se fallisce
        return shutil.get_terminal_size((80, 20)).columns - 2

    def log(self, msg):
        self.logs.append(msg)
        if len(self.logs) > 6: self.logs.pop(0)

    def roll(self, faces): return random.randint(1, faces)

    def reset(self):
        self.res = self.cfg["risorse_max"].copy()
        self.hex_curse = False
        self.log("Riposo Lungo completato.")

    def consume_slot(self):
        if self.res["pact_slots"] > 0:
            self.res["pact_slots"] -= 1
            return True, "Pact Slot"
        elif self.res["sorc_slots"] > 0:
            self.res["sorc_slots"] -= 1
            return True, "Sorc Slot"
        return False, "NO MANA"

    def execute_spell(self, spell_id):
        spell = SPELL_DB.get(spell_id)
        if not spell: return

        source_msg = "Trucchetto"
        if spell["livello"] > 0:
            ok, source_msg = self.consume_slot()
            if not ok:
                self.log(f"{C_W}Slot esauriti!{C_R}")
                return

        self.log(f"Cast: {spell['nome']} ({source_msg})")

        if spell.get("utility"):
            self.log(f"Effetto: {spell['desc']}")
            return

        auto_hit = spell.get("auto_hit", False)
        crit_min = 19 if self.hex_curse else 20
        
        if not auto_hit:
            beams = 2 if spell["nome"] == "Eldritch Blast" and self.cfg["livello_totale"] >= 5 else 1
            for _ in range(beams):
                d20 = self.roll(20)
                is_crit = d20 >= crit_min
                hit_val = d20 + self.atk_bonus
                self.log(f"TxC: {d20}+{self.atk_bonus}={C_H}{hit_val}{C_R}" + (f" {C_W}CRIT!{C_R}" if is_crit else ""))
                self.roll_damage(spell, is_crit)
        else:
            self.roll_damage(spell, False)

    def roll_damage(self, spell, is_crit):
        n, f = spell["dadi"]
        if is_crit: n *= 2
        base = sum(self.roll(f) for _ in range(n))
        bonus = spell.get("bonus_fisso", 0) + (self.prof if self.hex_curse else 0)
        final = base + bonus
        self.log(f" >> Danni: {C_H}{final}{C_R} {spell['tipo_danno']} ({base}+{bonus})")

    def draw_box(self, title, lines, w):
        def clean(s): return len(s.replace(C_H,"").replace(C_R,"").replace(C_W,"").replace(C_B,""))
        
        # Header Box
        title_str = f" {title} "
        top = f"{C_H}┌─{title_str}{'─'*(w - len(title_str) - 2)}┐{C_R}"
        print(top)
        
        for l in lines:
            line_content = l
            pad = w - clean(line_content) - 2
            if pad < 0: # Taglia la riga se troppo lunga per il monitor
                line_content = line_content[:w-5] + "..."
                pad = w - clean(line_content) - 2
            print(f"{C_B}│{C_R} {line_content}{' '*pad}{C_B}│{C_R}")
            
        print(f"{C_H}└{'─'*(w)}┘{C_R}")

    def run(self):
        while True:
            os.system('cls' if os.name=='nt' else 'clear')
            w = self.get_terminal_width()
            
            # Rendering Stato
            stats = [
                f"{self.cfg['nome']} | {self.cfg['classi']}",
                f"Bonus Atk: +{self.atk_bonus} | CD Salvezza: {self.cd_save}",
                f"Sorc Slots: {self.res['sorc_slots']}/{self.cfg['risorse_max']['sorc_slots']} | Pact Slots: {self.res['pact_slots']}/{self.cfg['risorse_max']['pact_slots']}",
                f"Hex Curse: {'ATTIVA' if self.hex_curse else 'OFF'} | Balance: {self.res['balance']}/{self.cfg['risorse_max']['balance']}"
            ]
            self.draw_box("STATO", stats, w)

            # Rendering Menu
            menu = [f"{eid:<2} | {s['nome']:<18} | Lv:{s['livello']} | {s['desc']}" for eid, s in SPELL_DB.items()]
            self.draw_box("GRIMORIO", menu, w)

            # Rendering Log
            self.draw_box("LOG DI COMBATTIMENTO", self.logs, w)

            print(f" {C_H}H{C_R}: Curse | {C_H}R{C_R}: Reset | {C_H}Q{C_R}: Esci")
            cmd = input(f"\n{C_H}command_prompt@{self.cfg['nome'].lower().replace(' ', '_')}> {C_R}").strip().upper()
            
            if cmd == "Q": break
            elif cmd == "R": self.reset()
            elif cmd == "H": 
                self.hex_curse = not self.hex_curse
                self.log("Curse stato cambiato.")
            elif cmd in SPELL_DB: 
                self.execute_spell(cmd)
            else:
                self.log("Comando non valido.")

if __name__ == "__main__":
    QuadrottoTUI().run()

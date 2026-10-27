import random
import os
import shutil
import json

from sorlock import box, C_H, C_W, C_R

# ==============================================================================
# Configuration & Persistence
# ==============================================================================

STATE_FILE = ".rowan_state.json"

ABILITIES = ["STR", "DEX", "CON", "INT", "WIS", "CHA"]
SAVE_PROFS = ["STR", "CON"]
HIT_DIE = 10  # Fighter

SKILLS = {
    "Acrobatics": "DEX", "Animal Handling": "WIS", "Arcana": "INT", "Athletics": "STR",
    "Deception": "CHA", "History": "INT", "Insight": "WIS", "Intimidation": "CHA",
    "Investigation": "INT", "Medicine": "WIS", "Nature": "INT", "Perception": "WIS",
    "Performance": "CHA", "Persuasion": "CHA", "Religion": "INT", "Sleight of Hand": "DEX",
    "Stealth": "DEX", "Survival": "WIS",
}

# Starting character (level 3); level-ups are saved in STATE_FILE
DEFAULT_CHAR = {
    "name": "Rowan Ashford",
    "race": "Variant Human",
    "background": "Noble",
    "level": 3,
    "stats": {"STR": 9, "DEX": 16, "CON": 14, "INT": 16, "WIS": 11, "CHA": 10},
    "ac_base": 11,  # leather armor + DEX
    "skills": ["Insight", "Perception", "History", "Persuasion", "Investigation"],
    "maneuvers": ["Maneuvering Attack", "Commander's Strike", "Distracting Strike"],
    "features": {
        "Fighting Style": "Thrown Weapon Fighting, draw with attack, +2 thrown dmg",
        "Second Wind": "Bonus action, heal d10 + level, 1/short rest",
        "Action Surge": "One extra action, per short rest",
        "Combat Superiority": "Maneuvers fueled by superiority dice, short rest",
        "Student of War": "Cartographer's tools",
        "Feat: Fey Touched": "Misty Step + Command 1/long rest, INT",
    },
    "info": [
        "Tools: chess set, cartographer's tools",
        "Armor: all + shields | Weapons: simple + martial",
        "Gear: leather armor, daggers, chess set, fine clothes,",
        "      signet ring, scroll of pedigree",
        "Languages: Common + 2 (to agree with the DM)",
    ],
}

# Fighter / Battle Master progression: {level: value}, value holds until the next key
SUP_DICE = {3: 4, 7: 5, 15: 6}
SUP_DIE = {3: 8, 10: 10, 18: 12}
MANEUVERS_KNOWN = {3: 3, 7: 5, 10: 7, 15: 9}
ATTACKS_PER_ACTION = {1: 1, 5: 2, 11: 3, 20: 4}
ACTION_SURGE = {2: 1, 17: 2}
INDOMITABLE = {1: 0, 9: 1, 13: 2, 17: 3}
ASI_LEVELS = [4, 6, 8, 12, 14, 16, 19]

FIGHTER_LEVELS = {
    5: {"Extra Attack": "2 attacks per Attack action"},
    7: {"Know Your Enemy": "1 min: compare 2 traits (STR/DEX/CON/AC/HP/lvl)"},
    9: {"Indomitable": "Reroll a failed save, 1/long rest"},
    10: {"Improved Combat Superiority": "Superiority dice become d10"},
    11: {"Extra Attack": "3 attacks per Attack action"},
    13: {"Indomitable": "Reroll a failed save, 2/long rest"},
    15: {"Relentless": "Roll initiative with no superiority dice: regain 1"},
    17: {"Action Surge": "One extra action, 2 per short rest", "Indomitable": "Reroll a failed save, 3/long rest"},
    18: {"Improved Combat Superiority": "Superiority dice become d12"},
    20: {"Extra Attack": "4 attacks per Attack action"},
}

MANEUVERS = {
    "Ambush": "+die to Stealth or initiative",
    "Bait and Switch": "Swap with ally within 5 ft, +die AC to one",
    "Brace": "Reaction: attack one entering reach, +die dmg",
    "Commander's Strike": "Bonus: skip attack, ally reacts, +die dmg",
    "Commanding Presence": "+die to Intimidation/Performance/Persuasion",
    "Disarming Attack": "On hit: +die dmg, STR save or drop an item",
    "Distracting Strike": "On hit: +die dmg, next ally attack has adv.",
    "Evasive Footwork": "When moving: +die AC",
    "Feinting Attack": "Bonus: advantage on next attack, +die dmg",
    "Goading Attack": "On hit: +die dmg, WIS save or disadv. vs others",
    "Grappling Strike": "After melee hit: bonus grapple, +die to check",
    "Lunging Attack": "Melee attack: +5 ft reach, +die dmg on hit",
    "Maneuvering Attack": "On hit: +die dmg, ally moves half speed, no OA",
    "Menacing Attack": "On hit: +die dmg, WIS save or frightened",
    "Parry": "Reaction: reduce melee dmg by die + DEX",
    "Precision Attack": "+die to an attack roll",
    "Pushing Attack": "On hit: +die dmg, STR save or pushed 15 ft",
    "Quick Toss": "Bonus: draw and throw a weapon, +die dmg",
    "Rally": "Bonus action: ally gains die + CHA temp HP",
    "Riposte": "Reaction when missed in melee: attack, +die dmg",
    "Sweeping Attack": "On melee hit: die dmg to another within 5 ft",
    "Tactical Assessment": "+die to Investigation, History or Insight",
    "Trip Attack": "On hit: +die dmg, STR save or prone",
}

# Dagger (PHB): d4 piercing, finesse, light, thrown 20/60 ft
# "ability" = add DEX to dmg, "style" = Thrown Weapon Fighting +2
ATTACKS = [
    {"name": "Thrown dagger", "ability": True, "style": True, "thrown": True, "desc": "20/60 ft"},
    {"name": "Melee dagger", "ability": True, "style": False},
    {"name": "Off-hand thrown", "ability": False, "style": True, "thrown": True, "desc": "Bonus action, 20/60 ft"},
    {"name": "Off-hand melee", "ability": False, "style": False, "desc": "Bonus action"},
]

# ==============================================================================
# Main Application
# ==============================================================================

# Terminal width from which the combat log goes beside the other boxes
SIDE_MIN, LOG_W = 120, 50

def roll(n, f): return sum(random.randint(1, f) for _ in range(n))
def by_level(lvl, table): return table[max(k for k in table if k <= lvl)]
def mod(score): return (score - 10) // 2

class RowanTUI:
    def __init__(self):
        self.logs = ["System start."]
        self.state = self.load_data()

    @property
    def c(self): return self.state["char"]

    # --- Data Management ---

    def save_data(self):
        try:
            with open(STATE_FILE, 'w', encoding='utf-8') as f:
                json.dump(self.state, f, indent=4)
        except Exception as e:
            self.log(f"Disk Error: {str(e)}")

    def load_data(self):
        saved = {}
        if os.path.exists(STATE_FILE):
            try:
                with open(STATE_FILE, 'r', encoding='utf-8') as f:
                    saved = json.load(f)
            except:
                pass
        self.state = {"daggers": 10, **saved, "char": {**json.loads(json.dumps(DEFAULT_CHAR)), **saved.get("char", {})}}
        self.state["res"] = {**self.max_res(), **saved.get("res", {})}
        self.state.setdefault("hp", self.hp_max())
        return self.state

    # --- Derived Values ---

    def prof(self): return (self.c["level"] - 1) // 4 + 2
    def mod(self, ab): return mod(self.c["stats"][ab])

    def hp_max(self):
        l, cm = self.c["level"], self.mod("CON")
        return HIT_DIE + cm + (l - 1) * (HIT_DIE // 2 + 1 + cm)

    def max_res(self):
        l = self.c["level"]
        return {
            "superiority": by_level(l, SUP_DICE), "second_wind": 1,
            "action_surge": by_level(l, ACTION_SURGE), "indomitable": by_level(l, INDOMITABLE),
            "misty_step": 1, "command": 1, "hit_dice": l,
        }

    def checks(self):
        p = self.prof()
        checks = {ab: self.mod(ab) for ab in ABILITIES}
        checks.update({f"{ab} save": self.mod(ab) + (p if ab in SAVE_PROFS else 0) for ab in ABILITIES})
        checks.update({s: self.mod(SKILLS[s]) + p for s in self.c["skills"]})
        return checks

    def attack_dmg(self, a): return (self.mod("DEX") if a["ability"] else 0) + (2 if a["style"] else 0)

    def actions(self):
        l = self.c["level"]
        acts = [("attack", a) for a in ATTACKS]
        acts += [("feature", f) for f in [
            {"name": "Second Wind", "res": "second_wind", "roll": (1, 10, l), "heal": True, "desc": f"Bonus action, heal d10+{l}"},
            {"name": "Action Surge", "res": "action_surge", "desc": "One extra action this turn"},
            {"name": "Indomitable", "res": "indomitable", "desc": "Reroll a failed save"},
            {"name": "Misty Step", "res": "misty_step", "desc": "Bonus action, teleport 30 ft"},
            {"name": "Command", "res": "command", "desc": f"WIS save DC {8 + self.prof() + self.mod('INT')}, one-word command"},
        ] if self.max_res()[f["res"]] > 0]
        return {str(i): act for i, act in enumerate(acts, 1)}

    # --- UI Helpers ---

    def log(self, msg):
        self.logs.append(msg)
        if len(self.logs) > 50: self.logs.pop(0)

    def ask_int(self, prompt):
        try: return int(input(f"{C_W}{prompt}{C_R}"))
        except ValueError: return 0

    def pick(self, title, labels):
        for i, l in enumerate(labels, 1): print(f" {C_H}{i:>2}{C_R}. {l}")
        i = self.ask_int(f"{title} #: ") - 1
        return i if 0 <= i < len(labels) else None

    # --- Combat Logic ---

    def attack(self, a):
        if a.get("thrown"):
            if self.state["daggers"] <= 0:
                self.log(f"{C_W}No daggers left!{C_R}")
                return
            self.state["daggers"] -= 1
        hit, dmg = self.mod("DEX") + self.prof(), self.attack_dmg(a)
        d20 = random.randint(1, 20)
        crit = d20 == 20
        self.log(f"{a['name']}: {d20}+{hit}={C_H}{d20 + hit}{C_R}" + (f" {C_W}CRIT!{C_R}" if crit else ""))
        base = roll(2 if crit else 1, 4)
        self.log(f" >> Damage: {C_H}{base + dmg}{C_R} piercing ({base}+{dmg})")

    def use_feature(self, feat):
        res = self.state["res"]
        if res[feat["res"]] <= 0:
            self.log(f"{C_W}{feat['name']}: none left!{C_R}")
            return
        res[feat["res"]] -= 1
        msg = f"{feat['name']}: {feat['desc']}"
        if "roll" in feat:
            n, f, b = feat["roll"]
            val = roll(n, f) + b
            msg += f" -> {C_H}{val}{C_R}"
            if feat.get("heal"): self.heal(val)
        self.log(msg)

    def superiority(self):
        if self.state["res"]["superiority"] <= 0:
            self.log(f"{C_W}No superiority dice left!{C_R}")
            return
        self.state["res"]["superiority"] -= 1
        die = by_level(self.c["level"], SUP_DIE)
        self.log(f"Superiority d{die}: {C_H}{roll(1, die)}{C_R}")

    def heal(self, n):
        self.state["hp"] = min(self.hp_max(), self.state["hp"] + n)

    def check(self):
        checks = self.checks()
        names = list(checks)
        idx = self.ask_int("Check # (see CHECKS box): ") - 1
        if not 0 <= idx < len(names): return
        name = names[idx]
        d20 = random.randint(1, 20)
        self.log(f"{name}: {d20}{checks[name]:+}={C_H}{d20 + checks[name]}{C_R}")

    def short_rest(self):
        res = self.state["res"]
        n = max(0, min(self.ask_int(f"Hit dice to spend (0-{res['hit_dice']}): "), res["hit_dice"]))
        healed = sum(max(1, roll(1, HIT_DIE) + self.mod("CON")) for _ in range(n))
        res["hit_dice"] -= n
        self.heal(healed)
        for k in ["superiority", "second_wind", "action_surge"]: res[k] = self.max_res()[k]
        self.log(f"Short Rest: +{healed} HP, maneuvers/Second Wind/Action Surge back.")

    def long_rest(self):
        max_res = self.max_res()
        hd = min(max_res["hit_dice"], self.state["res"]["hit_dice"] + max(1, max_res["hit_dice"] // 2))
        self.state["res"] = {**max_res, "hit_dice": hd}
        self.state["hp"] = self.hp_max()
        self.log("Long Rest completed.")

    # --- Leveling ---

    def track(self, fn):
        """Run fn, then grant current HP/resources by how much the maximums grew."""
        old_hp, old_res = self.hp_max(), self.max_res()
        fn()
        new_res = self.max_res()
        for k in new_res: self.state["res"][k] = max(0, self.state["res"][k] + new_res[k] - old_res[k])
        gain = self.hp_max() - old_hp
        self.state["hp"] = max(0, self.state["hp"] + gain)
        return gain

    def raise_ability(self, n, optional=False):
        while True:
            ab = input(f"{C_W}+{n} to ({'/'.join(ABILITIES)}{', blank = none' if optional else ''}): {C_R}").strip().upper()
            if optional and not ab: return
            if ab in ABILITIES:
                self.c["stats"][ab] = min(20, self.c["stats"][ab] + n)
                print(f"{ab} is now {self.c['stats'][ab]}")
                return

    def asi(self):
        print(f"\n{C_H}Ability Score Improvement{C_R}")
        i = self.pick("Choice", ["+2 to one ability", "+1 to two abilities", "Feat"])
        if i == 2:
            name = input(f"{C_W}Feat name: {C_R}").strip() or "Feat"
            self.c["features"][f"Feat: {name}"] = input(f"{C_W}What it does: {C_R}").strip()
            print("Does the feat give +1 to an ability?")
            self.raise_ability(1, optional=True)
        elif i == 1:
            self.raise_ability(1)
            self.raise_ability(1)
        else:
            self.raise_ability(2)

    def learn_maneuver(self, required=False):
        options = [m for m in MANEUVERS if m not in self.c["maneuvers"]]
        while True:
            i = self.pick("Maneuver", [f"{m}: {MANEUVERS[m]}" for m in options])
            if i is not None:
                self.c["maneuvers"].append(options[i])
                return
            if not required: return

    def extras(self):
        while True:
            print(f"\n{C_H}S{C_R}: Skill proficiency | {C_H}M{C_R}: Learn maneuver | {C_H}R{C_R}: Replace maneuver | "
                  f"{C_H}F{C_R}: Feature/note | {C_H}A{C_R}: +1 ability | {C_H}Enter{C_R}: Done")
            cmd = input(f"{C_H}extra> {C_R}").strip().upper()
            if not cmd: return
            elif cmd == "S":
                options = [s for s in SKILLS if s not in self.c["skills"]]
                i = self.pick("Skill", [f"{s} ({SKILLS[s]})" for s in options])
                if i is not None: self.c["skills"].append(options[i])
            elif cmd == "M": self.learn_maneuver()
            elif cmd == "R":
                i = self.pick("Forget", self.c["maneuvers"])
                if i is not None:
                    self.c["maneuvers"].pop(i)
                    self.learn_maneuver(required=True)
            elif cmd == "F":
                name = input(f"{C_W}Name: {C_R}").strip()
                if name: self.c["features"][name] = input(f"{C_W}Description: {C_R}").strip()
            elif cmd == "A": self.raise_ability(1)

    def level_up(self):
        old_known = by_level(self.c["level"], MANEUVERS_KNOWN)
        self.c["level"] += 1
        l = self.c["level"]
        os.system('cls' if os.name=='nt' else 'clear')
        print(f"{C_H}=== LEVEL UP: Fighter {l} ==={C_R}")
        for name, desc in FIGHTER_LEVELS.get(l, {}).items():
            self.c["features"][name] = desc
            print(f"New: {name}: {desc}")
        if l in ASI_LEVELS: self.asi()
        new = by_level(l, MANEUVERS_KNOWN) - old_known
        if new: print(f"\n{C_H}Learn {new} new maneuvers{C_R}")
        for _ in range(new): self.learn_maneuver(required=True)
        if new or l in ASI_LEVELS:
            rule = "Combat Superiority" if new else "Martial Versatility, optional TCE"
            print(f"{C_W}You may replace one known maneuver now (R, {rule}).{C_R}")
        print(f"\nAnything else from this level (feats, DM rewards, maneuver swap)?")
        self.extras()

    # --- Menus ---

    def run(self):
        while True:
            os.system('cls' if os.name=='nt' else 'clear')
            w = shutil.get_terminal_size((80, 20)).columns - 2
            c, s, r, mr = self.c, self.state, self.state["res"], self.max_res()
            l, p, dex = c["level"], self.prof(), self.mod("DEX")
            checks = self.checks()

            bar = "#" * int(20 * s["hp"] / self.hp_max())
            status = [
                f"{c['name']} | {c['race']} | Fighter {l} (Battle Master) | {c['background']}",
                f"HP: {s['hp']}/{self.hp_max()} [{bar:<20}] | AC {c['ac_base'] + dex} | Init {dex:+} | Prof +{p}",
                " | ".join(f"{k} {v}({mod(v):+})" for k, v in c["stats"].items()),
                f"Attacks/action: {by_level(l, ATTACKS_PER_ACTION)} | Maneuver DC {8 + p + max(dex, self.mod('STR'))} | "
                f"Passive Perception {10 + checks.get('Perception', self.mod('WIS'))}",
                f"Superiority d{by_level(l, SUP_DIE)}: {r['superiority']}/{mr['superiority']} | Second Wind: {r['second_wind']} | "
                f"Action Surge: {r['action_surge']}/{mr['action_surge']}" + (f" | Indomitable: {r['indomitable']}" if mr["indomitable"] else ""),
                f"Misty Step: {r['misty_step']} | Command: {r['command']} | Hit Dice d10: {r['hit_dice']}/{mr['hit_dice']} | Daggers: {s['daggers']}",
            ]
            left_w = w if w < SIDE_MIN else w - LOG_W - 3
            left = box("CHARACTER STATUS", status, left_w)

            acts = self.actions()
            menu = []
            for k, (kind, a) in acts.items():
                if kind == "attack":
                    dmg = self.attack_dmg(a)
                    menu.append(f"{k:<2} | {a['name']:<20} | {dex + p:+} | d4{f'{dmg:+}' if dmg else ''} {a.get('desc', '')}")
                else:
                    menu.append(f"{k:<2} | {a['name']:<20} | {a['desc']}")
            left += box("ACTIONS", menu, left_w)
            left += box("MANEUVERS (M: roll superiority die)", [f"{m}: {MANEUVERS.get(m, '')}" for m in c["maneuvers"]], left_w)

            check_lines = [f"{i}.{n} {v:+}" for i, (n, v) in enumerate(checks.items(), 1)]
            left += box("CHECKS", [" ".join(check_lines[i:i + 5]) for i in range(0, len(check_lines), 5)], left_w)
            left += box("PROFILE", [f"{k}: {v}" for k, v in c["features"].items()] + c["info"], left_w)
            if w < SIDE_MIN:
                print("\n".join(left + box("COMBAT LOG", self.logs[-6:], w)))
            else:
                n = len(left) - 2  # log fills the height of the left column
                log = self.logs[-n:]
                right = box("COMBAT LOG", log + [""] * (n - len(log)), LOG_W)
                print("\n".join(f"{l} {r}" for l, r in zip(left, right)))

            print(f" {C_H}D{C_R}: Damage | {C_H}H{C_R}: Heal | {C_H}I{C_R}: Initiative | {C_H}M{C_R}: Superiority | {C_H}T{C_R}: Check | "
                  f"{C_H}P{C_R}: Pick up daggers | {C_H}S{C_R}: Short Rest | {C_H}L{C_R}: Long Rest | "
                  f"{C_H}U{C_R}: Level Up | {C_H}E{C_R}: Edit | {C_H}Q{C_R}: Quit")
            cmd = input(f"\n{C_H}rowan> {C_R}").strip().upper()

            if cmd == "Q": break
            elif cmd == "D":
                n = self.ask_int("Damage taken: ")
                s["hp"] = max(0, s["hp"] - n)
                self.log(f"Took {n} damage." + (f" {C_W}DOWN! Death saves.{C_R}" if s["hp"] == 0 else ""))
            elif cmd == "H":
                n = self.ask_int("HP healed: ")
                self.heal(n)
                self.log(f"Healed {n} HP.")
            elif cmd == "I":
                d20 = random.randint(1, 20)
                self.log(f"Initiative: {d20}{dex:+}={C_H}{d20 + dex}{C_R}")
            elif cmd == "M": self.superiority()
            elif cmd == "T": self.check()
            elif cmd == "P":
                s["daggers"] = self.ask_int("Daggers in hand now: ")
                self.log(f"Daggers: {s['daggers']}")
            elif cmd == "S": self.short_rest()
            elif cmd == "L": self.long_rest()
            elif cmd == "U":
                if l >= 20:
                    self.log("Already level 20.")
                else:
                    gain = self.track(self.level_up)
                    self.log(f"{C_H}Level {c['level']}!{C_R} HP max {self.hp_max()} (+{gain})")
            elif cmd == "E": self.track(self.extras)
            elif cmd in acts:
                kind, a = acts[cmd]
                self.attack(a) if kind == "attack" else self.use_feature(a)
            else:
                self.log("Unknown command.")
            self.save_data()

if __name__ == "__main__":
    try:
        RowanTUI().run()
    except KeyboardInterrupt:
        print("\nExiting...")

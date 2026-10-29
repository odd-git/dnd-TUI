use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::OnceLock;

pub const ABILITIES: [&str; 6] = ["STR", "DEX", "CON", "INT", "WIS", "CHA"];

pub const SKILLS: [(&str, &str); 18] = [
    ("Acrobatics", "DEX"), ("Animal Handling", "WIS"), ("Arcana", "INT"), ("Athletics", "STR"),
    ("Deception", "CHA"), ("History", "INT"), ("Insight", "WIS"), ("Intimidation", "CHA"),
    ("Investigation", "INT"), ("Medicine", "WIS"), ("Nature", "INT"), ("Perception", "WIS"),
    ("Performance", "CHA"), ("Persuasion", "CHA"), ("Religion", "INT"), ("Sleight of Hand", "DEX"),
    ("Stealth", "DEX"), ("Survival", "WIS"),
];

pub const CONDITIONS: [&str; 14] = [
    "Blinded", "Charmed", "Deafened", "Frightened", "Grappled", "Incapacitated", "Invisible",
    "Paralyzed", "Petrified", "Poisoned", "Prone", "Restrained", "Stunned", "Unconscious",
];

pub fn mod_of(score: i32) -> i32 { (score - 10).div_euclid(2) }

/// Value of a level table ({level: value}) holding until the next key; 0 before the first.
pub fn at(table: &BTreeMap<u32, u32>, level: u32) -> u32 {
    table.range(..=level).next_back().map(|(_, v)| *v).unwrap_or(0)
}

// --- Rules data shipped inside the binary -----------------------------------

pub use crate::lists::Progression;

/// "Class/Subclass" (e.g. "Fighter/Battle Master") -> merged level tables.
pub fn progression(name: &str) -> Option<&'static Progression> {
    let (class, sub) = name.split_once('/').map_or((name, None), |(c, s)| (c, Some(s)));
    crate::lists::progression(class, sub)
}

pub fn maneuvers() -> &'static BTreeMap<String, String> {
    static M: OnceLock<BTreeMap<String, String>> = OnceLock::new();
    M.get_or_init(|| serde_json::from_str(include_str!("../data/maneuvers.json")).expect("maneuvers"))
}

/// (file name, contents) of the sheets seeded on first run.
pub const BUNDLED: [(&str, &str); 2] = [
    ("rowan.json", include_str!("../data/characters/rowan.json")),
    ("sorlock.json", include_str!("../data/characters/sorlock.json")),
];

// --- Character sheet + live state (one JSON file per character) --------------

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Feature { pub name: String, pub desc: String }

#[derive(Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum Max { N(u32), Kw(String) } // Kw: "level" or "table" (progression)

#[derive(Serialize, Deserialize, Clone)]
pub struct ResourceDef {
    pub id: String,
    pub name: String,
    #[serde(default)] pub recharge: String, // "short" | "long" | "" (never, e.g. items)
    pub max: Option<Max>,                   // None: a plain counter (items)
    pub start: Option<u32>,
    #[serde(default)] pub pips: bool,
    pub die: Option<String>,                // progression.dice table giving the die size
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct ActionDef {
    pub name: String,
    pub desc: String,
    pub ability: Option<String>,   // to-hit ability; with dice and no ability the damage is automatic
    pub dice: Option<(u32, u32)>,
    pub dmg_ability: bool,         // add the ability modifier to damage
    pub bonus: i32,
    pub dmg_type: String,
    pub level_bonus: bool,         // add character level (Second Wind)
    pub heal: bool,
    pub uses: Option<String>,      // resource spent
    pub cost: Vec<String>,         // first resource with something left is spent (spell slots)
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Toggle {
    pub name: String,
    pub crit_min: Option<u32>,
    pub dmg_bonus_prof: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Character {
    pub name: String,
    #[serde(default)] pub race: String,
    #[serde(default)] pub background: String,
    pub class_title: String,
    pub level: u32,
    pub hit_die: Option<u32>,
    pub abilities: BTreeMap<String, i32>,
    #[serde(default)] pub save_profs: Vec<String>,
    #[serde(default)] pub skills: Vec<String>,
    pub ac_base: Option<i32>, // armor base (+ shield, style bonus) before DEX
    #[serde(default)] pub ac_dex_max: Option<i32>, // medium armor 2, heavy 0; None = full DEX
    pub spell_ability: Option<String>,
    pub progression: Option<String>,
    pub maneuver_pool: Option<String>,
    #[serde(default)] pub maneuvers: Vec<String>,
    #[serde(default)] pub resources: Vec<ResourceDef>,
    #[serde(default)] pub actions: Vec<ActionDef>,
    #[serde(default)] pub toggles: Vec<Toggle>,
    #[serde(default)] pub features: Vec<Feature>,
    #[serde(default)] pub info: Vec<String>,
    #[serde(default)] pub journal: Vec<String>,
    // live state
    pub hp: Option<i32>,
    #[serde(default)] pub death: [u8; 2], // successes, failures
    #[serde(default)] pub conditions: Vec<String>,
    #[serde(default)] pub active: Vec<String>,
    #[serde(default)] pub res: BTreeMap<String, u32>,
}

impl Character {
    pub fn prog(&self) -> Option<&'static Progression> {
        self.progression.as_deref().and_then(progression)
    }
    pub fn prof(&self) -> i32 { (self.level as i32 - 1) / 4 + 2 }
    pub fn md(&self, ab: &str) -> i32 { mod_of(self.abilities.get(ab).copied().unwrap_or(10)) }
    pub fn ac(&self) -> Option<i32> {
        let dex = self.md("DEX");
        Some(self.ac_base? + self.ac_dex_max.map_or(dex, |m| dex.min(m)))
    }
    pub fn dc(&self, ab: &str) -> i32 { 8 + self.prof() + self.md(ab) }
    pub fn title(&self) -> String { self.class_title.replace("{level}", &self.level.to_string()) }

    pub fn hp_max(&self) -> Option<i32> {
        let d = self.hit_die? as i32;
        let cm = self.md("CON");
        Some(d + cm + (self.level as i32 - 1) * (d / 2 + 1 + cm))
    }

    pub fn def(&self, id: &str) -> Option<&ResourceDef> { self.resources.iter().find(|r| r.id == id) }

    pub fn res_max(&self, id: &str) -> Option<u32> {
        match self.def(id)?.max.as_ref()? {
            Max::N(n) => Some(*n),
            Max::Kw(k) if k == "level" => Some(self.level),
            Max::Kw(_) => Some(self.prog().and_then(|p| p.resources.get(id)).map(|t| at(t, self.level)).unwrap_or(0)),
        }
    }

    pub fn res_die(&self, id: &str) -> Option<u32> {
        if id == "hit_dice" { return self.hit_die; }
        let table = self.def(id)?.die.as_ref()?;
        self.prog()?.dice.get(table).map(|t| at(t, self.level))
    }

    /// Fill in state missing from a fresh sheet (first run, or a resource added later).
    pub fn init(&mut self) {
        for d in self.resources.clone() {
            if !self.res.contains_key(&d.id) {
                let v = self.res_max(&d.id).or(d.start).unwrap_or(0);
                self.res.insert(d.id, v);
            }
        }
        if self.hp.is_none() { self.hp = self.hp_max(); }
    }

    pub fn ability_names(&self) -> Vec<&'static str> {
        ABILITIES.iter().copied().filter(|a| self.abilities.contains_key(*a)).collect()
    }

    pub fn proficient(&self, name: &str) -> bool {
        match name.strip_suffix(" save") {
            Some(a) => self.save_profs.iter().any(|s| s == a),
            None => self.skills.iter().any(|s| s == name),
        }
    }

    /// Modifier of an ability ("DEX"), a save ("DEX save") or a skill ("Stealth").
    pub fn check_mod(&self, name: &str) -> i32 {
        let base = match name.strip_suffix(" save") {
            Some(a) => self.md(a),
            None if ABILITIES.contains(&name) => return self.md(name),
            None => self.md(SKILLS.iter().find(|(n, _)| *n == name).map_or("INT", |x| x.1)),
        };
        base + if self.proficient(name) { self.prof() } else { 0 }
    }

    /// Abilities, saves and proficient skills (the quick-roll list).
    pub fn checks(&self) -> Vec<(String, i32)> {
        let abs = self.ability_names();
        let mut names: Vec<String> = abs.iter().map(|a| a.to_string()).collect();
        names.extend(abs.iter().map(|a| format!("{a} save")));
        names.extend(self.skills.iter().cloned());
        names.into_iter().map(|n| { let m = self.check_mod(&n); (n, m) }).collect()
    }

    /// Replace `{dc:INT}` and `{level}` in sheet texts.
    pub fn fmt(&self, s: &str) -> String {
        let mut out = String::new();
        let mut rest = s;
        while let Some(i) = rest.find('{') {
            out.push_str(&rest[..i]);
            let tail = &rest[i..];
            let Some(j) = tail.find('}') else { out.push_str(tail); rest = ""; break };
            match tail[1..j].split_once(':') {
                Some(("dc", ab)) => out.push_str(&self.dc(ab).to_string()),
                _ if &tail[1..j] == "level" => out.push_str(&self.level.to_string()),
                _ => out.push_str(&tail[..=j]),
            }
            rest = &tail[j + 1..];
        }
        out.push_str(rest);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rowan() -> Character {
        let mut c: Character = serde_json::from_str(BUNDLED[0].1).unwrap();
        c.init();
        c
    }

    #[test]
    fn level_three_matches_the_paper_sheet() {
        let c = rowan();
        assert_eq!(c.hp_max(), Some(28));
        assert_eq!(c.prof(), 2);
        assert_eq!(c.dc("INT"), 13);
        assert_eq!(c.res["superiority"], 4);
        assert_eq!(c.res_max("indomitable"), Some(0));
        assert_eq!(c.fmt("DC {dc:INT}"), "DC 13");
    }

    #[test]
    fn proficiency_list_at_level_three() {
        let c = rowan();
        let want = [("STR save", 1), ("CON save", 4), ("Insight", 2), ("Perception", 2), ("History", 5),
                    ("Persuasion", 2), ("Investigation", 5), ("Arcana", 3), ("Nature", 3), ("Stealth", 3), ("Athletics", -1)];
        for (name, m) in want { assert_eq!(c.check_mod(name), m, "{name}"); }
        assert_eq!(c.skills.len(), 5);
    }

    #[test]
    fn level_seven_progression() {
        let mut c = rowan();
        c.level = 7;
        c.abilities.insert("DEX".into(), 18);
        c.abilities.insert("CON".into(), 15);
        assert_eq!(c.hp_max(), Some(60));
        assert_eq!(c.prof(), 3);
        assert_eq!(c.res_max("superiority"), Some(5));
        assert_eq!(c.res_die("superiority"), Some(8));
        assert_eq!(c.prog().unwrap().maneuvers_known[&7], 5);
    }

    #[test]
    fn bundled_sheets_parse() {
        for (_, json) in BUNDLED { serde_json::from_str::<Character>(json).unwrap(); }
        assert_eq!(maneuvers().len(), 23);
    }
}

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

/// What each exhaustion level adds (PHB appendix A); level 6 is death.
pub const EXHAUSTION: [&str; 6] = ["checks at disadvantage", "speed halved", "attacks and saves at disadvantage", "HP max halved", "speed 0", "dead"];

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
pub struct Feature {
    pub name: String,
    pub desc: String,
    #[serde(default, skip_serializing_if = "String::is_empty")] pub url: String, // the wiki page with the full text
}

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
    pub level_x: u32,              // add this many times the character level (Wholeness of Body: 3)
    pub heal: bool,
    pub uses: Option<String>,      // resource spent
    pub spend: u32,                // ...this many of it each time (0 means 1: Shadow Arts costs 2 ki)
    pub cost: Vec<String>,         // first resource with something left is spent (spell slots)
    pub dice_table: Option<String>, // level table giving the number of dice (Sneak Attack)
    pub die_table: Option<String>,  // level table giving the die size (Martial Arts)
    pub min_level: u32,             // hidden below this level
    pub pool: Option<String>,       // spend any number of points from this resource (Lay on Hands)
    pub special: Option<String>,    // own mechanics: smite, recover, font_slot, font_points, wild_shape
    pub hit: Option<i32>,           // fixed attack bonus (a beast's stat block) instead of ability + proficiency
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Toggle {
    pub name: String,
    pub crit_min: Option<u32>,
    pub dmg_bonus_prof: bool,
    pub desc: String,
    pub uses: Option<String>,        // resource spent when switched on (Rage)
    pub dmg_table: Option<String>,   // level table of a flat damage bonus
    pub dmg_ability: Option<String>, // ...applied to attacks made with this ability
    pub exhaust: bool,               // one exhaustion level when switched off (Frenzy)
    pub hit_ability: Option<String>, // this ability's modifier added to weapon attack rolls (Sacred Weapon)
    pub min_level: u32,              // hidden below this class level (Overchannel)
}

/// Something carried: weight is per item, in pounds.
#[derive(Serialize, Deserialize, Clone)]
pub struct Gear { pub name: String, pub qty: u32, #[serde(default)] pub weight: f32, #[serde(default)] pub attuned: bool }

/// Coin denominations and their value in copper, largest first (the order of `Character::coins`).
pub const COINS: [(&str, u32); 5] = [("pp", 1000), ("gp", 100), ("ep", 50), ("sp", 10), ("cp", 1)];

/// The beast a druid has turned into: its name and AC (its HP are the "beast_hp" counter, its attacks "Beast: " actions).
#[derive(Serialize, Deserialize, Clone)]
pub struct Beast { pub name: String, pub ac: i32, #[serde(default)] pub info: String, #[serde(default)] pub hp: i32 }

/// A class taken besides the main one (multiclassing): its "Class/Subclass" progression and levels in it.
#[derive(Serialize, Deserialize, Clone)]
pub struct Extra { pub prog: String, pub level: u32 }

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
    #[serde(default)] pub expertise: Vec<String>, // double proficiency
    #[serde(default)] pub cantrips: Vec<String>,
    #[serde(default)] pub spells: Vec<String>,    // spells known, or the wizard's spellbook
    #[serde(default)] pub prepared: Vec<String>,  // prepared spells (prepared / spellbook casters)
    pub ac_base: Option<i32>, // armor base (+ shield, style bonus) before DEX
    #[serde(default)] pub ac_dex_max: Option<i32>, // medium armor 2, heavy 0; None = full DEX
    #[serde(default)] pub ac_extra: Option<String>, // Unarmored Defense: second ability added with no armor
    #[serde(default)] pub ac_extra_shield: bool,    // ...and still while carrying a shield
    #[serde(default)] pub ac_armored: bool,
    #[serde(default)] pub ac_shield: bool,
    pub spell_ability: Option<String>,
    pub progression: Option<String>,
    #[serde(default)] pub multi: Vec<Extra>,
    pub maneuver_pool: Option<String>,
    #[serde(default)] pub maneuvers: Vec<String>,
    #[serde(default)] pub resources: Vec<ResourceDef>,
    #[serde(default)] pub actions: Vec<ActionDef>,
    #[serde(default)] pub toggles: Vec<Toggle>,
    #[serde(default)] pub features: Vec<Feature>,
    #[serde(default)] pub info: Vec<String>,
    #[serde(default)] pub journal: Vec<String>,
    #[serde(default)] pub inventory: Vec<Gear>,
    #[serde(default)] pub coins: [u32; 5], // pp gp ep sp cp
    // live state
    pub hp: Option<i32>,
    #[serde(default)] pub temp_hp: i32,
    #[serde(default)] pub beast: Option<Beast>,
    #[serde(default)] pub concentration: Option<String>, // spell being concentrated on
    #[serde(default)] pub death: [u8; 2], // successes, failures
    #[serde(default)] pub conditions: Vec<String>,
    #[serde(default)] pub exhaustion: u8, // 0-6
    #[serde(default)] pub inspiration: bool,
    #[serde(default)] pub portent: Vec<u32>, // Portent d20s rolled at the last long rest, not used yet
    #[serde(default)] pub active: Vec<String>,
    #[serde(default)] pub res: BTreeMap<String, u32>,
}

impl Character {
    pub fn prog(&self) -> Option<&'static Progression> {
        self.progression.as_deref().and_then(progression)
    }

    /// Levels in the main class; the tables of its progression are indexed by this, not by the character level.
    pub fn class_level(&self) -> u32 { self.level - self.multi.iter().map(|e| e.level).sum::<u32>() }

    /// Every class as ("Class/Subclass", levels in it), the main one first.
    pub fn classes(&self) -> Vec<(String, u32)> {
        self.progression.iter().map(|p| (p.clone(), self.class_level()))
            .chain(self.multi.iter().map(|e| (e.prog.clone(), e.level))).collect()
    }

    /// Classes that cast, with their casting rules.
    pub fn casters(&self) -> Vec<(&'static crate::lists::Casting, u32, String)> {
        self.classes().into_iter().filter(|(_, l)| *l > 0)
            .filter_map(|(p, l)| Some((crate::lists::casting_of(&p)?, l, p))).collect()
    }

    /// The class whose spell lists the sheet manages: the first that casts, as (rules, its levels, progression).
    // ponytail: a second caster class gets slots and the combined table but no known / prepared limits, add spells by hand in the editor
    pub fn cast_src(&self) -> Option<(&'static crate::lists::Casting, u32, String)> {
        self.casters().into_iter().find(|(c, l, _)| c.top_level(*l) > 0 || at(&c.cantrips, *l) > 0)
    }
    pub fn casting(&self) -> Option<&'static crate::lists::Casting> { self.cast_src().map(|x| x.0) }

    /// Spell slots of a level: multiclass casters pool their levels (full + half/2 + third/3) on the wizard table.
    fn slots(&self, l: usize) -> u32 {
        let cs: Vec<_> = self.casters().into_iter().filter(|(c, ..)| c.kind != "pact").collect();
        match cs.as_slice() {
            [] => 0,
            [(c, lv, _)] => c.slots_at(*lv, l),
            _ => {
                let cl: u32 = cs.iter().map(|(c, lv, _)| match c.kind.as_str() { "half" => lv / 2, "third" => lv / 3, _ => *lv }).sum();
                crate::lists::class_data("Wizard").and_then(|w| w.casting.as_ref()).map_or(0, |w| w.slots_at(cl, l))
            }
        }
    }

    /// True once there is anything to cast (a half caster has nothing at level 1).
    pub fn casts(&self) -> bool { self.cast_src().is_some() }

    /// Anything for the Spells tab: a casting class, or spells from a feat, race or pact on any sheet.
    pub fn has_spells(&self) -> bool { self.casts() || !self.cantrips.is_empty() || !self.spells.is_empty() || !self.rituals().is_empty() }

    /// Spells a subclass that does not cast gives: rituals (Totem Warrior) or cantrips (Shadow); never with slots.
    pub fn rituals(&self) -> Vec<String> {
        self.classes().iter().filter(|(p, _)| crate::lists::casting_of(p).is_none()).flat_map(|(p, l)| crate::lists::always_spells(p, *l)).collect()
    }

    /// On the spell list of one of the classes (patron spells included). Spells from elsewhere (Magic Initiate,
    /// Pact of the Tome, a DM's gift) are always castable and never count against spells known.
    /// Spellcasting ability for a spell: the class whose list has it (the better one when two do), the warlock's
    /// for pact gifts, otherwise the sheet's.
    pub fn ability_for(&self, name: &str) -> String {
        let sp = crate::lists::spell(name);
        let pact = self.features.iter().any(|f| f.name == format!("Bonus spell: {name}") && f.desc.starts_with("From Pact"));
        let mut best: Option<String> = None;
        for (c, l, p) in self.casters() {
            let ok = sp.is_some_and(|s| s.classes.contains(&c.list))
                || crate::lists::always_spells(&p, l).iter().chain(&crate::lists::expanded_spells(&p, 9)).any(|x| x == name)
                || (pact && c.kind == "pact");
            if ok && best.as_ref().is_none_or(|b| self.md(&c.ability) > self.md(b)) { best = Some(c.ability.clone()); }
        }
        best.or_else(|| self.spell_ability.clone()).unwrap_or_else(|| "INT".into())
    }

    /// "CHA DC 13 atk +5", one per casting ability the sheet uses.
    pub fn spell_dcs(&self) -> String {
        let mut abs: Vec<String> = self.casters().iter().map(|c| c.0.ability.clone()).collect();
        if abs.is_empty() { abs.extend(self.spell_ability.clone()); }
        abs.sort();
        abs.dedup();
        abs.iter().map(|a| format!("{a} DC {} atk {:+}", self.dc(a), self.prof() + self.md(a))).collect::<Vec<_>>().join(" | ")
    }

    /// Granted on top of the class picks (Pact of the Tome cantrips, Pact of the Chain's familiar).
    pub fn bonus_spell(&self, name: &str) -> bool { self.features.iter().any(|f| f.name == format!("Bonus spell: {name}")) }

    pub fn on_list(&self, name: &str) -> bool {
        let Some(sp) = crate::lists::spell(name) else { return false };
        self.casters().iter().any(|(c, _, p)| sp.classes.contains(&c.list) || crate::lists::expanded_spells(p, 9).contains(&sp.name))
    }

    /// Castable as a ritual (10 minutes longer, no slot): a wizard from the spellbook, a cleric or druid if prepared,
    /// a bard if known, any ritual the sheet has with Book of Ancient Secrets or the Ritual Caster feat, or a subclass's.
    pub fn ritual_ok(&self, name: &str) -> bool {
        let Some(sp) = crate::lists::spell(name).filter(|s| s.ritual && s.level > 0) else { return false };
        let (known, prepared) = (self.spells.contains(&sp.name), self.prepared.contains(&sp.name));
        let book = self.features.iter().any(|f| f.name == "Invocation: Book of Ancient Secrets" || f.name == "Feat: Ritual Caster");
        (book && (known || prepared)) || self.rituals().contains(&sp.name) || self.casters().iter().any(|(c, ..)| sp.classes.contains(&c.list) && match c.list.as_str() {
            "Wizard" | "Bard" => known,
            "Cleric" | "Druid" => prepared,
            _ => false,
        })
    }

    pub fn wealth_cp(&self) -> u32 { self.coins.iter().zip(COINS).map(|(n, (_, v))| n * v).sum() }

    /// Spend `n` coins of denomination `d` (index into COINS), breaking larger coins for change or paying the
    /// rest with smaller ones; false (nothing spent) if the purse is short.
    pub fn pay(&mut self, d: usize, n: u32) -> bool {
        if self.wealth_cp() < n * COINS[d].1 { return false; }
        while self.coins[d] < n {
            match (0..d).rev().find(|&h| self.coins[h] > 0) {
                Some(h) => { self.coins[h] -= 1; self.coins[d] += COINS[h].1 / COINS[d].1; }
                None => {
                    // only smaller coins left: pay what is missing with them, largest first
                    let mut rest = (n - self.coins[d]) * COINS[d].1;
                    self.coins[d] = 0;
                    for (n, (_, v)) in self.coins.iter_mut().zip(COINS).skip(d + 1) {
                        let k = (*n).min(rest / v);
                        *n -= k;
                        rest -= k * v;
                    }
                    return true;
                }
            }
        }
        self.coins[d] -= n;
        true
    }

    /// "2 pp, 15 gp (35 gp)": the coins held, then their worth in gold when not all gold.
    pub fn coins_text(&self) -> String {
        let held: Vec<String> = self.coins.iter().zip(COINS).filter(|(n, _)| **n > 0).map(|(n, (c, _))| format!("{n} {c}")).collect();
        if held.is_empty() { return "none".into(); }
        if self.coins.iter().enumerate().all(|(i, n)| i == 1 || *n == 0) { return held.join(", "); }
        let gp = self.wealth_cp() as f32 / 100.0;
        format!("{} ({} gp)", held.join(", "), if gp.fract() == 0.0 { format!("{gp}") } else { format!("{gp:.2}") })
    }

    pub fn cantrips_max(&self) -> usize { self.cast_src().map_or(0, |(c, l, _)| at(&c.cantrips, l) as usize) }

    /// Spells to know: the class table, or for a wizard 6 in the book plus 2 per level after the first.
    pub fn known_max(&self) -> usize {
        match self.cast_src() {
            Some((c, l, _)) if c.mode == "spellbook" => 6 + 2 * (l as usize - 1),
            Some((c, l, _)) => at(&c.known, l) as usize,
            None => 0,
        }
    }

    /// Spells a prepared / spellbook caster can have prepared (ability modifier + level, or half level).
    pub fn prepared_max(&self) -> usize {
        let Some((c, l, _)) = self.cast_src() else { return 0 };
        let lvl = if c.prepared.as_deref() == Some("mod+half") { l as i32 / 2 } else { l as i32 };
        (self.md(&c.ability) + lvl).max(1) as usize
    }

    /// Everything castable that is not a cantrip: known spells, or prepared ones plus domain / oath spells.
    pub fn castable(&self) -> Vec<String> {
        let off_list = self.spells.iter().filter(|s| !self.on_list(s)).cloned();
        let Some((c, l, prog)) = self.cast_src() else { return off_list.collect() };
        let mut v = crate::lists::always_spells(&prog, l);
        v.extend(if c.mode == "known" { self.spells.iter() } else { self.prepared.iter() }.cloned());
        v.extend(off_list);
        v.sort();
        v.dedup();
        v
    }

    /// Make sure the spell slot resources of the casting rules exist (new caster, or a subclass that casts).
    pub fn sync_spell_resources(&mut self) {
        let cs = self.casters();
        let Some((first, ..)) = cs.first() else { return };
        self.spell_ability = Some(first.ability.clone());
        let mut want: Vec<ResourceDef> = vec![];
        if cs.iter().any(|(c, ..)| c.kind == "pact") {
            want.push(ResourceDef { id: "pact".into(), name: "Pact slots".into(), recharge: "short".into(), max: Some(Max::Kw("pact".into())), start: None, pips: true, die: None });
        }
        if cs.iter().any(|(c, ..)| c.kind != "pact") {
            for l in 1..=9 {
                want.push(ResourceDef { id: format!("slot{l}"), name: format!("Level {l} slots"), recharge: "long".into(), max: Some(Max::Kw(format!("slot:{l}"))), start: None, pips: true, die: None });
            }
        }
        for d in want { if self.def(&d.id).is_none() { self.resources.push(d); } }
        self.init();
    }

    pub fn prof(&self) -> i32 { (self.level as i32 - 1) / 4 + 2 }
    /// "FINESSE" (Martial Arts) is the better of STR and DEX; any other ability is itself.
    pub fn ability<'a>(&self, ab: &'a str) -> &'a str {
        if ab != "FINESSE" { return ab; }
        if self.md("DEX") >= self.md("STR") { "DEX" } else { "STR" }
    }
    pub fn md(&self, ab: &str) -> i32 { mod_of(self.abilities.get(self.ability(ab)).copied().unwrap_or(10)) }
    pub fn ac(&self) -> Option<i32> {
        if let Some(b) = &self.beast { return Some(b.ac); }
        let dex = self.md("DEX");
        let extra = match &self.ac_extra {
            Some(ab) if !self.ac_armored && (self.ac_extra_shield || !self.ac_shield) => self.md(ab),
            _ => 0,
        };
        // Draconic Resilience: a better unarmored base (a shield still adds 2)
        let sub = self.classes().iter().filter_map(|(p, _)| crate::lists::sub_of(p)?.unarmored_ac).max().filter(|_| !self.ac_armored);
        let base = self.ac_base?.max(sub.map_or(0, |b| b + if self.ac_shield { 2 } else { 0 }));
        Some(base + self.ac_dex_max.map_or(dex, |m| dex.min(m)) + extra)
    }

    /// To-hit of an action: a beast's fixed bonus, or ability + proficiency + active toggles (Sacred Weapon).
    pub fn hit_bonus(&self, a: &ActionDef) -> Option<i32> {
        if a.hit.is_some() { return a.hit; }
        let on = self.toggles.iter().filter(|t| self.active.contains(&t.name));
        let extra: i32 = on.filter_map(|t| t.hit_ability.as_deref()).map(|ab| self.md(ab)).sum();
        a.ability.as_ref().map(|ab| self.md(ab) + self.prof() + extra)
    }

    /// Flat damage from active toggles (Rage) for an attack made with `ability`.
    pub fn toggle_bonus(&self, ability: Option<&str>) -> i32 {
        self.toggles.iter().filter(|t| self.active.contains(&t.name))
            .filter(|t| t.dmg_ability.is_some() && t.dmg_ability.as_deref() == ability.map(|a| self.ability(a)))
            .filter_map(|t| Some(self.table(t.dmg_table.as_ref()?)? as i32))
            .sum()
    }
    /// Dice of an action, the count coming from a level table when it has one.
    pub fn dice(&self, a: &ActionDef) -> Option<(u32, u32)> {
        let (n, sides) = a.dice?;
        let table = |t: &Option<String>| t.as_ref().and_then(|t| self.table(t).or_else(|| self.die_table(t)));
        Some((table(&a.dice_table).unwrap_or(n), table(&a.die_table).unwrap_or(sides)))
    }
    pub fn dc(&self, ab: &str) -> i32 { 8 + self.prof() + self.md(ab) }
    /// A level table of any of the classes, at the levels in that class.
    pub fn table(&self, id: &str) -> Option<u32> {
        self.classes().iter().find_map(|(p, l)| Some(at(progression(p)?.resources.get(id)?, *l)))
    }

    /// Attacks per action: the best of the classes, Extra Attack does not stack.
    pub fn attacks(&self) -> u32 {
        self.classes().iter().filter_map(|(p, l)| Some(at(&progression(p)?.attacks_per_action, *l))).max().unwrap_or(1)
    }
    pub fn title(&self) -> String {
        let mut t = self.class_title.replace("{level}", &self.class_level().to_string());
        for e in &self.multi {
            let (c, s) = e.prog.split_once('/').unwrap_or((&e.prog, ""));
            t += &format!(" / {c} {}{}", e.level, if s.is_empty() { String::new() } else { format!(" ({s})") });
        }
        t
    }

    pub fn hp_max(&self) -> Option<i32> {
        let d = self.hit_die? as i32;
        let cm = self.md("CON");
        let others: i32 = self.multi.iter().map(|e| {
            let hd = crate::lists::class_data(e.prog.split('/').next().unwrap_or("")).map_or(8, |c| c.hit_die) as i32;
            e.level as i32 * (hd / 2 + 1 + cm)
        }).sum();
        let subs: i32 = self.classes().iter().filter_map(|(p, l)| Some(crate::lists::sub_of(p)?.hp_per_level * *l as i32)).sum();
        let max = d + cm + (self.class_level() as i32 - 1) * (d / 2 + 1 + cm) + others + subs;
        Some(if self.exhaustion >= 4 { max / 2 } else { max })
    }

    /// Exhaustion effects so far, e.g. "checks at disadvantage, speed halved".
    pub fn exhaustion_text(&self) -> String { EXHAUSTION[..self.exhaustion.min(6) as usize].join(", ") }

    pub fn def(&self, id: &str) -> Option<&ResourceDef> { self.resources.iter().find(|r| r.id == id) }

    fn res_max_class(&self, class: &str) -> u32 {
        self.classes().iter().find(|(p, _)| p.split('/').next() == Some(class)).map_or(0, |c| c.1)
    }

    pub fn res_max(&self, id: &str) -> Option<u32> {
        match self.def(id)?.max.as_ref()? {
            Max::N(n) => Some(*n),
            Max::Kw(k) if k == "level" && id == "hit_dice" => Some(self.class_level()),
            Max::Kw(k) if k == "level" => Some(self.level),
            Max::Kw(k) if k.starts_with("class:") => Some(self.res_max_class(&k[6..])),
            Max::Kw(k) if k == "pact" => Some(self.casters().iter().find(|(c, ..)| c.kind == "pact").map_or(0, |(c, l, _)| at(&c.pact_slots, *l))),
            Max::Kw(k) if k.starts_with("slot:") => Some(self.slots(k[5..].parse().unwrap_or(1))),
            Max::Kw(k) if k.starts_with("mod:") => Some(self.md(&k[4..]).max(1) as u32),
            Max::Kw(k) if k == "arcane_ward" => Some((2 * self.res_max_class("Wizard") as i32 + self.md("INT")).max(0) as u32),
            Max::Kw(k) if k.starts_with("1+mod:") => Some((1 + self.md(&k[6..])).max(1) as u32),
            Max::Kw(_) => Some(self.prog().and_then(|p| p.resources.get(id)).map(|t| at(t, self.class_level())).or_else(|| self.table(id)).unwrap_or(0)),
        }
    }

    pub fn res_die(&self, id: &str) -> Option<u32> {
        if id == "hit_dice" { return self.hit_die; }
        if let Some(class) = id.strip_prefix("hd_") { return crate::lists::class_data(class).map(|c| c.hit_die); }
        self.die_table(self.def(id)?.die.as_ref()?)
    }

    /// A die size table of any of the classes (Bardic Inspiration), at the levels in that class.
    pub fn die_table(&self, id: &str) -> Option<u32> {
        self.classes().iter().find_map(|(p, l)| Some(at(progression(p)?.dice.get(id)?, *l)))
    }

    /// Fill in state missing from a fresh sheet (first run, or a resource added later).
    pub fn init(&mut self) {
        for d in self.resources.clone() {
            if !self.res.contains_key(&d.id) {
                let v = d.start.or_else(|| self.res_max(&d.id)).unwrap_or(0);
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
        let save = name.strip_suffix(" save");
        let ab = save.unwrap_or_else(|| if ABILITIES.contains(&name) { name } else { SKILLS.iter().find(|(n, _)| *n == name).map_or("INT", |x| x.1) });
        let mult = if self.expertise.iter().any(|s| s == name) { 2 } else { i32::from(self.proficient(name)) };
        // Remarkable Athlete (Champion): half proficiency, rounded up, to STR, DEX and CON checks without it
        let athlete = mult == 0 && save.is_none() && ["STR", "DEX", "CON"].contains(&ab) && self.features.iter().any(|f| f.name == "Remarkable Athlete");
        self.md(ab) + mult * self.prof() + if athlete { (self.prof() + 1) / 2 } else { 0 }
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

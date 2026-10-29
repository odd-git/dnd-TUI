//! Class features with mechanics of their own: option picks (Metamagic, Invocations, Pact Boon), Mystic Arcanum,
//! Magical Secrets, Flexible Casting, Arcane Recovery, Divine Smite, Wild Shape, multiclass proficiencies.

use super::*;
use crate::lists::{self, OptionDef};

/// Feature-name prefix of each option kind: a picked option is stored as the feature "Metamagic: Quickened Spell".
fn label(kind: &str) -> &'static str {
    match kind { "metamagic" => "Metamagic", "invocations" => "Invocation", "pact_boons" => "Pact Boon", _ => "Option" }
}

/// Sorcery points to create a slot of level 1..=5.
const FONT_COST: [u32; 5] = [2, 3, 5, 6, 7];

/// What a class grants when you multiclass into it (PHB chapter 6), and whether it adds one skill.
fn multiclass_profs(class: &str) -> (&'static str, bool) {
    match class {
        "Barbarian" => ("Shields, simple and martial weapons", false),
        "Bard" => ("Light armor, one musical instrument", true),
        "Cleric" | "Druid" => ("Light and medium armor, shields", false),
        "Fighter" | "Paladin" => ("Light and medium armor, shields, simple and martial weapons", false),
        "Monk" => ("Simple weapons, shortswords", false),
        "Ranger" => ("Light and medium armor, shields, simple and martial weapons", true),
        "Rogue" => ("Light armor, thieves' tools", true),
        "Warlock" => ("Light armor, simple weapons", false),
        _ => ("", false),
    }
}

/// "40 ft, climb 30 ft": walking speed first, then the others.
pub(super) fn speed_text(b: &lists::BeastDef) -> String {
    let walk = b.speed.get("walk").filter(|v| **v > 0).map(|v| format!("{v} ft"));
    walk.into_iter().chain(b.speed.iter().filter(|(k, v)| *k != "walk" && **v > 0).map(|(k, v)| format!("{k} {v} ft"))).collect::<Vec<_>>().join(", ")
}

impl App {
    fn level_of(&self, class: &str) -> u32 {
        self.ch.classes().iter().find(|(p, _)| p.split('/').next() == Some(class)).map_or(0, |c| c.1)
    }

    fn has_feature(&self, prefix: &str) -> usize { self.ch.features.iter().filter(|f| f.name.starts_with(prefix)).count() }

    /// Choices the sheet still owes for the levels it has, as steps to queue.
    pub(super) fn owed(&self) -> Vec<Step> {
        let mut v = vec![];
        for (prog, lvl) in self.ch.classes() {
            let Some(cd) = lists::class_data(prog.split('/').next().unwrap_or("")) else { continue };
            for (kind, table) in &cd.options {
                let have = self.has_feature(&format!("{}: ", label(kind)));
                for _ in have..at(table, lvl) as usize { v.push(Step::Opt(kind.clone())); }
            }
            if cd.name == "Warlock" {
                for (l, s) in [(11, 6), (13, 7), (15, 8), (17, 9)] {
                    if lvl >= l && self.ch.def(&format!("arcanum{s}")).is_none() { v.push(Step::Arcanum(s)); }
                }
            }
            if cd.name == "Bard" {
                let mut want = 2 * [10, 14, 18].iter().filter(|&&l| lvl >= l).count();
                if prog.ends_with("/Lore") && lvl >= 6 { want += 2; }
                for _ in self.has_feature("Magical Secret: ")..want { v.push(Step::Secret); }
            }
        }
        v
    }

    /// Resources and actions the class data has but the sheet lacks (a sheet made before they were added to the data).
    fn sync_class(&mut self) {
        let Some(prog) = self.ch.progression.clone() else { return };
        let (cls, sub) = prog.split_once('/').map_or((prog.as_str(), None), |(c, s)| (c, Some(s)));
        let Some(cd) = lists::class_data(cls) else { return };
        let sd = sub.and_then(|s| cd.subclasses.iter().find(|x| x.name == s));
        for r in cd.resources.iter().chain(sd.iter().flat_map(|s| s.resources.iter())) {
            if self.ch.def(&r.id).is_none() { self.ch.resources.push(r.clone()); }
        }
        for a in cd.actions.iter().chain(sd.iter().flat_map(|s| s.actions.iter())) {
            if !self.ch.actions.iter().any(|x| x.name == a.name) { self.ch.actions.push(a.clone()); }
        }
        for t in cd.toggles.iter().chain(sd.iter().flat_map(|s| s.toggles.iter())) {
            if !self.ch.toggles.iter().any(|x| x.name == t.name) { self.ch.toggles.push(t.clone()); }
        }
        let extra: Vec<String> = self.ch.multi.iter().map(|e| e.prog.split('/').next().unwrap_or("").to_string()).collect();
        for c in extra { self.hit_dice_pool(&c); }
        self.ch.sync_spell_resources();
        self.ch.init();
        // a sheet multiclassed before the pools were split may hold more dice than its main class gives
        for id in ["hit_dice".to_string()].into_iter().chain(self.ch.resources.iter().filter(|r| r.id.starts_with("hd_")).map(|r| r.id.clone()).collect::<Vec<_>>()) {
            if let (Some(max), Some(v)) = (self.ch.res_max(&id), self.ch.res.get_mut(&id)) { *v = (*v).min(max); }
        }
    }

    /// Queue what a sheet owes (a new character, or one made before these features existed).
    pub(super) fn catch_up(&mut self) {
        self.sync_class();
        let owed = self.owed();
        if owed.is_empty() { return; }
        self.flow_begin();
        self.queue.extend(owed);
        self.advance();
    }

    fn prereq_ok(&self, o: &OptionDef, lvl: u32) -> bool {
        let Some(p) = &o.prereq else { return true };
        p.split(", ").all(|part| {
            if let Some(n) = part.split(|c: char| !c.is_ascii_digit()).next().and_then(|n| n.parse::<u32>().ok()) { return lvl >= n; }
            if let Some(pact) = part.strip_suffix(" feature") { return self.ch.features.iter().any(|f| f.name == format!("Pact Boon: {pact}")); }
            if let Some(sp) = part.strip_suffix(" cantrip") { return self.ch.cantrips.iter().any(|c| c.eq_ignore_ascii_case(sp)); }
            true // "hex spell or a warlock feature that curses": left to the player
        })
    }

    pub(super) fn opt_items(&self, kind: &str) -> Vec<&'static OptionDef> {
        let lvl = self.level_of(if kind == "metamagic" { "Sorcerer" } else { "Warlock" });
        let have = |n: &str| self.ch.features.iter().any(|f| f.name == format!("{}: {n}", label(kind)));
        lists::options(kind).iter().filter(|o| !have(&o.name) && self.prereq_ok(o, lvl)).collect()
    }

    pub(super) fn opt_open(&mut self, kind: &str, step: Step) {
        let items = self.opt_items(kind).iter().map(|o| format!("{} ({}): {}", o.name, lists::abbr(&o.source), o.desc)).collect();
        self.list(&format!("Choose: {}", label(kind)), items, step, false);
    }

    pub(super) fn opt_pick(&mut self, kind: &str, i: usize) {
        let Some(o) = self.opt_items(kind).get(i).copied() else { return };
        self.snap_in_flow();
        self.ch.features.push(Feature { name: format!("{}: {}", label(kind), o.name), desc: o.desc.clone(), url: o.url.clone() });
        self.say(format!("{}: {}.", label(kind), o.name), Tone::Good);
        match o.name.as_str() {
            "Pact of the Tome" => for _ in 0..3 { self.queue.push_front(Step::AnyCantrip); },
            "Pact of the Chain" => self.grant_spell("Find Familiar", "Pact of the Chain"),
            _ => {}
        }
    }

    pub(super) fn any_cantrips(&self) -> Vec<String> {
        lists::spells().iter().filter(|s| s.level == 0 && !self.ch.cantrips.contains(&s.name)).map(|s| s.name.clone()).collect()
    }

    /// A spell given by a feature, outside the class picks: known, castable, not counted against spells known.
    pub(super) fn grant_spell(&mut self, name: &str, from: &str) {
        let Some(sp) = lists::spell(name) else { return };
        if sp.level == 0 { self.ch.cantrips.push(sp.name.clone()) } else { self.ch.spells.push(sp.name.clone()) }
        self.ch.features.push(Feature { name: format!("Bonus spell: {}", sp.name), desc: format!("From {from}"), url: sp.url.clone() });
        self.say(format!("{from}: {}.", sp.name), Tone::Good);
    }

    pub(super) fn arcanum_items(&self, level: u32) -> Vec<String> {
        lists::spells().iter().filter(|s| s.level == level && s.classes.iter().any(|c| c == "Warlock"))
            .map(|s| s.name.clone()).filter(|n| !self.ch.spells.contains(n)).collect()
    }

    pub(super) fn arcanum_pick(&mut self, level: u32, i: usize) {
        let Some(n) = self.arcanum_items(level).get(i).cloned() else { return };
        self.snap_in_flow();
        self.ch.spells.push(n.clone());
        let id = format!("arcanum{level}");
        self.ch.resources.push(ResourceDef { id, name: format!("Arcanum L{level}"), recharge: "long".into(), max: Some(Max::N(1)), start: None, pips: true, die: None });
        self.ch.init();
        self.say(format!("Mystic Arcanum: {n}."), Tone::Good);
    }

    /// Magical Secrets: a spell or cantrip from any class, up to the highest level the bard can cast.
    pub(super) fn secret_items(&self) -> Vec<&'static lists::Spell> {
        let top = lists::casting_of("Bard").map_or(0, |c| c.top_level(self.level_of("Bard")));
        lists::spells().iter().filter(|s| s.level <= top && !self.ch.spells.contains(&s.name) && !self.ch.cantrips.contains(&s.name)).collect()
    }

    pub(super) fn secret_pick(&mut self, i: usize) {
        let Some(s) = self.secret_items().get(i).copied() else { return };
        self.snap_in_flow();
        if s.level == 0 { self.ch.cantrips.push(s.name.clone()) } else { self.ch.spells.push(s.name.clone()) }
        self.ch.features.push(Feature { name: format!("Magical Secret: {}", s.name), desc: format!("{} spell from the {} list", s.school, s.classes.join("/")), url: s.url.clone() });
        self.say(format!("Magical Secret: {}.", s.name), Tone::Good);
    }

    /// Starting a second class: its multiclass proficiencies as a sheet line, and a skill when it grants one.
    pub(super) fn multiclass_into(&mut self, class: &str) {
        let (profs, skill) = multiclass_profs(class);
        if !profs.is_empty() { self.ch.info.push(format!("Multiclass {class}: {profs}")); }
        if skill { self.queue.push_back(Step::Skill); }
        self.hit_dice_pool(class);
    }

    /// A hit dice pool for a secondary class (the main class keeps "hit_dice").
    fn hit_dice_pool(&mut self, class: &str) {
        let id = format!("hd_{class}");
        if self.ch.def(&id).is_none() {
            self.ch.resources.push(ResourceDef { id, name: format!("Hit Dice ({class})"), recharge: String::new(), max: Some(Max::Kw(format!("class:{class}"))), start: None, pips: false, die: None });
        }
    }

    // --- actions with their own mechanics ----------------------------------------

    pub(super) fn special(&mut self, kind: &str, a: &ActionDef) {
        match kind {
            "smite" => {
                if self.slot_options(1).is_empty() { return self.say("Divine Smite: no slots left!", Tone::Warn); }
                self.open(Step::Smite);
            }
            "recover" => {
                // Arcane Recovery (wizard) or Natural Recovery (Circle of the Land): half the class level in slot levels
                let id = a.uses.clone().unwrap_or_else(|| "arcane_recovery".into());
                if self.have(&id) == 0 { return self.say(format!("{}: none left!", a.name), Tone::Warn); }
                self.snap();
                *self.ch.res.entry(id.clone()).or_insert(1) -= 1;
                self.open(Step::Recover(self.level_of(if id == "natural_recovery" { "Druid" } else { "Wizard" }).div_ceil(2)));
            }
            "font_slot" => self.open(Step::FontSlot),
            "font_points" => self.open(Step::FontPoints),
            "wild_shape" => {
                self.snap();
                if self.ch.active.iter().any(|x| x == "Wild Shape") {
                    self.end_beast();
                    return self.say("Wild Shape: back to normal form.", Tone::Dim);
                }
                if self.have("wild_shape") == 0 { return self.say("Wild Shape: none left!", Tone::Warn); }
                *self.ch.res.entry("wild_shape".into()).or_insert(1) -= 1;
                self.open(Step::BeastPick);
            }
            _ => {}
        }
    }

    /// Highest beast CR for Wild Shape (Circle of the Moon goes higher).
    /// Wild Shape limits: (max CR, no flying, no swimming, elementals allowed).
    fn wild_limits(&self) -> (f64, bool, bool, bool) {
        let l = self.level_of("Druid");
        let moon = self.ch.classes().iter().any(|(p, _)| p == "Druid/Moon");
        let cr = match (moon, l) { (true, l) if l >= 6 => (l / 3) as f64, (true, _) => 1.0, (_, l) if l >= 8 => 1.0, (_, l) if l >= 4 => 0.5, _ => 0.25 };
        (cr, l < 8, l < 4, moon && l >= 10)
    }

    /// SRD forms the druid can take now.
    pub(super) fn beast_options(&self) -> Vec<&'static lists::BeastDef> {
        let (cr, no_fly, no_swim, elem) = self.wild_limits();
        let has = |b: &lists::BeastDef, k: &str| b.speed.get(k).is_some_and(|v| *v > 0);
        lists::beasts().iter().filter(|b| if b.elemental { elem } else { b.cr <= cr && !(no_fly && has(b, "fly")) && !(no_swim && has(b, "swim")) }).collect()
    }

    pub(super) fn beast_pick(&mut self, i: usize) {
        let opts = self.beast_options();
        let Some(b) = opts.get(i).copied() else { return self.open(Step::BeastName) };
        if b.elemental {
            // an elemental costs two uses: one was spent opening the menu
            if self.have("wild_shape") == 0 {
                *self.ch.res.entry("wild_shape".into()).or_insert(0) += 1;
                return self.say("Elemental form needs two Wild Shape uses.", Tone::Warn);
            }
            *self.ch.res.entry("wild_shape".into()).or_insert(1) -= 1;
        }
        let info = format!("{}{}", speed_text(b), b.multiattack.as_ref().map_or(String::new(), |m| format!(". {m}")));
        self.beast_form(b.name.clone(), b.hp, b.ac, info);
        for (k, a) in b.attacks.iter().enumerate().rev() {
            self.ch.actions.insert(0, ActionDef {
                name: format!("Beast: {}", a.name), hit: Some(a.hit), dice: Some(a.dice.unwrap_or((0, 1))), bonus: a.bonus, dmg_type: a.kind.clone(),
                desc: if k == 0 { b.multiattack.clone().unwrap_or_default() } else { String::new() },
                ..Default::default()
            });
        }
    }

    pub(super) fn wild_cr(&self) -> String {
        let l = self.level_of("Druid");
        let moon = self.ch.classes().iter().any(|(p, _)| p == "Druid/Moon");
        match (moon, l) {
            (true, l) if l >= 6 => (l / 3).to_string(),
            (true, _) => "1".into(),
            (_, l) if l >= 8 => "1".into(),
            (_, l) if l >= 4 => "1/2 (no flying)".into(),
            _ => "1/4 (no flying or swimming)".into(),
        }
    }

    pub(super) fn smite_items(&self) -> Vec<String> {
        self.slot_options(1).iter().flat_map(|(_, l, left)| {
            let n = (1 + l).min(5);
            [format!("Level {l} slot ({left} left): {n}d8"), format!("Level {l} slot vs undead/fiend: {}d8", n + 1)]
        }).collect()
    }

    pub(super) fn smite_pick(&mut self, i: usize) {
        let Some((id, l, _)) = self.slot_options(1).get(i / 2).cloned() else { return };
        self.snap();
        if let Some(v) = self.ch.res.get_mut(&id) { *v -= 1; }
        let n = (1 + l).min(5) + (i % 2) as u32;
        let total: u32 = (0..n).map(|_| d(8)).sum();
        self.say(format!("Divine Smite (level {l}): {total} radiant ({n}d8, double the dice on a crit)"), Tone::Good);
    }

    /// Arcane or Natural Recovery: slot levels that still fit the budget and are not full.
    pub(super) fn recover_items(&self, budget: u32) -> Vec<u32> {
        (1..=budget.min(5)).filter(|l| {
            let id = format!("slot{l}");
            self.have(&id) < self.ch.res_max(&id).unwrap_or(0)
        }).collect()
    }

    pub(super) fn recover_pick(&mut self, budget: u32, i: usize) {
        let Some(&l) = self.recover_items(budget).get(i) else { return };
        *self.ch.res.entry(format!("slot{l}")).or_insert(0) += 1;
        self.say(format!("Level {l} slot recovered."), Tone::Good);
        if !self.recover_items(budget - l).is_empty() { self.open(Step::Recover(budget - l)); }
    }

    pub(super) fn font_slot_items(&self) -> Vec<u32> {
        (1..=5).filter(|&l| FONT_COST[l as usize - 1] <= self.have("sorcery_points") && self.ch.res_max(&format!("slot{l}")).unwrap_or(0) > 0).collect()
    }

    pub(super) fn font_slot_pick(&mut self, i: usize) {
        let Some(&l) = self.font_slot_items().get(i) else { return };
        self.snap();
        let cost = FONT_COST[l as usize - 1];
        *self.ch.res.entry("sorcery_points".into()).or_insert(0) -= cost;
        *self.ch.res.entry(format!("slot{l}")).or_insert(0) += 1;
        self.say(format!("Created a level {l} slot for {cost} sorcery points."), Tone::Good);
    }

    pub(super) fn font_points_items(&self) -> Vec<u32> { (1..=9).filter(|l| self.have(&format!("slot{l}")) > 0).collect() }

    pub(super) fn font_points_pick(&mut self, i: usize) {
        let Some(&l) = self.font_points_items().get(i) else { return };
        self.snap();
        let max = self.ch.res_max("sorcery_points").unwrap_or(0);
        *self.ch.res.entry(format!("slot{l}")).or_insert(1) -= 1;
        let p = self.ch.res.entry("sorcery_points".into()).or_insert(0);
        *p = (*p + l).min(max);
        self.say(format!("Level {l} slot into sorcery points ({} now).", self.have("sorcery_points")), Tone::Good);
    }

    /// Temporary HP go first; returns the damage left for real HP.
    pub(super) fn temp_soak(&mut self, n: i32) -> i32 {
        let t = self.ch.temp_hp.min(n);
        if t == 0 { return n; }
        self.ch.temp_hp -= t;
        self.say(format!("Temporary HP absorb {t} ({} left).", self.ch.temp_hp), Tone::Warn);
        n - t
    }

    /// Damage while concentrating: CON save, DC 10 or half the damage; dropping to 0 HP ends it outright.
    pub(super) fn concentration_check(&mut self, damage: i32) {
        let Some(sp) = self.ch.concentration.clone() else { return };
        if damage <= 0 { return; }
        if self.ch.hp == Some(0) {
            self.ch.concentration = None;
            return self.say(format!("Concentration on {sp} ends (down)."), Tone::Warn);
        }
        let dc = (damage / 2).max(10);
        let m = self.ch.check_mod("CON save");
        let (r, note) = self.d20(Roll::Save);
        let ok = r as i32 + m >= dc;
        self.say(format!("Concentration ({sp}): CON save {r}{m:+}={}{note} vs DC {dc}: {}", r as i32 + m, if ok { "held" } else { "LOST" }), if ok { Tone::Good } else { Tone::Warn });
        if !ok { self.ch.concentration = None; }
    }

    pub(super) fn beast_form(&mut self, name: String, hp: i32, ac: i32, info: String) {
        self.ch.res.insert("beast_hp".into(), hp.max(1) as u32);
        self.ch.active.push("Wild Shape".into());
        self.say(format!("Wild Shape: {name}, {} HP, AC {ac}.", hp.max(1)), Tone::Good);
        if !info.is_empty() { self.say(info.clone(), Tone::Dim); }
        self.ch.beast = Some(Beast { name, ac, info });
    }

    fn end_beast(&mut self) {
        self.ch.active.retain(|x| x != "Wild Shape");
        self.ch.res.remove("beast_hp");
        self.ch.beast = None;
        self.ch.actions.retain(|a| !a.name.starts_with("Beast: "));
    }

    /// Damage hits the beast form first; what is left over carries to the druid and ends the form.
    pub(super) fn beast_soak(&mut self, n: i32) -> i32 {
        if !self.ch.active.iter().any(|x| x == "Wild Shape") { return n; }
        let b = self.have("beast_hp") as i32;
        if n < b {
            self.ch.res.insert("beast_hp".into(), (b - n) as u32);
            self.say(format!("Beast form takes {n}, {} HP left.", b - n), Tone::Warn);
            return 0;
        }
        self.end_beast();
        self.say(format!("Beast form drops! {} damage carries over.", n - b), Tone::Warn);
        n - b
    }
}

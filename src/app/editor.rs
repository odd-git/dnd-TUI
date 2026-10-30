//! Free sheet editor: each menu entry of "Edit sheet" is a short chain of modals.

use super::*;
use crate::{builder, lists};

fn parse_dice(t: &str) -> Option<(u32, u32)> {
    let (n, s) = t.to_lowercase().split_once('d').map(|(a, b)| (a.trim().to_string(), b.trim().to_string()))?;
    let n = if n.is_empty() { 1 } else { n.parse().ok()? };
    let s: u32 = s.parse().ok()?;
    (n > 0 && s > 1).then_some((n, s))
}

/// "Rope (hempen) x2 10 lb": a name, then optionally a count and a weight per item.
fn parse_gear(t: &str) -> Option<Gear> {
    let mut words: Vec<&str> = t.split_whitespace().collect();
    let (mut qty, mut weight) = (1, 0.0);
    while let Some(w) = words.last().copied() {
        let lw = w.to_lowercase();
        if let Some(n) = lw.strip_prefix('x').and_then(|n| n.parse().ok()) { qty = n; }
        else if let Some(n) = lw.strip_suffix("lb").and_then(|n| n.parse().ok()) { weight = n; }
        else if lw == "lb" && words.len() > 1 && words[words.len() - 2].parse::<f32>().is_ok() { words.pop(); weight = words.last()?.parse().ok()?; }
        else { break; }
        words.pop();
    }
    (!words.is_empty() && qty > 0).then(|| Gear { name: words.join(" "), qty, weight, attuned: false })
}

/// "+15 gp", "-3 sp", "20gp": sign, denomination index into COINS, count.
fn parse_coins(t: &str) -> Option<(bool, usize, u32)> {
    let t = t.replace(' ', "").to_lowercase();
    let (spend, t) = match t.strip_prefix('-') { Some(r) => (true, r.to_string()), None => (false, t.trim_start_matches('+').to_string()) };
    let d = COINS.iter().position(|(c, _)| t.ends_with(c))?;
    Some((spend, d, t[..t.len() - 2].parse().ok()?))
}

/// A feat is offered when its ability minimums hold and, for racial feats, the race matches.
fn feat_ok(c: &Character, f: &lists::FeatDef) -> bool {
    if c.features.iter().any(|x| x.name == format!("Feat: {}", f.name)) { return false; }
    if f.min_ability.iter().any(|(a, m)| c.abilities.get(a).copied().unwrap_or(0) < *m) { return false; }
    match &f.prereq {
        Some(p) => {
            let race = p.split(" (").next().unwrap_or(p).to_lowercase();
            let is_race = lists::lineages().iter().any(|l| l.name.to_lowercase() == race);
            !is_race || c.race.to_lowercase().contains(&race)
        }
        None => true,
    }
}

impl App {
    fn feat_choices(&self) -> Vec<&'static lists::FeatDef> { lists::feats().iter().filter(|f| feat_ok(&self.ch, f)).collect() }

    pub(super) fn edit_open(&mut self, step: Step) {
        let c = &self.ch;
        match step.clone() {
            Step::Ident => {
                let items = vec![format!("Name: {}", c.name), format!("Race: {}", c.race), format!("Background: {}", c.background)];
                self.list("Identity", items, step, true);
            }
            Step::IdentSet(k) => self.input(["Name", "Race", "Background"][k], step, false),
            Step::Scores => {
                let items = c.ability_names().iter().map(|a| format!("{a} {}", c.abilities[*a])).collect();
                self.list("Ability scores", items, step, true);
            }
            Step::ScoreSet(a) => self.input(format!("{a} score"), step, true),
            Step::Unskill => { let items = c.skills.clone(); self.list("Remove which skill?", items, step, true) }
            Step::ArmorPick => {
                let mut items = vec!["No armor".to_string()];
                items.extend(lists::armors().iter().filter(|a| a.category != "Shield").map(|a| format!("{} (AC {})", a.name, a.ac)));
                self.list("Armor", items, step, true);
            }
            Step::ShieldPick(_) => self.list("Shield?", vec!["No".into(), "Yes".into()], step, true),
            Step::WeaponPick => {
                let items = lists::weapons().iter().map(|w| format!("{} ({}d{} {})", w.name, w.dice.0, w.dice.1, w.dmg_type)).collect();
                self.list("Add a weapon attack", items, step, true);
            }
            Step::CustomName => self.input("Attack name", step, false),
            Step::CustomAb(_) => {
                let mut items: Vec<String> = c.ability_names().iter().map(|a| format!("{a} (to hit and damage)")).collect();
                items.push("None (automatic damage)".into());
                self.list("Attack ability", items, step, true);
            }
            Step::CustomDice(..) => self.input("Damage dice, e.g. 1d8 (empty: no damage)", step, false),
            Step::CustomType(..) => self.input("Damage type", step, false),
            Step::RmAction => { let items = c.actions.iter().map(|a| a.name.clone()).collect(); self.list("Remove which action?", items, step, true) }
            Step::SpellAdd => {
                let items = self.spell_add_items().iter().map(|s| format!("{} {}{}", if s.level == 0 { "C".to_string() } else { s.level.to_string() }, s.name,
                    if self.ch.on_list(&s.name) { String::new() } else { format!("  [{}]", s.classes.join("/")) })).collect();
                self.list("Add spell", items, step, true);
            }
            Step::SpellDel => { let items = self.spell_known_names(); self.list("Remove spell", items, step, true) }
            Step::FeatList => {
                let items = self.feat_choices().iter().map(|f| match &f.prereq {
                    Some(p) => format!("{} ({})  [{p}]", f.name, lists::abbr(&f.source)),
                    None => format!("{} ({})", f.name, lists::abbr(&f.source)),
                }).collect();
                self.list("Feat", items, step, true);
            }
            Step::FeatAsi(name) => {
                let opts = Self::feat_asi(&name).map(|a| a.options.clone()).unwrap_or_default();
                self.list(&format!("{name}: +1 to"), opts, step, false);
            }
            Step::ResName => self.input("Resource name (e.g. Rage)", step, false),
            Step::ResMax(_) => self.input("Uses", step, true),
            Step::ResRecharge(..) => self.list("Recharges on", vec!["Short rest".into(), "Long rest".into(), "Never".into()], step, true),
            Step::RmRes => { let items = c.resources.iter().map(|r| r.name.clone()).collect(); self.list("Remove which resource?", items, step, true) }
            Step::RmFeat => { let items = c.features.iter().map(|f| f.name.clone()).collect(); self.list("Remove which feature?", items, step, true) }
            Step::ItemAdd => self.input("Item, e.g. Rope (hempen) x1 10 lb", step, false),
            Step::ItemDel => { let items = c.inventory.iter().map(|g| format!("{} x{}", g.name, g.qty)).collect(); self.list("Remove which item?", items, step, true) }
            Step::Attune => {
                let n = c.inventory.iter().filter(|g| g.attuned).count();
                let items = c.inventory.iter().map(|g| format!("[{}] {}", if g.attuned { "x" } else { " " }, g.name)).collect();
                self.list(&format!("Attunement ({n}/3)"), items, step, true);
            }
            Step::Coins => self.input(format!("Coins: {}. Add or spend, e.g. +15 gp, -3 sp", c.coins_text()), step, false),
            _ => {}
        }
    }

    fn feat_asi(name: &str) -> Option<&'static lists::FeatAsi> {
        lists::feats().iter().find(|f| f.name == name)?.asi.as_ref()
    }

    pub(super) fn edit_pick(&mut self, step: Step, i: usize) {
        match step {
            Step::Ident => self.open(Step::IdentSet(i)),
            Step::Scores => { if let Some(a) = self.ch.ability_names().get(i) { self.open(Step::ScoreSet(a.to_string())); } }
            Step::Unskill => { if i < self.ch.skills.len() { self.ch.skills.remove(i); } }
            Step::ArmorPick => {
                let armor = (i > 0).then(|| lists::armors().iter().filter(|a| a.category != "Shield").nth(i - 1).map(|a| a.name.clone())).flatten();
                self.open(Step::ShieldPick(armor));
            }
            Step::ShieldPick(armor) => {
                let defense = builder::style_effect(&self.ch) == Some("defense") && armor.is_some();
                (self.ch.ac_base, self.ch.ac_dex_max) = builder::armor_class(armor.as_deref(), i == 1, defense);
                self.ch.ac_armored = armor.is_some();
                self.ch.ac_shield = i == 1;
                let ac = self.ch.ac().unwrap_or(0);
                self.say(format!("AC is now {ac}."), Tone::Good);
            }
            Step::WeaponPick => {
                if let Some(w) = lists::weapons().get(i) {
                    let style = builder::style_effect(&self.ch);
                    for a in builder::weapon_actions(w, style) {
                        self.ch.actions.retain(|x| x.name != a.name);
                        self.say(format!("Added attack: {}.", a.name), Tone::Good);
                        self.ch.actions.push(a);
                    }
                }
            }
            Step::CustomAb(name) => {
                let ab = self.ch.ability_names().get(i).map(|a| a.to_string());
                self.open(Step::CustomDice(name, ab));
            }
            Step::RmAction => { if i < self.ch.actions.len() { let a = self.ch.actions.remove(i); self.say(format!("Removed {}.", a.name), Tone::Dim); } }
            Step::SpellAdd => {
                let Some(s) = self.spell_add_items().get(i).copied() else { return };
                if s.level == 0 { self.ch.cantrips.push(s.name.clone()) } else { self.ch.spells.push(s.name.clone()) }
                self.say(format!("Added {}.", s.name), Tone::Good);
            }
            Step::SpellDel => {
                let n = self.spell_known_names().get(i).cloned();
                if let Some(n) = n {
                    self.ch.cantrips.retain(|x| *x != n);
                    self.ch.spells.retain(|x| *x != n);
                    self.ch.prepared.retain(|x| *x != n);
                    self.say(format!("Removed {n}."), Tone::Dim);
                }
            }
            Step::FeatList => {
                let Some(f) = self.feat_choices().get(i).copied() else { return };
                self.ch.features.push(Feature { name: format!("Feat: {}", f.name), desc: f.desc.clone(), url: f.url.clone() });
                self.say(format!("Feat: {}.", f.name), Tone::Good);
                match f.asi.as_ref().map(|a| a.options.len()) {
                    Some(1) => self.feat_bump(&f.name, 0),
                    Some(n) if n > 1 => self.open(Step::FeatAsi(f.name.clone())),
                    _ => {}
                }
            }
            Step::FeatAsi(name) => self.feat_bump(&name, i),
            Step::ResRecharge(name, max) => {
                let id = name.to_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect::<String>();
                let recharge = ["short", "long", ""][i.min(2)].to_string();
                self.ch.resources.retain(|r| r.id != id);
                self.ch.resources.push(ResourceDef { id, name, recharge, max: Some(Max::N(max)), start: None, pips: max <= 6, die: None });
                self.ch.init();
            }
            Step::RmRes => {
                if i < self.ch.resources.len() { let r = self.ch.resources.remove(i); self.ch.res.remove(&r.id); }
            }
            Step::RmFeat if i < self.ch.features.len() => { self.ch.features.remove(i); }
            Step::ItemDel if i < self.ch.inventory.len() => { let g = self.ch.inventory.remove(i); self.say(format!("Removed {}.", g.name), Tone::Dim); }
            Step::Attune if i < self.ch.inventory.len() => {
                let full = self.ch.inventory.iter().filter(|g| g.attuned).count() >= 3;
                let g = &mut self.ch.inventory[i];
                if !g.attuned && full { return self.say("Already attuned to 3 items.", Tone::Warn); }
                g.attuned = !g.attuned;
                let msg = format!("{} {}.", if g.attuned { "Attuned to" } else { "Attunement ended:" }, g.name);
                self.say(msg, Tone::Good);
            }
            _ => {}
        }
    }

    fn feat_bump(&mut self, feat: &str, i: usize) {
        let Some(a) = Self::feat_asi(feat) else { return };
        let (ab, amt) = (a.options[i].clone(), a.amount);
        if let Some(v) = self.ch.abilities.get_mut(&ab) {
            *v = (*v + amt).min(20);
            let msg = format!("{ab} is now {v}");
            self.say(msg, Tone::Good);
        }
    }

    pub(super) fn edit_input(&mut self, step: Step, text: String) {
        match step {
            Step::IdentSet(k) if !text.is_empty() => match k {
                0 => self.ch.name = text,
                1 => self.ch.race = text,
                _ => self.ch.background = text,
            },
            Step::ScoreSet(a) => {
                if let Ok(n) = text.parse::<i32>() { self.ch.abilities.insert(a, n.clamp(1, 30)); }
            }
            Step::CustomName => { if !text.is_empty() { self.open(Step::CustomAb(text)); } }
            Step::CustomDice(name, ab) => {
                if text.is_empty() {
                    self.ch.actions.push(ActionDef { name, ability: ab, ..Default::default() });
                } else if let Some(dice) = parse_dice(&text) {
                    self.open(Step::CustomType(name, ab, dice));
                } else {
                    self.open(Step::CustomDice(name, ab));
                }
            }
            Step::CustomType(name, ab, dice) => {
                self.ch.actions.push(ActionDef { name, dice: Some(dice), dmg_ability: ab.is_some(), ability: ab, dmg_type: text, ..Default::default() });
            }
            Step::ResName => { if !text.is_empty() { self.open(Step::ResMax(text)); } }
            Step::ItemAdd => match parse_gear(&text) {
                Some(g) => {
                    self.say(format!("Added {} x{}.", g.name, g.qty), Tone::Good);
                    match self.ch.inventory.iter_mut().find(|x| x.name.eq_ignore_ascii_case(&g.name)) {
                        Some(x) => x.qty += g.qty,
                        None => self.ch.inventory.push(g),
                    }
                }
                None if text.is_empty() => {}
                None => self.say("Write the item as: Rope (hempen) x1 10 lb", Tone::Warn),
            },
            Step::Coins => match parse_coins(&text) {
                Some((true, d, n)) if !self.ch.pay(d, n) => self.say(format!("Not enough coins for {n} {}.", COINS[d].0), Tone::Warn),
                Some((spend, d, n)) => {
                    if !spend { self.ch.coins[d] += n; }
                    self.say(format!("Coins: {}.", self.ch.coins_text()), Tone::Good);
                }
                None if text.is_empty() => {}
                None => self.say("Write coins as +15 gp or -3 sp.", Tone::Warn),
            },
            Step::ResMax(name) => {
                if let Ok(n) = text.parse::<u32>() { if n > 0 { self.open(Step::ResRecharge(name, n)); } }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gear_and_coin_input() {
        let g = parse_gear("Rope (hempen) x2 10 lb").unwrap();
        assert_eq!((g.name.as_str(), g.qty, g.weight), ("Rope (hempen)", 2, 10.0));
        let g = parse_gear("Potion of healing 0.5lb").unwrap();
        assert_eq!((g.name.as_str(), g.qty, g.weight), ("Potion of healing", 1, 0.5));
        assert!(parse_gear("x3").is_none());
        assert_eq!(parse_coins("+15 gp"), Some((false, 1, 15)));
        assert_eq!(parse_coins("-3sp"), Some((true, 3, 3)));
        assert_eq!(parse_coins("20 GP"), Some((false, 1, 20)));
        assert_eq!(parse_coins("20"), None);
    }
}

//! Spells: casting, learning, preparing.

use super::*;
use crate::lists::{self, Spell};

impl App {
    /// Tabs on offer: Spells only once there is something to cast.
    pub fn tabs(&self) -> Vec<(Tab, &'static str)> {
        TABS.iter().copied().filter(|(t, _)| *t != Tab::Spells || self.ch.has_spells()).collect()
    }

    /// Slots a spell of `level` can be cast with: (resource id, slot level, left).
    pub(super) fn slot_options(&self, level: u32) -> Vec<(String, u32, u32)> {
        let mut v = vec![];
        if let Some((c, cl, _)) = self.ch.casters().into_iter().find(|(c, ..)| c.kind == "pact") {
            let pl = at(&c.pact_level, cl);
            if pl >= level && self.have("pact") > 0 { v.push(("pact".to_string(), pl, self.have("pact"))); }
        }
        for l in level.max(1)..=9 {
            let id = format!("slot{l}");
            let left = self.have(&id);
            if left > 0 { v.push((id, l, left)); }
        }
        // Mystic Arcanum: once per long rest, at the spell's own level
        let id = format!("arcanum{level}");
        if level >= 6 && self.have(&id) > 0 { v.push((id, level, 1)); }
        v
    }

    /// Slots this spell may use: none for a spellbook ritual the wizard has not prepared.
    /// Spell Mastery and Signature Spells come first: they cost no slot.
    fn cast_slots(&self, sp: &Spell) -> Vec<(String, u32, u32)> {
        if !self.ch.castable().contains(&sp.name) { return vec![] }
        let mut v = vec![];
        if self.ch.features.iter().any(|f| f.name == format!("Spell Mastery: {}", sp.name)) { v.push(("mastery".to_string(), sp.level, 0)); }
        let sig = format!("sig:{}", sp.name);
        if self.have(&sig) > 0 { v.push((sig.clone(), 3, self.have(&sig))); }
        for (_, l, id) in self.ch.innate().into_iter().filter(|x| x.0 == sp.name) { if self.have(&id) > 0 { v.push((id.clone(), l, 1)); } }
        v.extend(self.slot_options(sp.level));
        v
    }

    pub(super) fn cast(&mut self, name: &str) {
        let Some(sp) = lists::spell(name) else { return };
        if sp.level == 0 { return self.cast_effect(sp, 0); }
        let opts = self.cast_slots(sp);
        match (opts.len(), self.ch.ritual_ok(name)) {
            (0, false) => self.say(format!("{}: no slots left!", sp.name), Tone::Warn),
            (1, false) => self.cast_with(sp, &opts[0].0.clone(), opts[0].1),
            _ => self.open(Step::CastSlot(sp.name.clone())),
        }
    }

    pub(super) fn cast_slot_items(&self, name: &str) -> Vec<String> {
        let Some(sp) = lists::spell(name) else { return vec![] };
        let mut v: Vec<String> = self.cast_slots(sp).iter().map(|(id, l, left)| {
            match id.as_str() {
                "pact" => format!("Pact slot, level {l} ({left} left)"),
                "mastery" => format!("Spell Mastery, level {l} (no slot)"),
                s if s.starts_with("sig:") => format!("Signature Spell, level {l} ({left} left)"),
                s if s.starts_with("innate") => format!("Racial, level {l} (once per long rest)"),
                _ => format!("Level {l} slot ({left} left)"),
            }
        }).collect();
        if self.ch.ritual_ok(name) { v.push("As a ritual (+10 min, no slot)".into()); }
        v
    }

    pub(super) fn cast_slot_pick(&mut self, name: &str, i: usize) {
        let Some(sp) = lists::spell(name) else { return };
        match self.cast_slots(sp).get(i).cloned() {
            Some((id, l, _)) => self.cast_with(sp, &id, l),
            None if self.ch.ritual_ok(name) => { self.snap(); self.say(format!("{} as a ritual: 10 minutes longer.", sp.name), Tone::Dim); self.cast_effect(sp, sp.level); }
            None => {}
        }
    }

    fn cast_with(&mut self, sp: &'static Spell, id: &str, slot_level: u32) {
        self.snap();
        if let Some(v) = self.ch.res.get_mut(id) { *v -= 1; }
        self.cast_effect(sp, slot_level);
    }

    fn cast_effect(&mut self, sp: &'static Spell, slot_level: u32) {
        let ab = self.ch.ability_for(&sp.name);
        let tag = if slot_level > sp.level { format!(" at level {slot_level}") } else { String::new() };
        let conc = if sp.conc { " [concentration]" } else { "" };
        if sp.conc {
            if let Some(old) = self.ch.concentration.replace(sp.name.clone()).filter(|o| *o != sp.name) { self.say(format!("Concentration on {old} ends."), Tone::Dim); }
        }
        self.say(format!("Cast {}{tag}{conc}", sp.name), Tone::Good);
        if sp.level > 0 {
            self.last_cast = Some((slot_level.max(sp.level), sp.school == "Necromancy"));
            self.school_cast(&sp.school, slot_level.max(sp.level));
        }
        let mut crit = false;
        if sp.attack {
            let hit = self.ch.md(&ab) + self.ch.prof();
            let (r, note) = self.d20(Roll::Attack);
            crit = r == 20;
            self.say(format!("Spell attack: {r}{hit:+}={}{note}{}", r as i32 + hit, if crit { " CRIT!" } else { "" }), Tone::Norm);
        }
        if sp.save.is_some() {
            let dc = self.ch.dc(&ab);
            self.say(format!("Save: {} DC {dc}", sp.save.as_deref().unwrap_or("")), Tone::Warn);
        }
        let Some((n, sides, kind)) = &sp.dmg else {
            if !sp.attack && sp.save.is_none() { self.say(sp.desc.clone(), Tone::Dim); }
            return;
        };
        let above = slot_level.saturating_sub(sp.level);
        let tier = 1 + u32::from(self.ch.level >= 5) + u32::from(self.ch.level >= 11) + u32::from(self.ch.level >= 17);
        let mut dice = *n;
        dice += sp.upcast_dice(slot_level);
        if sp.scales && !sp.beams { dice = n * tier; }
        if sp.healing {
            let has = |n: &str| self.ch.features.iter().any(|f| f.name == n);
            let mut m = if sp.heal_mod { self.ch.md(&ab) } else { 0 };
            if has("Disciple of Life") && sp.level > 0 { m += 2 + slot_level as i32; }
            let supreme = has("Supreme Healing"); // healing dice count as their maximum
            let total = (0..dice).map(|_| if supreme { *sides } else { d(*sides) } as i32).sum::<i32>() + m;
            return self.say(format!(" >> Heals {total} HP ({dice}d{sides}{m:+}{})", if supreme { ", maximum" } else { "" }), Tone::Good);
        }
        let hits = sp.count + if sp.count_up { above } else { 0 } + if sp.beams { tier - 1 } else { 0 };
        let rolls = if crit { 2 * dice } else { dice };
        // Potent Spellcasting (Knowledge, Light): WIS to the damage of cleric cantrips
        let potent = sp.level == 0 && sp.classes.iter().any(|c| c == "Cleric") && self.ch.features.iter().any(|f| f.name == "Potent Spellcasting");
        let add = sp.flat as i32 + if potent { self.ch.md("WIS") } else { 0 } + if sp.dmg_mod { self.ch.md(&ab) } else { 0 };
        // Overchannel (Evocation, 14th level): a 1st-5th level spell deals its maximum damage
        let overchannel = (1..=5).contains(&sp.level) && sp.classes.iter().any(|c| c == "Wizard") && self.ch.active.iter().any(|t| t == "Overchannel");
        let mut per: Vec<u32> = (0..hits).map(|_| ((0..rolls).map(|_| if overchannel { *sides } else { d(*sides) }).sum::<u32>() as i32 + add).max(0) as u32).collect();
        let mut flat = if add != 0 { format!("{add:+}") } else { String::new() };
        // bonuses to one damage roll of the spell
        let has = |n: &str| self.ch.features.iter().any(|f| f.name == n);
        let element = self.ch.features.iter().find(|f| f.name.starts_with("Dragon Ancestor: ")).map(|f| f.desc.to_lowercase());
        let mut first = vec![];
        if has("Elemental Affinity") && element.as_deref() == Some(kind.to_lowercase().as_str()) { first.push((self.ch.md("CHA"), "affinity")); }
        if has("Empowered Evocation") && sp.school == "Evocation" && sp.classes.iter().any(|c| c == "Wizard") { first.push((self.ch.md("INT").max(1), "empowered")); }
        for (b, label) in first {
            per[0] = (per[0] as i32 + b).max(0) as u32;
            flat += &format!(" {b:+} {label}{}", if hits > 1 { " on the first" } else { "" });
        }
        if overchannel { flat += ", maximum"; }
        if hits == 1 {
            self.say(format!(" >> Damage: {} {kind} ({rolls}d{sides}{flat})", per[0]), Tone::Good);
        } else {
            let each = per.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", ");
            let roll_each = if sp.attack { ", attack roll for each" } else { "" };
            self.say(format!(" >> {hits} x {rolls}d{sides}{flat} {kind}: {each} (total {}{roll_each})", per.iter().sum::<u32>()), Tone::Good);
        }
        if overchannel { self.overchannel_cost(sp.level); }
    }

    /// Overchannel switches off; after the first use since a long rest it costs 2d12 necrotic per spell level, one d12 more each time.
    fn overchannel_cost(&mut self, level: u32) {
        self.ch.active.retain(|t| t != "Overchannel");
        let used = self.have("overchannel_used");
        self.ch.res.insert("overchannel_used".into(), used + 1);
        if used == 0 { return; }
        let n = (used + 1) * level;
        let total: u32 = (0..n).map(|_| d(12)).sum();
        self.say(format!("Overchannel: you take {total} necrotic ({n}d12, ignores resistance and immunity)"), Tone::Warn);
    }

    // --- learning and preparing ---------------------------------------------------

    /// Spells of the class list still to learn, how many to pick.
    pub(super) fn learn_options(&self, cantrips: bool) -> (usize, Vec<String>) {
        let Some((c, cl, prog)) = self.ch.cast_src() else { return (0, vec![]) };
        let have = if cantrips { &self.ch.cantrips } else { &self.ch.spells };
        let want = if cantrips { self.ch.cantrips_max() } else if c.mode == "prepared" { 0 } else { self.ch.known_max() };
        let names: Vec<String> = lists::spell_choices(c, &prog, cl, cantrips).iter()
            .map(|s| s.name.clone()).filter(|n| !have.contains(n)).collect();
        // Mystic Arcanum spells (above the slot levels) do not count against spells known
        let top = c.top_level(cl);
        let counted = have.iter().filter(|n| self.ch.on_list(n) && !self.ch.bonus_spell(n) && lists::spell(n).is_none_or(|s| s.level <= top)).count();
        (want.saturating_sub(counted).min(names.len()), names)
    }

    pub(super) fn prepare_items(&self) -> (usize, Vec<String>) {
        let names = prepare_options(&self.ch);
        (self.ch.prepared_max().min(names.len()), names)
    }

    pub(super) fn open_learn(&mut self, cantrips: bool, step: Step) {
        let (n, names) = self.learn_options(cantrips);
        if n == 0 { return self.advance(); }
        let items = names.iter().map(|n| self.spell_line(n)).collect();
        let hint = self.ch.cast_src().filter(|_| !cantrips).map_or(String::new(), |(c, cl, _)| c.schools_hint(cl));
        self.multi(&format!("Learn {n} {}{hint}", if cantrips { "cantrips" } else { "spells" }), items, n, step);
    }

    pub(super) fn open_prepare(&mut self, step: Step) {
        let (n, names) = self.prepare_items();
        if n == 0 || self.ch.casting().is_some_and(|c| c.mode == "known") { return self.advance(); }
        let items = names.iter().map(|n| self.spell_line(n)).collect();
        self.multi(&format!("Prepare {n} spells"), items, n, step);
    }

    fn spell_line(&self, name: &str) -> String {
        match lists::spell(name) {
            Some(s) => format!("{name} (L{}, {}{})", s.level, s.school, if s.conc { ", conc." } else { "" }),
            None => name.to_string(),
        }
    }

    pub(super) fn multi(&mut self, title: &str, items: Vec<String>, limit: usize, step: Step) {
        self.query.clear();
        self.modal = Some(Modal::Multi { title: title.into(), on: vec![false; items.len()], items, sel: 0, limit, step });
    }

    pub(super) fn spells_done(&mut self, step: Step, picked: Vec<usize>) {
        match step {
            Step::Learn(cantrips) => {
                let (_, names) = self.learn_options(cantrips);
                let mut all = self.ch.spells.clone();
                all.extend(picked.iter().map(|&i| names[i].clone()));
                if let Some((c, cl, _)) = self.ch.cast_src().filter(|(c, cl, _)| !cantrips && !c.schools_ok(*cl, &all)) {
                    self.say(format!("Too many spells outside the schools{}", c.schools_hint(cl)), Tone::Warn);
                    return self.open(step);
                }
                self.snap_in_flow();
                for i in picked {
                    let n = names[i].clone();
                    self.say(format!("Learned {n}."), Tone::Good);
                    if cantrips { self.ch.cantrips.push(n) } else { self.ch.spells.push(n) }
                }
            }
            Step::Prepare => {
                let (_, names) = self.prepare_items();
                self.snap_in_flow();
                self.ch.prepared = picked.iter().map(|&i| names[i].clone()).collect();
                self.say(format!("Prepared {} spells.", self.ch.prepared.len()), Tone::Good);
            }
            _ => {}
        }
    }

    /// Editor: any spell of the class list (all levels, cantrips included) not yet known.
    pub(super) fn spell_add_items(&self) -> Vec<&'static Spell> {
        // the class lists first, then every other official spell (feats, races, pacts, DM gifts)
        let mut v: Vec<&'static Spell> = lists::spells().iter()
            .filter(|s| !self.ch.cantrips.contains(&s.name) && !self.ch.spells.contains(&s.name)).collect();
        v.sort_by_key(|s| (!self.ch.on_list(&s.name), s.level, s.name.clone()));
        v
    }

    pub(super) fn spell_known_names(&self) -> Vec<String> {
        self.ch.cantrips.iter().chain(&self.ch.spells).cloned().collect()
    }
}

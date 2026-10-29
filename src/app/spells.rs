//! Spells: casting, learning, preparing.

use super::*;
use crate::lists::{self, Spell};

impl App {
    /// Tabs on offer: Spells only once there is something to cast.
    pub fn tabs(&self) -> Vec<(Tab, &'static str)> {
        TABS.iter().copied().filter(|(t, _)| *t != Tab::Spells || self.ch.casts()).collect()
    }

    /// Rows of the Spells tab: cantrips, then castable spells by level.
    pub fn spell_rows(&self) -> Vec<&'static Spell> {
        let by = |names: &[String]| {
            let mut v: Vec<&'static Spell> = names.iter().filter_map(|n| lists::spell(n)).collect();
            v.sort_by(|a, b| (a.level, &a.name).cmp(&(b.level, &b.name)));
            v
        };
        let all: Vec<String> = self.ch.cantrips.iter().cloned().chain(self.ch.castable()).collect();
        let mut rows = by(&all);
        rows.dedup_by_key(|s| &s.name);
        rows
    }

    /// Slots a spell of `level` can be cast with: (resource id, slot level, left).
    fn slot_options(&self, level: u32) -> Vec<(String, u32, u32)> {
        let mut v = vec![];
        if let Some((c, cl, _)) = self.ch.casters().into_iter().find(|(c, ..)| c.kind == "pact") {
            let pl = at(&c.pact_level, cl);
            if pl >= level && self.have("pact") > 0 { v.push(("pact".to_string(), pl, self.have("pact"))); }
        } else {
            for l in level.max(1)..=9 {
                let id = format!("slot{l}");
                let left = self.have(&id);
                if left > 0 { v.push((id, l, left)); }
            }
        }
        v
    }

    pub(super) fn cast(&mut self, name: &str) {
        let Some(sp) = lists::spell(name) else { return };
        if sp.level == 0 { return self.cast_effect(sp, 0); }
        let opts = self.slot_options(sp.level);
        match opts.len() {
            0 => self.say(format!("{}: no slots left!", sp.name), Tone::Warn),
            1 => self.cast_with(sp, &opts[0].0.clone(), opts[0].1),
            _ => self.open(Step::CastSlot(sp.name.clone())),
        }
    }

    pub(super) fn cast_slot_items(&self, name: &str) -> Vec<String> {
        let Some(sp) = lists::spell(name) else { return vec![] };
        self.slot_options(sp.level).iter().map(|(id, l, left)| {
            if id == "pact" { format!("Pact slot, level {l} ({left} left)") } else { format!("Level {l} slot ({left} left)") }
        }).collect()
    }

    pub(super) fn cast_slot_pick(&mut self, name: &str, i: usize) {
        let Some(sp) = lists::spell(name) else { return };
        if let Some((id, l, _)) = self.slot_options(sp.level).get(i).cloned() { self.cast_with(sp, &id, l); }
    }

    fn cast_with(&mut self, sp: &'static Spell, id: &str, slot_level: u32) {
        self.snap();
        if let Some(v) = self.ch.res.get_mut(id) { *v -= 1; }
        self.cast_effect(sp, slot_level);
    }

    fn cast_effect(&mut self, sp: &'static Spell, slot_level: u32) {
        let ab = self.ch.casting().map(|c| c.ability.clone()).unwrap_or_else(|| "INT".into());
        let tag = if slot_level > sp.level { format!(" at level {slot_level}") } else { String::new() };
        let conc = if sp.conc { " [concentration]" } else { "" };
        self.say(format!("Cast {}{tag}{conc}", sp.name), Tone::Good);
        let mut crit = false;
        if sp.attack {
            let hit = self.ch.md(&ab) + self.ch.prof();
            let (r, note) = self.d20();
            crit = r == 20;
            self.say(format!("Spell attack: {r}{hit:+}={}{note}{}", r as i32 + hit, if crit { " CRIT!" } else { "" }), Tone::Norm);
        }
        if sp.save.is_some() {
            let dc = self.ch.dc(&ab);
            self.say(format!("Save: {} DC {dc}", sp.save.as_deref().unwrap_or("")), Tone::Warn);
        }
        match &sp.dmg {
            Some((n, sides, kind)) if !sp.healing => {
                let mut dice = *n;
                if sp.level > 0 { dice += sp.upcast.map_or(0, |u| u.0) * slot_level.saturating_sub(sp.level); }
                if sp.scales { dice = n * (1 + u32::from(self.ch.level >= 5) + u32::from(self.ch.level >= 11) + u32::from(self.ch.level >= 17)); }
                let rolls = if crit { 2 * dice } else { dice };
                let total: u32 = (0..rolls).map(|_| d(*sides)).sum();
                self.say(format!(" >> Damage: {total} {kind} ({rolls}d{sides})"), Tone::Good);
            }
            _ => { if !sp.attack && sp.save.is_none() { self.say(sp.desc.clone(), Tone::Dim); } }
        }
    }

    // --- learning and preparing ---------------------------------------------------

    /// Spells of the class list still to learn, how many to pick.
    pub(super) fn learn_options(&self, cantrips: bool) -> (usize, Vec<String>) {
        let Some((c, cl, prog)) = self.ch.cast_src() else { return (0, vec![]) };
        let have = if cantrips { &self.ch.cantrips } else { &self.ch.spells };
        let want = if cantrips { self.ch.cantrips_max() } else if c.mode == "prepared" { 0 } else { self.ch.known_max() };
        let names: Vec<String> = lists::spell_choices(c, &prog, cl, cantrips).iter()
            .map(|s| s.name.clone()).filter(|n| !have.contains(n)).collect();
        (want.saturating_sub(have.len()).min(names.len()), names)
    }

    pub(super) fn prepare_items(&self) -> (usize, Vec<String>) {
        let names = prepare_options(&self.ch);
        (self.ch.prepared_max().min(names.len()), names)
    }

    pub(super) fn open_learn(&mut self, cantrips: bool, step: Step) {
        let (n, names) = self.learn_options(cantrips);
        if n == 0 { return self.advance(); }
        let items = names.iter().map(|n| self.spell_line(n)).collect();
        self.multi(&format!("Learn {n} {}", if cantrips { "cantrips" } else { "spells" }), items, n, step);
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
        let mut v: Vec<&'static Spell> = vec![];
        for (c, cl, prog) in self.ch.casters() {
            for cantrips in [true, false] { v.extend(lists::spell_choices(c, &prog, cl, cantrips)); }
        }
        v.sort_by(|a, b| (a.level, &a.name).cmp(&(b.level, &b.name)));
        v.dedup_by_key(|s| &s.name);
        v.retain(|s| !self.ch.cantrips.contains(&s.name) && !self.ch.spells.contains(&s.name));
        v
    }

    pub(super) fn spell_known_names(&self) -> Vec<String> {
        self.ch.cantrips.iter().chain(&self.ch.spells).cloned().collect()
    }
}

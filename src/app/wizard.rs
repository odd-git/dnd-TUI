//! New-character wizard: one modal per stage, the answers pile up in a Draft.

use super::*;
use crate::builder::{build, Draft};
use super::prepare_options;
use crate::lists;

const LANGUAGES: [&str; 16] = [
    "Dwarvish", "Elvish", "Giant", "Gnomish", "Goblin", "Halfling", "Orc", "Abyssal", "Celestial",
    "Draconic", "Deep Speech", "Infernal", "Primordial", "Sylvan", "Undercommon", "Common",
];
const ARRAY: [i32; 6] = [15, 14, 13, 12, 10, 8];
const METHODS: [&str; 4] = ["Standard array (15 14 13 12 10 8)", "Roll 4d6, drop the lowest", "Point buy (27 points)", "Manual"];

#[derive(Clone, Copy)]
pub enum Wz {
    Name, Level, Lineage, Sub, Class, Subclass, Background, Method, Score(usize), Asi(usize),
    ClassSkills, FreeSkills, Languages, Tool, Feat(usize), FeatPick(usize), Style, Maneuvers, Expertise, Cantrips, Spells, Prepare,
    Armor, Shield, Weapons, Review,
}

pub struct Wiz { d: Draft, pool: Vec<i32>, points: i32, method: usize }

fn cost(score: i32) -> i32 { match score { 8 => 0, 9 => 1, 10 => 2, 11 => 3, 12 => 4, 13 => 5, 14 => 7, _ => 9 } }

fn tag(name: &str, source: &str) -> String { format!("{name} ({})", lists::abbr(source)) }

impl App {
    pub(super) fn wiz_start(&mut self) {
        self.wiz = Some(Wiz { d: Draft { level: 1, ..Default::default() }, pool: vec![], points: 27, method: 0 });
        self.wiz_open(Wz::Name);
    }

    pub(super) fn wiz_abort(&mut self) {
        self.wiz = None;
        self.modal = None;
        self.queue.clear();
    }

    fn d(&mut self) -> &mut Draft { &mut self.wiz.as_mut().unwrap().d }
    fn dr(&self) -> &Draft { &self.wiz.as_ref().unwrap().d }

    fn lineage(&self) -> &'static lists::Lineage { lists::lineages().iter().find(|l| l.name == self.dr().lineage).expect("lineage") }
    fn class(&self) -> &'static lists::ClassData { lists::class_data(&self.dr().class).expect("class") }
    fn bg(&self) -> Option<&'static lists::Background> { lists::backgrounds().iter().find(|b| b.name == self.dr().background) }
    fn eff(&self) -> lists::Effective {
        let l = self.lineage();
        l.effective(self.dr().sub.as_ref().and_then(|s| l.subs.iter().position(|x| &x.name == s)))
    }

    fn known_skills(&self) -> Vec<String> {
        let d = self.dr();
        let mut v = self.bg().map(|b| b.skills.clone()).unwrap_or_default();
        v.extend(self.eff().skills);
        v.extend(d.class_skills.clone());
        v.extend(d.free_skills.clone());
        v
    }

    fn prog(&self) -> String {
        let d = self.dr();
        match &d.subclass { Some(s) => format!("{}/{s}", d.class), None => d.class.clone() }
    }

    fn casting_mode(&self) -> String { lists::casting_of(&self.prog()).map(|c| c.mode.clone()).unwrap_or_default() }

    /// (how many to pick, names offered) for cantrips or known / spellbook spells.
    fn spell_pick(&self, cantrips: bool) -> (usize, Vec<String>) {
        let prog = self.prog();
        if lists::casting_of(&prog).is_none() { return (0, vec![]) }
        let c = build(self.dr());
        let n = if cantrips { c.cantrips_max() } else if c.casting().is_some_and(|x| x.mode == "prepared") { 0 } else { c.known_max() };
        let names = lists::spell_choices(c.casting().unwrap(), &prog, c.level, cantrips).iter().map(|s| s.name.clone()).collect::<Vec<_>>();
        (n.min(names.len()), names)
    }

    fn prepare_pick(&self) -> (usize, Vec<String>) {
        let c = build(self.dr());
        let Some(cast) = c.casting() else { return (0, vec![]) };
        if cast.mode == "known" { return (0, vec![]); }
        let names = prepare_options(&c);
        (c.prepared_max().min(names.len()), names)
    }

    fn expertise_count(&self) -> usize { 2 * self.class().expertise.iter().filter(|&&l| l <= self.dr().level).count() }

    fn free_skill_count(&self) -> usize {
        let sub = self.class().subclasses.iter().find(|s| Some(&s.name) == self.dr().subclass.as_ref()).map_or(0, |s| s.skills);
        (self.eff().extra_skills + self.bg().map_or(0, |b| b.extra_skills)) as usize + sub
    }
    fn language_count(&self) -> usize { (self.eff().extra_languages + self.bg().map_or(0, |b| b.extra_languages)) as usize }

    fn maneuver_count(&self) -> usize {
        let d = self.dr();
        let known = lists::progression(&d.class, d.subclass.as_deref()).map_or(0, |p| at(&p.maneuvers_known, d.level));
        let pool = self.class().subclasses.iter().any(|s| Some(&s.name) == d.subclass.as_ref() && s.maneuver_pool.is_some());
        if pool { known as usize } else { 0 }
    }

    // lists whose position maps to a choice; used both to show and to resolve
    fn lineage_names() -> Vec<String> { lists::lineages().iter().map(|l| tag(&l.name, &l.source)).collect() }
    fn feat_options(&self) -> Vec<&'static lists::FeatDef> {
        lists::feats().iter().filter(|f| !self.dr().feats.contains(&f.name)).collect()
    }
    fn free_skill_options(&self) -> Vec<String> {
        let known = self.known_skills();
        SKILLS.iter().map(|s| s.0.to_string()).filter(|s| !known.contains(s)).collect()
    }
    fn class_skill_options(&self) -> Vec<String> {
        let known = self.known_skills();
        self.class().skills.from.iter().filter(|s| !known.contains(s)).cloned().collect()
    }
    fn language_options(&self) -> Vec<String> {
        let have = self.eff().languages;
        LANGUAGES.iter().map(|s| s.to_string()).filter(|s| !have.contains(s)).collect()
    }
    fn armor_options() -> Vec<&'static lists::Armor> { lists::armors().iter().filter(|a| a.category != "Shield").collect() }
    fn score_options(&self) -> Vec<i32> {
        let w = self.wiz.as_ref().unwrap();
        if w.method == 2 { (8..=15).filter(|s| cost(*s) <= w.points).collect() } else { w.pool.clone() }
    }
    fn asi_options(&self) -> Vec<&'static str> {
        let d = self.dr();
        ABILITIES.iter().copied().filter(|a| !d.asi_picks.iter().any(|p| p == a) && !self.eff().fixed.contains_key(*a)).collect()
    }

    fn wiz_list(&mut self, title: &str, items: Vec<String>, w: Wz) { self.list(title, items, Step::Wiz(w), true); }

    fn wiz_multi(&mut self, title: &str, items: Vec<String>, limit: usize, w: Wz) {
        let mut items = items;
        if limit == 0 { items.push("Done".into()); }
        self.query.clear();
        let on = vec![false; items.len()];
        self.modal = Some(Modal::Multi { title: title.into(), items, sel: 0, on, limit, step: Step::Wiz(w) });
    }

    /// Open a stage, or skip to the next one when it does not apply.
    pub(super) fn wiz_open(&mut self, w: Wz) {
        let skip = |s: &mut Self, next: Wz| s.wiz_open(next);
        match w {
            Wz::Name => self.input("Character name", Step::Wiz(w), false),
            Wz::Level => self.wiz_list("Starting level (ability score improvements start at 4: level up later)", vec!["1".into(), "2".into(), "3".into()], w),
            Wz::Lineage => self.wiz_list("Race / lineage", Self::lineage_names(), w),
            Wz::Sub => {
                let l = self.lineage();
                if l.subs.is_empty() { return skip(self, Wz::Class); }
                let mut items = vec!["(none)".to_string()];
                items.extend(l.subs.iter().map(|s| s.name.clone()));
                self.wiz_list(&format!("{}: subrace / variant", l.name), items, w);
            }
            Wz::Class => self.wiz_list("Class", lists::classes().iter().map(|c| format!("{} (d{})", c.name, c.hit_die)).collect(), w),
            Wz::Subclass => {
                let c = self.class();
                if self.dr().level < c.subclass_level { return skip(self, Wz::Background); }
                let items = c.subclasses.iter().map(|s| tag(&s.name, &s.source)).collect();
                self.wiz_list(&c.subclass_title.clone(), items, w);
            }
            Wz::Background => self.wiz_list("Background", lists::backgrounds().iter().map(|b| tag(&b.name, &b.source)).collect(), w),
            Wz::Method => self.wiz_list("Ability scores", METHODS.iter().map(|s| s.to_string()).collect(), w),
            Wz::Score(i) => {
                let ab = ABILITIES[i];
                let wz = self.wiz.as_ref().unwrap();
                if wz.method == 3 { return self.input(format!("{ab} score (3-20)"), Step::Wiz(w), true); }
                let opts = self.score_options();
                if wz.method != 2 && opts.len() == 1 { return self.wiz_score(i, 0); }
                let left = wz.points;
                let items = opts.iter().map(|s| if wz.method == 2 { format!("{s}  ({} pts, {} left)", cost(*s), left - cost(*s)) } else { s.to_string() }).collect();
                self.wiz_list(&format!("{ab} score"), items, w);
            }
            Wz::Asi(k) => {
                let total: u32 = self.eff().choose.iter().map(|c| c.n).sum();
                if k as u32 >= total { return skip(self, Wz::ClassSkills); }
                let amt = self.eff().choose.iter().flat_map(|c| std::iter::repeat_n(c.amount, c.n as usize)).nth(k).unwrap_or(1);
                let items = self.asi_options().iter().map(|a| a.to_string()).collect();
                self.wiz_list(&format!("{} ({}/{total}): +{amt} to", self.lineage().name, k + 1), items, w);
            }
            Wz::ClassSkills => {
                let c = self.class();
                self.wiz_multi(&format!("{} skills: choose {}", c.name, c.skills.choose), self.class_skill_options(), c.skills.choose, w);
            }
            Wz::FreeSkills => {
                let n = self.free_skill_count();
                if n == 0 { return skip(self, Wz::Languages); }
                self.wiz_multi(&format!("Extra skills: choose {n}"), self.free_skill_options(), n, w);
            }
            Wz::Languages => {
                let n = self.language_count();
                if n == 0 { return skip(self, Wz::Tool); }
                self.wiz_multi(&format!("Extra languages: choose {n}"), self.language_options(), n, w);
            }
            Wz::Tool => {
                let t = self.bg().map(|b| b.tools.clone()).unwrap_or_default();
                if t.is_empty() { return skip(self, Wz::Feat(0)); }
                if t.to_lowercase().contains("one ") || t.to_lowercase().contains("choice") {
                    return self.input(format!("Tool proficiency ({t})"), Step::Wiz(w), false);
                }
                self.d().tools.push(t);
                skip(self, Wz::Feat(0));
            }
            Wz::Feat(k) => {
                if k as u32 >= self.eff().feats { return skip(self, Wz::Style); }
                let items = self.feat_options().iter().map(|f| match &f.prereq { Some(p) => format!("{}  [{p}]", tag(&f.name, &f.source)), None => tag(&f.name, &f.source) }).collect();
                self.wiz_list("Feat", items, w);
            }
            Wz::FeatPick(k) => {
                let name = self.dr().feats[k].clone();
                let opts = lists::feats().iter().find(|f| f.name == name).and_then(|f| f.asi.as_ref()).map(|a| a.options.clone()).unwrap_or_default();
                match opts.len() {
                    0 => { self.d().feat_picks.push(String::new()); skip(self, Wz::Feat(k + 1)) }
                    1 => { self.d().feat_picks.push(opts[0].clone()); skip(self, Wz::Feat(k + 1)) }
                    _ => self.wiz_list(&format!("{name}: +1 to"), opts, w),
                }
            }
            Wz::Style => {
                let c = self.class();
                if c.styles.is_empty() || self.dr().level < c.style_level { return skip(self, Wz::Maneuvers); }
                let items = c.styles.iter().map(|s| format!("{}: {}", s.name, s.desc)).collect();
                self.wiz_list("Fighting style", items, w);
            }
            Wz::Maneuvers => {
                let n = self.maneuver_count();
                if n == 0 { return skip(self, Wz::Expertise); }
                let items = maneuvers().iter().map(|(k, v)| format!("{k}: {v}")).collect();
                self.wiz_multi(&format!("Maneuvers: choose {n}"), items, n, w);
            }
            Wz::Expertise => {
                let n = self.expertise_count();
                if n == 0 { return skip(self, Wz::Cantrips); }
                self.wiz_multi(&format!("Expertise (double proficiency): choose {n}"), self.known_skills(), n, w);
            }
            Wz::Cantrips => {
                let (n, names) = self.spell_pick(true);
                if n == 0 { return skip(self, Wz::Spells); }
                self.wiz_multi(&format!("Cantrips: choose {n}"), names, n, w);
            }
            Wz::Spells => {
                let (n, names) = self.spell_pick(false);
                if n == 0 { return skip(self, Wz::Prepare); }
                let title = if self.casting_mode() == "spellbook" { "Spellbook" } else { "Spells known" };
                self.wiz_multi(&format!("{title}: choose {n}"), names, n, w);
            }
            Wz::Prepare => {
                let (n, names) = self.prepare_pick();
                if n == 0 { return skip(self, Wz::Armor); }
                self.wiz_multi(&format!("Prepare spells: choose {n}"), names, n, w);
            }
            Wz::Armor => {
                let mut items = vec!["No armor".to_string()];
                items.extend(Self::armor_options().iter().map(|a| format!("{} (AC {})", a.name, a.ac)));
                self.wiz_list("Armor", items, w);
            }
            Wz::Shield => self.wiz_list("Shield?", vec!["No".into(), "Yes".into()], w),
            Wz::Weapons => {
                let items = lists::weapons().iter().map(|x| format!("{} ({}d{} {})", x.name, x.dice.0, x.dice.1, x.dmg_type)).collect();
                self.wiz_multi("Weapons (Enter toggles, pick Done at the end)", items, 0, w);
            }
            Wz::Review => {
                let c = build(self.dr());
                let mut items: Vec<String> = vec![
                    format!("{} | {} | {} | {}", c.name, c.race, c.title(), c.background),
                    format!("HP {}  AC {}  Prof +{}", c.hp_max().unwrap_or(0), c.ac().unwrap_or(0), c.prof()),
                    ABILITIES.iter().map(|a| format!("{a} {}({:+})", c.abilities[*a], c.md(a))).collect::<Vec<_>>().join(" "),
                    format!("Skills: {}", c.skills.join(", ")),
                    format!("Attacks: {}", c.actions.iter().filter(|a| a.ability.is_some()).map(|a| a.name.clone()).collect::<Vec<_>>().join(", ")),
                ];
                items.push("Create character".into());
                items.push("Cancel".into());
                self.wiz_list("Review", items, w);
            }
        }
    }

    fn wiz_score(&mut self, i: usize, pick: usize) {
        let opts = self.score_options();
        let Some(&s) = opts.get(pick) else { return };
        let w = self.wiz.as_mut().unwrap();
        if w.method == 2 { w.points -= cost(s); } else if let Some(p) = w.pool.iter().position(|x| *x == s) { w.pool.remove(p); }
        w.d.base.insert(ABILITIES[i].to_string(), s);
        self.wiz_open(if i < 5 { Wz::Score(i + 1) } else { Wz::Asi(0) });
    }

    pub(super) fn wiz_input(&mut self, w: Wz, text: String) {
        match w {
            Wz::Name => {
                if text.is_empty() { return self.wiz_open(Wz::Name); }
                self.d().name = text;
                self.wiz_open(Wz::Level);
            }
            Wz::Tool => {
                if !text.is_empty() { self.d().tools.push(text); }
                self.wiz_open(Wz::Feat(0));
            }
            Wz::Score(i) => match text.parse::<i32>() {
                Ok(s) if (3..=20).contains(&s) => {
                    self.d().base.insert(ABILITIES[i].to_string(), s);
                    self.wiz_open(if i < 5 { Wz::Score(i + 1) } else { Wz::Asi(0) });
                }
                _ => self.wiz_open(w),
            },
            _ => {}
        }
    }

    pub(super) fn wiz_pick(&mut self, w: Wz, i: usize) {
        match w {
            Wz::Level => { self.d().level = i as u32 + 1; self.wiz_open(Wz::Lineage); }
            Wz::Lineage => {
                let l = &lists::lineages()[i];
                let d = self.d();
                d.lineage = l.name.clone();
                d.sub = None;
                self.wiz_open(Wz::Sub);
            }
            Wz::Sub => {
                let l = self.lineage();
                if i > 0 { self.d().sub = Some(l.subs[i - 1].name.clone()); }
                self.wiz_open(Wz::Class);
            }
            Wz::Class => { let c = lists::classes()[i].name.clone(); self.d().class = c; self.d().subclass = None; self.wiz_open(Wz::Subclass); }
            Wz::Subclass => { let s = self.class().subclasses[i].name.clone(); self.d().subclass = Some(s); self.wiz_open(Wz::Background); }
            Wz::Background => { self.d().background = lists::backgrounds()[i].name.clone(); self.wiz_open(Wz::Method); }
            Wz::Method => {
                let mut rolled: Vec<i32> = vec![];
                if i == 1 {
                    for _ in 0..6 {
                        let mut r: Vec<i32> = (0..4).map(|_| d(6) as i32).collect();
                        r.sort();
                        rolled.push(r[1..].iter().sum());
                    }
                    rolled.sort_by(|a, b| b.cmp(a));
                    let msg = format!("Rolled: {}", rolled.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(" "));
                    self.say(msg, Tone::Good);
                }
                let w = self.wiz.as_mut().unwrap();
                w.method = i;
                w.points = 27;
                w.pool = if i == 0 { ARRAY.to_vec() } else { rolled };
                self.wiz_open(Wz::Score(0));
            }
            Wz::Score(n) => self.wiz_score(n, i),
            Wz::Asi(k) => {
                let a = self.asi_options()[i].to_string();
                self.d().asi_picks.push(a);
                self.wiz_open(Wz::Asi(k + 1));
            }
            Wz::Feat(k) => {
                let f = self.feat_options()[i].name.clone();
                self.d().feats.push(f);
                self.wiz_open(Wz::FeatPick(k));
            }
            Wz::FeatPick(k) => {
                let name = self.dr().feats[k].clone();
                let opts = lists::feats().iter().find(|f| f.name == name).and_then(|f| f.asi.as_ref()).map(|a| a.options.clone()).unwrap_or_default();
                self.d().feat_picks.push(opts[i].clone());
                self.wiz_open(Wz::Feat(k + 1));
            }
            Wz::Style => { let s = self.class().styles[i].name.clone(); self.d().style = Some(s); self.wiz_open(Wz::Maneuvers); }
            Wz::Armor => {
                self.d().armor = if i == 0 { None } else { Some(Self::armor_options()[i - 1].name.clone()) };
                self.wiz_open(Wz::Shield);
            }
            Wz::Shield => { self.d().shield = i == 1; self.wiz_open(Wz::Weapons); }
            Wz::Review => {
                match i {
                    5 => self.wiz_finish(),
                    6 => self.wiz_abort(),
                    _ => self.wiz_open(Wz::Review),
                }
            }
            _ => {}
        }
    }

    pub(super) fn wiz_multi_done(&mut self, w: Wz, picked: Vec<usize>) {
        match w {
            Wz::ClassSkills => {
                let opts = self.class_skill_options();
                self.d().class_skills = picked.iter().map(|&i| opts[i].clone()).collect();
                self.wiz_open(Wz::FreeSkills);
            }
            Wz::FreeSkills => {
                let opts = self.free_skill_options();
                self.d().free_skills = picked.iter().map(|&i| opts[i].clone()).collect();
                self.wiz_open(Wz::Languages);
            }
            Wz::Languages => {
                let opts = self.language_options();
                self.d().languages = picked.iter().map(|&i| opts[i].clone()).collect();
                self.wiz_open(Wz::Tool);
            }
            Wz::Maneuvers => {
                let names: Vec<String> = maneuvers().keys().cloned().collect();
                self.d().maneuvers = picked.iter().map(|&i| names[i].clone()).collect();
                self.wiz_open(Wz::Expertise);
            }
            Wz::Expertise => {
                let opts = self.known_skills();
                self.d().expertise = picked.iter().map(|&i| opts[i].clone()).collect();
                self.wiz_open(Wz::Cantrips);
            }
            Wz::Cantrips => {
                let (_, names) = self.spell_pick(true);
                self.d().cantrips = picked.iter().map(|&i| names[i].clone()).collect();
                self.wiz_open(Wz::Spells);
            }
            Wz::Spells => {
                let (_, names) = self.spell_pick(false);
                self.d().spells = picked.iter().map(|&i| names[i].clone()).collect();
                self.wiz_open(Wz::Prepare);
            }
            Wz::Prepare => {
                let (_, names) = self.prepare_pick();
                self.d().prepared = picked.iter().map(|&i| names[i].clone()).collect();
                self.wiz_open(Wz::Armor);
            }
            Wz::Weapons => {
                self.d().weapons = picked.iter().filter_map(|&i| lists::weapons().get(i)).map(|x| x.name.clone()).collect();
                self.wiz_open(Wz::Review);
            }
            _ => {}
        }
    }

    fn wiz_finish(&mut self) {
        let mut c = build(self.dr());
        self.wiz = None;
        let slug: String = c.name.to_lowercase().chars().map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' }).collect();
        let dir = data_dir();
        let mut path = dir.join(format!("{slug}.json"));
        for n in 2.. {
            if !path.exists() { break; }
            path = dir.join(format!("{slug}-{n}.json"));
        }
        c.init();
        if let Ok(json) = serde_json::to_string_pretty(&c) { let _ = std::fs::write(&path, json); }
        self.sheets.push((path.clone(), c));
        self.sheets.sort_by(|a, b| a.1.name.cmp(&b.1.name));
        let i = self.sheets.iter().position(|s| s.0 == path).unwrap_or(0);
        self.psel = i;
        self.open_sheet(i);
    }
}

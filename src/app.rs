use std::collections::{BTreeMap, VecDeque};
use std::path::PathBuf;

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;

use crate::model::*;

mod classfeat;
mod editor;
mod spells;
mod wizard;

#[derive(Clone, Copy, PartialEq)]
pub enum Tab { Combat, Spells, Character, Journal }
pub const TABS: [(Tab, &str); 4] = [(Tab::Combat, "Combat"), (Tab::Spells, "Spells"), (Tab::Character, "Character"), (Tab::Journal, "Journal")];

#[derive(Clone, Copy, PartialEq)]
#[allow(clippy::enum_variant_names)] // Adv::Adv reads as the rule's own word
pub enum Adv { Normal, Adv, Dis }

/// Kind of d20 roll, for the disadvantage exhaustion imposes.
#[derive(Clone, Copy, PartialEq)]
pub enum Roll { Check, Attack, Save }

#[derive(Clone, Copy)]
pub enum Tone { Norm, Good, Warn, Dim }

#[derive(Clone, Copy)]
pub enum Item { Action(usize), Toggle(usize) }

/// Everything a key or a mouse click can ask for.
#[derive(Clone, PartialEq)]
pub enum Cmd {
    Tab(Tab), Pick(usize), Select(usize), Click(usize), Enter, Up, Down, PageUp, PageDown,
    Damage, Heal, Temp, EndConc, Init, Maneuver, Roll(String), CheckMenu, Adv, Inspire, Cond, Exhaustion, Death, Items, Short, Long,
    Level, Edit, Undo, Back, New, Delete, Cast, Prepare, SpellClick(usize), JournalAdd, JournalDel, JournalSel(usize), ModalPick(usize),
}

/// What a modal is asking; the answer is handled in `resolve_*`.
#[derive(Clone)]
pub enum Step {
    Damage, Heal, Check, Cond, Exhaustion, Items, SetItem(String), Short,
    Asi, Ability { n: i32, optional: bool }, FeatList, FeatAsi(String),
    Maneuver { required: bool }, ReplaceAsk, Forget, Extras, Skill, NoteName, NoteDesc(String), JournalAdd,
    Subclass, DeleteSheet(usize), Expertise, Style, Pool(String),
    Learn(bool), Prepare, CastSlot(String), SpellAdd, SpellDel, LevelClass,
    Opt(String), Arcanum(u32), Secret, Smite, Recover(u32), FontSlot, FontPoints, BeastPick, BeastName, BeastHp(String), BeastAc(String, i32), BeastAtk, Hd(usize), AnyCantrip, TempHp,
    // editor
    Ident, IdentSet(usize), Scores, ScoreSet(String), Unskill, ArmorPick, ShieldPick(Option<String>), WeaponPick,
    CustomName, CustomAb(String), CustomDice(String, Option<String>), CustomType(String, Option<String>, (u32, u32)),
    RmAction, ResName, ResMax(String), ResRecharge(String, u32), RmRes, RmFeat,
    Wiz(wizard::Wz),
}

pub enum Modal {
    List { title: String, items: Vec<String>, sel: usize, step: Step, cancel: bool },
    Input { title: String, buf: String, step: Step, numeric: bool },
    /// Pick several: Enter toggles; confirms at `limit` picks, or on the last item ("Done") when limit is 0.
    Multi { title: String, items: Vec<String>, sel: usize, on: Vec<bool>, limit: usize, step: Step },
}

/// Snapshot taken when a multi-step flow (level up, edit) starts, to adjust current values at its end.
struct Flow { level: u32, hp_max: Option<i32>, res: BTreeMap<String, u32> }

pub struct Entry(pub String, pub Tone);

pub struct App {
    pub picking: bool,
    pub sheets: Vec<(PathBuf, Character)>,
    pub psel: usize,
    path: PathBuf,
    pub ch: Character,
    pub tab: Tab,
    pub sel: usize,
    pub jsel: usize,
    pub ssel: usize,
    pub scroll: u16,
    pub log_off: usize,
    pub adv: Adv,
    pub log: Vec<Entry>,
    pub modal: Option<Modal>,
    pub hot: Vec<(Rect, Cmd)>,
    pub quit: bool,
    undo: Vec<Character>,
    queue: VecDeque<Step>,
    flow: Option<Flow>,
    lv: usize, // class being leveled: 0 the main one, n the n-th of `multi`
    wiz: Option<wizard::Wiz>,
    pub query: String,
    pub searching: bool,
}

/// Case-insensitive match used by every search box; an empty query matches everything.
pub fn hit(query: &str, text: &str) -> bool { query.is_empty() || text.to_lowercase().contains(&query.to_lowercase()) }

fn d(sides: u32) -> u32 { rand::random_range(1..=sides) }

pub(crate) fn data_dir() -> PathBuf {
    let base = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/share"));
    base.join("dnd-tui")
}

/// Seed the bundled sheets on first run, then read every sheet in the data dir.
fn load_sheets() -> Vec<(PathBuf, Character)> {
    let dir = data_dir();
    let _ = std::fs::create_dir_all(&dir);
    // seeded once: a deleted bundled sheet must stay deleted
    let marker = dir.join(".seeded");
    if !marker.exists() {
        for (name, json) in BUNDLED {
            let p = dir.join(name);
            if !p.exists() { let _ = std::fs::write(p, json); }
        }
        let _ = std::fs::write(marker, "");
    }
    let mut v: Vec<(PathBuf, Character)> = std::fs::read_dir(&dir).into_iter().flatten().flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .filter_map(|p| serde_json::from_str(&std::fs::read_to_string(&p).ok()?).ok().map(|c| (p, c)))
        .collect();
    v.sort_by(|a, b| a.1.name.cmp(&b.1.name));
    v
}

impl App {
    pub fn new() -> Self {
        let sheets = load_sheets();
        let ch = sheets.first().map(|s| s.1.clone()).unwrap_or_else(|| serde_json::from_str(BUNDLED[0].1).expect("bundled sheet")); // placeholder while picking
        App {
            picking: true, sheets, psel: 0, path: PathBuf::new(), ch, tab: Tab::Combat, sel: 0, jsel: 0, ssel: 0,
            scroll: 0, log_off: 0, adv: Adv::Normal, log: vec![Entry("System start.".into(), Tone::Dim)],
            modal: None, hot: vec![], quit: false, undo: vec![], queue: VecDeque::new(), flow: None, lv: 0, wiz: None, query: String::new(), searching: false,
        }
    }

    // --- helpers -----------------------------------------------------------

    fn say(&mut self, s: impl Into<String>, t: Tone) {
        self.log.push(Entry(s.into(), t));
        if self.log.len() > 200 { self.log.remove(0); }
        self.log_off = 0;
    }

    fn snap(&mut self) {
        self.undo.push(self.ch.clone());
        if self.undo.len() > 100 { self.undo.remove(0); }
    }

    /// Hit dice pools: the main class, then one per extra class.
    fn hd_pools(&self) -> Vec<String> {
        self.ch.resources.iter().filter(|r| r.id == "hit_dice" || r.id.starts_with("hd_")).map(|r| r.id.clone()).collect()
    }

    fn have(&self, id: &str) -> u32 { self.ch.res.get(id).copied().unwrap_or(0) }

    fn res_name(&self, id: &str) -> String {
        self.ch.def(id).map(|r| r.name.clone()).unwrap_or_else(|| id.to_string())
    }

    pub fn save(&self) {
        if !self.picking {
            if let Ok(json) = serde_json::to_string_pretty(&self.ch) { let _ = std::fs::write(&self.path, json); }
        }
    }

    /// Actions and toggles shown in the list (an action whose resource has no uses at this level is hidden).
    pub fn visible(&self) -> Vec<Item> {
        let mut v: Vec<Item> = (0..self.ch.actions.len())
            .filter(|&i| self.ch.actions[i].uses.as_ref().is_none_or(|u| self.ch.res_max(u) != Some(0)) && self.ch.class_level() >= self.ch.actions[i].min_level)
            // in beast form the druid's own weapons give way to the beast's attacks
            .filter(|&i| self.ch.beast.is_none() || self.ch.actions[i].ability.is_none() || self.ch.actions[i].hit.is_some())
            .map(Item::Action).collect();
        v.extend((0..self.ch.toggles.len()).map(Item::Toggle));
        v
    }

    fn d20(&mut self, kind: Roll) -> (u32, String) {
        let (a, b) = (d(20), d(20));
        let tired = self.ch.exhaustion >= if kind == Roll::Check { 1 } else { 3 };
        let adv = match (self.adv, tired) { (Adv::Adv, true) => Adv::Normal, (_, true) => Adv::Dis, (a, false) => a };
        let out = match adv {
            Adv::Normal => (a, String::new()),
            Adv::Adv => (a.max(b), format!(" adv({a},{b})")),
            Adv::Dis => (a.min(b), format!(" dis({a},{b})")),
        };
        self.adv = Adv::Normal; // one roll only
        out
    }

    fn heal(&mut self, n: i32) {
        let Some(max) = self.ch.hp_max() else { return };
        let hp = self.ch.hp.unwrap_or(0);
        self.ch.hp = Some((hp + n).min(max));
        if hp == 0 && n > 0 { self.ch.death = [0, 0]; }
    }

    // --- input -------------------------------------------------------------

    pub fn on_key(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') { self.quit = true; return; }
        if self.searching { return self.search_key(key); }
        let searchable = match &self.modal {
            Some(Modal::Input { .. }) => false,
            Some(_) => true,
            None => !self.picking && matches!(self.tab, Tab::Character | Tab::Journal | Tab::Spells),
        };
        if key.code == KeyCode::Char('/') && searchable { self.searching = true; self.query.clear(); return; }
        if let Some(Modal::Input { buf, numeric, .. }) = &mut self.modal {
            match key.code {
                KeyCode::Char(c) if !*numeric || c.is_ascii_digit() || (c == '-' && buf.is_empty()) => buf.push(c),
                KeyCode::Backspace => { buf.pop(); }
                KeyCode::Enter => self.modal_confirm(None),
                KeyCode::Esc => self.close_modal(),
                _ => {}
            }
            return;
        }
        if self.modal.is_some() {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => self.exec(Cmd::Up),
                KeyCode::Down | KeyCode::Char('j') => self.exec(Cmd::Down),
                KeyCode::Enter => self.modal_confirm(None),
                KeyCode::Esc => {
                    if matches!(self.modal, Some(Modal::List { cancel: true, .. }) | Some(Modal::Multi { .. })) { self.close_modal(); }
                }
                _ => {}
            }
            return;
        }
        let c = match key.code { KeyCode::Char(c) => c.to_ascii_lowercase(), _ => '\0' };
        let cmd = match key.code {
            KeyCode::Up => Cmd::Up, KeyCode::Down => Cmd::Down, KeyCode::PageUp => Cmd::PageUp, KeyCode::PageDown => Cmd::PageDown,
            KeyCode::Enter => Cmd::Enter, KeyCode::Esc => Cmd::Back,
            KeyCode::Tab | KeyCode::BackTab => {
                let tabs = self.tabs();
                let i = tabs.iter().position(|t| t.0 == self.tab).unwrap_or(0);
                let n = if key.code == KeyCode::Tab { (i + 1) % tabs.len() } else { (i + tabs.len() - 1) % tabs.len() };
                Cmd::Tab(tabs[n].0)
            }
            KeyCode::F(n @ 1..=4) => match self.tabs().get(n as usize - 1) { Some(t) => Cmd::Tab(t.0), None => return },
            KeyCode::Delete if self.picking => Cmd::Delete,
            KeyCode::Delete | KeyCode::Backspace if self.tab == Tab::Journal && !self.picking => Cmd::JournalDel,
            _ => match (self.tab, c) {
                (_, 'n') if self.picking => Cmd::New,
                (_, 'd') if self.picking => Cmd::Delete,
                (_, 'q') => Cmd::Back, (_, 'z') => Cmd::Undo, (_, 'k') => Cmd::Up, (_, 'j') => Cmd::Down,
                (Tab::Combat, 'd') => Cmd::Damage, (Tab::Combat, 'h') => Cmd::Heal, (Tab::Combat, 'g') => Cmd::Temp, (_, 'o') if self.ch.concentration.is_some() => Cmd::EndConc, (Tab::Combat, 'i') => Cmd::Init,
                (Tab::Combat, 'm') => Cmd::Maneuver, (Tab::Combat, 't') => Cmd::CheckMenu, (Tab::Combat, 'v') => Cmd::Adv,
                (Tab::Combat, 'c') => Cmd::Cond, (Tab::Combat, 'f') => Cmd::Exhaustion, (Tab::Combat, 'n') => Cmd::Inspire, (Tab::Combat, 'x') => Cmd::Death, (Tab::Combat, 'p') => Cmd::Items,
                (Tab::Combat, 's') => Cmd::Short, (Tab::Combat, 'l') => Cmd::Long, (Tab::Combat, 'u') => Cmd::Level,
                (Tab::Combat, 'e') => Cmd::Edit, (Tab::Combat, '1'..='9') => Cmd::Select(c as usize - '1' as usize),
                (Tab::Journal, 'a') => Cmd::JournalAdd,
                (Tab::Spells, 'p') => Cmd::Prepare,
                (Tab::Spells, 'c') => Cmd::Cast,
                (Tab::Spells, 'e') => Cmd::Edit, (Tab::Spells, 's') => Cmd::Short, (Tab::Spells, 'l') => Cmd::Long,
                _ => return,
            },
        };
        self.exec(cmd);
    }

    pub fn on_click(&mut self, col: u16, row: u16) {
        let hit = self.hot.iter().rev().find(|(r, _)| col >= r.x && col < r.x + r.width && row >= r.y && row < r.y + r.height);
        if let Some((_, cmd)) = hit.cloned() { self.exec(cmd); }
    }

    pub fn on_scroll(&mut self, up: bool) {
        if self.modal.is_some() || self.picking || self.tab == Tab::Journal {
            self.exec(if up { Cmd::Up } else { Cmd::Down });
        } else {
            self.exec(if up { Cmd::PageUp } else { Cmd::PageDown });
        }
    }

    fn search_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char(c) => self.query.push(c),
            KeyCode::Backspace => { self.query.pop(); }
            KeyCode::Enter => { self.searching = false; return; }
            KeyCode::Esc => { self.searching = false; self.query.clear(); }
            _ => return,
        }
        self.refit();
    }

    /// After the query changes, move the selection to the first thing that still matches.
    fn refit(&mut self) {
        let q = self.query.clone();
        match &mut self.modal {
            Some(Modal::List { items, sel, .. }) => {
                if !hit(&q, &items[*sel]) { if let Some(i) = items.iter().position(|x| hit(&q, x)) { *sel = i; } }
            }
            Some(Modal::Multi { items, sel, limit, .. }) => {
                let n = items.len();
                let ok = |i: usize, x: &String| hit(&q, x) || (*limit == 0 && i == n - 1);
                if !ok(*sel, &items[*sel]) { if let Some(i) = items.iter().enumerate().position(|(i, x)| ok(i, x)) { *sel = i; } }
            }
            Some(Modal::Input { .. }) => {}
            None => match self.tab {
                Tab::Journal => { if let Some(i) = self.ch.journal.iter().position(|x| hit(&q, x)) { self.jsel = i; } }
                Tab::Spells => { if let Some(i) = self.spell_rows().iter().position(|x| hit(&q, &x.name)) { self.ssel = i; } }
                _ => self.scroll = 0,
            },
        }
    }

    /// Esc: cancel the wizard, or just skip the step.
    fn close_modal(&mut self) {
        self.query.clear();
        if self.wiz.is_some() { return self.wiz_abort(); }
        self.modal = None;
        self.advance();
    }

    fn multi_toggle(&mut self, pick: Option<usize>) {
        let Some(Modal::Multi { items, sel, on, limit, .. }) = &mut self.modal else { return };
        let i = pick.unwrap_or(*sel).min(items.len() - 1);
        *sel = i;
        let done_row = *limit == 0 && i == items.len() - 1;
        if !done_row { on[i] = !on[i]; }
        let n = on.iter().take(if *limit == 0 { items.len() - 1 } else { items.len() }).filter(|x| **x).count();
        if done_row || (*limit > 0 && n == *limit) {
            let Some(Modal::Multi { on, step, items, limit, .. }) = self.modal.take() else { return };
            let last = if limit == 0 { items.len() - 1 } else { items.len() };
            self.query.clear();
            let picked = (0..last).filter(|&i| on[i]).collect();
            match step {
                Step::Wiz(w) => self.wiz_multi_done(w, picked),
                Step::Learn(_) | Step::Prepare => self.spells_done(step, picked),
                Step::Expertise => {
                    let opts = self.expertise_options();
                    for i in picked { let s = opts[i].clone(); self.snap_in_flow(); self.say(format!("Expertise: {s}."), Tone::Good); self.ch.expertise.push(s); }
                }
                _ => {}
            }
            self.advance();
        }
    }

    fn modal_confirm(&mut self, pick: Option<usize>) {
        // never act on a row the search has hidden
        let hidden = match &self.modal {
            Some(Modal::List { items, sel, .. }) => !hit(&self.query, &items[pick.unwrap_or(*sel).min(items.len() - 1)]),
            Some(Modal::Multi { items, sel, limit, .. }) => {
                let i = pick.unwrap_or(*sel).min(items.len() - 1);
                !hit(&self.query, &items[i]) && !(*limit == 0 && i == items.len() - 1)
            }
            _ => false,
        };
        if hidden { return; }
        if matches!(self.modal, Some(Modal::Multi { .. })) { return self.multi_toggle(pick); }
        self.query.clear();
        match self.modal.take() {
            Some(Modal::List { items, sel, step, .. }) => { self.resolve_list(step, pick.unwrap_or(sel).min(items.len().saturating_sub(1))); }
            Some(Modal::Input { buf, step, .. }) => self.resolve_input(step, buf.trim().to_string()),
            Some(Modal::Multi { .. }) | None => {}
        }
        self.advance();
    }

    pub fn exec(&mut self, cmd: Cmd) {
        if let Cmd::ModalPick(i) = cmd { return self.modal_confirm(Some(i)); }
        if matches!(self.modal, Some(Modal::List { .. } | Modal::Multi { .. })) {
            let keep_last = matches!(self.modal, Some(Modal::Multi { limit: 0, .. }));
            let Some(Modal::List { sel, items, .. } | Modal::Multi { sel, items, .. }) = &mut self.modal else { return };
            let n = items.len();
            let ok = |i: usize| hit(&self.query, &items[i]) || (keep_last && i == n - 1);
            match cmd {
                Cmd::Up => { if let Some(j) = (0..*sel).rev().find(|&j| ok(j)) { *sel = j; } }
                Cmd::Down => { if let Some(j) = (*sel + 1..n).find(|&j| ok(j)) { *sel = j; } }
                _ => {}
            }
            return;
        }
        if self.modal.is_some() { return; }
        if self.picking {
            match cmd {
                Cmd::Up => self.psel = self.psel.saturating_sub(1),
                Cmd::Down => self.psel = (self.psel + 1).min(self.sheets.len()),
                Cmd::Enter => self.pick_sheet(self.psel),
                Cmd::Pick(i) => self.pick_sheet(i),
                Cmd::New => self.wiz_start(),
                Cmd::Delete => { if self.psel < self.sheets.len() { self.open(Step::DeleteSheet(self.psel)); } }
                Cmd::Back => self.quit = true,
                _ => {}
            }
            return;
        }
        match cmd {
            Cmd::Tab(t) => { self.tab = t; self.scroll = 0; self.query.clear(); }
            Cmd::Back if !self.query.is_empty() => { self.query.clear(); }
            Cmd::Back => {
                self.save();
                if let Some(s) = self.sheets.iter_mut().find(|s| s.0 == self.path) { s.1 = self.ch.clone(); }
                self.picking = true;
            }
            Cmd::Undo => match self.undo.pop() {
                Some(prev) => { self.ch = prev; self.modal = None; self.queue.clear(); self.flow = None; self.say("Undone.", Tone::Dim); }
                None => self.say("Nothing to undo.", Tone::Dim),
            },
            Cmd::Up | Cmd::Down => {
                let down = cmd == Cmd::Down;
                match self.tab {
                    Tab::Combat => {
                        let n = self.visible().len();
                        self.sel = if down { (self.sel + 1).min(n.saturating_sub(1)) } else { self.sel.saturating_sub(1) };
                    }
                    Tab::Spells => {
                        let n = self.spell_rows().len();
                        self.ssel = if down { (self.ssel + 1).min(n.saturating_sub(1)) } else { self.ssel.saturating_sub(1) };
                    }
                    Tab::Character => self.scroll = if down { self.scroll + 1 } else { self.scroll.saturating_sub(1) },
                    Tab::Journal => {
                        let j = &self.ch.journal;
                        let ok = |i: usize| hit(&self.query, &j[i]);
                        let next = if down { (self.jsel + 1..j.len()).find(|&i| ok(i)) } else { (0..self.jsel).rev().find(|&i| ok(i)) };
                        if let Some(i) = next { self.jsel = i; }
                    }
                }
            }
            Cmd::PageUp | Cmd::PageDown => {
                let down = cmd == Cmd::PageDown;
                match self.tab {
                    Tab::Combat => self.log_off = if down { self.log_off.saturating_sub(3) } else { (self.log_off + 3).min(self.log.len()) },
                    _ => self.scroll = if down { self.scroll + 8 } else { self.scroll.saturating_sub(8) },
                }
            }
            Cmd::Select(i) => { if i < self.visible().len() { self.sel = i; } }
            Cmd::Enter if self.tab == Tab::Spells => self.exec(Cmd::Cast),
            Cmd::Enter => self.run(self.sel),
            Cmd::Click(i) => { self.sel = i; self.run(i); }
            Cmd::Adv => {
                self.adv = match self.adv { Adv::Normal => Adv::Adv, Adv::Adv => Adv::Dis, Adv::Dis => Adv::Normal };
            }
            Cmd::Damage | Cmd::Heal if self.ch.hp_max().is_none() => self.say("HP are not tracked on this sheet.", Tone::Warn),
            Cmd::Damage => self.open(Step::Damage),
            Cmd::Heal => self.open(Step::Heal),
            Cmd::Temp => self.open(Step::TempHp),
            Cmd::EndConc => { if let Some(s) = self.ch.concentration.clone() { self.snap(); self.ch.concentration = None; self.say(format!("Concentration on {s} ends."), Tone::Dim); } }
            Cmd::Init => {
                let (r, note) = self.d20(Roll::Check);
                let m = self.ch.md("DEX");
                self.say(format!("Initiative: {r}{m:+}={}{note}", r as i32 + m), Tone::Good);
            }
            Cmd::Maneuver => self.maneuver_die(),
            Cmd::CheckMenu => self.open(Step::Check),
            Cmd::Roll(n) => self.roll_check(&n),
            Cmd::Cond => self.open(Step::Cond),
            Cmd::Exhaustion => self.open(Step::Exhaustion),
            Cmd::Inspire => {
                self.snap();
                self.ch.inspiration = !self.ch.inspiration;
                if self.ch.inspiration { self.say("Inspiration gained.", Tone::Good); }
                else { self.adv = Adv::Adv; self.say("Inspiration spent: the next roll has advantage.", Tone::Good); }
            }
            Cmd::Death => self.death_save(),
            Cmd::Items => self.items(),
            Cmd::Short => self.open(Step::Short),
            Cmd::Long => self.long_rest(),
            Cmd::Level => self.level_up(),
            Cmd::Edit => { self.flow_begin(); self.queue.push_back(Step::Extras); self.advance(); }
            Cmd::JournalAdd => self.open(Step::JournalAdd),
            Cmd::JournalDel => {
                if self.jsel < self.ch.journal.len() {
                    self.snap();
                    self.ch.journal.remove(self.jsel);
                    self.jsel = self.jsel.saturating_sub(1);
                }
            }
            Cmd::JournalSel(i) => self.jsel = i,
            Cmd::Cast => { if let Some(sp) = self.spell_rows().get(self.ssel) { let n = sp.name.clone(); self.cast(&n); } }
            Cmd::SpellClick(i) => { self.ssel = i; self.exec(Cmd::Cast); }
            Cmd::Prepare => {
                if self.ch.casting().is_some_and(|c| c.mode == "known") { self.say("Known-spell casters do not prepare.", Tone::Dim); }
                else { self.flow_begin(); self.queue.push_back(Step::Prepare); self.advance(); }
            }
            Cmd::Pick(_) | Cmd::ModalPick(_) | Cmd::New | Cmd::Delete => {}
        }
        let n = self.visible().len();
        self.sel = self.sel.min(n.saturating_sub(1));
    }

    /// The last row of the picker is "+ New character".
    fn pick_sheet(&mut self, i: usize) {
        if i >= self.sheets.len() { self.wiz_start() } else { self.open_sheet(i) }
    }

    fn delete_sheet(&mut self, i: usize) {
        let (path, c) = self.sheets.remove(i);
        let _ = std::fs::remove_file(path);
        self.psel = self.psel.min(self.sheets.len());
        self.say(format!("Deleted {}.", c.name), Tone::Dim);
    }

    fn open_sheet(&mut self, i: usize) {
        let (path, ch) = self.sheets[i].clone();
        self.path = path;
        self.ch = ch;
        self.ch.init();
        self.picking = false;
        self.tab = Tab::Combat;
        self.sel = 0;
        self.undo.clear();
        self.adv = Adv::Normal;
        self.log = vec![Entry(format!("{} ready.", self.ch.name), Tone::Dim)];
        self.catch_up();
    }

    // --- rolls -------------------------------------------------------------

    fn run(&mut self, vi: usize) {
        let Some(item) = self.visible().get(vi).copied() else { return };
        let ai = match item {
            Item::Toggle(t) => {
                self.snap();
                let n = self.ch.toggles[t].name.clone();
                if let Some(p) = self.ch.active.iter().position(|x| *x == n) {
                    self.ch.active.remove(p);
                    self.say(format!("{n}: OFF"), Tone::Dim);
                } else {
                    if let Some(u) = self.ch.toggles[t].uses.clone() {
                        if self.have(&u) == 0 { return self.say(format!("{n}: none left!"), Tone::Warn); }
                        if let Some(v) = self.ch.res.get_mut(&u) { *v -= 1; }
                    }
                    self.ch.active.push(n.clone());
                    self.say(format!("{n}: ON"), Tone::Good);
                }
                return;
            }
            Item::Action(a) => a,
        };
        let a = self.ch.actions[ai].clone();
        if let Some(id) = &a.pool { return self.open(Step::Pool(id.clone())); }
        if let Some(s) = &a.special { return self.special(s, &a); }
        if let Some(u) = &a.uses {
            if self.have(u) == 0 { return self.say(format!("{}: none left!", a.name), Tone::Warn); }
        }
        let mut slot = None;
        if !a.cost.is_empty() {
            match a.cost.iter().find(|r| self.have(r) > 0) {
                Some(r) => slot = Some(r.clone()),
                None => return self.say("No slots left!", Tone::Warn),
            }
        }
        self.snap();
        for r in a.uses.iter().chain(slot.iter()) {
            if let Some(v) = self.ch.res.get_mut(r) { *v -= 1; }
        }
        let Some((n, sides)) = self.ch.dice(&a) else {
            let from = slot.map(|s| format!(" [{}]", self.res_name(&s))).unwrap_or_default();
            return self.say(format!("{}{from}: {}", a.name, self.ch.fmt(&a.desc)), Tone::Norm);
        };
        let prof = self.ch.prof();
        let mut crit = false;
        if let Some(hit) = a.hit.or_else(|| a.ability.as_ref().map(|ab| self.ch.md(ab) + prof)) {
            let crit_min = self.ch.toggles.iter().filter(|t| self.ch.active.contains(&t.name)).filter_map(|t| t.crit_min).min().unwrap_or(20);
            let (r, note) = self.d20(Roll::Attack);
            crit = r >= crit_min;
            let tag = if crit { " CRIT!" } else if r == 1 { " miss (nat 1)" } else { "" };
            self.say(format!("{}: {r}{hit:+}={}{note}{tag}", a.name, r as i32 + hit), if crit { Tone::Good } else { Tone::Norm });
            if r == 1 { return; }
        }
        let rolls = if crit { 2 * n } else { n };
        let base: i32 = (0..rolls).map(|_| d(sides) as i32).sum();
        let mut bonus = a.bonus;
        if a.dmg_ability { bonus += a.ability.as_deref().map_or(0, |ab| self.ch.md(ab)); }
        if a.level_bonus { bonus += self.ch.level as i32; }
        if !a.heal { bonus += self.ch.toggle_bonus(a.ability.as_deref()); }
        if !a.heal && self.ch.toggles.iter().any(|t| t.dmg_bonus_prof && self.ch.active.contains(&t.name)) { bonus += prof; }
        if a.heal {
            self.heal(base + bonus);
            self.say(format!("{}: healed {} ({base}{bonus:+})", a.name, base + bonus), Tone::Good);
        } else {
            self.say(format!(" >> Damage: {} {} ({base}{bonus:+})", base + bonus, a.dmg_type), Tone::Good);
        }
    }

    fn maneuver_die(&mut self) {
        let Some(pool) = self.ch.maneuver_pool.clone() else { return self.say("No maneuvers on this sheet.", Tone::Warn) };
        if self.have(&pool) == 0 { return self.say("No superiority dice left!", Tone::Warn); }
        self.snap();
        *self.ch.res.get_mut(&pool).unwrap() -= 1;
        let die = self.ch.res_die(&pool).unwrap_or(8);
        self.say(format!("Superiority d{die}: {}", d(die)), Tone::Good);
    }

    fn roll_check(&mut self, name: &str) {
        let m = self.ch.check_mod(name);
        let (r, note) = self.d20(if name.ends_with(" save") { Roll::Save } else { Roll::Check });
        self.say(format!("{name}: {r}{m:+}={}{note}", r as i32 + m), Tone::Good);
    }

    fn death_save(&mut self) {
        if self.ch.hp != Some(0) { return self.say("Death saves are for 0 HP.", Tone::Dim); }
        self.snap();
        let r = d(20);
        if r == 20 {
            self.ch.hp = Some(1);
            self.ch.death = [0, 0];
            return self.say("Death save: 20, back on your feet with 1 HP!", Tone::Good);
        }
        let (i, n) = if r == 1 { (1, 2) } else if r >= 10 { (0, 1) } else { (1, 1) };
        self.ch.death[i] = (self.ch.death[i] + n).min(3);
        let [ok, ko] = self.ch.death;
        let end = if ok >= 3 { " Stable." } else if ko >= 3 { " Dead." } else { "" };
        self.say(format!("Death save: {r} ({}){end}", if i == 0 { "success" } else { "failure" }), if i == 0 { Tone::Good } else { Tone::Warn });
    }

    fn items(&mut self) {
        let items: Vec<String> = self.ch.resources.iter().filter(|r| r.max.is_none()).map(|r| r.id.clone()).collect();
        match items.len() {
            0 => self.say("No items to count.", Tone::Dim),
            1 => self.open(Step::SetItem(items[0].clone())),
            _ => self.open(Step::Items),
        }
    }

    fn reset(&mut self, long: bool) {
        for def in self.ch.resources.clone() {
            let Some(max) = self.ch.res_max(&def.id) else { continue };
            let v = self.ch.res.entry(def.id.clone()).or_insert(0);
            if def.id == "hit_dice" || def.id.starts_with("hd_") {
                if long { *v = (*v + (max / 2).max(1)).min(max); }
            } else if def.recharge == "short" || (long && def.recharge == "long") {
                *v = max;
            }
        }
    }

    fn long_rest(&mut self) {
        self.snap();
        if (1..6).contains(&self.ch.exhaustion) { self.ch.exhaustion -= 1; self.say(format!("Exhaustion drops to {}.", self.ch.exhaustion), Tone::Good); }
        self.reset(true);
        self.ch.hp = self.ch.hp_max().or(self.ch.hp);
        self.ch.death = [0, 0];
        self.ch.temp_hp = 0;
        self.say("Long Rest completed.", Tone::Good);
    }

    // --- modals ------------------------------------------------------------

    fn unknown_maneuvers(&self) -> Vec<String> {
        maneuvers().keys().filter(|m| !self.ch.maneuvers.contains(m)).cloned().collect()
    }

    fn extras_options(&self) -> Vec<(&'static str, &'static str)> {
        let mut v = vec![("ident", "Name / race / background"), ("scores", "Ability scores"), ("skill", "Add skill proficiency"), ("unskill", "Remove skill proficiency"),
                         ("armor", "Armor and shield"), ("weapon", "Add weapon attack"), ("custom", "Add custom attack"), ("rmaction", "Remove attack or action")];
        if self.ch.maneuver_pool.is_some() { v.push(("learn", "Learn maneuver")); v.push(("replace", "Replace maneuver")); }
        v.push(("spelladd", "Add spell"));
        if self.ch.has_spells() { v.push(("spelldel", "Remove spell")); }
        if self.ch.casts() && self.ch.casting().is_some_and(|c| c.mode != "known") { v.push(("prepare", "Prepare spells")); }
        v.extend([("feat", "Add feat"), ("resource", "Add resource"), ("rmres", "Remove resource"), ("note", "Add feature / note"), ("rmfeat", "Remove feature"), ("done", "Done")]);
        v
    }

    fn list(&mut self, title: &str, items: Vec<String>, step: Step, cancel: bool) {
        self.query.clear();
        self.modal = Some(Modal::List { title: title.into(), items, sel: 0, step, cancel });
    }

    fn input(&mut self, title: impl Into<String>, step: Step, numeric: bool) {
        self.modal = Some(Modal::Input { title: title.into(), buf: String::new(), step, numeric });
    }

    fn open(&mut self, step: Step) {
        match step.clone() {
            Step::Damage => self.input("Damage taken", step, true),
            Step::Heal => self.input("HP healed", step, true),
            Step::TempHp => self.input(format!("Temporary HP (now {}, the higher one stays)", self.ch.temp_hp), step, true),
            Step::Check => {
                let items = self.ch.checks().iter().map(|(n, m)| format!("{n} {m:+}")).collect();
                self.list("Roll a check", items, step, true);
            }
            Step::Cond => self.open_cond(0),
            Step::Exhaustion => {
                let items = (0..=6).map(|n| format!("{n}: {}", if n == 0 { "none".into() } else { EXHAUSTION[..n].join(", ") })).collect();
                self.list("Exhaustion level", items, step, true);
                if let Some(Modal::List { sel, .. }) = &mut self.modal { *sel = self.ch.exhaustion as usize; }
            }
            Step::Items => {
                let items = self.ch.resources.iter().filter(|r| r.max.is_none()).map(|r| format!("{}: {}", r.name, self.have(&r.id))).collect();
                self.list("Which item?", items, step, true);
            }
            Step::SetItem(id) => { let t = format!("{} now in hand", self.res_name(&id)); self.input(t, step, true) }
            Step::Short => {
                if self.hd_pools().is_empty() { self.resolve_input(step, "0".into()) } else { self.open(Step::Hd(0)) }
            }
            Step::Asi => self.list("Ability Score Improvement", vec!["+2 to one ability".into(), "+1 to two abilities".into(), "Feat".into()], step, false),
            Step::Ability { n, optional } => {
                let mut items: Vec<String> = self.ch.ability_names().iter().map(|a| format!("{a} ({})", self.ch.abilities[*a])).collect();
                if optional { items.push("None".into()); }
                self.list(&format!("+{n} to which ability?"), items, step, false);
            }
            Step::Maneuver { required } => {
                let items = self.unknown_maneuvers().iter().map(|m| format!("{m}: {}", maneuvers()[m])).collect();
                self.list("Learn a maneuver", items, step, !required);
            }
            Step::ReplaceAsk => self.list("Replace a known maneuver?", vec!["No".into(), "Yes".into()], step, true),
            Step::Forget => { let items = self.ch.maneuvers.clone(); self.list("Forget which maneuver?", items, step, true) }
            Step::Extras => {
                let items = self.extras_options().iter().map(|o| o.1.to_string()).collect();
                self.list("Edit sheet", items, step, true);
            }
            Step::Skill => {
                let items = SKILLS.iter().filter(|(n, _)| !self.ch.skills.iter().any(|s| s == n)).map(|(n, a)| format!("{n} ({a})")).collect();
                self.list("Skill proficiency", items, step, true);
            }
            Step::NoteName => self.input("Feature / note name", step, false),
            Step::NoteDesc(_) => self.input("Description", step, false),
            Step::JournalAdd => self.input("Journal note", step, false),
            Step::Subclass => {
                let Some(c) = self.class_data() else { return };
                let items = c.subclasses.iter().map(|s| format!("{} ({})", s.name, crate::lists::abbr(&s.source))).collect();
                self.list(&c.subclass_title.clone(), items, step, false);
            }
            Step::LevelClass => { let items = self.level_options().into_iter().map(|o| o.0).collect(); self.list("Level up in which class?", items, step, true) }
            Step::Opt(ref kind) => { let k = kind.clone(); self.opt_open(&k, step) }
            Step::Arcanum(l) => { let items = self.arcanum_items(l); self.list(&format!("Mystic Arcanum: a level {l} warlock spell"), items, step, false) }
            Step::Secret => {
                let items = self.secret_items().iter().map(|s| format!("{} {} ({})", if s.level == 0 { "C".into() } else { s.level.to_string() }, s.name, s.classes.join("/"))).collect();
                self.list("Magical Secrets: a spell from any class", items, step, false);
            }
            Step::Smite => { let items = self.smite_items(); self.list("Divine Smite: which slot?", items, step, true) }
            Step::Recover(b) => {
                let items = self.recover_items(b).iter().map(|l| format!("Level {l} slot ({}/{})", self.have(&format!("slot{l}")), self.ch.res_max(&format!("slot{l}")).unwrap_or(0))).collect();
                self.list(&format!("Arcane Recovery: {b} slot levels left"), items, step, true);
            }
            Step::FontSlot => {
                let items: Vec<String> = self.font_slot_items().iter().map(|l| format!("Level {l} slot for {} points", [2, 3, 5, 6, 7][*l as usize - 1])).collect();
                if items.is_empty() { return self.say("Not enough sorcery points.", Tone::Warn); }
                self.list(&format!("Create a slot ({} points)", self.have("sorcery_points")), items, step, true);
            }
            Step::FontPoints => {
                let items: Vec<String> = self.font_points_items().iter().map(|l| format!("Level {l} slot: +{l} points")).collect();
                if items.is_empty() { return self.say("No slots to convert.", Tone::Warn); }
                self.list("Slot into sorcery points", items, step, true);
            }
            Step::AnyCantrip => { let items = self.any_cantrips(); self.list("Book of Shadows: a cantrip from any class", items, step, false) }
            Step::BeastPick => {
                let mut items: Vec<String> = self.beast_options().iter().map(|b| {
                    format!("{} (CR {}) AC {} HP {}, {}{}", b.name, b.challenge, b.ac, b.hp, classfeat::speed_text(b), if b.elemental { "  [2 uses]" } else { "" })
                }).collect();
                items.push("Other beast (enter its stats)".into());
                let t = format!("Wild Shape: max CR {}", self.wild_cr());
                self.list(&t, items, step, true);
            }
            Step::BeastName => { let t = format!("Beast (max CR {})", self.wild_cr()); self.input(t, step, false) }
            Step::BeastHp(ref n) => { let t = format!("{n}: hit points"); self.input(t, step, true) }
            Step::BeastAc(ref n, _) => { let t = format!("{n}: armor class"); self.input(t, step, true) }
            Step::BeastAtk => self.input("Beast attack, e.g. Bite +4 2d4+2 piercing (empty: done)", step, false),
            Step::Hd(i) => {
                let Some(id) = self.hd_pools().get(i).cloned() else { return };
                let t = format!("{} d{} to spend (0-{})", self.res_name(&id), self.ch.res_die(&id).unwrap_or(0), self.have(&id));
                self.input(t, step, true);
            }
            Step::Learn(c) => self.open_learn(c, step),
            Step::Prepare => self.open_prepare(step),
            Step::CastSlot(ref name) => { let items = self.cast_slot_items(name); self.list("Cast with which slot?", items, step, true); }
            Step::Pool(id) => {
                let t = format!("{}: points to spend ({} left)", self.res_name(&id), self.have(&id));
                self.input(t, step, true);
            }
            Step::Style => {
                let Some(c) = self.class_data() else { return };
                let items = c.styles.iter().map(|s| format!("{}: {}", s.name, s.desc)).collect();
                self.list("Fighting style", items, step, false);
            }
            Step::Expertise => {
                self.query.clear();
                let items = self.expertise_options();
                self.modal = Some(Modal::Multi { title: "Expertise: choose 2 skills".into(), on: vec![false; items.len()], items, sel: 0, limit: 2, step });
            }
            Step::Wiz(w) => self.wiz_open(w),
            Step::DeleteSheet(i) => {
                let n = self.sheets[i].1.name.clone();
                self.list(&format!("Delete {n}? This removes the file."), vec!["No".into(), "Yes, delete".into()], step, true);
            }
            _ => self.edit_open(step),
        }
    }

    fn open_cond(&mut self, sel: usize) {
        let items = CONDITIONS.iter().map(|c| format!("[{}] {c}", if self.ch.conditions.iter().any(|x| x == c) { "x" } else { " " })).collect();
        self.modal = Some(Modal::List { title: "Conditions (Enter toggles, Esc closes)".into(), items, sel, step: Step::Cond, cancel: true });
    }

    /// Open the next queued step; when the flow is over, apply what changed.
    fn advance(&mut self) {
        if self.modal.is_some() { return; }
        if let Some(s) = self.queue.pop_front() { return self.open(s); }
        self.flow_end();
    }

    fn resolve_input(&mut self, step: Step, text: String) {
        let num = text.parse::<i32>();
        match step {
            Step::Damage | Step::Heal => {
                let Ok(n) = num else { return };
                self.snap();
                if matches!(step, Step::Heal) {
                    self.heal(n);
                    return self.say(format!("Healed {n} HP."), Tone::Good);
                }
                let taken = n;
                let n = self.beast_soak(n);
                let n = self.temp_soak(n);
                if n > 0 {
                    let hp = self.ch.hp.unwrap_or(0);
                    self.ch.hp = Some((hp - n).max(0));
                    let mut msg = format!("Took {n} damage.");
                    if hp == 0 { self.ch.death[1] = (self.ch.death[1] + 1).min(3); msg += " Death save failure."; }
                    else if self.ch.hp == Some(0) { self.ch.death = [0, 0]; msg += " DOWN! Death saves."; }
                    self.say(msg, Tone::Warn);
                }
                self.concentration_check(taken);
            }
            Step::Pool(id) => {
                let n = num.unwrap_or(0).clamp(0, self.have(&id) as i32);
                if n == 0 { return; }
                self.snap();
                if let Some(v) = self.ch.res.get_mut(&id) { *v -= n as u32; }
                let (name, left) = (self.res_name(&id), self.have(&id));
                self.say(format!("{name}: spent {n}, {left} left."), Tone::Good);
            }
            Step::SetItem(id) => {
                let Ok(n) = num else { return };
                self.snap();
                self.ch.res.insert(id.clone(), n.max(0) as u32);
                let name = self.res_name(&id);
                self.say(format!("{name}: {}", n.max(0)), Tone::Dim);
            }
            Step::Short => {
                self.snap();
                self.reset(false);
                self.say("Short Rest: short-rest resources back.", Tone::Good);
            }
            Step::Hd(i) => {
                let pools = self.hd_pools();
                let id = pools[i].clone();
                if i == 0 { self.snap(); }
                let hd = self.have(&id) as i32;
                let n = num.unwrap_or(0).clamp(0, hd);
                let (die, con) = (self.ch.res_die(&id).unwrap_or(0), self.ch.md("CON"));
                let healed: i32 = (0..n).map(|_| (d(die.max(1)) as i32 + con).max(1)).sum();
                if let Some(v) = self.ch.res.get_mut(&id) { *v = (hd - n) as u32; }
                self.heal(healed);
                if n > 0 { self.say(format!("{n} x d{die}: +{healed} HP."), Tone::Good); }
                if i + 1 < pools.len() { return self.open(Step::Hd(i + 1)); }
                self.reset(false);
                self.say("Short Rest: short-rest resources back.", Tone::Good);
            }
            Step::BeastName => { let n = if text.trim().is_empty() { "Beast".to_string() } else { text.trim().to_string() }; self.open(Step::BeastHp(n)) }
            Step::BeastHp(name) => { if let Ok(hp) = num { self.open(Step::BeastAc(name, hp)) } }
            Step::BeastAc(name, hp) => { if let Ok(ac) = num { self.beast_form(name, hp, ac, String::new()); self.open(Step::BeastAtk); } }
            Step::BeastAtk => {
                if text.trim().is_empty() { return; }
                match parse_attack(&text) {
                    Some(a) => { self.say(format!("{}: {:+} to hit.", a.name, a.hit.unwrap_or(0)), Tone::Dim); self.ch.actions.insert(0, a); }
                    None => self.say("Could not read that: write name, +to-hit, dice, type.", Tone::Warn),
                }
                self.open(Step::BeastAtk);
            }
            Step::TempHp => {
                let Ok(n) = num else { return };
                self.snap();
                self.ch.temp_hp = self.ch.temp_hp.max(n.max(0));
                self.say(format!("Temporary HP: {}.", self.ch.temp_hp), Tone::Good);
            }
            Step::NoteName => { if !text.is_empty() { self.open(Step::NoteDesc(text)); } }
            Step::NoteDesc(name) => { self.snap_in_flow(); self.ch.features.push(Feature { name, desc: text, ..Default::default() }); }
            Step::JournalAdd => {
                if !text.is_empty() { self.snap(); self.ch.journal.push(format!("- {text}")); self.jsel = self.ch.journal.len() - 1; }
            }
            Step::Wiz(w) => self.wiz_input(w, text),
            _ => self.edit_input(step, text),
        }
    }

    fn resolve_list(&mut self, step: Step, i: usize) {
        match step {
            Step::Check => { if let Some(n) = self.ch.checks().get(i).map(|c| c.0.clone()) { self.roll_check(&n); } }
            Step::Cond => {
                self.snap();
                let c = CONDITIONS[i].to_string();
                match self.ch.conditions.iter().position(|x| *x == c) {
                    Some(p) => { self.ch.conditions.remove(p); }
                    None => self.ch.conditions.push(c),
                }
                self.open_cond(i);
            }
            Step::Exhaustion => {
                self.snap();
                self.ch.exhaustion = i.min(6) as u8;
                if let (Some(hp), Some(max)) = (self.ch.hp, self.ch.hp_max()) { self.ch.hp = Some(hp.min(max)); }
                if i == 6 { self.ch.hp = Some(0); self.ch.death = [0, 3]; }
                self.say(format!("Exhaustion {i}{}", if i == 0 { String::new() } else { format!(": {}", self.ch.exhaustion_text()) }), if i == 0 { Tone::Good } else { Tone::Warn });
            }
            Step::Items => {
                let ids: Vec<String> = self.ch.resources.iter().filter(|r| r.max.is_none()).map(|r| r.id.clone()).collect();
                if let Some(id) = ids.get(i) { self.open(Step::SetItem(id.clone())); }
            }
            Step::Asi => match i {
                0 => self.open(Step::Ability { n: 2, optional: false }),
                1 => { self.queue.push_front(Step::Ability { n: 1, optional: false }); self.open(Step::Ability { n: 1, optional: false }) }
                _ => self.open(Step::FeatList),
            },
            Step::Ability { n, .. } => {
                if let Some(ab) = self.ch.ability_names().get(i).copied() {
                    self.snap_in_flow();
                    let v = self.ch.abilities.get_mut(ab).unwrap();
                    *v = (*v + n).min(20);
                    let msg = format!("{ab} is now {v}");
                    self.say(msg, Tone::Good);
                }
            }
            Step::Maneuver { .. } => {
                if let Some(m) = self.unknown_maneuvers().get(i).cloned() {
                    self.snap_in_flow();
                    self.say(format!("Learned {m}."), Tone::Good);
                    self.ch.maneuvers.push(m);
                }
            }
            Step::ReplaceAsk => { if i == 1 { self.open(Step::Forget); } }
            Step::Forget => {
                if i < self.ch.maneuvers.len() {
                    self.snap_in_flow();
                    let m = self.ch.maneuvers.remove(i);
                    self.say(format!("Forgot {m}."), Tone::Dim);
                    self.open(Step::Maneuver { required: true });
                }
            }
            Step::Skill => {
                let names: Vec<&str> = SKILLS.iter().map(|s| s.0).filter(|n| !self.ch.skills.iter().any(|s| s == n)).collect();
                if let Some(n) = names.get(i) {
                    self.snap_in_flow();
                    self.ch.skills.push(n.to_string());
                    self.say(format!("Proficient in {n}."), Tone::Good);
                }
            }
            Step::Subclass => { if let Some(c) = self.class_data() { self.choose_subclass(c, i); } }
            Step::BeastPick => self.beast_pick(i),
            Step::Opt(kind) => self.opt_pick(&kind, i),
            Step::Arcanum(l) => self.arcanum_pick(l, i),
            Step::Secret => self.secret_pick(i),
            Step::AnyCantrip => { if let Some(n) = self.any_cantrips().get(i).cloned() { self.grant_spell(&n, "Pact of the Tome"); } }
            Step::Smite => self.smite_pick(i),
            Step::Recover(b) => self.recover_pick(b, i),
            Step::FontSlot => self.font_slot_pick(i),
            Step::FontPoints => self.font_points_pick(i),
            Step::LevelClass => { if let Some((_, t, new)) = self.level_options().into_iter().nth(i) { self.level_in(t, new); } }
            Step::CastSlot(name) => self.cast_slot_pick(&name, i),
            Step::Style => {
                if let Some(s) = self.class_data().and_then(|c| c.styles.get(i)) {
                    self.snap_in_flow();
                    let f = Feature { name: "Fighting Style".into(), desc: format!("{}: {}", s.name, s.desc), ..Default::default() };
                    match self.ch.features.iter_mut().find(|x| x.name == f.name) { Some(x) => *x = f, None => self.ch.features.push(f) }
                    self.say(format!("Fighting style: {}.", s.name), Tone::Good);
                }
            }
            Step::Wiz(w) => self.wiz_pick(w, i),
            Step::DeleteSheet(n) => { if i == 1 { self.delete_sheet(n); } }
            Step::Extras => {
                let key = self.extras_options()[i].0;
                if key == "done" { return; }
                self.queue.push_front(Step::Extras);
                self.open(match key {
                    "ident" => Step::Ident, "scores" => Step::Scores, "unskill" => Step::Unskill, "armor" => Step::ArmorPick,
                    "weapon" => Step::WeaponPick, "custom" => Step::CustomName, "rmaction" => Step::RmAction, "feat" => Step::FeatList,
                    "resource" => Step::ResName, "rmres" => Step::RmRes, "rmfeat" => Step::RmFeat,
                    "skill" => Step::Skill, "spelladd" => Step::SpellAdd, "spelldel" => Step::SpellDel, "prepare" => Step::Prepare,
                    "learn" => Step::Maneuver { required: false },
                    "replace" => Step::Forget,
                    _ => Step::NoteName,
                });
            }
            _ => self.edit_pick(step, i),
        }
    }

    // --- leveling ----------------------------------------------------------

    /// Inside a flow the opening snapshot already covers the changes.
    fn snap_in_flow(&mut self) { if self.flow.is_none() { self.snap(); } }

    fn flow_begin(&mut self) {
        self.snap();
        self.lv = 0;
        let res = self.ch.resources.iter().filter_map(|r| Some((r.id.clone(), self.ch.res_max(&r.id)?))).collect();
        self.flow = Some(Flow { level: self.ch.level, hp_max: self.ch.hp_max(), res });
    }

    fn flow_end(&mut self) {
        let Some(f) = self.flow.take() else { return };
        self.lv = 0;
        let mut gain = 0;
        if let (Some(old), Some(new)) = (f.hp_max, self.ch.hp_max()) {
            gain = new - old;
            self.ch.hp = Some((self.ch.hp.unwrap_or(0) + gain).max(0));
        }
        for (id, old) in f.res {
            if let Some(new) = self.ch.res_max(&id) {
                let v = self.ch.res.entry(id).or_insert(0);
                *v = (*v as i64 + new as i64 - old as i64).max(0) as u32;
            }
        }
        if self.ch.level != f.level {
            let max = self.ch.hp_max().map(|m| format!(" HP max {m} ({gain:+})")).unwrap_or_default();
            self.say(format!("Level {}!{max}", self.ch.level), Tone::Good);
        }
    }

    /// Class data of a sheet that has a class progression.
    fn class_data(&self) -> Option<&'static crate::lists::ClassData> {
        let prog = if self.lv > 0 { &self.ch.multi.get(self.lv - 1)?.prog } else { self.ch.progression.as_ref()? };
        let name = prog.split('/').next()?;
        crate::lists::class_data(name)
    }

    /// Give a character the subclass `i` of its class: tables, resources, actions, features so far.
    fn choose_subclass(&mut self, c: &'static crate::lists::ClassData, i: usize) {
        let sub = &c.subclasses[i];
        self.snap_in_flow();
        let prog = format!("{}/{}", c.name, sub.name);
        if self.lv > 0 {
            let e = &mut self.ch.multi[self.lv - 1];
            e.prog = prog;
            let level = e.level;
            self.take_class(self.lv, level);
            return self.finish_class();
        }
        let level = self.ch.class_level();
        self.ch.progression = Some(prog);
        self.ch.class_title = format!("{} {{level}} ({})", c.name, sub.name);
        self.ch.maneuver_pool = sub.maneuver_pool.clone();
        for r in &sub.resources { if self.ch.def(&r.id).is_none() { self.ch.resources.push(r.clone()); } }
        for a in &sub.actions { if !self.ch.actions.iter().any(|x| x.name == a.name) { self.ch.actions.push(a.clone()); } }
        for (_, fs) in sub.features.range(..=level) {
            for f in fs {
                self.ch.features.push(f.clone());
                self.say(format!("New: {}: {}", f.name, f.desc), Tone::Good);
            }
        }
        let want = self.ch.prog().map_or(0, |p| at(&p.maneuvers_known, level));
        for _ in self.ch.maneuvers.len() as u32..want { self.queue.push_front(Step::Maneuver { required: true }); }
        self.finish_class();
    }

    fn finish_class(&mut self) {
        self.ch.sync_spell_resources();
        self.ch.init();
    }

    /// Bring the resources, actions and toggles of a secondary class up to its `level` (its level tables are looked up by `Character::table`).
    fn take_class(&mut self, target: usize, level: u32) {
        let prog = self.ch.multi[target - 1].prog.clone();
        let (cls, sub) = prog.split_once('/').map_or((prog.as_str(), None), |(c, s)| (c, Some(s)));
        let Some(cd) = crate::lists::class_data(cls) else { return };
        let sd = sub.and_then(|s| cd.subclasses.iter().find(|x| x.name == s));
        let defs = cd.resources.iter().chain(sd.iter().flat_map(|s| s.resources.iter()));
        for r in defs {
            let max = match &r.max {
                Some(Max::Kw(k)) if k == "level" => Some(Max::N(level)),
                m => m.clone(),
            };
            match self.ch.resources.iter_mut().find(|x| x.id == r.id) {
                Some(x) => x.max = max,
                None => self.ch.resources.push(ResourceDef { max, ..r.clone() }),
            }
        }
        for a in cd.actions.iter().chain(sd.iter().flat_map(|s| s.actions.iter())) {
            if a.min_level <= level && !self.ch.actions.iter().any(|x| x.name == a.name) { self.ch.actions.push(ActionDef { min_level: 0, ..a.clone() }); }
        }
        for t in &cd.toggles { if !self.ch.toggles.iter().any(|x| x.name == t.name) { self.ch.toggles.push(t.clone()); } }
        for (_, fs) in sd.iter().flat_map(|s| s.features.range(..=level)) {
            for f in fs { if !self.ch.features.iter().any(|x| x.name == f.name) { self.ch.features.push(f.clone()); } }
        }
    }

    /// Ability scores a class asks for before you can multiclass into it (any of the groups, all within a group).
    fn can_enter(&self, class: &str) -> bool {
        let need: &[&str] = match class {
            "Barbarian" => &["STR"], "Bard" | "Sorcerer" | "Warlock" => &["CHA"], "Cleric" | "Druid" => &["WIS"], "Monk" | "Ranger" => &["DEX", "WIS"],
            "Paladin" => &["STR", "CHA"], "Rogue" => &["DEX"], "Wizard" => &["INT"], "Fighter" => &[],
            _ => &[],
        };
        let ok = |a: &str| self.ch.abilities.get(a).copied().unwrap_or(0) >= 13;
        need.iter().all(|a| ok(a)) && (class != "Fighter" || ok("STR") || ok("DEX"))
    }

    /// Menu of the level up: each class the character has, then the classes it could start (label, class index, new class).
    fn level_options(&self) -> Vec<(String, usize, Option<&'static str>)> {
        let mut v: Vec<(String, usize, Option<&'static str>)> = self.ch.classes().iter().enumerate()
            .map(|(i, (p, l))| (format!("{} {l} -> {}", p.replace('/', " / "), l + 1), i, None)).collect();
        let have: Vec<String> = self.ch.classes().iter().map(|(p, _)| p.split('/').next().unwrap_or("").to_string()).collect();
        let leaving_ok = self.ch.classes().iter().all(|(p, _)| self.can_enter(p.split('/').next().unwrap_or("")));
        if leaving_ok {
            for c in crate::lists::classes().iter().filter(|c| !have.contains(&c.name) && self.can_enter(&c.name)) {
                v.push((format!("New class: {}", c.name), self.ch.multi.len() + 1, Some(c.name.as_str())));
            }
        }
        v
    }

    fn expertise_options(&self) -> Vec<String> {
        self.ch.skills.iter().filter(|s| !self.ch.expertise.contains(s)).cloned().collect()
    }

    fn level_up(&mut self) {
        if self.ch.prog().is_none() { return self.say("No level table for this sheet.", Tone::Warn) }
        if self.ch.level >= 20 { return self.say("Already level 20.", Tone::Dim); }
        self.flow_begin();
        self.queue.push_back(Step::LevelClass);
        self.advance();
    }

    /// Level up in class `target` (0 the main one, n the n-th of `multi`), starting `new` first when given.
    fn level_in(&mut self, target: usize, new: Option<&str>) {
        if let Some(n) = new { self.ch.multi.push(Extra { prog: n.to_string(), level: 0 }); self.multiclass_into(n); }
        let (prog, old) = self.ch.classes()[target].clone();
        let (cls, sub) = prog.split_once('/').map_or((prog.as_str(), None), |(c, s)| (c, Some(s)));
        let Some(p) = crate::lists::progression(cls, sub) else { return };
        let old_known = at(&p.maneuvers_known, old);
        self.ch.level += 1;
        if target > 0 { self.ch.multi[target - 1].level += 1; }
        let l = old + 1;
        self.lv = target;
        if new.is_some() { self.say(format!("Multiclass: {cls}."), Tone::Good); }
        for f in p.features.get(&l).into_iter().flatten() {
            match self.ch.features.iter_mut().find(|x| x.name == f.name) {
                Some(x) => { x.desc = f.desc.clone(); x.url = f.url.clone(); }
                None => self.ch.features.push(f.clone()),
            }
            self.say(format!("New: {}: {}", f.name, f.desc), Tone::Good);
        }
        if target > 0 { self.take_class(target, l); }
        self.ch.sync_spell_resources();
        let asi = p.asi_levels.contains(&l);
        let new_m = if target == 0 { at(&p.maneuvers_known, l).saturating_sub(old_known) } else { 0 };
        let cd = crate::lists::class_data(cls);
        if cd.is_some_and(|c| c.subclass_level == l) && sub.is_none() { self.queue.push_back(Step::Subclass); }
        self.queue.push_back(Step::Learn(true));
        self.queue.push_back(Step::Learn(false));
        self.queue.push_back(Step::Prepare);
        let owed = self.owed();
        self.queue.extend(owed);
        // the main class got its level 1 style from the wizard; a new class gets it here
        if cd.is_some_and(|c| c.style_level == l && (l > 1 || target > 0) && !c.styles.is_empty()) { self.queue.push_back(Step::Style); }
        if cd.is_some_and(|c| c.expertise.contains(&l)) { self.queue.push_back(Step::Expertise); }
        if asi { self.queue.push_back(Step::Asi); }
        for _ in 0..new_m { self.queue.push_back(Step::Maneuver { required: true }); }
        if new_m > 0 || (asi && target == 0 && self.ch.maneuver_pool.is_some()) { self.queue.push_back(Step::ReplaceAsk); }
        self.queue.push_back(Step::Extras);
        self.advance();
    }
}

/// What a prepared / spellbook caster can pick to prepare: the spellbook, or the class list (minus always-prepared spells).
pub(crate) fn prepare_options(c: &Character) -> Vec<String> {
    let Some((cast, cl, prog)) = c.cast_src() else { return vec![] };
    let always = crate::lists::always_spells(&prog, cl);
    if cast.mode == "spellbook" { return c.spells.iter().filter(|s| !always.contains(s) && c.on_list(s)).cloned().collect(); }
    crate::lists::spell_choices(cast, &prog, cl, false).iter()
        .map(|s| s.name.clone()).filter(|n| !always.contains(n)).collect()
}

/// "Bite +4 2d4+2 piercing" -> a beast attack with a fixed to-hit.
fn parse_attack(line: &str) -> Option<ActionDef> {
    let t: Vec<&str> = line.split_whitespace().collect();
    let h = t.iter().position(|w| w.len() > 1 && w.starts_with(['+', '-']) && w[1..].chars().all(|c| c.is_ascii_digit()))?;
    let hit: i32 = t[h].parse().ok()?;
    let dice = t.get(h + 1)?;
    let (main, bonus) = match dice.find(['+', '-']) { Some(k) => (&dice[..k], dice[k..].parse().ok()?), None => (*dice, 0) };
    let (n, s) = main.split_once('d')?;
    let n = if n.is_empty() { 1 } else { n.parse().ok()? };
    Some(ActionDef {
        name: format!("Beast: {}", if h == 0 { "Attack".to_string() } else { t[..h].join(" ") }),
        desc: String::new(), hit: Some(hit), dice: Some((n, s.parse().ok()?)), bonus, dmg_type: t[h + 2..].join(" "),
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn beast_attack_lines() {
        let a = super::parse_attack("Bite +4 2d4+2 piercing").unwrap();
        assert_eq!((a.name.as_str(), a.hit, a.dice, a.bonus, a.dmg_type.as_str()), ("Beast: Bite", Some(4), Some((2, 4)), 2, "piercing"));
        let b = super::parse_attack("Giant claws +6 d8 slashing").unwrap();
        assert_eq!((b.name.as_str(), b.dice, b.bonus), ("Beast: Giant claws", Some((1, 8)), 0));
        assert!(super::parse_attack("Bite 2d4").is_none());
    }
}

use std::collections::{BTreeMap, VecDeque};
use std::path::PathBuf;

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;

use crate::model::*;

mod editor;
mod spells;
mod wizard;

#[derive(Clone, Copy, PartialEq)]
pub enum Tab { Combat, Spells, Character, Journal }
pub const TABS: [(Tab, &str); 4] = [(Tab::Combat, "Combat"), (Tab::Spells, "Spells"), (Tab::Character, "Character"), (Tab::Journal, "Journal")];

#[derive(Clone, Copy, PartialEq)]
pub enum Adv { Normal, Adv, Dis }

#[derive(Clone, Copy)]
pub enum Tone { Norm, Good, Warn, Dim }

#[derive(Clone, Copy)]
pub enum Item { Action(usize), Toggle(usize) }

/// Everything a key or a mouse click can ask for.
#[derive(Clone, PartialEq)]
pub enum Cmd {
    Tab(Tab), Pick(usize), Select(usize), Click(usize), Enter, Up, Down, PageUp, PageDown,
    Damage, Heal, Init, Maneuver, Roll(String), CheckMenu, Adv, Cond, Death, Items, Short, Long,
    Level, Edit, Undo, Back, New, Delete, Cast, Prepare, SpellClick(usize), JournalAdd, JournalDel, JournalSel(usize), ModalPick(usize),
}

/// What a modal is asking; the answer is handled in `resolve_*`.
#[derive(Clone)]
pub enum Step {
    Damage, Heal, Check, Cond, Items, SetItem(String), Short,
    Asi, Ability { n: i32, optional: bool }, FeatList, FeatAsi(String),
    Maneuver { required: bool }, ReplaceAsk, Forget, Extras, Skill, NoteName, NoteDesc(String), JournalAdd,
    Subclass, DeleteSheet(usize), Expertise, Style, Pool(String),
    Learn(bool), Prepare, CastSlot(String), SpellAdd, SpellDel,
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
    wiz: Option<wizard::Wiz>,
    pub query: String,
    pub searching: bool,
}

/// Case-insensitive match used by every search box; an empty query matches everything.
pub fn hit(query: &str, text: &str) -> bool { query.is_empty() || text.to_lowercase().contains(&query.to_lowercase()) }

fn d(sides: u32) -> u32 { rand::random_range(1..=sides) }

fn data_dir() -> PathBuf {
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
            modal: None, hot: vec![], quit: false, undo: vec![], queue: VecDeque::new(), flow: None, wiz: None, query: String::new(), searching: false,
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
            .filter(|&i| self.ch.actions[i].uses.as_ref().map_or(true, |u| self.ch.res_max(u) != Some(0)) && self.ch.level >= self.ch.actions[i].min_level)
            .map(Item::Action).collect();
        v.extend((0..self.ch.toggles.len()).map(Item::Toggle));
        v
    }

    fn d20(&mut self) -> (u32, String) {
        let (a, b) = (d(20), d(20));
        let out = match self.adv {
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
            None => !self.picking && matches!(self.tab, Tab::Character | Tab::Journal),
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
                (Tab::Combat, 'd') => Cmd::Damage, (Tab::Combat, 'h') => Cmd::Heal, (Tab::Combat, 'i') => Cmd::Init,
                (Tab::Combat, 'm') => Cmd::Maneuver, (Tab::Combat, 't') => Cmd::CheckMenu, (Tab::Combat, 'v') => Cmd::Adv,
                (Tab::Combat, 'c') => Cmd::Cond, (Tab::Combat, 'x') => Cmd::Death, (Tab::Combat, 'p') => Cmd::Items,
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
            Cmd::Enter => self.run(self.sel),
            Cmd::Click(i) => { self.sel = i; self.run(i); }
            Cmd::Adv => {
                self.adv = match self.adv { Adv::Normal => Adv::Adv, Adv::Adv => Adv::Dis, Adv::Dis => Adv::Normal };
            }
            Cmd::Damage | Cmd::Heal if self.ch.hp_max().is_none() => self.say("HP are not tracked on this sheet.", Tone::Warn),
            Cmd::Damage => self.open(Step::Damage),
            Cmd::Heal => self.open(Step::Heal),
            Cmd::Init => {
                let (r, note) = self.d20();
                let m = self.ch.md("DEX");
                self.say(format!("Initiative: {r}{m:+}={}{note}", r as i32 + m), Tone::Good);
            }
            Cmd::Maneuver => self.maneuver_die(),
            Cmd::CheckMenu => self.open(Step::Check),
            Cmd::Roll(n) => self.roll_check(&n),
            Cmd::Cond => self.open(Step::Cond),
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
        if let Some(ab) = &a.ability {
            let hit = self.ch.md(ab) + prof;
            let crit_min = self.ch.toggles.iter().filter(|t| self.ch.active.contains(&t.name)).filter_map(|t| t.crit_min).min().unwrap_or(20);
            let (r, note) = self.d20();
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
        let (r, note) = self.d20();
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
            if def.id == "hit_dice" {
                if long { *v = (*v + (max / 2).max(1)).min(max); }
            } else if def.recharge == "short" || (long && def.recharge == "long") {
                *v = max;
            }
        }
    }

    fn long_rest(&mut self) {
        self.snap();
        self.reset(true);
        self.ch.hp = self.ch.hp_max().or(self.ch.hp);
        self.ch.death = [0, 0];
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
        if self.ch.casts() {
            v.extend([("spelladd", "Add spell"), ("spelldel", "Remove spell")]);
            if self.ch.casting().is_some_and(|c| c.mode != "known") { v.push(("prepare", "Prepare spells")); }
        }
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
            Step::Check => {
                let items = self.ch.checks().iter().map(|(n, m)| format!("{n} {m:+}")).collect();
                self.list("Roll a check", items, step, true);
            }
            Step::Cond => self.open_cond(0),
            Step::Items => {
                let items = self.ch.resources.iter().filter(|r| r.max.is_none()).map(|r| format!("{}: {}", r.name, self.have(&r.id))).collect();
                self.list("Which item?", items, step, true);
            }
            Step::SetItem(id) => { let t = format!("{} now in hand", self.res_name(&id)); self.input(t, step, true) }
            Step::Short => {
                let hd = self.have("hit_dice");
                if self.ch.def("hit_dice").is_some() { self.input(format!("Hit dice to spend (0-{hd})"), step, true); }
                else { self.resolve_input(step, "0".into()); }
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
                let hp = self.ch.hp.unwrap_or(0);
                self.ch.hp = Some((hp - n).max(0));
                let mut msg = format!("Took {n} damage.");
                if hp == 0 && n > 0 { self.ch.death[1] = (self.ch.death[1] + 1).min(3); msg += " Death save failure."; }
                else if hp > 0 && self.ch.hp == Some(0) { self.ch.death = [0, 0]; msg += " DOWN! Death saves."; }
                self.say(msg, Tone::Warn);
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
                let hd = self.have("hit_dice") as i32;
                let n = num.unwrap_or(0).clamp(0, hd);
                self.snap();
                let (die, con) = (self.ch.hit_die.unwrap_or(0), self.ch.md("CON"));
                let healed: i32 = (0..n).map(|_| (d(die.max(1)) as i32 + con).max(1)).sum();
                if let Some(v) = self.ch.res.get_mut("hit_dice") { *v = (hd - n) as u32; }
                self.heal(healed);
                self.reset(false);
                self.say(format!("Short Rest: +{healed} HP, short-rest resources back."), Tone::Good);
            }
            Step::NoteName => { if !text.is_empty() { self.open(Step::NoteDesc(text)); } }
            Step::NoteDesc(name) => { self.snap_in_flow(); self.ch.features.push(Feature { name, desc: text }); }
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
            Step::CastSlot(name) => self.cast_slot_pick(&name, i),
            Step::Style => {
                if let Some(s) = self.class_data().and_then(|c| c.styles.get(i)) {
                    self.snap_in_flow();
                    let f = Feature { name: "Fighting Style".into(), desc: format!("{}: {}", s.name, s.desc) };
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
        let res = self.ch.resources.iter().filter_map(|r| Some((r.id.clone(), self.ch.res_max(&r.id)?))).collect();
        self.flow = Some(Flow { level: self.ch.level, hp_max: self.ch.hp_max(), res });
    }

    fn flow_end(&mut self) {
        let Some(f) = self.flow.take() else { return };
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
        let name = self.ch.progression.as_deref()?.split('/').next()?;
        crate::lists::class_data(name)
    }

    /// Give a character the subclass `i` of its class: tables, resources, actions, features so far.
    fn choose_subclass(&mut self, c: &'static crate::lists::ClassData, i: usize) {
        let sub = &c.subclasses[i];
        self.snap_in_flow();
        let level = self.ch.level;
        self.ch.progression = Some(format!("{}/{}", c.name, sub.name));
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
        self.ch.sync_spell_resources();
        self.ch.init();
    }

    fn expertise_options(&self) -> Vec<String> {
        self.ch.skills.iter().filter(|s| !self.ch.expertise.contains(s)).cloned().collect()
    }

    fn level_up(&mut self) {
        let Some(p) = self.ch.prog() else { return self.say("No level table for this sheet.", Tone::Warn) };
        if self.ch.level >= 20 { return self.say("Already level 20.", Tone::Dim); }
        self.flow_begin();
        let old_known = at(&p.maneuvers_known, self.ch.level);
        self.ch.level += 1;
        let l = self.ch.level;
        for f in p.features.get(&l).into_iter().flatten() {
            match self.ch.features.iter_mut().find(|x| x.name == f.name) {
                Some(x) => x.desc = f.desc.clone(),
                None => self.ch.features.push(f.clone()),
            }
            self.say(format!("New: {}: {}", f.name, f.desc), Tone::Good);
        }
        let asi = p.asi_levels.contains(&l);
        let new = at(&p.maneuvers_known, l).saturating_sub(old_known);
        if self.class_data().is_some_and(|c| c.subclass_level == l) && !self.ch.progression.as_deref().unwrap_or("").contains('/') {
            self.queue.push_back(Step::Subclass);
        }
        self.queue.push_back(Step::Learn(true));
        self.queue.push_back(Step::Learn(false));
        self.queue.push_back(Step::Prepare);
        if self.class_data().is_some_and(|c| c.style_level == l && l > 1 && !c.styles.is_empty()) { self.queue.push_back(Step::Style); }
        if self.class_data().is_some_and(|c| c.expertise.contains(&l)) { self.queue.push_back(Step::Expertise); }
        if asi { self.queue.push_back(Step::Asi); }
        for _ in 0..new { self.queue.push_back(Step::Maneuver { required: true }); }
        if new > 0 || (asi && self.ch.maneuver_pool.is_some()) { self.queue.push_back(Step::ReplaceAsk); }
        self.queue.push_back(Step::Extras);
        self.advance();
    }
}

/// What a prepared / spellbook caster can pick to prepare: the spellbook, or the class list (minus always-prepared spells).
pub(crate) fn prepare_options(c: &Character) -> Vec<String> {
    let Some(cast) = c.casting() else { return vec![] };
    let always = crate::lists::always_spells(c.progression.as_deref().unwrap_or(""), c.level);
    if cast.mode == "spellbook" { return c.spells.iter().filter(|s| !always.contains(s)).cloned().collect(); }
    crate::lists::spell_choices(cast, c.progression.as_deref().unwrap_or(""), c.level, false).iter()
        .map(|s| s.name.clone()).filter(|n| !always.contains(n)).collect()
}

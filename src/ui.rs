use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use crate::app::*;
use crate::model::*;

type Hot = Vec<(Rect, Cmd)>;

const GREEN: Color = Color::Green;

fn block(title: &str) -> Block<'static> {
    Block::default().borders(Borders::ALL).border_style(Style::new().fg(GREEN))
        .title(Span::styled(format!(" {title} "), Style::new().fg(GREEN).bold()))
}

fn tone(t: Tone) -> Style {
    match t {
        Tone::Norm => Style::new(),
        Tone::Good => Style::new().fg(GREEN),
        Tone::Warn => Style::new().fg(Color::Yellow),
        Tone::Dim => Style::new().fg(Color::DarkGray),
    }
}

fn selected() -> Style { Style::new().bg(GREEN).fg(Color::Black) }

/// Place items of the given widths left to right, wrapping at `width`; returns positions and row count.
fn flow(widths: &[u16], width: u16) -> (Vec<(u16, u16)>, u16) {
    let (mut x, mut y) = (0, 0);
    let mut pos = vec![];
    for &w in widths {
        if x > 0 && x + w > width { x = 0; y += 1; }
        pos.push((x, y));
        x += w + 2;
    }
    (pos, y + 1)
}

fn rows_needed(lines: &[Line], width: u16) -> u16 {
    lines.iter().map(|l| (l.width() as u16).div_ceil(width.max(1)).max(1)).sum()
}

fn bar(cur: i32, max: i32) -> String {
    let n = if max > 0 { (20 * cur.max(0) / max) as usize } else { 0 };
    format!("{}{}", "█".repeat(n), "░".repeat(20 - n))
}

pub fn draw(f: &mut Frame, app: &App) -> Hot {
    let mut hot: Hot = vec![];
    let area = f.area();
    if app.picking {
        draw_picker(f, app, area, &mut hot);
    } else {
        let foot = footer(app);
        let widths: Vec<u16> = foot.iter().map(|(k, l, _)| (k.len() + l.len() + 2) as u16).collect();
        let (pos, rows) = flow(&widths, area.width);
        let parts = Layout::vertical([Constraint::Length(1), Constraint::Min(3), Constraint::Length(rows)]).split(area);

        let mut x = parts[0].x;
        for (tab, name) in app.tabs() {
            let w = name.len() as u16 + 2;
            let r = Rect::new(x, parts[0].y, w, 1);
            let style = if tab == app.tab { selected() } else { Style::new().fg(GREEN) };
            f.render_widget(Paragraph::new(format!(" {name} ")).style(style), r);
            hot.push((r, Cmd::Tab(tab)));
            x += w + 1;
        }
        let who = if app.searching || !app.query.is_empty() { search_text(app) } else { format!("{}  (Tab/F1-F4 switch, / search)", app.ch.name) };
        f.render_widget(Paragraph::new(who).style(tone(Tone::Dim)).alignment(Alignment::Right), parts[0]);

        match app.tab {
            Tab::Combat => draw_combat(f, app, parts[1], &mut hot),
            Tab::Spells => draw_spells(f, app, parts[1], &mut hot),
            Tab::Character => draw_character(f, app, parts[1]),
            Tab::Journal => draw_journal(f, app, parts[1], &mut hot),
        }

        for ((k, l, cmd), (px, py)) in foot.into_iter().zip(pos) {
            let r = Rect::new(parts[2].x + px, parts[2].y + py, (k.len() + l.len() + 2) as u16, 1);
            let line = Line::from(vec![Span::styled(format!(" {k}"), Style::new().fg(GREEN).bold()), Span::raw(format!(" {l}"))]);
            f.render_widget(Paragraph::new(line), r);
            hot.push((r, cmd));
        }
    }
    if app.modal.is_some() {
        hot.clear(); // a modal takes every click
        draw_modal(f, app, area, &mut hot);
    }
    hot
}

fn footer(app: &App) -> Vec<(String, String, Cmd)> {
    let s = |k: &str, l: &str, c: Cmd| (k.to_string(), l.to_string(), c);
    match app.tab {
        Tab::Combat => {
            let mut v = vec![s("Enter", "Run", Cmd::Enter), s("D", "Damage", Cmd::Damage), s("H", "Heal", Cmd::Heal), s("I", "Init", Cmd::Init)];
            if app.ch.maneuver_pool.is_some() { v.push(s("M", "Maneuver die", Cmd::Maneuver)); }
            let adv = match app.adv { Adv::Normal => "Roll: normal", Adv::Adv => "Roll: ADVANTAGE", Adv::Dis => "Roll: DISADVANTAGE" };
            v.extend([s("T", "Check", Cmd::CheckMenu), s("V", adv, Cmd::Adv), s("C", "Conditions", Cmd::Cond), s("X", "Death save", Cmd::Death)]);
            if app.ch.resources.iter().any(|r| r.max.is_none()) { v.push(s("P", "Items", Cmd::Items)); }
            v.extend([s("S", "Short rest", Cmd::Short), s("L", "Long rest", Cmd::Long), s("U", "Level up", Cmd::Level),
                      s("E", "Edit", Cmd::Edit), s("Z", "Undo", Cmd::Undo), s("Q", "Sheets", Cmd::Back)]);
            v
        }
        Tab::Spells => {
            let mut v = vec![s("Enter", "Cast", Cmd::Cast)];
            if app.ch.casting().is_some_and(|c| c.mode != "known") { v.push(s("P", "Prepare", Cmd::Prepare)); }
            v.extend([s("E", "Edit", Cmd::Edit), s("Z", "Undo", Cmd::Undo), s("S", "Short rest", Cmd::Short), s("L", "Long rest", Cmd::Long), s("Q", "Sheets", Cmd::Back)]);
            v
        }
        Tab::Character => vec![s("↑↓", "Scroll", Cmd::Down), s("/", "Search", Cmd::Down), s("Z", "Undo", Cmd::Undo), s("Q", "Sheets", Cmd::Back)],
        Tab::Journal => vec![s("A", "Add note", Cmd::JournalAdd), s("/", "Search", Cmd::Down), s("Del", "Delete note", Cmd::JournalDel), s("Z", "Undo", Cmd::Undo), s("Q", "Sheets", Cmd::Back)],
    }
}

fn draw_picker(f: &mut Frame, app: &App, area: Rect, hot: &mut Hot) {
    let h = (app.sheets.len() as u16 + 5).min(area.height);
    let r = Rect::new(area.x + area.width.saturating_sub(60) / 2, area.y + area.height.saturating_sub(h) / 2, area.width.min(60), h);
    let b = block("DND TUI: which sheet?");
    let inner = b.inner(r);
    f.render_widget(b, r);
    for (i, (_, c)) in app.sheets.iter().enumerate() {
        let row = Rect::new(inner.x, inner.y + i as u16, inner.width, 1);
        if row.y >= inner.y + inner.height { break; }
        let style = if i == app.psel { selected() } else { Style::new() };
        f.render_widget(Paragraph::new(format!(" {}  -  {}", c.name, c.title())).style(style), row);
        hot.push((row, Cmd::Pick(i)));
    }
    let new = Rect::new(inner.x, inner.y + app.sheets.len() as u16, inner.width, 1);
    if new.y < inner.y + inner.height {
        f.render_widget(Paragraph::new(" + New character").style(if app.psel == app.sheets.len() { selected() } else { Style::new() }), new);
        hot.push((new, Cmd::New));
    }
    let hint = Rect::new(inner.x, inner.y + inner.height.saturating_sub(1), inner.width, 1);
    f.render_widget(Paragraph::new(" ↑↓ select, Enter open, N new, D delete, Q quit").style(tone(Tone::Dim)), hint);
}

fn status_lines(app: &App) -> Vec<Line<'static>> {
    let c = &app.ch;
    let mut v: Vec<Line> = vec![];
    let who = [c.name.as_str(), c.race.as_str(), &c.title(), c.background.as_str()].iter().filter(|s| !s.is_empty()).map(|s| s.to_string()).collect::<Vec<_>>().join(" | ");
    v.push(Line::from(who));

    let mut l2 = vec![];
    if let (Some(hp), Some(max)) = (c.hp, c.hp_max()) {
        let col = if hp * 4 <= max { Color::Red } else if hp * 2 <= max { Color::Yellow } else { GREEN };
        l2.push(Span::raw(format!("HP {hp}/{max} ")));
        l2.push(Span::styled(bar(hp, max), Style::new().fg(col)));
        l2.push(Span::raw(" | "));
    }
    if let Some(ac) = c.ac() { l2.push(Span::raw(format!("AC {ac} | "))); }
    l2.push(Span::raw(format!("Init {:+} | Prof +{}", c.md("DEX"), c.prof())));
    v.push(Line::from(l2));

    if c.hp == Some(0) {
        let mark = |n: u8, on: &str| format!("{}{}", on.repeat(n as usize), "○".repeat(3 - n as usize));
        v.push(Line::from(Span::styled(format!("DOWN: death saves  {}  {}", mark(c.death[0], "✔"), mark(c.death[1], "✘")), Style::new().fg(Color::Red).bold())));
    }
    v.push(Line::from(ABILITIES.iter().filter(|a| c.abilities.contains_key(**a)).map(|a| format!("{a} {}({:+})", c.abilities[*a], c.md(a))).collect::<Vec<_>>().join(" | ")));

    let mut d = vec![];
    if let Some(p) = c.prog() { if !p.attacks_per_action.is_empty() { d.push(format!("Attacks/action: {}", at(&p.attacks_per_action, c.level))); } }
    if c.maneuver_pool.is_some() { d.push(format!("Maneuver DC {}", 8 + c.prof() + c.md("STR").max(c.md("DEX")))); }
    if let Some(sa) = &c.spell_ability { d.push(format!("Spell DC {} (atk {:+})", c.dc(sa), c.prof() + c.md(sa))); }
    if c.abilities.contains_key("WIS") {
        let pp = c.checks().into_iter().find(|(n, _)| n == "Perception").map_or(c.md("WIS"), |x| x.1);
        d.push(format!("Passive Perception {}", 10 + pp));
    }
    if !d.is_empty() { v.push(Line::from(d.join(" | "))); }

    let mut r: Vec<Span> = vec![];
    for def in c.resources.iter().filter(|d| !(d.id.starts_with("slot") || d.id == "pact")) {
        let cur = c.res.get(&def.id).copied().unwrap_or(0);
        let die = c.res_die(&def.id).map(|x| format!(" d{x}")).unwrap_or_default();
        let text = match c.res_max(&def.id) {
            Some(0) => continue,
            Some(max) if def.pips && max <= 12 => format!("{}{die} {}{}", def.name, "●".repeat(cur.min(max) as usize), "○".repeat((max - cur.min(max)) as usize)),
            Some(max) => format!("{}{die} {cur}/{max}", def.name),
            None => format!("{}: {cur}", def.name),
        };
        if !r.is_empty() { r.push(Span::styled("  ", Style::new())); }
        r.push(Span::styled(text, if cur == 0 && def.pips { tone(Tone::Dim) } else { Style::new() }));
    }
    if !r.is_empty() { v.push(Line::from(r)); }
    if let Some(l) = slots_line(c) { v.push(l); }

    let mut st = vec![];
    if !c.conditions.is_empty() { st.push(Span::styled(format!("Conditions: {}", c.conditions.join(", ")), Style::new().fg(Color::Yellow))); }
    if !c.active.is_empty() { if !st.is_empty() { st.push(Span::raw("  ")); } st.push(Span::styled(format!("Active: {}", c.active.join(", ")), Style::new().fg(GREEN))); }
    if !st.is_empty() { v.push(Line::from(st)); }
    v
}

fn action_row(c: &Character, it: Item) -> (String, bool) {
    match it {
        Item::Toggle(t) => {
            let tg = &c.toggles[t];
            (format!("{:<20} [{}]", tg.name, if c.active.contains(&tg.name) { "ON " } else { "off" }), true)
        }
        Item::Action(i) => {
            let a = &c.actions[i];
            let dice = c.dice(a);
            let hit = match (&a.ability, dice) { (Some(ab), Some(_)) => format!("{:+}", c.md(ab) + c.prof()), (None, Some(_)) => if a.heal { "heal".into() } else { "auto".into() }, _ => String::new() };
            let dmg = dice.map(|(n, s)| {
                let mut b = a.bonus + if a.level_bonus { c.level as i32 } else { 0 };
                if a.dmg_ability { b += a.ability.as_deref().map_or(0, |x| c.md(x)); }
                b += c.toggle_bonus(a.ability.as_deref());
                format!("{}d{s}{}", if n == 1 { String::new() } else { n.to_string() }, if b != 0 { format!("{b:+}") } else { String::new() })
            }).unwrap_or_default();
            let left = a.uses.as_ref().map(|u| c.res.get(u).copied().unwrap_or(0));
            let uses = left.map(|n| format!(" x{n}")).unwrap_or_default();
            (format!("{:<20} {:>5} {:<8} {}{uses}", a.name, hit, dmg, c.fmt(&a.desc)), left != Some(0))
        }
    }
}

fn draw_combat(f: &mut Frame, app: &App, area: Rect, hot: &mut Hot) {
    let (left, mid, right) = if area.width >= 124 {
        // checks get their own column between the sections and the log
        let c = Layout::horizontal([Constraint::Min(40), Constraint::Length(30), Constraint::Length(52)]).split(area);
        (c[0], Some(c[1]), c[2])
    } else if area.width >= 110 {
        let c = Layout::horizontal([Constraint::Min(40), Constraint::Length(52)]).split(area);
        (c[0], None, c[1])
    } else {
        let c = Layout::vertical([Constraint::Min(12), Constraint::Length(9)]).split(area);
        (c[0], None, c[1])
    };
    draw_log(f, app, right, hot);

    let ch = &app.ch;
    let st = status_lines(app);
    let st_h = rows_needed(&st, left.width.saturating_sub(2)) + 2;
    let vis = app.visible();
    let known = if ch.maneuver_pool.is_some() { ch.maneuvers.len() as u16 + 2 } else { 0 };
    let checks = ch.checks();
    let labels: Vec<String> = checks.iter().map(|(n, m)| format!("{n} {m:+}")).collect();
    let (cpos, crows) = flow(&labels.iter().map(|l| l.chars().count() as u16).collect::<Vec<_>>(), left.width.saturating_sub(2));
    let parts = Layout::vertical([Constraint::Length(st_h), Constraint::Length(vis.len() as u16 + 2), Constraint::Length(known), Constraint::Length(if mid.is_some() { 0 } else { crows + 2 }), Constraint::Min(0)]).split(left);

    f.render_widget(Paragraph::new(st).wrap(Wrap { trim: false }).block(block("CHARACTER STATUS")), parts[0]);

    let b = block("ACTIONS (↑↓ or 1-9, Enter runs, click runs)");
    let inner = b.inner(parts[1]);
    f.render_widget(b, parts[1]);
    for (vi, it) in vis.iter().enumerate() {
        if vi as u16 >= inner.height { break; }
        let (text, ready) = action_row(ch, *it);
        let style = if vi == app.sel { selected() } else if ready { Style::new() } else { tone(Tone::Dim) };
        let row = Rect::new(inner.x, inner.y + vi as u16, inner.width, 1);
        f.render_widget(Paragraph::new(format!("{:>2} {text}", vi + 1)).style(style), row);
        hot.push((row, Cmd::Click(vi)));
    }

    if known > 0 {
        let pool = ch.maneuver_pool.clone().unwrap_or_default();
        let b = block(&format!("MANEUVERS (M or click: roll d{}, {} left)", ch.res_die(&pool).unwrap_or(8), ch.res.get(&pool).copied().unwrap_or(0)));
        let inner = b.inner(parts[2]);
        f.render_widget(b, parts[2]);
        for (i, m) in ch.maneuvers.iter().enumerate() {
            if i as u16 >= inner.height { break; }
            let row = Rect::new(inner.x, inner.y + i as u16, inner.width, 1);
            f.render_widget(Paragraph::new(format!("{m}: {}", maneuvers().get(m).map_or("", |s| s.as_str()))), row);
            hot.push((row, Cmd::Maneuver));
        }
    }

    if let Some(m) = mid {
        return draw_sheet_column(f, ch, m, hot);
    }
    let b = block("CHECKS (click to roll)");
    let inner = b.inner(parts[3]);
    f.render_widget(b, parts[3]);
    for (i, l) in labels.iter().enumerate() {
        let (x, y) = cpos[i];
        if y >= inner.height { break; }
        let r = Rect::new(inner.x + x, inner.y + y, l.chars().count() as u16, 1);
        f.render_widget(Paragraph::new(l.clone()), r);
        hot.push((r, Cmd::Roll(checks[i].0.clone())));
    }
}

const FULL: [&str; 6] = ["Strength", "Dexterity", "Constitution", "Intelligence", "Wisdom", "Charisma"];

/// One row per line, each clickable; rows past the box are dropped.
fn rows(f: &mut Frame, b: Block, area: Rect, lines: Vec<(Line<'static>, Cmd)>, hot: &mut Hot) {
    let inner = b.inner(area);
    f.render_widget(b, area);
    for (i, (line, cmd)) in lines.into_iter().enumerate() {
        if i as u16 >= inner.height { break; }
        let row = Rect::new(inner.x, inner.y + i as u16, inner.width, 1);
        f.render_widget(Paragraph::new(line), row);
        hot.push((row, cmd));
    }
}

/// Abilities, saving throws and skills as three boxes, like the paper sheet.
fn draw_sheet_column(f: &mut Frame, c: &Character, area: Rect, hot: &mut Hot) {
    let abs = c.ability_names();
    let skills: Vec<&str> = SKILLS.iter().filter(|(_, a)| c.abilities.contains_key(*a)).map(|s| s.0).collect();
    let parts = Layout::vertical([Constraint::Length(abs.len() as u16 + 2), Constraint::Length(abs.len() as u16 + 2), Constraint::Length(skills.len() as u16 + 2), Constraint::Min(0)]).split(area);
    let mark = |name: &str| if c.proficient(name) { Span::styled("●", Style::new().fg(GREEN)) } else { Span::styled("○", tone(Tone::Dim)) };

    let lines = abs.iter().map(|a| (Line::from(format!("{a:<4}{:>+3}   ({})", c.md(a), c.abilities[*a])), Cmd::Roll(a.to_string()))).collect();
    rows(f, block("ABILITIES"), parts[0], lines, hot);

    let lines = abs.iter().map(|a| {
        let name = format!("{a} save");
        let full = FULL[ABILITIES.iter().position(|x| x == a).unwrap()];
        (Line::from(vec![mark(&name), Span::raw(format!(" {:>+3}  {full}", c.check_mod(&name)))]), Cmd::Roll(name))
    }).collect();
    rows(f, block("SAVING THROWS"), parts[1], lines, hot);

    let lines = skills.iter().map(|n| {
        let ab = SKILLS.iter().find(|s| s.0 == *n).unwrap().1;
        (Line::from(vec![mark(n), Span::raw(format!(" {:>+3}  {n:<15} {ab}", c.check_mod(n)))]), Cmd::Roll(n.to_string()))
    }).collect();
    rows(f, block("SKILLS"), parts[2], lines, hot);
}

fn draw_log(f: &mut Frame, app: &App, area: Rect, hot: &mut Hot) {
    let adv = match app.adv { Adv::Normal => "normal", Adv::Adv => "ADVANTAGE", Adv::Dis => "DISADVANTAGE" };
    let b = block(&format!("COMBAT LOG | next roll: {adv}"));
    let inner = b.inner(area);
    f.render_widget(b, area);
    hot.push((Rect::new(area.x, area.y, area.width, 1), Cmd::Adv)); // click the title to cycle
    let h = inner.height as usize;
    let end = app.log.len().saturating_sub(app.log_off);
    let lines: Vec<Line> = app.log[end.saturating_sub(h)..end].iter().map(|e| Line::styled(e.0.clone(), tone(e.1))).collect();
    f.render_widget(Paragraph::new(lines), inner);
}

fn draw_character(f: &mut Frame, app: &App, area: Rect) {
    let c = &app.ch;
    let head = |t: &str| Line::styled(t.to_string(), Style::new().fg(GREEN).bold());
    let mut v: Vec<Line> = vec![head("ABILITIES AND SAVES")];
    for a in ABILITIES.iter().filter(|a| c.abilities.contains_key(**a)) {
        let prof = if c.save_profs.iter().any(|s| s == a) { " ✔ save proficiency" } else { "" };
        v.push(Line::from(format!("  {a} {:>2} ({:+})  save {:+}{prof}", c.abilities[*a], c.md(a), c.md(a) + if prof.is_empty() { 0 } else { c.prof() })));
    }
    if !c.skills.is_empty() {
        v.push(Line::default());
        v.push(head("SKILLS"));
        for (n, m) in c.checks().iter().filter(|(n, _)| c.skills.contains(n)) { v.push(Line::from(format!("  {n} {m:+}"))); }
    }
    v.push(Line::default());
    v.push(head("FEATURES"));
    for ft in &c.features { v.push(Line::from(vec![Span::styled(format!("  {}: ", ft.name), Style::new().bold()), Span::raw(c.fmt(&ft.desc))])); }
    if !c.maneuvers.is_empty() {
        v.push(Line::default());
        v.push(head("MANEUVERS"));
        for m in &c.maneuvers { v.push(Line::from(vec![Span::styled(format!("  {m}: "), Style::new().bold()), Span::raw(maneuvers().get(m).cloned().unwrap_or_default())])); }
    }
    if !c.info.is_empty() {
        v.push(Line::default());
        v.push(head("PROFICIENCIES AND EQUIPMENT"));
        v.extend(c.info.iter().map(|s| Line::from(format!("  {s}"))));
    }
    if !app.query.is_empty() {
        // keep each section's heading only when something under it matches
        let mut out: Vec<Line> = vec![];
        let mut group: Vec<Line> = vec![];
        let mut flush = |group: &mut Vec<Line<'static>>, out: &mut Vec<Line<'static>>| {
            if group.len() > 1 {
                if !out.is_empty() { out.push(Line::default()); }
                out.append(group);
            }
            group.clear();
        };
        for l in v {
            let text = l.to_string();
            if text.trim().is_empty() { flush(&mut group, &mut out); }
            else if group.is_empty() || hit(&app.query, &text) { group.push(l); }
        }
        flush(&mut group, &mut out);
        if out.is_empty() { out.push(Line::styled("Nothing matches.", tone(Tone::Dim))); }
        v = out;
    }
    let max = (rows_needed(&v, area.width.saturating_sub(2)) + 2).saturating_sub(area.height);
    f.render_widget(Paragraph::new(v).wrap(Wrap { trim: false }).scroll((app.scroll.min(max), 0)).block(block(&format!("{} | {}", c.name, c.title()))), area);
}

fn draw_journal(f: &mut Frame, app: &App, area: Rect, hot: &mut Hot) {
    let b = block("JOURNAL (auto-saved)");
    let inner = b.inner(area);
    f.render_widget(b, area);
    if app.ch.journal.is_empty() {
        return f.render_widget(Paragraph::new("Journal is empty. Press A to add a note.").style(tone(Tone::Dim)), inner);
    }
    let h = inner.height as usize;
    let shown: Vec<(usize, &String)> = app.ch.journal.iter().enumerate().filter(|(_, n)| hit(&app.query, n)).collect();
    let pos = shown.iter().position(|(i, _)| *i == app.jsel).unwrap_or(0);
    let start = pos.saturating_sub(h.saturating_sub(1));
    for (k, (i, note)) in shown.iter().enumerate().skip(start).take(h) {
        let i = *i;
        let row = Rect::new(inner.x, inner.y + (k - start) as u16, inner.width, 1);
        f.render_widget(Paragraph::new((*note).clone()).style(if i == app.jsel { selected() } else { Style::new() }), row);
        hot.push((row, Cmd::JournalSel(i)));
    }
}

fn draw_modal(f: &mut Frame, app: &App, area: Rect, hot: &mut Hot) {
    match app.modal.as_ref().unwrap() {
        Modal::List { title, items, sel, cancel, .. } => {
            let hint = if *cancel { "↑↓ Enter, click, Esc cancels" } else { "↑↓ Enter or click (required)" };
            list_box(f, area, hot, title, items, *sel, hint, app, false);
        }
        Modal::Multi { title, items, sel, on, limit, .. } => {
            let rows: Vec<String> = items.iter().zip(on).enumerate()
                .map(|(i, (it, x))| if *limit == 0 && i == items.len() - 1 { format!("--- {it} ---") } else { format!("[{}] {it}", if *x { "x" } else { " " }) })
                .collect();
            let hint = if *limit > 0 { format!("Enter toggles, confirms at {limit}, Esc cancels") } else { "Enter toggles, Done confirms, Esc cancels".into() };
            list_box(f, area, hot, title, &rows, *sel, &hint, app, *limit == 0);
        }
        Modal::Input { title, buf, .. } => {
            let w = 56.min(area.width.saturating_sub(2));
            let r = Rect::new(area.x + (area.width - w) / 2, area.y + area.height.saturating_sub(5) / 2, w, 5.min(area.height));
            f.render_widget(Clear, r);
            let b = block(title);
            let inner = b.inner(r);
            f.render_widget(b, r);
            f.render_widget(Paragraph::new(format!(" {buf}▏")), inner);
            f.render_widget(Paragraph::new(" Enter confirms, Esc cancels").style(tone(Tone::Dim)), Rect::new(inner.x, inner.y + 1.min(inner.height.saturating_sub(1)) + 1, inner.width, 1));
        }
    }
}

fn search_text(app: &App) -> String { format!("/ {}{}", app.query, if app.searching { "▏" } else { "" }) }

fn list_box(f: &mut Frame, area: Rect, hot: &mut Hot, title: &str, items: &[String], sel: usize, hint: &str, app: &App, keep_last: bool) {
    let n = items.len();
    let shown: Vec<usize> = (0..n).filter(|&i| hit(&app.query, &items[i]) || (keep_last && i == n - 1)).collect();
    let w = (items.iter().map(|s| s.chars().count()).max().unwrap_or(10).max(title.chars().count()) as u16 + 6).clamp(30, area.width.saturating_sub(4));
    let h = (shown.len() as u16 + 3).min(area.height.saturating_sub(2));
    let r = Rect::new(area.x + (area.width - w) / 2, area.y + (area.height - h) / 2, w, h);
    f.render_widget(Clear, r);
    let b = block(title);
    let inner = b.inner(r);
    f.render_widget(b, r);
    let rows = inner.height.saturating_sub(1) as usize; // last row is the hint
    let pos = shown.iter().position(|&i| i == sel).unwrap_or(0);
    let start = pos.saturating_sub(rows.saturating_sub(1));
    for (k, &i) in shown.iter().enumerate().skip(start).take(rows) {
        let row = Rect::new(inner.x, inner.y + (k - start) as u16, inner.width, 1);
        f.render_widget(Paragraph::new(format!(" {}", items[i])).style(if i == sel { selected() } else { Style::new() }), row);
        hot.push((row, Cmd::ModalPick(i)));
    }
    let hint = if app.searching || !app.query.is_empty() { format!("{}  (Enter keeps, Esc clears)", search_text(app)) } else { format!("{hint}, / searches") };
    f.render_widget(Paragraph::new(format!(" {hint}")).style(tone(Tone::Dim)), Rect::new(inner.x, inner.y + inner.height.saturating_sub(1), inner.width, 1));
}

/// "Slots L1 ●●○ L2 ●" for the status box and the Spells tab.
fn slots_line(c: &Character) -> Option<Line<'static>> {
    let mut parts: Vec<String> = vec![];
    for def in c.resources.iter().filter(|d| d.id.starts_with("slot") || d.id == "pact") {
        let (cur, max) = (c.res.get(&def.id).copied().unwrap_or(0), c.res_max(&def.id).unwrap_or(0));
        if max == 0 { continue; }
        let label = if def.id == "pact" { format!("Pact L{}", c.casting().map_or(0, |k| at(&k.pact_level, c.level))) } else { format!("L{}", &def.id[4..]) };
        parts.push(format!("{label} {}{}", "●".repeat(cur.min(max) as usize), "○".repeat((max - cur.min(max)) as usize)));
    }
    (!parts.is_empty()).then(|| Line::from(format!("Slots {}", parts.join("  "))))
}

fn draw_spells(f: &mut Frame, app: &App, area: Rect, hot: &mut Hot) {
    let c = &app.ch;
    let cols = Layout::horizontal([Constraint::Percentage(55), Constraint::Percentage(45)]).split(area);
    let b = block(&format!("SPELLS | {} DC {} atk {:+}", c.spell_ability.as_deref().unwrap_or(""), c.spell_ability.as_deref().map_or(0, |a| c.dc(a)), c.spell_ability.as_deref().map_or(0, |a| c.prof() + c.md(a))));
    let inner = b.inner(cols[0]);
    f.render_widget(b, cols[0]);
    let rows = app.spell_rows();
    let mut head = vec![];
    if let Some(l) = slots_line(c) { head.push(l); }
    if let Some(cast) = c.casting() {
        if cast.mode != "known" { head.push(Line::styled(format!("Prepared {}/{}", c.prepared.len(), c.prepared_max()), tone(Tone::Dim))); }
    }
    let hh = head.len() as u16;
    f.render_widget(Paragraph::new(head), Rect::new(inner.x, inner.y, inner.width, hh));
    let h = inner.height.saturating_sub(hh) as usize;
    let start = app.ssel.saturating_sub(h.saturating_sub(1));
    let mut last = u32::MAX;
    let _ = &mut last;
    for (k, sp) in rows.iter().enumerate().skip(start).take(h) {
        let row = Rect::new(inner.x, inner.y + hh + (k - start) as u16, inner.width, 1);
        let lvl = if sp.level == 0 { "C".to_string() } else { sp.level.to_string() };
        let usable = sp.level == 0 || c.casting().is_some_and(|k| k.kind == "pact" && c.res.get("pact").copied().unwrap_or(0) > 0)
            || (sp.level..=9).any(|l| c.res.get(&format!("slot{l}")).copied().unwrap_or(0) > 0);
        let style = if k == app.ssel { selected() } else if usable { Style::new() } else { tone(Tone::Dim) };
        let tag = format!("{}{}", if sp.conc { "c" } else { " " }, if sp.ritual { "r" } else { " " });
        f.render_widget(Paragraph::new(format!(" {lvl} {tag} {}", sp.name)).style(style), row);
        hot.push((row, Cmd::SpellClick(k)));
    }
    let db = block("DETAILS");
    let di = db.inner(cols[1]);
    f.render_widget(db, cols[1]);
    if let Some(sp) = rows.get(app.ssel) {
        let mut lines = vec![
            Line::styled(format!("{} ({})", sp.name, if sp.level == 0 { format!("{} cantrip", sp.school) } else { format!("level {} {}", sp.level, sp.school) }), Style::new().fg(GREEN).bold()),
            Line::from(format!("{} | {} | {}", sp.time, sp.range, sp.comp)),
            Line::from(format!("{}{}{}", sp.duration, if sp.conc { " (concentration)" } else { "" }, if sp.ritual { " (ritual)" } else { "" })),
            Line::default(),
            Line::from(sp.desc.clone()),
        ];
        if let Some((n, s, k)) = &sp.dmg { lines.push(Line::default()); lines.push(Line::styled(format!("Damage {n}d{s} {k}{}", sp.upcast.map_or(String::new(), |u| format!(", +{}d{} per slot level", u.0, u.1))), tone(Tone::Dim))); }
        lines.push(Line::styled(format!("{}", lists_abbr(&sp.source)), tone(Tone::Dim)));
        f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), di);
    } else {
        f.render_widget(Paragraph::new("No spells yet. E opens the editor to add some.").style(tone(Tone::Dim)), di);
    }
}

fn lists_abbr(s: &str) -> String { crate::lists::abbr(s) }

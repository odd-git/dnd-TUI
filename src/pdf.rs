//! Printable PDF of a character sheet, to play on paper after building the character here.
//! No dependency: a PDF 1.4 file with the standard Helvetica fonts, text, lines and boxes.

use crate::model::*;

const W: f32 = 595.0; // A4 in points
const H: f32 = 842.0;
const M: f32 = 36.0;
const FOOT: f32 = 44.0; // bottom margin, with the page footer in it
const LEFT: f32 = 156.0; // abilities, saves and skills
const GAP: f32 = 12.0;

/// Helvetica advance widths of ASCII 32..=126, in 1/1000 em.
const WIDTHS: [u16; 95] = [
    278, 278, 355, 556, 556, 889, 667, 191, 333, 333, 389, 584, 278, 333, 278, 278, 556, 556, 556, 556, 556, 556, 556, 556, 556, 556, 278, 278, 584, 584, 584, 556,
    1015, 667, 667, 722, 722, 667, 611, 778, 722, 278, 500, 667, 556, 833, 722, 778, 667, 778, 722, 667, 611, 722, 667, 944, 667, 667, 611, 278, 278, 278, 469, 556,
    333, 556, 556, 500, 556, 556, 278, 556, 556, 222, 222, 500, 222, 833, 556, 556, 556, 556, 333, 500, 278, 556, 500, 722, 500, 500, 500, 334, 260, 334, 584,
];

fn width(c: char) -> f32 {
    match c { ' '..='~' => WIDTHS[c as usize - 32] as f32, '’' | '‘' => 222.0, '“' | '”' => 333.0, _ => 556.0 }
}

fn text_width(s: &str, size: f32, bold: bool) -> f32 {
    s.chars().map(width).sum::<f32>() * size / 1000.0 * if bold { 1.1 } else { 1.0 }
}

/// Greedy line breaking: the first line is `first` points wide, the others `rest`.
fn wrap(s: &str, size: f32, first: f32, rest: f32) -> Vec<String> {
    let (mut lines, mut cur, mut limit) = (vec![], String::new(), first);
    for word in s.split_whitespace() {
        let cand = if cur.is_empty() { word.to_string() } else { format!("{cur} {word}") };
        if cur.is_empty() || text_width(&cand, size, false) <= limit { cur = cand; }
        else { lines.push(std::mem::replace(&mut cur, word.to_string())); limit = rest; }
    }
    if !cur.is_empty() || lines.is_empty() { lines.push(cur); }
    lines
}

/// A PDF string literal in WinAnsi: Latin-1 as is, a few typographic marks folded, the rest a "?".
fn literal(s: &str) -> String {
    let mut out = String::from("(");
    for ch in s.chars() {
        let b = match ch {
            ' '..='~' => ch as u8,
            '\u{a0}'..='\u{ff}' => ch as u32 as u8,
            '’' | '‘' => b'\'',
            '“' | '”' => b'"',
            '–' | '—' | '−' => b'-',
            _ => b'?',
        };
        match b {
            b'(' | b')' | b'\\' => { out.push('\\'); out.push(b as char); }
            0x20..=0x7e => out.push(b as char),
            _ => out.push_str(&format!("\\{b:03o}")),
        }
    }
    out + ")"
}

fn text_op(x: f32, y: f32, size: f32, bold: bool, s: &str) -> String {
    format!("BT /F{} {size} Tf {x:.1} {y:.1} Td {} Tj ET\n", if bold { 2 } else { 1 }, literal(s))
}

fn circle_op(cx: f32, cy: f32, r: f32, fill: bool) -> String {
    let k = 0.5523 * r;
    format!("{:.1} {:.1} m {:.1} {:.1} {:.1} {:.1} {:.1} {:.1} c {:.1} {:.1} {:.1} {:.1} {:.1} {:.1} c {:.1} {:.1} {:.1} {:.1} {:.1} {:.1} c {:.1} {:.1} {:.1} {:.1} {:.1} {:.1} c {}\n",
        cx + r, cy, cx + r, cy + k, cx + k, cy + r, cx, cy + r, cx - k, cy + r, cx - r, cy + k, cx - r, cy,
        cx - r, cy - k, cx - k, cy - r, cx, cy - r, cx + k, cy - r, cx + r, cy - k, cx + r, cy, if fill { "f" } else { "S" })
}

/// The pages being drawn, and where the flowing text goes next (`y` is the top of the next line).
/// `join` is the bottom of the left column: text flowing beside it goes full width from there (0: no column).
struct Sheet { pages: Vec<String>, x: f32, y: f32, w: f32, join: f32 }

impl Sheet {
    fn out(&mut self, s: &str) { self.pages.last_mut().unwrap().push_str(s); }
    fn text(&mut self, x: f32, y: f32, size: f32, bold: bool, s: &str) { self.out(&text_op(x, y, size, bold, s)); }
    fn centered(&mut self, x: f32, w: f32, y: f32, size: f32, bold: bool, s: &str) {
        self.text(x + (w - text_width(s, size, bold)) / 2.0, y, size, bold, s);
    }
    fn line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, w: f32) { self.out(&format!("{w} w {x1:.1} {y1:.1} m {x2:.1} {y2:.1} l S\n")); }
    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32) { self.out(&format!("0.7 w {x:.1} {y:.1} {w:.1} {h:.1} re S\n")); }
    fn gray(&mut self, g: f32) { self.out(&format!("{g} g {g} G\n")); }
    /// A proficiency dot: empty, filled, or filled with a ring (expertise).
    fn pip(&mut self, cx: f32, cy: f32, level: u8) {
        self.out(&format!("0.7 w\n{}", circle_op(cx, cy, 3.2, level > 0)));
        if level > 1 { self.out(&circle_op(cx, cy, 5.0, false)); }
    }

    /// Start a new page when `h` points do not fit; flowing text goes back to the full width.
    fn need(&mut self, h: f32) {
        if self.join > 0.0 && self.y - h < self.join { self.rejoin(); }
        if self.y - h < FOOT {
            self.pages.push(String::new());
            (self.x, self.y, self.w, self.join) = (M, H - M, W - 2.0 * M, 0.0);
        }
    }

    /// Leave the column beside the left one: full width, below whichever column ends lower.
    fn rejoin(&mut self) {
        self.y = self.y.min(if self.join > 0.0 { self.join } else { self.y });
        (self.x, self.w, self.join) = (M, W - 2.0 * M, 0.0);
    }

    fn heading(&mut self, t: &str) {
        self.need(40.0);
        self.y -= 8.0;
        self.text(self.x, self.y - 8.0, 9.0, true, &t.to_uppercase());
        self.line(self.x, self.y - 12.0, self.x + self.w, self.y - 12.0, 0.8);
        self.y -= 18.0;
    }

    /// A paragraph with a bold lead ("Rage: ..."), wrapped over the pages.
    fn entry(&mut self, lead: &str, body: &str, size: f32) {
        let lead = if lead.is_empty() { String::new() } else { format!("{lead}: ") };
        let lw = text_width(&lead, size, true) + if lead.is_empty() { 0.0 } else { 2.0 };
        let lh = size * 1.28;
        for (i, l) in wrap(body, size, self.w - lw, self.w).iter().enumerate() {
            self.need(lh);
            let (x, y) = (self.x, self.y - size);
            if i == 0 { self.text(x, y, size, true, &lead); }
            self.text(if i == 0 { x + lw } else { x }, y, size, false, l);
            self.y -= lh;
        }
        self.y -= 2.0;
    }

    /// A row of `n` empty boxes to tick.
    fn boxes(&mut self, x: f32, y: f32, n: u32) {
        for i in 0..n { self.rect(x + i as f32 * 10.0, y, 7.0, 7.0); }
    }
}

fn speed(c: &Character) -> String {
    c.info.iter().find_map(|l| l.strip_prefix("Speed ")).unwrap_or("30 ft").to_string()
}

fn header(s: &mut Sheet, c: &Character) {
    s.text(M, s.y - 20.0, 22.0, true, &c.name);
    s.y -= 28.0;
    let who = [c.race.as_str(), &c.title(), c.background.as_str()].iter().filter(|t| !t.is_empty()).map(|t| t.to_string()).collect::<Vec<_>>().join("  |  ");
    s.text(M, s.y - 9.0, 10.0, false, &who);
    s.y -= 16.0;
    s.line(M, s.y, W - M, s.y, 1.5);
    s.y -= 10.0;
}

/// Armor class, initiative, speed, proficiency, passive Perception and maximum hit points, then the paper fields.
fn stats(s: &mut Sheet, c: &Character) {
    let pp = 10 + c.check_mod("Perception");
    let cells = [
        ("ARMOR CLASS", c.ac().map_or("-".into(), |a| a.to_string())), ("INITIATIVE", format!("{:+}", c.check_mod("DEX"))),
        ("SPEED", speed(c)), ("PROFICIENCY", format!("{:+}", c.prof())),
        ("PASSIVE PERCEPTION", pp.to_string()), ("HIT POINT MAX", c.hp_max().map_or("-".into(), |h| h.to_string())),
    ];
    let bw = (W - 2.0 * M - 5.0 * 6.0) / 6.0;
    for (i, (label, v)) in cells.iter().enumerate() {
        let x = M + i as f32 * (bw + 6.0);
        s.rect(x, s.y - 40.0, bw, 40.0);
        s.centered(x, bw, s.y - 10.0, 5.5, true, label);
        s.centered(x, bw, s.y - 32.0, 17.0, true, v);
    }
    s.y -= 54.0;
    // fields to fill in at the table
    let y = s.y - 8.0;
    s.text(M, y, 8.0, true, "Current HP");
    s.line(M + 48.0, y - 2.0, M + 110.0, y - 2.0, 0.5);
    s.text(M + 124.0, y, 8.0, true, "Temp HP");
    s.line(M + 163.0, y - 2.0, M + 220.0, y - 2.0, 0.5);
    s.text(M + 238.0, y, 8.0, true, "Death saves");
    for (k, label) in ["success", "failure"].iter().enumerate() {
        let x0 = M + 290.0 + k as f32 * 100.0;
        s.text(x0, y, 7.0, false, label);
        for i in 0..3 { s.out(&format!("0.7 w\n{}", circle_op(x0 + 36.0 + i as f32 * 10.0, y + 2.5, 3.2, false))); }
    }
    s.text(W - M - 56.0, y, 8.0, true, "Inspiration");
    s.rect(W - M - 7.0, y - 2.0, 7.0, 7.0);
    s.y -= 20.0;
}

/// The left column: abilities, saving throws and skills, at the top of the first page.
fn left_column(s: &mut Sheet, c: &Character) -> f32 {
    let (x, mut y) = (M, s.y);
    let abs = c.ability_names();
    let title = |s: &mut Sheet, y: &mut f32, t: &str| {
        s.text(x, *y - 8.0, 9.0, true, t);
        s.line(x, *y - 12.0, x + LEFT, *y - 12.0, 0.8);
        *y -= 18.0;
    };
    title(s, &mut y, "ABILITIES");
    for a in &abs {
        s.rect(x, y - 28.0, LEFT, 28.0);
        s.text(x + 8.0, y - 13.0, 11.0, true, a);
        s.text(x + 8.0, y - 23.0, 6.5, false, FULL[ABILITIES.iter().position(|n| n == a).unwrap()]);
        s.centered(x + 40.0, 60.0, y - 21.0, 17.0, true, &format!("{:+}", c.md(a)));
        s.rect(x + 112.0, y - 22.0, 36.0, 16.0);
        s.centered(x + 112.0, 36.0, y - 18.0, 10.0, false, &c.abilities[*a].to_string());
        y -= 32.0;
    }
    y -= 4.0;
    title(s, &mut y, "SAVING THROWS");
    let row = |s: &mut Sheet, y: &mut f32, name: &str, label: &str| {
        let lvl = if c.expertise.iter().any(|e| e == name) { 2 } else { u8::from(c.proficient(name)) };
        s.pip(x + 6.0, *y - 5.0, lvl);
        s.text(x + 16.0, *y - 8.0, 8.5, true, &format!("{:+}", c.check_mod(name)));
        s.text(x + 38.0, *y - 8.0, 8.5, false, label);
        *y -= 12.5;
    };
    for a in &abs { row(s, &mut y, &format!("{a} save"), FULL[ABILITIES.iter().position(|n| n == a).unwrap()]); }
    y -= 4.0;
    title(s, &mut y, "SKILLS");
    for (n, a) in SKILLS.iter().filter(|(_, a)| c.abilities.contains_key(*a)) { row(s, &mut y, n, &format!("{n} ({a})")); }
    y
}

fn attacks(s: &mut Sheet, c: &Character) {
    let (acts, togs) = (c.shown_actions(), c.shown_toggles());
    if acts.is_empty() && togs.is_empty() { return; }
    s.heading("Attacks and actions");
    let size = 8.0;
    let notes_w = s.w - 222.0;
    let x0 = s.x;
    for (label, dx) in [("NAME", 0.0), ("ATK", 108.0), ("DAMAGE", 142.0), ("NOTES", 222.0)] { s.text(x0 + dx, s.y - 7.0, 6.5, true, label); }
    s.y -= 11.0;
    let mut rows: Vec<(String, String, String, String)> = acts.iter().map(|&i| {
        let a = &c.actions[i];
        let (hit, dmg) = c.attack_text(a);
        let uses = a.uses.as_ref().and_then(|u| c.def(u)).filter(|r| r.name != a.name).map(|r| format!(" (uses {})", r.name)).unwrap_or_default();
        (a.name.clone(), hit, if dmg.is_empty() { dmg } else { format!("{dmg} {}", a.dmg_type) }, format!("{}{uses}", c.fmt(&a.desc)))
    }).collect();
    rows.extend(togs.iter().map(|&t| (format!("{} (on/off)", c.toggles[t].name), String::new(), String::new(), c.fmt(&c.toggles[t].desc))));
    for (name, hit, dmg, notes) in rows {
        let (nl, dl, tl) = (wrap(&name, size, 104.0, 104.0), wrap(&dmg, 7.5, 76.0, 76.0), wrap(&notes, 7.0, notes_w, notes_w));
        let h = nl.len().max(dl.len()).max(tl.len()) as f32 * 9.5 + 3.0;
        s.need(h);
        let x0 = s.x; // a page break puts the table at the full width
        for (i, l) in nl.iter().enumerate() { s.text(x0, s.y - 8.0 - i as f32 * 9.5, size, i == 0, l); }
        s.text(x0 + 108.0, s.y - 8.0, size, true, &hit);
        for (i, l) in dl.iter().filter(|_| !dmg.is_empty()).enumerate() { s.text(x0 + 142.0, s.y - 8.0 - i as f32 * 9.5, 7.5, false, l); }
        for (i, l) in tl.iter().enumerate() { s.text(x0 + 222.0, s.y - 8.0 - i as f32 * 9.5, 7.0, false, l); }
        s.line(x0, s.y - h + 1.0, x0 + s.w, s.y - h + 1.0, 0.2);
        s.y -= h;
    }
}

fn resources(s: &mut Sheet, c: &Character) {
    let list: Vec<&ResourceDef> = c.resources.iter()
        .filter(|d| !(d.id.starts_with("slot") || d.id == "pact" || d.id == "beast_hp") && c.res_max(&d.id) != Some(0)).collect();
    if list.is_empty() { return; }
    s.heading("Resources");
    for def in list {
        s.need(13.0);
        let die = c.res_die(&def.id).map(|x| format!(" d{x}")).unwrap_or_default();
        let (x, y) = (s.x, s.y - 9.0);
        s.text(x, y, 8.5, true, &format!("{}{die}", def.name));
        let when = match def.recharge.as_str() { "short" => "short rest", "long" => "long rest", _ => "" };
        s.gray(0.4);
        s.text(x + 150.0, y, 7.0, false, when);
        s.gray(0.0);
        match c.res_max(&def.id) {
            Some(max) if def.pips && max <= 12 => s.boxes(x + 200.0, y - 1.0, max),
            Some(max) => { s.text(x + 200.0, y, 8.5, false, &format!("max {max}   now")); s.line(x + 252.0, y - 1.5, x + 290.0, y - 1.5, 0.5); }
            None => { s.text(x + 200.0, y, 8.5, false, "now"); s.line(x + 216.0, y - 1.5, x + 254.0, y - 1.5, 0.5); s.text(x + 222.0, y, 8.5, true, &c.res.get(&def.id).copied().unwrap_or(0).to_string()); }
        }
        s.y -= 13.0;
    }
}

fn slots(s: &mut Sheet, c: &Character) {
    let rows = c.slot_rows();
    if rows.is_empty() { return; }
    s.heading("Spell slots");
    for (label, _, max) in rows {
        s.need(13.0);
        let (x, y) = (s.x, s.y - 9.0);
        s.text(x, y, 8.5, true, &label.replace('L', "Level "));
        s.boxes(x + 60.0, y - 1.0, max);
        s.y -= 13.0;
    }
}

fn spells(s: &mut Sheet, c: &Character) {
    if !c.has_spells() { return; }
    s.heading(&format!("Spells  -  {}", c.spell_dcs()));
    let rows = c.spell_rows();
    let mut last = u32::MAX;
    for sp in &rows {
        if sp.level != last {
            last = sp.level;
            s.need(24.0);
            s.y -= 3.0;
            s.text(s.x, s.y - 8.0, 8.0, true, &if sp.level == 0 { "CANTRIPS".to_string() } else { format!("LEVEL {}", sp.level) });
            s.y -= 11.0;
        }
        let tags = format!("{}{}", if sp.conc { " (concentration)" } else { "" }, if sp.ritual { " (ritual)" } else { "" });
        s.entry(&sp.name, &format!("{}; {}; {}; {}; {}{tags}", sp.school, sp.time, sp.range, sp.comp, sp.duration), 7.5);
    }
    // a spellbook holds more than what is prepared
    let book: Vec<String> = c.spells.iter().filter(|n| !rows.iter().any(|r| r.name.eq_ignore_ascii_case(n))).cloned().collect();
    if !book.is_empty() {
        s.y -= 3.0;
        s.entry("Also in the spellbook", &book.join(", "), 8.0);
    }
}

fn inventory(s: &mut Sheet, c: &Character) {
    s.heading("Inventory");
    for g in &c.inventory {
        let w = if g.weight > 0.0 { format!(", {} lb", g.weight * g.qty as f32) } else { String::new() };
        s.entry("", &format!("{}{} x{}{w}", g.name, if g.attuned { " (attuned)" } else { "" }, g.qty), 8.5);
    }
    let lb: f32 = c.inventory.iter().map(|g| g.weight * g.qty as f32).sum();
    if lb > 0.0 { s.entry("Carried", &format!("{lb} lb"), 8.5); }
    s.need(14.0);
    let mut x = s.x;
    for ((name, _), n) in COINS.iter().zip(c.coins) {
        s.text(x, s.y - 9.0, 8.5, true, &name.to_uppercase());
        s.text(x + 18.0, s.y - 9.0, 8.5, false, &n.to_string());
        s.line(x + 16.0, s.y - 11.0, x + 52.0, s.y - 11.0, 0.4);
        x += 70.0;
    }
    s.y -= 16.0;
}

pub fn sheet(c: &Character) -> Vec<u8> {
    let mut s = Sheet { pages: vec![String::new()], x: M, y: H - M, w: W - 2.0 * M, join: 0.0 };
    header(&mut s, c);
    stats(&mut s, c);
    s.join = left_column(&mut s, c);
    (s.x, s.w) = (M + LEFT + GAP, W - 2.0 * M - LEFT - GAP);
    attacks(&mut s, c);
    resources(&mut s, c);
    slots(&mut s, c);
    // the features fill the column beside the abilities, then go full width
    s.heading("Features");
    for f in &c.features { s.entry(&f.name, &c.fmt(&f.desc), 8.5); }
    s.rejoin();
    if !c.maneuvers.is_empty() {
        s.heading("Maneuvers");
        for m in &c.maneuvers { s.entry(m, maneuvers().get(m).map_or("", |d| d.as_str()), 8.5); }
    }
    spells(&mut s, c);
    if !c.info.is_empty() {
        s.heading("Proficiencies and equipment");
        for l in &c.info { s.entry("", l, 8.5); }
    }
    inventory(&mut s, c);
    s.heading("Notes");
    for n in &c.journal { s.entry("", n, 8.5); }
    // ruled lines down to the end of the last page
    s.gray(0.75);
    while s.y - 18.0 > FOOT {
        s.y -= 18.0;
        s.line(M, s.y, W - M, s.y, 0.4);
    }
    s.gray(0.0);
    finish(s.pages, c)
}

/// Page footers, then the file: catalog, page tree, two fonts, a page and its content per page, the info dictionary.
fn finish(mut pages: Vec<String>, c: &Character) -> Vec<u8> {
    let total = pages.len();
    for (i, p) in pages.iter_mut().enumerate() {
        p.push_str("0.45 g\n");
        p.push_str(&text_op(M, 24.0, 7.0, false, &format!("{} - {}", c.name, c.title())));
        p.push_str(&text_op(W - M - 90.0, 24.0, 7.0, false, &format!("page {} of {total}   dnd-tui", i + 1)));
    }
    let info = 5 + 2 * total;
    let mut objs: Vec<String> = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".into(),
        format!("<< /Type /Pages /Kids [{}] /Count {total} >>", (0..total).map(|i| format!("{} 0 R", 5 + 2 * i)).collect::<Vec<_>>().join(" ")),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".into(),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding >>".into(),
    ];
    for (i, p) in pages.iter().enumerate() {
        objs.push(format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {W} {H}] /Resources << /Font << /F1 3 0 R /F2 4 0 R >> >> /Contents {} 0 R >>", 6 + 2 * i));
        objs.push(format!("<< /Length {} >>\nstream\n{p}endstream", p.len()));
    }
    objs.push(format!("<< /Title {} /Producer (dnd-tui) >>", literal(&c.name)));
    let mut out = b"%PDF-1.4\n".to_vec();
    let mut at = vec![];
    for (i, o) in objs.iter().enumerate() {
        at.push(out.len());
        out.extend(format!("{} 0 obj\n{o}\nendobj\n", i + 1).bytes());
    }
    let xref = out.len();
    out.extend(format!("xref\n0 {}\n0000000000 65535 f \n", objs.len() + 1).bytes());
    for a in at { out.extend(format!("{a:010} 00000 n \n").bytes()); }
    out.extend(format!("trailer\n<< /Size {} /Root 1 0 R /Info {info} 0 R >>\nstartxref\n{xref}\n%%EOF\n", objs.len() + 1).bytes());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bundled() -> Vec<Character> {
        BUNDLED.iter().map(|(_, json)| { let mut c: Character = serde_json::from_str(json).unwrap(); c.init(); c }).collect()
    }

    #[test]
    fn the_file_is_well_formed() {
        for c in bundled() {
            let pdf = sheet(&c);
            let text = String::from_utf8_lossy(&pdf);
            assert!(text.starts_with("%PDF-1.4\n") && text.ends_with("%%EOF\n"));
            // every xref entry points at its object
            let xref = text.rfind("xref\n0 ").unwrap();
            for (i, line) in text[xref..].lines().skip(3).take_while(|l| l.ends_with(" n ")).enumerate() {
                let at: usize = line[..10].parse().unwrap();
                assert!(text[at..].starts_with(&format!("{} 0 obj", i + 1)), "object {}", i + 1);
            }
            assert!(text.contains(&literal(&c.name)));
        }
    }

    #[test]
    fn the_sheet_carries_the_numbers() {
        let c = &bundled()[0];
        let text = String::from_utf8_lossy(&sheet(c)).to_string();
        for want in ["(28)", "(+3)", "Second Wind", "Superiority"] { assert!(text.contains(want), "{want}"); }
    }

    #[test]
    fn text_is_escaped_and_folded_to_winansi() {
        assert_eq!(literal("a (b) \\ é “x” …"), "(a \\(b\\) \\\\ \\351 \"x\" ?)");
    }

    #[test]
    fn wrapping_respects_the_width() {
        let lines = wrap("the quick brown fox jumps over the lazy dog", 10.0, 60.0, 80.0);
        assert!(lines.len() > 2);
        assert!(lines.iter().skip(1).all(|l| text_width(l, 10.0, false) <= 80.0));
    }
}

//! The state of the App as JSON for the web page, which draws it as HTML and sends `Cmd`s back.
//! Texts come from the same helpers the terminal UI uses, so both show the same thing.
use ratatui::text::Line;
use serde_json::{json, Value};

use crate::app::{Adv, App, Item, Modal, Tone};
use crate::model::maneuvers;
use crate::ui;

/// A line as spans of text with a colour name ("green", "darkgray"...) and boldness.
fn spans(l: &Line) -> Value {
    l.spans.iter().map(|s| {
        let st = l.style.patch(s.style);
        json!({ "t": s.content, "c": st.fg.map(|c| format!("{c:?}").to_lowercase()), "b": st.add_modifier.contains(ratatui::style::Modifier::BOLD) })
    }).collect()
}

fn lines(v: &[Line]) -> Value { v.iter().map(spans).collect() }

fn tone(t: Tone) -> &'static str {
    match t { Tone::Norm => "norm", Tone::Good => "good", Tone::Warn => "warn", Tone::Dim => "dim" }
}

fn modal(app: &App) -> Value {
    match &app.modal {
        None => Value::Null,
        Some(Modal::List { title, items, sel, cancel, .. }) =>
            json!({ "kind": "list", "title": title, "items": items, "sel": sel, "cancel": cancel }),
        Some(Modal::Multi { title, items, sel, on, limit, .. }) =>
            json!({ "kind": "multi", "title": title, "items": items, "sel": sel, "on": on, "limit": limit }),
        Some(Modal::Input { title, buf, numeric, .. }) =>
            json!({ "kind": "input", "title": title, "buf": buf, "numeric": numeric }),
    }
}

pub fn view(app: &App) -> Value {
    let log: Vec<Value> = app.log.iter().rev().take(60).map(|e| json!({ "t": e.0, "tone": tone(e.1) })).collect();
    let base = json!({ "picking": app.picking, "modal": modal(app), "query": app.query, "log": log });
    if app.picking {
        let sheets: Vec<Value> = app.sheets.iter().map(|(_, c)| json!({ "name": c.name, "title": c.title() })).collect();
        return merge(base, json!({ "sheets": sheets, "psel": app.psel }));
    }
    let c = &app.ch;
    let tabs: Vec<Value> = app.tabs().iter().map(|(t, n)| json!({ "id": t, "name": n })).collect();
    let commands: Vec<Value> = ui::footer(app).into_iter().map(|(k, l, cmd)| json!({ "key": k, "label": l, "cmd": cmd })).collect();
    let actions: Vec<Value> = app.visible().into_iter().map(|it| match it {
        Item::Toggle(t) => { let tg = &c.toggles[t]; json!({ "toggle": true, "name": tg.name, "on": c.active.contains(&tg.name) }) }
        Item::Action(i) => {
            let a = &c.actions[i];
            let (hit, dmg) = c.attack_text(a);
            let left = a.uses.as_ref().map(|u| c.res.get(u).copied().unwrap_or(0));
            json!({ "toggle": false, "name": a.name, "hit": hit.trim(), "dmg": dmg.trim(), "desc": c.fmt(&a.desc), "uses": left, "ready": left != Some(0) })
        }
    }).collect();
    let checks: Vec<Value> = c.checks().into_iter().map(|(n, m)| json!({ "name": n, "mod": m, "prof": c.proficient(&n) })).collect();
    let maneuver = c.maneuver_pool.as_ref().map(|pool| json!({
        "die": c.res_die(pool).unwrap_or(8), "left": c.res.get(pool).copied().unwrap_or(0),
        "list": c.maneuvers.iter().map(|m| json!({ "name": m, "desc": maneuvers().get(m) })).collect::<Vec<_>>(),
    }));
    let rows = c.spell_rows();
    let spells = json!({
        "slots": ui::slots_line(c).map(|l| l.to_string()),
        "dcs": c.spell_dcs(),
        "prepared": c.casting().filter(|k| k.mode != "known").map(|_| format!("Prepared {}/{}", c.prepared.len(), c.prepared_max())),
        "rows": rows.iter().map(|sp| json!({ "name": sp.name, "level": sp.level, "conc": sp.conc, "ritual": sp.ritual, "usable": ui::spell_usable(c, sp) })).collect::<Vec<_>>(),
        "sel": app.ssel,
        "detail": rows.get(app.ssel).map(|sp| lines(&ui::spell_detail(c, sp))),
    });
    let status = ui::status_lines(app);
    merge(base, json!({
        "tab": app.tab, "tabs": tabs, "commands": commands,
        "name": c.name, "title": c.title(),
        "hp": c.hp, "hp_max": c.hp_max(), "temp_hp": c.temp_hp, "ac": c.ac(), "init": c.check_mod("DEX"), "prof": c.prof(),
        "status": lines(&status[2.min(status.len())..]),
        "adv": match app.adv { Adv::Normal => "normal", Adv::Adv => "advantage", Adv::Dis => "disadvantage" },
        "actions": actions, "sel": app.sel, "checks": checks, "maneuver": maneuver, "spells": spells,
        "character": lines(&ui::character_lines(app)),
        "journal": c.journal, "jsel": app.jsel,
    }))
}

fn merge(mut a: Value, b: Value) -> Value {
    if let (Value::Object(a), Value::Object(b)) = (&mut a, b) { a.extend(b); }
    a
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Cmd;

    #[test]
    fn every_bundled_sheet_renders_with_and_without_a_question() {
        // a fresh App per sheet: going back to the list would save the sheet to disk
        let sheets = || crate::model::BUNDLED.iter().map(|(n, j)| (n.into(), serde_json::from_str(j).unwrap())).collect();
        assert_eq!(view(&App::new_with(sheets()))["sheets"].as_array().map(|v| v.len()), Some(crate::model::BUNDLED.len()));
        for i in 0..crate::model::BUNDLED.len() {
            let mut app = App::new_with(sheets());
            app.exec(Cmd::Pick(i));
            let v = view(&app);
            assert!(v["actions"].as_array().is_some_and(|a| !a.is_empty()) && v["hp"].is_number());
            app.exec(Cmd::Damage);
            assert_eq!(view(&app)["modal"]["kind"], "input");
            app.submit("3");
            assert!(view(&app)["modal"].is_null());
        }
        // commands go to the page and come back as JSON
        let cmd: Cmd = serde_json::from_value(json!({ "Roll": "DEX save" })).unwrap();
        assert!(cmd == Cmd::Roll("DEX save".into()));
    }
}

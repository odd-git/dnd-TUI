//! Turns the choices of the creation wizard into a Character. Pure: no UI, no I/O.

use std::collections::BTreeMap;

use crate::lists::{self, Weapon};
use crate::model::{ActionDef, Character, Feature, ResourceDef, ABILITIES};

#[derive(Default, Clone)]
pub struct Draft {
    pub name: String,
    pub lineage: String,
    pub sub: Option<String>,
    pub class: String,
    pub subclass: Option<String>,
    pub level: u32,
    pub background: String,
    pub base: BTreeMap<String, i32>,
    pub asi_picks: Vec<String>,   // one ability per "choose" slot of the lineage, in order
    pub class_skills: Vec<String>,
    pub free_skills: Vec<String>, // lineage/background "of your choice"
    pub feats: Vec<String>,
    pub feat_picks: Vec<String>,  // ability raised by each feat, "" when it has no choice
    pub style: Option<String>,
    pub maneuvers: Vec<String>,
    pub weapons: Vec<String>,
    pub armor: Option<String>,
    pub shield: bool,
    pub languages: Vec<String>,
    pub tools: Vec<String>,
}

fn push_unique(v: &mut Vec<String>, items: impl IntoIterator<Item = String>) {
    for i in items { if !v.contains(&i) { v.push(i); } }
}

fn put_feature(v: &mut Vec<Feature>, f: &Feature) {
    match v.iter_mut().find(|x| x.name == f.name) {
        Some(x) => x.desc = f.desc.clone(),
        None => v.push(f.clone()),
    }
}

fn weapon_actions(w: &Weapon, c: &Character, style: Option<&str>) -> Vec<ActionDef> {
    // finesse: whichever of STR/DEX is higher
    let ability = if w.ranged || (w.finesse && c.md("DEX") >= c.md("STR")) { "DEX" } else { "STR" };
    let mk = |name: String, desc: String, bonus: i32| ActionDef {
        name, desc, ability: Some(ability.into()), dice: Some(w.dice), dmg_ability: true, bonus,
        dmg_type: w.dmg_type.clone(), ..Default::default()
    };
    let thrown_bonus = if style == Some("thrown") { 2 } else { 0 };
    let mut out = vec![];
    if w.ranged {
        let bonus = if w.thrown.is_some() { thrown_bonus } else { 0 };
        out.push(mk(w.name.clone(), w.props.clone(), bonus));
    } else {
        let duel = if style == Some("dueling") && !w.two_handed { 2 } else { 0 };
        out.push(mk(w.name.clone(), w.props.clone(), duel));
        if let Some(range) = &w.thrown {
            out.push(mk(format!("Thrown {}", w.name.to_lowercase()), format!("{range} ft"), thrown_bonus));
        }
    }
    out
}

pub fn build(d: &Draft) -> Character {
    let lin = lists::lineages().iter().find(|l| l.name == d.lineage).expect("lineage");
    let sub_i = d.sub.as_ref().and_then(|s| lin.subs.iter().position(|x| &x.name == s));
    let eff = lin.effective(sub_i);
    let class = lists::class_data(&d.class).expect("class");
    let sub = d.subclass.as_ref().and_then(|s| class.subclasses.iter().find(|x| &x.name == s));
    let bg = lists::backgrounds().iter().find(|b| b.name == d.background);

    // ability scores: base + lineage fixed + lineage picks + feat picks
    let mut abilities: BTreeMap<String, i32> = ABILITIES.iter().map(|a| (a.to_string(), d.base.get(*a).copied().unwrap_or(10))).collect();
    for (a, v) in &eff.fixed { *abilities.entry(a.clone()).or_insert(10) += v; }
    let amounts = eff.choose.iter().flat_map(|c| std::iter::repeat(c.amount).take(c.n as usize));
    for (a, amt) in d.asi_picks.iter().zip(amounts) { *abilities.entry(a.clone()).or_insert(10) += amt; }
    for (feat, pick) in d.feats.iter().zip(d.feat_picks.iter().map(Some).chain(std::iter::repeat(None))) {
        let amt = lists::feats().iter().find(|f| &f.name == feat).and_then(|f| f.asi.as_ref()).map_or(0, |a| a.amount);
        if let Some(p) = pick.filter(|p| !p.is_empty()) { *abilities.entry(p.clone()).or_insert(10) += amt; }
    }

    let mut skills = vec![];
    push_unique(&mut skills, bg.iter().flat_map(|b| b.skills.clone()));
    push_unique(&mut skills, eff.skills.clone());
    push_unique(&mut skills, d.class_skills.clone());
    push_unique(&mut skills, d.free_skills.clone());

    let mut features: Vec<Feature> = vec![];
    for t in &eff.traits { put_feature(&mut features, t); }
    if let Some(f) = bg.and_then(|b| b.feature.as_ref()) { put_feature(&mut features, &Feature { name: f.clone(), desc: format!("{} background feature", d.background) }); }
    if let Some(s) = d.style.as_ref().and_then(|s| class.styles.iter().find(|x| &x.name == s)) {
        put_feature(&mut features, &Feature { name: "Fighting Style".into(), desc: format!("{}: {}", s.name, s.desc) });
    }
    for f in &d.feats {
        let desc = lists::feats().iter().find(|x| &x.name == f).map(|x| x.desc.clone()).unwrap_or_default();
        put_feature(&mut features, &Feature { name: format!("Feat: {f}"), desc });
    }
    for (_, fs) in class.features.range(..=d.level) { for f in fs { put_feature(&mut features, f); } }
    if let Some(s) = sub { for (_, fs) in s.features.range(..=d.level) { for f in fs { put_feature(&mut features, f); } } }

    let mut resources: Vec<ResourceDef> = class.resources.clone();
    if let Some(s) = sub { resources.extend(s.resources.clone()); }
    resources.push(ResourceDef { id: "hit_dice".into(), name: "Hit Dice".into(), recharge: String::new(), max: Some(crate::model::Max::Kw("level".into())), start: None, pips: false, die: None });
    let mut actions: Vec<ActionDef> = class.actions.clone();
    if let Some(s) = sub { actions.extend(s.actions.clone()); }

    let title = match sub { Some(s) => format!("{} {{level}} ({})", class.name, s.name), None => format!("{} {{level}}", class.name) };
    let mut c = Character {
        name: d.name.clone(),
        race: match &d.sub { Some(s) => format!("{s} {}", d.lineage), None => d.lineage.clone() },
        background: d.background.clone(),
        class_title: title,
        level: d.level,
        hit_die: Some(class.hit_die),
        abilities,
        save_profs: class.saves.clone(),
        skills,
        ac_base: None,
        ac_dex_max: None,
        spell_ability: None,
        progression: Some(match sub { Some(s) => format!("{}/{}", class.name, s.name), None => class.name.clone() }),
        maneuver_pool: sub.and_then(|s| s.maneuver_pool.clone()),
        maneuvers: d.maneuvers.clone(),
        resources,
        actions: vec![],
        toggles: vec![],
        features,
        info: vec![],
        journal: vec![],
        hp: None,
        death: [0; 2],
        conditions: vec![],
        active: vec![],
        res: BTreeMap::new(),
    };

    // armor class
    let armor = d.armor.as_ref().and_then(|a| lists::armors().iter().find(|x| &x.name == a));
    let defense = d.style.as_ref().and_then(|s| class.styles.iter().find(|x| &x.name == s)).and_then(|s| s.effect.as_deref()) == Some("defense");
    let (base, cap) = match armor {
        Some(a) => (a.ac + i32::from(defense), match a.dex.as_str() { "none" => Some(0), "max2" => Some(2), _ => None }),
        None => (10, None),
    };
    c.ac_base = Some(base + if d.shield { 2 } else { 0 });
    c.ac_dex_max = cap;

    // attacks from the chosen weapons, then the class actions
    let style = d.style.as_ref().and_then(|s| class.styles.iter().find(|x| &x.name == s)).and_then(|s| s.effect.as_deref());
    for w in d.weapons.iter().filter_map(|n| lists::weapons().iter().find(|w| &w.name == n)) {
        c.actions.extend(weapon_actions(w, &c, style));
    }
    c.actions.extend(actions);

    let mut info = vec![];
    let mut tools = class.tools.clone();
    push_unique(&mut tools, d.tools.clone());
    if !tools.is_empty() { info.push(format!("Tools: {}", tools.join(", "))); }
    info.push(format!("Armor: {} | Weapons: {}", class.armor.join(", "), class.weapons.join(", ")));
    let mut langs = eff.languages.clone();
    push_unique(&mut langs, d.languages.clone());
    info.push(format!("Languages: {}", langs.join(", ")));
    info.push(format!("Speed {} ft", eff.speed.unwrap_or(30)));
    let mut gear: Vec<String> = d.weapons.clone();
    gear.extend(d.armor.clone());
    if d.shield { gear.push("Shield".into()); }
    if !gear.is_empty() { info.push(format!("Gear: {}", gear.join(", "))); }
    c.info = info;

    c.init();
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rowan() -> Draft {
        let s = |x: &str| x.to_string();
        Draft {
            name: s("Rowan Ashford"), lineage: s("Human"), sub: Some(s("Variant")),
            class: s("Fighter"), subclass: Some(s("Battle Master")), level: 3, background: s("Noble"),
            base: [("STR", 9), ("DEX", 15), ("CON", 13), ("INT", 15), ("WIS", 11), ("CHA", 10)].iter().map(|(a, v)| (a.to_string(), *v)).collect(),
            asi_picks: vec![s("DEX"), s("CON")],
            class_skills: vec![s("Insight"), s("Perception")],
            free_skills: vec![s("Investigation")],
            feats: vec![s("Fey Touched")], feat_picks: vec![s("INT")],
            style: Some(s("Thrown Weapon Fighting")),
            maneuvers: vec![s("Maneuvering Attack"), s("Commander's Strike"), s("Distracting Strike")],
            weapons: vec![s("Dagger")], armor: Some(s("Leather")),
            ..Default::default()
        }
    }

    #[test]
    fn rebuilding_rowan_matches_his_sheet() {
        let c = build(&rowan());
        let paper: Character = serde_json::from_str(crate::model::BUNDLED[0].1).unwrap();
        assert_eq!(c.abilities, paper.abilities);
        assert_eq!(c.hp_max(), Some(28));
        assert_eq!(c.dc("INT"), 13);
        assert_eq!(c.ac(), Some(14));
        assert_eq!(c.res["superiority"], 4);
        assert_eq!(c.res_die("superiority"), Some(8));
        assert_eq!(c.res_max("action_surge"), Some(1));
        let mut a = c.skills.clone(); a.sort();
        let mut b = paper.skills.clone(); b.sort();
        assert_eq!(a, b);
        assert_eq!(c.save_profs, paper.save_profs);
        assert!(c.actions.iter().any(|x| x.name == "Thrown dagger" && x.bonus == 2));
    }

    #[test]
    fn every_list_loads_and_resolves() {
        for l in lists::lineages() {
            l.effective(None);
            for i in 0..l.subs.len() { l.effective(Some(i)); }
        }
        assert!(lists::backgrounds().len() > 50 && lists::feats().len() > 50);
        assert!(!lists::weapons().is_empty() && !lists::armors().is_empty());
        for c in lists::classes() {
            for s in &c.subclasses { assert!(lists::progression(&c.name, Some(&s.name)).is_some()); }
        }
    }
}

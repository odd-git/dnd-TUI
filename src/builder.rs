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
    pub expertise: Vec<String>,
    pub cantrips: Vec<String>,
    pub spells: Vec<String>,
    pub prepared: Vec<String>,
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
        Some(x) => { x.desc = f.desc.clone(); x.url = f.url.clone(); }
        None => v.push(f.clone()),
    }
}

pub fn weapon_actions(w: &Weapon, c: &Character, style: Option<&str>) -> Vec<ActionDef> {
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

/// (AC before DEX, DEX cap) for the armor, shield and Defense style.
pub fn armor_class(armor: Option<&str>, shield: bool, defense: bool) -> (Option<i32>, Option<i32>) {
    let a = armor.and_then(|a| lists::armors().iter().find(|x| x.name == a));
    let (base, cap) = match a {
        Some(a) => (a.ac + i32::from(defense), match a.dex.as_str() { "none" => Some(0), "max2" => Some(2), _ => None }),
        None => (10, None),
    };
    (Some(base + if shield { 2 } else { 0 }), cap)
}

/// Effect tag of the fighting style written on a sheet ("thrown", "dueling", "defense"...).
pub fn style_effect(c: &Character) -> Option<&'static str> {
    let class = lists::class_data(c.progression.as_deref()?.split('/').next()?)?;
    let desc = &c.features.iter().find(|f| f.name == "Fighting Style")?.desc;
    class.styles.iter().find(|s| desc.starts_with(&s.name))?.effect.as_deref()
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
    let amounts = eff.choose.iter().flat_map(|c| std::iter::repeat_n(c.amount, c.n as usize));
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
    if let Some(f) = bg.and_then(|b| b.feature.as_ref()) { put_feature(&mut features, &Feature { name: f.clone(), desc: format!("{} background feature", d.background), url: bg.map_or(String::new(), |b| b.url.clone()) }); }
    for f in &d.feats {
        let (desc, url) = lists::feats().iter().find(|x| &x.name == f).map(|x| (x.desc.clone(), x.url.clone())).unwrap_or_default();
        put_feature(&mut features, &Feature { name: format!("Feat: {f}"), desc, url });
    }
    for (_, fs) in class.features.range(..=d.level) { for f in fs { put_feature(&mut features, f); } }
    if let Some(s) = sub { for (_, fs) in s.features.range(..=d.level) { for f in fs { put_feature(&mut features, f); } } }
    if let Some(s) = d.style.as_ref().filter(|_| d.level >= class.style_level).and_then(|s| class.styles.iter().find(|x| &x.name == s)) {
        put_feature(&mut features, &Feature { name: "Fighting Style".into(), desc: format!("{}: {}", s.name, s.desc), ..Default::default() });
    }

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
        multi: vec![],
        temp_hp: 0,
        beast: None,
        concentration: None,
        level: d.level,
        hit_die: Some(class.hit_die),
        abilities,
        save_profs: class.saves.clone(),
        skills,
        expertise: d.expertise.clone(),
        cantrips: d.cantrips.clone(),
        spells: d.spells.clone(),
        prepared: d.prepared.clone(),
        ac_base: None,
        ac_dex_max: None,
        ac_extra: class.unarmored.as_ref().map(|u| u.ability.clone()),
        ac_extra_shield: class.unarmored.as_ref().is_some_and(|u| u.shield),
        ac_armored: d.armor.is_some(),
        ac_shield: d.shield,
        spell_ability: None,
        progression: Some(match sub { Some(s) => format!("{}/{}", class.name, s.name), None => class.name.clone() }),
        maneuver_pool: sub.and_then(|s| s.maneuver_pool.clone()),
        maneuvers: d.maneuvers.clone(),
        resources,
        actions: vec![],
        toggles: class.toggles.iter().chain(sub.iter().flat_map(|s| s.toggles.iter())).cloned().collect(),
        features,
        info: vec![],
        journal: vec![],
        hp: None,
        death: [0; 2],
        conditions: vec![], exhaustion: 0, inspiration: false,
        inventory: vec![], coins: [0; 5],
        active: vec![],
        res: BTreeMap::new(),
    };

    // armor class
    let defense = d.style.as_ref().and_then(|s| class.styles.iter().find(|x| &x.name == s)).and_then(|s| s.effect.as_deref()) == Some("defense");
    (c.ac_base, c.ac_dex_max) = armor_class(d.armor.as_deref(), d.shield, defense);

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

    c.sync_spell_resources();
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
    fn armor_class_rules() {
        assert_eq!(armor_class(Some("Chain Mail"), true, false), (Some(18), Some(0)));
        assert_eq!(armor_class(Some("Halfplate"), false, true), (Some(16), Some(2)));
        assert_eq!(armor_class(None, true, true), (Some(12), None));
    }

    fn barbarian(level: u32) -> Character {
        let s = |x: &str| x.to_string();
        build(&Draft {
            name: s("Grok"), lineage: s("Human"), class: s("Barbarian"), subclass: Some(s("Berserker")), level, background: s("Soldier"),
            base: [("STR", 15), ("DEX", 13), ("CON", 13), ("INT", 8), ("WIS", 10), ("CHA", 8)].iter().map(|(a, v)| (a.to_string(), *v)).collect(),
            class_skills: vec![s("Athletics"), s("Perception")], ..Default::default()
        })
    }

    #[test]
    fn barbarian_matches_the_wikidot_table() {
        // (level, proficiency, rages, rage damage); CON 14 after the Human bonus
        for (lvl, prof, rages, dmg) in [(1, 2, 2, 2), (5, 3, 3, 2), (11, 4, 4, 3), (20, 6, 99, 4)] {
            let c = barbarian(lvl);
            assert_eq!(c.prof(), prof, "prof at {lvl}");
            assert_eq!(c.res_max("rage"), Some(rages), "rages at {lvl}");
            let mut raging = c.clone();
            raging.active.push("Rage".into());
            assert_eq!(raging.toggle_bonus(Some("STR")), dmg, "rage damage at {lvl}");
            assert_eq!(raging.toggle_bonus(Some("DEX")), 0);
        }
        assert_eq!(barbarian(1).hp_max(), Some(14)); // 12 + CON 2
        assert_eq!(barbarian(5).hp_max(), Some(14 + 4 * (7 + 2)));
        assert!(barbarian(11).features.iter().any(|f| f.name == "Relentless Rage"));
        assert!(!barbarian(5).features.iter().any(|f| f.name == "Relentless Rage"));
        assert!(barbarian(3).features.iter().any(|f| f.name == "Frenzy"));
    }

    #[test]
    fn totem_warrior_casts_its_rituals_only() {
        let mut c = barbarian(3);
        assert!(!c.has_spells());
        c.progression = Some("Barbarian/Totem Warrior".into());
        assert_eq!(c.rituals(), ["Beast Sense", "Speak with Animals"]);
        assert!(c.has_spells() && c.ritual_ok("Speak with Animals") && !c.ritual_ok("Commune with Nature"));
        c.level = 10;
        assert!(c.ritual_ok("Commune with Nature"));
        assert!(c.castable().is_empty());
    }

    fn bard(sub: &str, level: u32) -> Character {
        let s = |x: &str| x.to_string();
        build(&Draft {
            name: s("Lute"), lineage: s("Human"), class: s("Bard"), subclass: Some(s(sub)), level, background: s("Entertainer"),
            base: [("STR", 8), ("DEX", 14), ("CON", 12), ("INT", 10), ("WIS", 10), ("CHA", 15)].iter().map(|(a, v)| (a.to_string(), *v)).collect(),
            class_skills: vec![s("Insight"), s("Persuasion"), s("History")], ..Default::default()
        })
    }

    #[test]
    fn lore_rolls_the_inspiration_die() {
        let words = |c: &Character| c.actions.iter().find(|a| a.name == "Cutting Words").and_then(|a| c.dice(a));
        assert_eq!(words(&bard("Lore", 3)), Some((1, 6)));
        assert_eq!(words(&bard("Lore", 10)), Some((1, 10)));
        assert_eq!(words(&bard("Valor", 3)), None);
        let c = lists::class_data("Bard").unwrap();
        assert_eq!(c.subclasses.iter().find(|s| s.name == "Lore").map(|s| s.skills), Some(3));
    }

    #[test]
    fn valor_gets_extra_attack_at_six() {
        assert_eq!(bard("Valor", 5).attacks(), 1);
        assert_eq!(bard("Valor", 6).attacks(), 2);
        assert_eq!(bard("Lore", 6).attacks(), 1);
    }

    fn cleric(sub: &str, level: u32) -> Character {
        let s = |x: &str| x.to_string();
        build(&Draft {
            name: s("Pell"), lineage: s("Human"), class: s("Cleric"), subclass: Some(s(sub)), level, background: s("Acolyte"),
            base: [("STR", 13), ("DEX", 10), ("CON", 14), ("INT", 8), ("WIS", 15), ("CHA", 12)].iter().map(|(a, v)| (a.to_string(), *v)).collect(),
            class_skills: vec![s("Medicine"), s("History")], ..Default::default()
        })
    }

    #[test]
    fn knowledge_channel_divinity_options() {
        let c = cleric("Knowledge", 6);
        let read = c.actions.iter().find(|a| a.name == "Read Thoughts").expect("read thoughts");
        assert_eq!((read.uses.as_deref(), read.min_level), (Some("channel_divinity"), 6));
        assert!(c.fmt(&read.desc).contains("DC 14")); // 8 + 3 + WIS 16
        assert!(c.actions.iter().any(|a| a.name == "Knowledge of the Ages"));
    }

    #[test]
    fn divine_strike_grows_at_fourteen() {
        let strike = |c: &Character| c.actions.iter().find(|a| a.name == "Divine Strike").map(|a| (c.dice(a), a.dmg_type.clone(), a.min_level));
        assert_eq!(strike(&cleric("Life", 8)), Some((Some((1, 8)), "radiant".into(), 8)));
        assert_eq!(strike(&cleric("Life", 14)).and_then(|s| s.0), Some((2, 8)));
        assert!(cleric("Life", 2).actions.iter().any(|a| a.name == "Preserve Life"));
    }

    #[test]
    fn light_domain_flare_and_dawn() {
        let c = cleric("Light", 2);
        assert_eq!(c.res_max("warding_flare"), Some(3)); // WIS 16
        assert!(c.castable().contains(&"Light".to_string()));
        let dawn = c.actions.iter().find(|a| a.name == "Radiance of the Dawn").expect("dawn");
        assert!(dawn.level_bonus && c.dice(dawn) == Some((2, 10)) && dawn.uses.as_deref() == Some("channel_divinity"));
    }

    #[test]
    fn domains_with_wis_uses_per_long_rest() {
        for (sub, id) in [("Tempest", "wrath_of_the_storm"), ("War", "war_priest")] {
            let c = cleric(sub, 1);
            assert_eq!(c.res_max(id), Some(3), "{sub}");
            assert_eq!(c.def(id).map(|r| r.recharge.as_str()), Some("long"));
        }
        let wrath = cleric("Tempest", 1);
        assert_eq!(wrath.actions.iter().find(|a| a.name == "Wrath of the Storm").and_then(|a| wrath.dice(a)), Some((2, 8)));
    }

    #[test]
    fn circle_of_the_land_by_terrain() {
        let s = |x: &str| x.to_string();
        let druid = |sub: &str, level| build(&Draft {
            name: s("Moss"), lineage: s("Human"), class: s("Druid"), subclass: Some(s(sub)), level, background: s("Hermit"),
            base: [("STR", 10), ("DEX", 14), ("CON", 13), ("INT", 12), ("WIS", 15), ("CHA", 8)].iter().map(|(a, v)| (a.to_string(), *v)).collect(),
            class_skills: vec![s("Nature"), s("Survival")], ..Default::default()
        });
        let c = druid("Land: Desert", 5);
        assert!(["Blur", "Silence", "Create Food and Water", "Protection from Energy"].iter().all(|n| c.castable().contains(&n.to_string())));
        assert!(!c.castable().contains(&"Blight".to_string()));
        assert_eq!(c.res_max("natural_recovery"), Some(1));
        assert!(cleric("Trickery", 3).castable().contains(&"Pass Without Trace".to_string()));
    }

    #[test]
    fn champion_crits_and_athletics() {
        let champ = |level| build(&Draft { subclass: Some("Champion".into()), level, maneuvers: vec![], ..rowan() });
        assert_eq!(champ(3).table("crit_range"), Some(19));
        assert_eq!(champ(15).table("crit_range"), Some(18));
        assert_eq!(build(&rowan()).table("crit_range"), None);
        let c = champ(7); // prof +3, DEX 16, STR 9; Perception proficient, Acrobatics not
        assert_eq!(c.check_mod("Acrobatics"), 3 + 2);
        assert_eq!(c.check_mod("STR"), -1 + 2);
        assert_eq!(c.check_mod("DEX save"), 3);
        assert_eq!(c.check_mod("Arcana"), champ(3).check_mod("Arcana"));
        assert_eq!(champ(3).check_mod("Acrobatics"), 3);
    }

    #[test]
    fn eldritch_knight_keeps_to_its_schools() {
        let ek = lists::casting_of("Fighter/Eldritch Knight").unwrap();
        let v = |names: &[&str]| names.iter().map(|n| n.to_string()).collect::<Vec<_>>();
        assert!(ek.schools_ok(3, &v(&["Shield", "Magic Missile", "Find Familiar"])));
        assert!(!ek.schools_ok(3, &v(&["Shield", "Sleep", "Find Familiar"])));
        assert!(ek.schools_ok(8, &v(&["Shield", "Sleep", "Find Familiar"])));
        assert_eq!(ek.schools_hint(8), " (abjuration or evocation; 2 of any school)");
        assert!(lists::casting_of("Wizard").unwrap().schools_ok(1, &v(&["Sleep", "Find Familiar"])));
        let at = lists::casting_of("Rogue/Arcane Trickster").unwrap();
        assert!(at.schools_ok(3, &v(&["Sleep", "Charm Person", "Shield"])) && !at.schools_ok(3, &v(&["Sleep", "Shield", "Magic Missile"])));
    }

    fn monk_sub(sub: &str, level: u32) -> Character {
        let s = |x: &str| x.to_string();
        build(&Draft {
            name: s("Reed"), lineage: s("Human"), class: s("Monk"), subclass: Some(s(sub)), level, background: s("Hermit"),
            base: [("STR", 10), ("DEX", 15), ("CON", 13), ("INT", 8), ("WIS", 14), ("CHA", 10)].iter().map(|(a, v)| (a.to_string(), *v)).collect(),
            class_skills: vec![s("Acrobatics"), s("Insight")], ..Default::default()
        })
    }

    #[test]
    fn open_hand_wholeness_from_sixth_level() {
        assert_eq!(monk_sub("Open Hand", 5).res_max("wholeness_of_body"), Some(0));
        let c = monk_sub("Open Hand", 6);
        assert_eq!(c.res_max("wholeness_of_body"), Some(1));
        let w = c.actions.iter().find(|a| a.name == "Wholeness of Body").unwrap();
        assert!(w.heal && w.level_x == 3 && c.dice(w).is_none());
    }

    #[test]
    fn shadow_arts_costs_two_ki() {
        let c = monk_sub("Shadow", 3);
        assert_eq!(c.rituals(), ["Minor Illusion"]);
        assert!(c.has_spells());
        assert_eq!(c.actions.iter().find(|a| a.name == "Shadow Arts").map(|a| (a.uses.as_deref(), a.spend)), Some((Some("ki"), 2)));
    }

    #[test]
    fn finesse_follows_the_better_ability() {
        let mut c = barbarian(3); // STR 16, DEX 14
        c.active.push("Rage".into());
        assert_eq!((c.ability("FINESSE"), c.md("FINESSE")), ("STR", 3));
        assert_eq!(c.toggle_bonus(Some("FINESSE")), 2); // a raging monk strikes with STR
        c.abilities.insert("DEX".into(), 18);
        assert_eq!((c.md("FINESSE"), c.toggle_bonus(Some("FINESSE"))), (4, 0));
    }

    #[test]
    fn four_elements_disciplines() {
        let d = lists::options("disciplines");
        assert_eq!(d.len(), 16);
        let fire = d.iter().find(|o| o.name == "Flames of the Phoenix").unwrap();
        assert_eq!((fire.action.as_ref().map(|a| (a.spend, a.uses.as_deref())), fire.prereq.as_deref()), (Some((4, Some("ki"))), Some("11th level")));
        assert!(d.iter().all(|o| o.action.as_ref().is_some_and(|a| a.spend > 0) && !o.desc.is_empty()));
        let sub = lists::class_data("Monk").unwrap().subclasses.iter().find(|s| s.name == "Four Elements").unwrap();
        assert_eq!(crate::model::at(&sub.options["disciplines"], 11), 3);
    }

    fn paladin(sub: &str, level: u32) -> Character {
        let s = |x: &str| x.to_string();
        build(&Draft {
            name: s("Vow"), lineage: s("Human"), class: s("Paladin"), subclass: Some(s(sub)), level, background: s("Soldier"),
            base: [("STR", 15), ("DEX", 10), ("CON", 13), ("INT", 8), ("WIS", 12), ("CHA", 14)].iter().map(|(a, v)| (a.to_string(), *v)).collect(),
            class_skills: vec![s("Athletics"), s("Persuasion")], weapons: vec![s("Longsword")], ..Default::default()
        })
    }

    #[test]
    fn sacred_weapon_adds_charisma_to_hit() {
        let mut c = paladin("Devotion", 3);
        let sword = c.actions.iter().find(|a| a.name.starts_with("Longsword")).cloned().expect("longsword");
        assert_eq!(c.hit_bonus(&sword), Some(3 + 2)); // STR 16, prof 2
        let t = c.toggles.iter().find(|t| t.name == "Sacred Weapon").expect("toggle");
        assert_eq!(t.uses.as_deref(), Some("channel_divinity"));
        c.active.push("Sacred Weapon".into());
        assert_eq!(c.hit_bonus(&sword), Some(3 + 2 + 2)); // + CHA 15
        assert_eq!(c.res_max("holy_nimbus"), Some(0));
        assert_eq!(paladin("Devotion", 20).res_max("holy_nimbus"), Some(1));
    }

    #[test]
    fn ancients_sentinel_and_champion() {
        let c = paladin("Ancients", 15);
        assert_eq!((c.res_max("undying_sentinel"), c.res_max("elder_champion")), (Some(1), Some(0)));
        let s = c.actions.iter().find(|a| a.name == "Undying Sentinel").unwrap();
        assert!(s.heal && s.bonus == 1 && c.dice(s).is_none());
        assert!(c.actions.iter().any(|a| a.name == "Nature's Wrath" && a.uses.as_deref() == Some("channel_divinity")));
    }

    #[test]
    fn vengeance_vow_and_abjure() {
        let c = paladin("Vengeance", 3);
        assert!(c.toggles.iter().any(|t| t.name == "Vow of Enmity" && t.uses.as_deref() == Some("channel_divinity")));
        let abjure = c.actions.iter().find(|a| a.name == "Abjure Enemy").unwrap();
        assert!(c.fmt(&abjure.desc).contains("DC 12")); // 8 + 2 + CHA 15
    }

    #[test]
    fn hunter_choices_by_level() {
        let sub = lists::class_data("Ranger").unwrap().subclasses.iter().find(|s| s.name == "Hunter").unwrap();
        for (kind, level, n) in [("hunters_prey", 3, 3), ("defensive_tactics", 7, 3), ("multiattack", 11, 2), ("superior_defense", 15, 3)] {
            assert_eq!(crate::model::at(&sub.options[kind], level), 1, "{kind}");
            assert_eq!(lists::options(kind).len(), n, "{kind}");
        }
        let slayer = lists::options("hunters_prey").iter().find(|o| o.name == "Colossus Slayer").and_then(|o| o.action.as_ref()).unwrap();
        assert_eq!(slayer.dice, Some((1, 8)));
    }

    #[test]
    fn draconic_resilience_hp_and_ac() {
        let s = |x: &str| x.to_string();
        let sorc = |sub: &str, level| build(&Draft {
            name: s("Ember"), lineage: s("Human"), class: s("Sorcerer"), subclass: Some(s(sub)), level, background: s("Sage"),
            base: [("STR", 8), ("DEX", 14), ("CON", 13), ("INT", 10), ("WIS", 12), ("CHA", 15)].iter().map(|(a, v)| (a.to_string(), *v)).collect(),
            class_skills: vec![s("Arcana"), s("Insight")], ..Default::default()
        });
        let (d, w) = (sorc("Draconic Bloodline", 5), sorc("Wild Magic", 5));
        assert_eq!(d.hp_max().unwrap() - w.hp_max().unwrap(), 5);
        assert_eq!((d.ac(), w.ac()), (Some(13 + 2), Some(10 + 2)));
        let mut armored = d.clone();
        armored.ac_armored = true;
        (armored.ac_base, armored.ac_dex_max) = armor_class(Some("Leather"), false, false);
        assert_eq!(armored.ac(), Some(11 + 2));
        let mut shield = d.clone();
        (shield.ac_shield, shield.ac_base) = (true, Some(12));
        assert_eq!(shield.ac(), Some(13 + 2 + 2));
        assert_eq!(lists::options("dragon_ancestor").iter().find(|o| o.name == "Red").map(|o| o.desc.as_str()), Some("Fire"));
        assert_eq!(lists::sub_of("Sorcerer/Wild Magic").map(|s| s.surge_table.len()), Some(50));
        let bend = w.actions.iter().find(|a| a.name == "Bend Luck").unwrap();
        assert_eq!((bend.spend, w.dice(bend), w.res_max("tides_of_chaos")), (2, Some((1, 4)), Some(1)));
    }

    #[test]
    fn berserker_frenzy_and_presence() {
        let c = barbarian(3);
        assert!(c.toggles.iter().any(|t| t.name == "Frenzy" && t.exhaust));
        assert!(c.toggles.iter().any(|t| t.name == "Rage"));
        let presence = c.actions.iter().find(|a| a.name == "Intimidating Presence").expect("presence");
        assert_eq!(presence.min_level, 10);
        assert!(c.fmt(&presence.desc).contains("DC 9")); // 8 + prof 2 + CHA -1 (9 after the Human bonus)
    }

    #[test]
    fn unarmored_defense_stops_with_armor() {
        let mut c = barbarian(1);
        assert_eq!(c.ac(), Some(10 + 2 + 2)); // DEX 14, CON 14
        c.ac_shield = true;
        assert_eq!(c.ac(), Some(14)); // barbarians keep it with a shield
        c.ac_armored = true;
        (c.ac_base, c.ac_dex_max) = armor_class(Some("Leather"), false, false);
        assert_eq!(c.ac(), Some(13));
    }

    fn rogue(level: u32) -> Character {
        let s = |x: &str| x.to_string();
        build(&Draft {
            name: s("Vex"), lineage: s("Human"), class: s("Rogue"), subclass: Some(s("Thief")), level, background: s("Criminal"),
            base: [("STR", 8), ("DEX", 15), ("CON", 13), ("INT", 12), ("WIS", 10), ("CHA", 14)].iter().map(|(a, v)| (a.to_string(), *v)).collect(),
            class_skills: ["Acrobatics", "Insight", "Perception", "Persuasion"].iter().map(|x| s(x)).collect(),
            expertise: vec![s("Stealth"), s("Acrobatics")], ..Default::default()
        })
    }

    #[test]
    fn rogue_matches_the_wikidot_table() {
        // (level, proficiency, sneak attack dice)
        for (lvl, prof, dice) in [(1, 2, 1), (5, 3, 3), (11, 4, 6), (20, 6, 10)] {
            let c = rogue(lvl);
            assert_eq!(c.prof(), prof, "prof at {lvl}");
            let sneak = c.actions.iter().find(|a| a.name == "Sneak Attack").unwrap();
            assert_eq!(c.dice(sneak), Some((dice, 6)), "sneak attack at {lvl}");
        }
        assert_eq!(rogue(1).hp_max(), Some(8 + 2)); // CON 14
        assert!(rogue(1).features.iter().any(|f| f.name == "Expertise"));
        assert!(rogue(3).features.iter().any(|f| f.name == "Fast Hands"));
    }

    #[test]
    fn expertise_doubles_proficiency() {
        let c = rogue(1);
        // DEX 16: Stealth and Acrobatics expert (+3 +4), Insight plain (+0 +2), Arcana untrained
        assert_eq!(c.check_mod("Stealth"), 3 + 4);
        assert_eq!(c.check_mod("Insight"), 2);
        assert_eq!(c.check_mod("Arcana"), 1);
    }

    fn monk(level: u32) -> Character {
        let s = |x: &str| x.to_string();
        build(&Draft {
            name: s("Lin"), lineage: s("Human"), class: s("Monk"), subclass: Some(s("Open Hand")), level, background: s("Hermit"),
            base: [("STR", 10), ("DEX", 15), ("CON", 13), ("INT", 8), ("WIS", 14), ("CHA", 8)].iter().map(|(a, v)| (a.to_string(), *v)).collect(),
            class_skills: vec![s("Acrobatics"), s("Athletics")], ..Default::default()
        })
    }

    #[test]
    fn monk_matches_the_wikidot_table() {
        // (level, proficiency, ki, martial arts die)
        for (lvl, prof, ki, die) in [(1, 2, 0, 4), (5, 3, 5, 6), (11, 4, 11, 8), (17, 6, 17, 10), (20, 6, 20, 10)] {
            let c = monk(lvl);
            assert_eq!(c.prof(), prof, "prof at {lvl}");
            assert_eq!(c.res_max("ki"), Some(ki), "ki at {lvl}");
            let strike = c.actions.iter().find(|a| a.name == "Unarmed Strike").unwrap();
            assert_eq!(c.dice(strike), Some((1, die)), "martial arts at {lvl}");
            assert_eq!(c.md(strike.ability.as_deref().unwrap()), 3); // DEX over STR
        }
        assert_eq!(monk(1).ac(), Some(10 + 3 + 2)); // DEX 16, WIS 15
        assert_eq!(monk(1).hp_max(), Some(8 + 2)); // CON 14
        assert!(monk(5).features.iter().any(|f| f.name == "Extra Attack"));
        assert!(monk(3).features.iter().any(|f| f.name == "Deflect Missiles"));
    }

    fn half(class: &str, sub: &str, level: u32) -> Character {
        let s = |x: &str| x.to_string();
        build(&Draft {
            name: s("Sir"), lineage: s("Human"), class: s(class), subclass: (level >= 3).then(|| s(sub)), level, background: s("Soldier"),
            base: [("STR", 15), ("DEX", 10), ("CON", 13), ("INT", 8), ("WIS", 12), ("CHA", 14)].iter().map(|(a, v)| (a.to_string(), *v)).collect(),
            class_skills: vec![s("Athletics"), s("Insight"), s("Religion"), s("Persuasion")].into_iter().filter(|k| lists::class_data(class).unwrap().skills.from.contains(k)).collect(),
            style: Some(s("Defense")), ..Default::default()
        })
    }

    #[test]
    fn paladin_matches_the_wikidot_table() {
        let c = half("Paladin", "Devotion", 1);
        assert_eq!(c.hp_max(), Some(10 + 2)); // CON 14
        assert_eq!(c.res_max("divine_sense"), Some(1 + 2)); // CHA 15
        assert_eq!(c.res_max("lay_on_hands"), Some(5));
        assert!(!c.features.iter().any(|f| f.name == "Fighting Style")); // from level 2
        for (lvl, prof, loh, cd) in [(3, 2, 15, 1), (6, 3, 30, 2), (11, 4, 55, 2), (18, 6, 90, 3), (20, 6, 100, 3)] {
            let c = half("Paladin", "Devotion", lvl);
            assert_eq!(c.prof(), prof, "prof at {lvl}");
            assert_eq!(c.res_max("lay_on_hands"), Some(loh), "lay on hands at {lvl}");
            assert_eq!(c.res_max("channel_divinity"), Some(cd), "channel divinity at {lvl}");
        }
        let c = half("Paladin", "Devotion", 5);
        assert!(c.features.iter().any(|f| f.name == "Extra Attack"));
        assert!(c.features.iter().any(|f| f.name == "Fighting Style" && f.desc.starts_with("Defense")));
        assert_eq!(c.actions.iter().find(|a| a.name == "Lay on Hands").unwrap().pool.as_deref(), Some("lay_on_hands"));
    }

    #[test]
    fn ranger_matches_the_wikidot_table() {
        let c = half("Ranger", "Hunter", 1);
        assert_eq!(c.hp_max(), Some(10 + 2));
        assert!(c.features.iter().any(|f| f.name == "Favored Enemy"));
        assert!(half("Ranger", "Hunter", 5).features.iter().any(|f| f.name == "Extra Attack"));
        let c = half("Ranger", "Hunter", 11);
        assert_eq!(c.prof(), 4);
        assert!(c.features.iter().any(|f| f.name == "Hide in Plain Sight"));
        assert!(half("Ranger", "Hunter", 14).features.iter().any(|f| f.name == "Vanish"));
        assert!(half("Ranger", "Hunter", 3).features.iter().any(|f| f.name == "Hunter's Prey"));
    }

    #[test]
    fn multiclass_splits_levels_and_pools_slots() {
        use crate::model::Extra;
        let mut c = half("Paladin", "Devotion", 3); // Con 13+1 = +2, d10
        let solo = c.hp_max().unwrap();
        c.level = 5;
        c.multi.push(Extra { prog: "Wizard/School of Evocation".into(), level: 2 });
        c.sync_spell_resources();
        assert_eq!(c.class_level(), 3);
        // wizard d6 levels add 4 + 2 each
        assert_eq!(c.hp_max().unwrap(), solo + 2 * (6 / 2 + 1 + c.md("CON")));
        assert!(c.title().contains("Paladin 3") && c.title().contains("Wizard 2"));
        // paladin 3 -> 1 caster level, wizard 2 -> 2: pooled level 3 gives 4 first-level and 2 second-level slots
        assert_eq!((c.res_max("slot1"), c.res_max("slot2"), c.res_max("slot3")), (Some(4), Some(2), Some(0)));
        assert_eq!(c.prof(), 3);
    }

    #[test]
    fn class_options_and_multiclass_tables() {
        use crate::model::{at, Extra, Max, ResourceDef};
        assert_eq!(lists::options("metamagic").len(), 10);
        assert!(lists::options("invocations").iter().any(|o| o.name == "Agonizing Blast" && o.prereq.as_deref() == Some("eldritch blast cantrip")));
        assert_eq!(lists::options("pact_boons").len(), 4);
        let s = lists::class_data("Sorcerer").unwrap();
        assert_eq!((at(&s.options["metamagic"], 2), at(&s.options["metamagic"], 3), at(&s.options["metamagic"], 17)), (0, 2, 4));
        let w = lists::class_data("Warlock").unwrap();
        assert_eq!((at(&w.options["invocations"], 2), at(&w.options["invocations"], 18), at(&w.options["pact_boons"], 3)), (2, 8, 1));
        // a Paladin 3 taking 2 levels of Barbarian: rage table read at barbarian level, hit dice per class
        let mut c = half("Paladin", "Devotion", 3);
        c.level = 5;
        c.multi.push(Extra { prog: "Barbarian".into(), level: 2 });
        assert_eq!((c.table("rage"), c.table("rage_damage"), c.table("lay_on_hands")), (Some(2), Some(2), Some(15)));
        c.resources.push(ResourceDef { id: "hd_Barbarian".into(), name: "hd".into(), recharge: String::new(), max: Some(Max::Kw("class:Barbarian".into())), start: None, pips: false, die: None });
        assert_eq!((c.res_max("hit_dice"), c.res_max("hd_Barbarian"), c.res_die("hd_Barbarian")), (Some(3), Some(2), Some(12)));
    }

    #[test]
    fn sorlock_is_a_sorcerer_warlock_multiclass() {
        let mut c: Character = serde_json::from_str(crate::model::BUNDLED[1].1).unwrap();
        c.sync_spell_resources();
        assert_eq!(c.title(), "Sorcerer 1 / Warlock 1");
        assert_eq!((c.class_level(), c.prof(), c.hp_max()), (1, 2, Some(6 + 5))); // d6 max + d8 average, CON 10
        // sorcerer slots and the warlock's pact slot stay separate pools
        assert_eq!((c.res_max("slot1"), c.res_max("slot2"), c.res_max("pact")), (Some(2), Some(0), Some(1)));
        assert_eq!(c.ability_for("Eldritch Blast"), "CHA");
        let cast = c.castable();
        assert!(["Magic Missile", "Shield", "Mage Armor", "Wrathful Smite", "Alarm"].iter().all(|s| cast.contains(&s.to_string())));
    }

    #[test]
    fn exhaustion_stacks_and_halves_hp_at_four() {
        let mut c = rogue(1);
        let full = c.hp_max().unwrap();
        c.exhaustion = 3;
        assert_eq!((c.hp_max(), c.exhaustion_text().as_str()), (Some(full), "checks at disadvantage, speed halved, attacks and saves at disadvantage"));
        c.exhaustion = 4;
        assert_eq!(c.hp_max(), Some(full / 2));
    }

    #[test]
    fn rituals_follow_each_class_rule() {
        let mut w = half("Wizard", "School of Evocation", 1);
        w.spells = vec!["Find Familiar".into(), "Magic Missile".into()]; // in the book, not prepared
        assert!(w.ritual_ok("Find Familiar") && !w.ritual_ok("Magic Missile") && !w.castable().contains(&"Find Familiar".to_string()));
        let mut c = half("Cleric", "Life", 1);
        assert!(!c.ritual_ok("Detect Magic"));
        c.prepared.push("Detect Magic".into());
        assert!(c.ritual_ok("Detect Magic"));
        let mut s = half("Sorcerer", "Draconic Bloodline", 1);
        s.spells.push("Detect Magic".into());
        assert!(!s.ritual_ok("Detect Magic"));
        s.features.push(crate::model::Feature { name: "Feat: Ritual Caster".into(), ..Default::default() });
        assert!(s.ritual_ok("Detect Magic"));
    }

    #[test]
    fn paying_breaks_larger_coins_or_uses_smaller_ones() {
        let mut c = rogue(1);
        c.coins = [0, 2, 0, 0, 0]; // 2 gp
        assert!(c.pay(3, 5)); // 5 sp: one gp broken into 10 sp
        assert_eq!(c.coins, [0, 1, 0, 5, 0]);
        c.coins = [0, 0, 1, 6, 0]; // 1 ep + 6 sp = 110 cp
        assert!(c.pay(1, 1)); // 1 gp paid with the ep and 5 sp
        assert_eq!(c.coins, [0, 0, 0, 1, 0]);
        assert!(!c.pay(1, 1) && c.coins == [0, 0, 0, 1, 0]); // short: nothing spent
        assert_eq!(c.coins_text(), "1 sp (0.10 gp)");
    }

    #[test]
    fn spells_from_outside_the_class_list() {
        let mut c = half("Ranger", "Hunter", 1); // no spells yet at ranger level 1
        assert!(!c.has_spells());
        c.spells.push("Find Familiar".into());
        assert!(c.has_spells() && c.castable() == vec!["Find Familiar".to_string()]);
        let mut p = half("Paladin", "Devotion", 5);
        p.spells.push("Fire Bolt".into());
        p.prepared.push("Bless".into());
        assert!(!p.on_list("Fire Bolt") && p.on_list("Bless"));
        let cast = p.castable();
        assert!(cast.contains(&"Fire Bolt".to_string()) && cast.contains(&"Bless".to_string()));
    }

    #[test]
    fn multiclass_casters_use_their_own_ability() {
        use crate::model::Extra;
        let mut c = half("Paladin", "Devotion", 3); // CHA 14+1, WIS 12+1
        c.level = 5;
        c.multi.push(Extra { prog: "Cleric/Life".into(), level: 2 });
        assert_eq!(c.ability_for("Bless"), "CHA");          // on both lists: the better modifier
        assert_eq!(c.ability_for("Sacred Flame"), "WIS");   // cleric only
        assert_eq!(c.ability_for("Find Steed"), "CHA");     // paladin only
        assert!(c.spell_dcs().contains("CHA DC") && c.spell_dcs().contains("WIS DC"));
    }

    #[test]
    fn srd_beasts_load() {
        let b = lists::beasts();
        assert!(b.len() > 90 && b.iter().filter(|x| x.elemental).count() == 4);
        let wolf = b.iter().find(|x| x.name == "Wolf").unwrap();
        assert_eq!((wolf.ac, wolf.hp, wolf.attacks[0].hit, wolf.attacks[0].dice, wolf.attacks[0].bonus), (13, 11, 4, Some((2, 4)), 2));
        let cat = b.iter().find(|x| x.name == "Cat").unwrap();
        assert_eq!((cat.attacks[0].dice, cat.attacks[0].bonus), (None, 1));
    }

    #[test]
    fn casters_match_the_wikidot_tables() {
        use crate::model::at;
        let w = lists::casting_of("Wizard/School of Evocation").unwrap();
        assert_eq!((w.slots_at(1, 1), w.slots_at(5, 3), w.slots_at(11, 6), w.slots_at(20, 9)), (2, 2, 1, 1));
        assert_eq!((at(&w.cantrips, 1), at(&w.cantrips, 4), at(&w.cantrips, 10)), (3, 4, 5));
        let k = lists::casting_of("Warlock/The Fiend").unwrap();
        assert_eq!((at(&k.pact_slots, 1), at(&k.pact_slots, 2), at(&k.pact_slots, 11), at(&k.pact_level, 9)), (1, 2, 3, 5));
        let b = lists::casting_of("Bard/College of Lore").unwrap();
        assert_eq!((at(&b.known, 1), at(&b.known, 20)), (4, 22));
        let ek = lists::casting_of("Fighter/Eldritch Knight").unwrap();
        assert_eq!((ek.slots_at(3, 1), ek.slots_at(7, 2), at(&ek.cantrips, 10)), (2, 2, 3));
        let at_ = lists::casting_of("Rogue/Arcane Trickster").unwrap();
        assert_eq!(at(&at_.cantrips, 3), 3);
        assert!(lists::always_spells("Cleric/Life", 3).iter().any(|s| s == "Bless"));
        assert!(lists::casting_of("Barbarian/Path of the Berserker").is_none());
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

// dnd-tui in the browser: draws the App's view (JSON from Rust) as HTML and sends taps back as commands.
"use strict";

let w; // the wasm exports
let v; // the last view
let lastLog = "";
let logOpen = false;

const $ = (s) => document.querySelector(s);
const esc = (s) => String(s ?? "").replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c]);
const sign = (n) => (n >= 0 ? "+" : "") + n;
const cmd = (c) => esc(JSON.stringify(c));
const hit = (q, t) => !q || t.toLowerCase().includes(q.toLowerCase());

// a line from Rust: spans with a colour name; "More: url" becomes a link
function line(spans) {
  if (!spans.length) return '<div class="gap"></div>';
  return '<div class="ln">' + spans.map((s) => {
    const t = esc(s.t).replace(/(https?:\/\/\S+)/g, '<a href="$1" target="_blank" rel="noopener">$1</a>');
    return `<span class="c-${s.c || "norm"}${s.b ? " b" : ""}">${t}</span>`;
  }).join("") + "</div>";
}
const lines = (ls) => (ls || []).map(line).join("");

// a status line cut at " | " and double spaces, so on a phone it wraps between items, never inside one
function chunks(spans) {
  const items = [""];
  for (const s of spans) {
    esc(s.t).split(/ \| |\s{2,}/).forEach((p, i) => {
      if (i) items.push("");
      if (p.trim()) items[items.length - 1] += `<span class="c-${s.c || "norm"}${s.b ? " b" : ""}">${p}</span>`;
    });
  }
  return '<div class="ln chunks">' + items.filter((x) => x).map((x) => `<span>${x}</span>`).join("") + "</div>";
}

// a box with its title in the border, as in the terminal
const box = (title, body, cls = "") => `<fieldset class="box ${cls}"><legend>${title}</legend>${body}</fieldset>`;

function btn(label, c, cls = "") {
  return `<button class="${cls}" data-cmd="${cmd(c)}">${esc(label)}</button>`;
}

// --- screens --------------------------------------------------------------

function picker() {
  const rows = v.sheets.map((s, i) => `
    <div class="sheet">
      <button class="open" data-cmd="${cmd({ Pick: i })}"><b>${esc(s.name)}</b><small>${esc(s.title)}</small></button>
      <div class="row">
        <button data-sel="${i}" data-then="${cmd("Export")}">PDF</button>
        <button data-sel="${i}" data-then="${cmd("ExportJson")}">Export</button>
        <button class="danger" data-sel="${i}" data-then="${cmd("Delete")}">Delete</button>
      </div>
    </div>`).join("");
  return `<header class="top"><h1>DND TUI</h1><small>D&amp;D 5e character sheets</small></header>
    <main>${box("CHARACTERS", rows || '<p class="dim">No characters yet.</p>', "sheets")}
      <div class="row big">${btn("+ New character", "New", "primary")}${btn("Import sheet", "Import")}</div>
    </main>`;
}

function header() {
  const bar = v.hp_max ? `<span class="bar"><i style="width:${Math.max(0, Math.min(100, (100 * v.hp) / v.hp_max))}%" class="${v.hp * 4 <= v.hp_max ? "low" : v.hp * 2 <= v.hp_max ? "mid" : ""}"></i></span>` : "";
  const hp = v.hp_max ? `
    <button class="hp" data-sheet="hp">HP <b>${v.hp}/${v.hp_max}</b>${v.temp_hp ? ` <em>+${v.temp_hp}</em>` : ""} ${bar}</button>` : "";
  const adv = { normal: "Roll: normal", advantage: "ADVANTAGE", disadvantage: "DISADVANTAGE" }[v.adv];
  return `<header class="top">
    <div class="row">
      ${btn("‹", "Back", "icon")}
      <div class="who"><b>${esc(v.name)}</b><small>${esc(v.title)}</small></div>
      <button class="icon" data-log="1" title="Log">☰</button>
      ${btn("↶", "Undo", "icon")}
    </div>
    ${hp}
    <div class="row stats">
      ${v.ac != null ? `<span>AC <b>${v.ac}</b></span>` : ""}
      <button data-cmd="${cmd("Init")}">Init <b>${sign(v.init)}</b></button>
      <span>Prof <b>+${v.prof}</b></span>
      <button class="adv adv-${v.adv}" data-cmd="${cmd("Adv")}">${adv}</button>
    </div>
  </header>`;
}

// the tab's commands as the terminal's footer: key and label; the header already has Undo and Back
function commands() {
  const skip = new Set(['"Back"', '"Undo"', '"Down"', '"Enter"', '"Cast"']);
  return box("COMMANDS", '<div class="cmds">' + v.commands.filter((c) => !skip.has(JSON.stringify(c.cmd)))
    .map((c) => `<button data-cmd="${cmd(c.cmd)}"><b>${esc(c.key)}</b> ${esc(c.label)}</button>`).join("") + "</div>", "wide");
}

function search() {
  return `<input class="search" type="search" placeholder="Search" value="${esc(v.query)}" data-query="1">`;
}

const mark = (on) => `<span class="${on ? "c-green" : "c-darkgray"}">${on ? "●" : "○"}</span>`;

function combat() {
  const acts = v.actions.map((a, i) => a.toggle
    ? `<button class="item" data-cmd="${cmd({ Click: i })}"><b>${esc(a.name)}</b><span class="${a.on ? "tag on" : "dim"}">${a.on ? "ON" : "off"}</span></button>`
    : `<button class="item act ${a.ready ? "" : "spent"} ${i === v.sel ? "sel" : ""}" data-cmd="${cmd({ Click: i })}">
        <span class="top"><b>${esc(a.name)}</b><span>${esc(a.hit)} <span class="c-red">${esc(a.dmg)}</span>${a.uses != null ? ` <span class="dim">×${a.uses}</span>` : ""}</span></span>
        ${a.desc ? `<small>${esc(a.desc)}</small>` : ""}
      </button>`).join("");
  const man = v.maneuver ? box(`MANEUVERS · d${v.maneuver.die} · ${v.maneuver.left} left`,
    v.maneuver.list.map((m) => `<button class="item act" data-cmd="${cmd("Maneuver")}"><b>${esc(m.name)}</b><small>${esc(m.desc)}</small></button>`).join("")) : "";
  const abs = v.abilities.map((a) => `<div class="ab">
      <button data-cmd="${cmd({ Roll: a.ab })}"><small>${a.ab}</small><b>${sign(a.mod)}</b><small>${a.score}</small></button>
      <button class="save" data-cmd="${cmd({ Roll: a.ab + " save" })}">${mark(a.save_prof)} ${sign(a.save)}</button>
    </div>`).join("");
  const skills = v.skills.map((s) => `<button class="skill" data-cmd="${cmd({ Roll: s.name })}">${mark(s.prof)} <b>${sign(s.mod)}</b> ${esc(s.name)} <small>${s.ab}</small></button>`).join("");
  return `<div>${box("STATUS", v.status.map(chunks).join(""))}${box("ACTIONS", acts)}${man}</div>
    <div>${box('ABILITIES <small>tap: check · bottom: save</small>', `<div class="abs">${abs}</div>`)}
    ${box("SKILLS", `<div class="skills">${skills}</div>`)}</div>
    ${commands()}`;
}

function spells() {
  const s = v.spells;
  const groups = [];
  s.rows.forEach((sp, i) => {
    if (!hit(v.query, sp.name)) return;
    if (!groups.length || groups[groups.length - 1].level !== sp.level) groups.push({ level: sp.level, html: "" });
    const open = i === s.sel;
    groups[groups.length - 1].html += `<button class="item ${sp.usable ? "" : "spent"} ${open ? "sel" : ""}" data-cmd="${cmd({ Select: i })}">
        <b>${esc(sp.name)}</b><span class="dim">${sp.conc ? "C" : ""}${sp.ritual ? " R" : ""}</span></button>
      ${open ? `<div class="detail">${lines(s.detail)}<div class="row">${btn("Cast", { SpellClick: i }, "primary")}</div></div>` : ""}`;
  });
  const rows = groups.map((g) => box(g.level ? "LEVEL " + g.level : "CANTRIPS", g.html)).join("");
  return `${box("SPELLCASTING", `${s.slots ? `<div class="ln">${esc(s.slots)}</div>` : ""}<div class="ln dim">${esc(s.dcs)}${s.prepared ? " · " + esc(s.prepared) : ""}</div>`, "wide")}
    ${search()}${rows || '<p class="dim">No spells yet: Edit adds some.</p>'}
    ${commands()}`;
}

function character() {
  return `${search()}${box("CHARACTER", lines(v.character), "text wide")}${commands()}`;
}

function journal() {
  const notes = v.journal.map((n, i) => hit(v.query, n)
    ? `<button class="item note ${i === v.jsel ? "sel" : ""}" data-cmd="${cmd({ JournalSel: i })}">${esc(n)}</button>` : "").join("");
  return `${search()}${box("JOURNAL", notes || '<p class="dim">The journal is empty.</p>', "wide")}${commands()}`;
}

function tabs() {
  return '<nav class="tabs">' + v.tabs.map((t) =>
    `<button class="${t.id === v.tab ? "on" : ""}" data-cmd="${cmd({ Tab: t.id })}">${esc(t.name)}</button>`).join("") + "</nav>";
}

// --- sheets (bottom panels) -----------------------------------------------

function modal() {
  const m = v.modal;
  if (!m) return "";
  let body;
  if (m.kind === "input") {
    body = `<form data-submit="1"><input name="t" value="${esc(m.buf)}" ${m.numeric ? 'inputmode="numeric" pattern="-?[0-9]*"' : ""} autocomplete="off" autofocus>
      <div class="row">${'<button type="button" data-cancel="1">Cancel</button>'}<button class="primary">OK</button></div></form>`;
  } else {
    const done = m.kind === "multi" && m.limit === 0;
    const items = m.items.map((t, i) => {
      if (done && i === m.items.length - 1) return "";
      const on = m.kind === "multi" ? `<span class="tick">${m.on[i] ? "✓" : ""}</span>` : "";
      return `<button class="item ${i === m.sel ? "sel" : ""} ${m.on && m.on[i] ? "on" : ""}" data-pick="${i}" data-text="${esc(t.toLowerCase())}">${on}${esc(t)}</button>`;
    }).join("");
    const filter = m.items.length > 8 ? `<input class="search" type="search" placeholder="Search" value="${esc(v.query)}" data-filter="1">` : "";
    const foot = [m.kind === "multi" && m.limit ? `<small>Pick ${m.limit}</small>` : "",
      done ? `<button class="primary" data-pick="${m.items.length - 1}">${esc(m.items[m.items.length - 1])}</button>` : "",
      m.cancel !== false ? '<button data-cancel="1">Cancel</button>' : ""].join("");
    body = `${filter}<div class="items">${items}</div><div class="row">${foot}</div>`;
  }
  return `<div class="veil"></div><section class="sheet-panel"><h2>${esc(m.title)}</h2>${body}</section>`;
}

function hpSheet() {
  return `<div class="veil" data-close="1"></div><section class="sheet-panel"><h2>Hit points</h2>
    <div class="row big">${btn("Damage", "Damage", "danger")}${btn("Heal", "Heal", "primary")}${btn("Temp HP", "Temp")}</div>
    <div class="row"><button data-close="1">Close</button></div></section>`;
}

function logPanel() {
  return `<div class="veil" data-close="1"></div><section class="sheet-panel log"><h2>Log</h2>
    ${v.log.map((e) => `<div class="t-${e.tone}">${esc(e.t)}</div>`).join("")}
    <div class="row"><button data-close="1">Close</button></div></section>`;
}

// --- render and events ----------------------------------------------------

let panel = ""; // "hp" | "log" | ""

function render() {
  v = JSON.parse(w.view());
  if (v.modal) panel = "";
  let html;
  if (v.picking) html = picker();
  else {
    const body = { Combat: combat, Spells: spells, Character: character, Journal: journal }[v.tab]();
    html = header() + `<main>${body}</main>` + tabs();
  }
  // the entries added since the last render (an attack logs the roll, then the damage)
  const fresh = [];
  for (const e of v.log) { if (e.t === lastLog || fresh.length === 4) break; fresh.push(e); }
  const toast = fresh.length && lastLog !== null && !v.picking
    ? `<div class="toast" data-log="1">${fresh.reverse().map((e) => `<div class="t-${e.tone}">${esc(e.t)}</div>`).join("")}</div>` : "";
  lastLog = v.log[0] ? v.log[0].t : "";
  html += toast + modal() + (panel === "hp" ? hpSheet() : panel === "log" ? logPanel() : "");
  $("#app").innerHTML = html;
  const f = $("#app [autofocus]");
  if (f) f.focus();
  const t = $(".toast");
  if (t) setTimeout(() => t.remove(), 5000);
}

function run(c) { w.exec(typeof c === "string" ? c : JSON.stringify(c)); render(); }

document.addEventListener("click", (e) => {
  const b = e.target.closest("button, .veil");
  if (!b || !w) return;
  const d = b.dataset;
  if (d.cmd) { panel = ""; run(d.cmd); }
  else if (d.sel) { w.exec(JSON.stringify({ Select: +d.sel })); run(d.then); }
  else if (d.pick) run({ ModalPick: +d.pick });
  else if (d.cancel) { w.cancel(); render(); }
  else if (d.sheet) { panel = d.sheet; render(); }
  else if (d.log) { panel = "log"; render(); }
  else if (d.close) { panel = ""; render(); }
});

document.addEventListener("submit", (e) => {
  e.preventDefault();
  w.submit(e.target.elements.t.value);
  render();
});

document.addEventListener("input", (e) => {
  const d = e.target.dataset;
  if (d.filter) { // filter the rows in place so the field keeps focus
    const q = e.target.value.toLowerCase();
    w.set_query(e.target.value);
    document.querySelectorAll(".items .item").forEach((b) => { b.hidden = !b.dataset.text.includes(q); });
  } else if (d.query) {
    w.set_query(e.target.value);
    const pos = e.target.selectionStart;
    render();
    const s = $("[data-query]");
    if (s) { s.focus(); s.setSelectionRange(pos, pos); }
  }
});

// desktop: the terminal keys still work outside text fields
document.addEventListener("keydown", (e) => {
  if (!w || e.metaKey || e.altKey || (e.ctrlKey && e.key !== "c")) return;
  if (e.target.matches("input, textarea")) {
    if (e.key === "Escape") { w.cancel(); render(); }
    return;
  }
  if (e.key === "F5" || e.key === "F12") return;
  if (panel && e.key === "Escape") { panel = ""; render(); return; }
  if (w.key(e.key, e.ctrlKey, e.shiftKey)) { e.preventDefault(); render(); }
});

// an imported sheet arrives asynchronously: refresh the list when it does
setInterval(() => { if (w && v && v.picking) { const before = v.sheets.length; const n = JSON.parse(w.view()).sheets.length; if (n !== before) render(); } }, 700);

function start() { if (w) return; w = window.wasmBindings; render(); }
addEventListener("TrunkApplicationStarted", start);
if (window.wasmBindings) start();

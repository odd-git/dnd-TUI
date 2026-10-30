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

function btn(label, c, cls = "") {
  return `<button class="${cls}" data-cmd="${cmd(c)}">${esc(label)}</button>`;
}

// --- screens --------------------------------------------------------------

function picker() {
  const cards = v.sheets.map((s, i) => `
    <div class="card sheet">
      <button class="open" data-cmd="${cmd({ Pick: i })}"><b>${esc(s.name)}</b><small>${esc(s.title)}</small></button>
      <div class="row">
        <button data-sel="${i}" data-then="${cmd("Export")}">PDF</button>
        <button data-sel="${i}" data-then="${cmd("ExportJson")}">Export</button>
        <button class="danger" data-sel="${i}" data-then="${cmd("Delete")}">Delete</button>
      </div>
    </div>`).join("");
  return `<header class="top"><h1>DND TUI</h1><small>D&amp;D 5e character sheets</small></header>
    <main class="sheets">${cards}
      <div class="row big">${btn("+ New character", "New", "primary")}${btn("Import sheet", "Import")}</div>
    </main>`;
}

function header() {
  const hp = v.hp_max ? `
    <button class="hp" data-sheet="hp">
      <span>HP <b>${v.hp}</b>/${v.hp_max}${v.temp_hp ? ` <em>+${v.temp_hp}</em>` : ""}</span>
      <i style="width:${Math.max(0, Math.min(100, (100 * v.hp) / v.hp_max))}%" class="${v.hp * 4 <= v.hp_max ? "low" : v.hp * 2 <= v.hp_max ? "mid" : ""}"></i>
    </button>` : "";
  const adv = { normal: "Normal", advantage: "Advantage", disadvantage: "Disadvantage" }[v.adv];
  return `<header class="top">
    <div class="row">
      ${btn("‹", "Back", "icon")}
      <div class="who"><b>${esc(v.name)}</b><small>${esc(v.title)}</small></div>
      <button class="icon" data-log="1" title="Log">☰</button>
      ${btn("↶", "Undo", "icon")}
    </div>
    ${hp}
    <div class="row chips">
      ${v.ac != null ? `<span class="chip">AC <b>${v.ac}</b></span>` : ""}
      <button class="chip" data-cmd="${cmd("Init")}">Init <b>${sign(v.init)}</b></button>
      <span class="chip">Prof <b>+${v.prof}</b></span>
      <button class="chip adv-${v.adv}" data-cmd="${cmd("Adv")}">${adv}</button>
    </div>
  </header>`;
}

// the tab's commands as buttons; the header already has Undo and Back
function commands() {
  const skip = new Set(['"Back"', '"Undo"', '"Down"', '"Enter"', '"Cast"']);
  return '<div class="cmds">' + v.commands.filter((c) => !skip.has(JSON.stringify(c.cmd)))
    .map((c) => btn(c.label, c.cmd)).join("") + "</div>";
}

function search() {
  return `<input class="search" type="search" placeholder="Search" value="${esc(v.query)}" data-query="1">`;
}

function combat() {
  const acts = v.actions.map((a, i) => a.toggle
    ? `<button class="card act" data-cmd="${cmd({ Click: i })}"><b>${esc(a.name)}</b><span class="tag ${a.on ? "on" : ""}">${a.on ? "ON" : "off"}</span></button>`
    : `<button class="card act ${a.ready ? "" : "spent"}" data-cmd="${cmd({ Click: i })}">
        <span class="top"><b>${esc(a.name)}</b>${a.hit ? `<span class="tag">${esc(a.hit)}</span>` : ""}${a.dmg ? `<span class="tag dmg">${esc(a.dmg)}</span>` : ""}${a.uses != null ? `<span class="tag">×${a.uses}</span>` : ""}</span>
        ${a.desc ? `<small>${esc(a.desc)}</small>` : ""}
      </button>`).join("");
  const man = v.maneuver ? `<h2>Maneuvers <small>d${v.maneuver.die}, ${v.maneuver.left} left</small></h2>` +
    v.maneuver.list.map((m) => `<button class="card act" data-cmd="${cmd("Maneuver")}"><b>${esc(m.name)}</b><small>${esc(m.desc)}</small></button>`).join("") : "";
  const checks = v.checks.map((c) => `<button class="check ${c.prof ? "prof" : ""}" data-cmd="${cmd({ Roll: c.name })}"><span>${esc(c.name)}</span><b>${sign(c.mod)}</b></button>`).join("");
  return `<section class="card status">${lines(v.status)}</section>
    <h2>Actions</h2>${acts}${man}
    <h2>Checks</h2><div class="checks">${checks}</div>
    <h2>More</h2>${commands()}`;
}

function spells() {
  const s = v.spells;
  let last = -1;
  const rows = s.rows.map((sp, i) => {
    if (!hit(v.query, sp.name)) return "";
    const head = sp.level !== last ? `<h3>${sp.level ? "Level " + sp.level : "Cantrips"}</h3>` : "";
    last = sp.level;
    const open = i === s.sel;
    return head + `<div class="card spell ${sp.usable ? "" : "spent"} ${open ? "open" : ""}">
      <button class="name" data-cmd="${cmd({ Select: i })}"><b>${esc(sp.name)}</b>${sp.conc ? '<span class="tag">C</span>' : ""}${sp.ritual ? '<span class="tag">R</span>' : ""}</button>
      ${open ? `<div class="detail">${lines(s.detail)}<div class="row">${btn("Cast", { SpellClick: i }, "primary")}</div></div>` : ""}
    </div>`;
  }).join("");
  return `<section class="card">${s.slots ? `<div>${esc(s.slots)}</div>` : ""}<small>${esc(s.dcs)}${s.prepared ? " · " + esc(s.prepared) : ""}</small></section>
    ${search()}${rows || '<p class="dim">No spells yet: Edit adds some.</p>'}
    <h2>More</h2>${commands()}`;
}

function character() {
  return `${search()}<section class="card text">${lines(v.character)}</section><h2>More</h2>${commands()}`;
}

function journal() {
  const notes = v.journal.map((n, i) => hit(v.query, n)
    ? `<button class="card note ${i === v.jsel ? "open" : ""}" data-cmd="${cmd({ JournalSel: i })}">${esc(n)}</button>` : "").join("");
  return `${search()}${notes || '<p class="dim">The journal is empty.</p>'}<h2>More</h2>${commands()}`;
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
      const on = m.kind === "multi" ? `<span class="box">${m.on[i] ? "✓" : ""}</span>` : "";
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

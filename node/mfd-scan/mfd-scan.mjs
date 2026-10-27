#!/usr/bin/env node
// mfd-scan v0.2.0 — MantoFreeDisk minimal disk space analyzer
// Zero-dependency, cross-platform (Windows/macOS/Linux, Node >= 20).
// Piano: docs/PIANO-MantoFreeDisk.md — Analisi di riferimento: docs/ANALISI-TreeSizeFree.md

process.env.UV_THREADPOOL_SIZE = process.env.UV_THREADPOOL_SIZE || '16';

import { readdir, stat, writeFile } from 'node:fs/promises';
import { join, resolve, basename, sep } from 'node:path';
import { platform, cpus } from 'node:os';
import { pathToFileURL } from 'node:url';
import { version as nodeVersion } from 'node:process';

const TOOL = 'mfd-scan';
const TOOL_VERSION = '0.2.0';

// ---------------- CLI ----------------
function usage() {
  return `Uso: node mfd-scan.mjs [path] [opzioni]        → report completo (predefinito)
     node mfd-scan.mjs ls <path> [opzioni]     → elenco aggregato stile TreeSize (one-shot)
     node mfd-scan.mjs nav <path>              → navigazione interattiva nel terminale (TUI)

Opzioni comuni:
  --follow            segue symlink/junction (default: contati e non seguiti)
  --no-color          disattiva i colori ANSI
  -h, --help          mostra questo aiuto
  -v, --version       versione

Report completo (predefinito):
  --top N             elementi mostrati nelle classifiche e nell'albero (default 15)
  --depth N           profondità massima dell'albero stampato (default 3)
  --json [file]       snapshot JSON piatto (su file se specificato, altrimenti stdout)

ls — elenco aggregato one-shot (adatto anche ad agenti AI):
  -d, --depth N       profondità di visualizzazione (default 1; 0 = illimitata)
  -f, --files         include anche i file singoli, oltre alle cartelle
  --min DIM           omette le voci più piccole, es. 10MB, 1.5GiB, 500KB (default 0)
  --top N             massimo voci mostrate per cartella (default 30)
  --sort MODO         ordinamento: size (default) | name
  --json [file]       albero JSON annidato (contratto per agenti; file o stdout)

nav — TUI interattiva:
  ↑↓ muovi · →/Invio entra nella cartella · ←/Backspace torna su
  PgUp/PgDn pagina · Home/End inizio/fine · j/k/h/l alias vim · q/Esc/Ctrl+C esci

Esempi:
  node mfd-scan.mjs D:\\SVILUPPO --top 20 --depth 4
  node mfd-scan.mjs ls D:\\SVILUPPO -d 2
  node mfd-scan.mjs ls D:\\SVILUPPO -f --min 100MB --json
  node mfd-scan.mjs ls / --json snapshot.json
  node mfd-scan.mjs nav D:\\SVILUPPO`;
}

// Converte "10MB", "1.5GiB", "500KB", "1048576" in byte BigInt (null se non valido)
function parseSize(v) {
  const s = String(v ?? '').trim();
  if (/^\d+$/.test(s)) return BigInt(s);
  const m = /^(\d+(?:[.,]\d+)?)\s*(b|kb|mb|gb|tb|kib|mib|gib|tib)$/i.exec(s);
  if (!m) return null;
  const n = parseFloat(m[1].replace(',', '.'));
  if (!Number.isFinite(n) || n < 0) return null;
  const mult = { b: 1, kb: 1e3, mb: 1e6, gb: 1e9, tb: 1e12, kib: 1024, mib: 1024 ** 2, gib: 1024 ** 3, tib: 1024 ** 4 }[m[2].toLowerCase()];
  return BigInt(Math.round(n * mult));
}

function parseArgs(argv) {
  let cmd = null;                       // null = report (comportamento storico), 'ls', 'nav'
  const args = [...argv];
  if (args.length > 0 && (args[0] === 'ls' || args[0] === 'nav')) cmd = args.shift();

  const opts = {
    cmd, root: null,
    top: cmd === 'ls' ? 30 : 15,        // ls: max voci per cartella; report: dimensione classifiche
    depth: cmd === 'ls' ? 1 : 3,        // ls: 1 livello (stile dir); report: albero completo
    files: false, minBytes: 0n, sort: 'size',
    json: false, jsonFile: null, follow: false, color: true, help: false, version: false,
  };
  const requireMode = (flag, modes) => {
    if (!modes.includes(opts.cmd)) {
      const names = modes.map((m) => (m === null ? 'report (predefinito)' : m)).join(' / ');
      fail(`Opzione ${flag} disponibile solo in: ${names}`);
    }
  };

  for (let i = 0; i < args.length; i++) {
    const a = args[i];
    if (a === '-h' || a === '--help') opts.help = true;
    else if (a === '-v' || a === '--version') opts.version = true;
    else if (a === '--follow') opts.follow = true;
    else if (a === '--no-color') opts.color = false;
    else if (a === '-f' || a === '--files') { requireMode(a, ['ls']); opts.files = true; }
    else if (a === '--sort') {
      requireMode(a, ['ls']);
      const v = args[++i];
      if (v !== 'size' && v !== 'name') fail(`Valore non valido per --sort: ${v} (ammessi: size, name)`);
      opts.sort = v;
    }
    else if (a === '--min') {
      requireMode(a, ['ls']);
      const v = parseSize(args[++i]);
      if (v === null) fail(`Valore non valido per --min: ${args[i]} (esempi: 10MB, 1.5GiB, 500KB, 1048576)`);
      opts.minBytes = v;
    }
    else if (a === '--top') {
      requireMode(a, [null, 'ls']);
      const v = Number(args[++i]);
      if (!Number.isInteger(v) || v < 1) fail(`Valore non valido per --top: ${args[i]}`);
      opts.top = v;
    }
    else if (a === '-d' || a === '--depth') {
      requireMode(a, [null, 'ls']);
      const v = Number(args[++i]);
      const min = opts.cmd === 'ls' ? 0 : 1;   // in ls: 0 = profondità illimitata
      if (!Number.isInteger(v) || v < min) fail(`Valore non valido per ${a}: ${args[i]}${opts.cmd === 'ls' ? ' (in ls: 0 = illimitata)' : ''}`);
      opts.depth = v;
    }
    else if (a === '--json') {
      requireMode(a, [null, 'ls']);
      opts.json = true;
      const next = args[i + 1];
      if (next && !next.startsWith('-')) { opts.jsonFile = next; i++; }
    }
    else if (a.startsWith('-')) fail(`Opzione sconosciuta: ${a}`);
    else if (opts.root === null) opts.root = a;
    else fail(`Argomento duplicato: ${a}`);
  }
  if (opts.root === null) opts.root = '.';
  return opts;
}

function fail(msg) { console.error(`Errore: ${msg}\n\n${usage()}`); process.exit(2); }

// ---------------- helpers ----------------
const useColor = (opts) => opts.color && process.stdout.isTTY;
const c = (code, s, on) => (on ? `\x1b[${code}m${s}\x1b[0m` : s);

const numFmt = new Intl.NumberFormat('it-IT', { maximumFractionDigits: 1 });
function humanBytes(n) {
  const units = ['B', 'KiB', 'MiB', 'GiB', 'TiB', 'PiB'];
  let u = 0, v = typeof n === 'bigint' ? Number(n) : n;
  while (v >= 1024 && u < units.length - 1) { v /= 1024; u++; }
  return `${numFmt.format(v)} ${units[u]}`;
}
const pct = (part, total) => (total > 0 ? (Number(part) / Number(total)) * 100 : 0);
function bar(ratio, width = 20) {
  const filled = Math.max(ratio > 0 ? 1 : 0, Math.round(ratio * width));
  return '█'.repeat(Math.min(filled, width)) + '░'.repeat(Math.max(width - filled, 0));
}
function extOf(name) {
  const i = name.lastIndexOf('.');
  return i > 0 ? name.slice(i).toLowerCase() : '';
}
const extLabel = (e) => (e === '' ? '(senza estensione)' : e);

// mappa parentId -> array di nodi figli (riusata da report, ls e nav)
function childrenMap(R) {
  const m = new Map();
  for (const n of R.nodes) {
    if (n.parentId === null) continue;
    if (!m.has(n.parentId)) m.set(n.parentId, []);
    m.get(n.parentId).push(n);
  }
  return m;
}

// ---------------- async dir queue ----------------
class DirQueue {
  constructor() { this.q = []; this.waiters = []; this.closed = false; }
  push(x) { const w = this.waiters.shift(); if (w) w(x); else this.q.push(x); }
  async pop() {
    if (this.q.length) return this.q.shift();
    if (this.closed) return null;
    return new Promise((r) => this.waiters.push(r));
  }
  close() { this.closed = true; this.waiters.forEach((w) => w(null)); this.waiters = []; }
}

// ---------------- scan engine ----------------
async function scan(rootPath, opts, onProgress) {
  const rootStat = await stat(rootPath).catch(() => null);
  if (!rootStat) fail(`Percorso inesistente o non accessibile: ${rootPath}`);
  if (!rootStat.isDirectory()) fail(`Il percorso non è una cartella: ${rootPath}`);

  const queue = new DirQueue();
  const nodes = [];                 // tutti i nodi cartella (flat)
  const seenInodes = new Set();     // dedup hardlink
  const byExtension = new Map();    // ext -> {bytes, files}
  const errors = [];                // primi messaggi d'errore
  let files = 0, symlinks = 0, hardlinkedFiles = 0, totalBytes = 0n, totalBytesOnDisk = 0n;
  let inflight = 0, finished = false;

  const newNode = (parent, name, path) => {
    const n = { id: nodes.length, parentId: parent ? parent.id : null, name, path,
                bytes: 0n, bytesOnDisk: 0n, files: 0, dirs: 0, symlinks: 0, errors: 0, fileEntries: [] };
    nodes.push(n);
    return n;
  };

  // top file (bounded, ordinato desc per byte)
  const TOP_CAP = 500;
  const topFiles = [];
  const addTopFile = (f) => {
    if (topFiles.length >= TOP_CAP && f.bytes <= topFiles[topFiles.length - 1].bytes) return;
    let lo = 0, hi = topFiles.length;
    while (lo < hi) { const mid = (lo + hi) >> 1; if (topFiles[mid].bytes > f.bytes) lo = mid + 1; else hi = mid; }
    topFiles.splice(lo, 0, f);
    if (topFiles.length > TOP_CAP) topFiles.pop();
  };

  const recordFile = async (node, fullPath, name) => {
    const st = await stat(fullPath).catch((e) => {
      node.errors++;
      if (errors.length < 5) errors.push(`${fullPath}: ${e.code || e.message}`);
      return null;
    });
    if (!st) return;
    const size = BigInt(st.size);
    const onDisk = st.blocks > 0 ? BigInt(st.blocks) * 512n : size;
    if (st.nlink > 1 && st.ino !== 0) {          // hardlink: conta una sola volta
      const key = `${st.dev}:${st.ino}`;
      if (seenInodes.has(key)) { hardlinkedFiles++; return; }
      seenInodes.add(key);
    }
    node.bytes += size; node.bytesOnDisk += onDisk; node.files++;
    node.fileEntries.push({ name, bytes: st.size, hardlinks: st.nlink > 1 ? st.nlink : 1 });
    files++; totalBytes += size; totalBytesOnDisk += onDisk;
    const e = extOf(name);
    const agg = byExtension.get(e) || { bytes: 0n, files: 0 };
    agg.bytes += size; agg.files++; byExtension.set(e, agg);
    addTopFile({ folderId: node.id, name, path: fullPath, bytes: st.size,
                 mtime: st.mtime.toISOString(), hardlinks: st.nlink > 1 ? st.nlink : 1 });
  };

  const processEntry = async (node, fullPath, ent) => {
    if (ent.isSymbolicLink() && !opts.follow) { node.symlinks++; symlinks++; return; }
    if (ent.isDirectory() || (ent.isSymbolicLink() && opts.follow)) {
      const st = await stat(fullPath).catch(() => null);   // segue il link se --follow
      if (!st) { node.errors++; return; }
      if (st.isDirectory()) {
        node.dirs++;
        queue.push(newNode(node, ent.name, fullPath));
      } else {
        await recordFile(node, fullPath, ent.name);
      }
      return;
    }
    if (ent.isFile() || ent.isSymbolicLink()) { await recordFile(node, fullPath, ent.name); return; }
    // FIFO/socket/device: ignorati
  };

  const CHUNK = 64;   // stat in parallelo a blocchi, per non saturare la threadpool
  const processDir = async (node) => {
    inflight++;
    try {
      const entries = await readdir(node.path, { withFileTypes: true });
      for (let i = 0; i < entries.length; i += CHUNK) {
        const chunk = entries.slice(i, i + CHUNK);
        await Promise.all(chunk.map((ent) => processEntry(node, join(node.path, ent.name), ent)));
      }
    } catch (e) {
      node.errors++;
      if (errors.length < 5) errors.push(`${node.path}: ${e.code || e.message}`);
    } finally {
      inflight--;
      if (inflight === 0 && queue.q.length === 0) { finished = true; queue.close(); }
    }
  };

  const root = newNode(null, basename(rootPath) || rootPath, rootPath);
  queue.push(root);

  const WORKERS = Math.max(4, Math.min(32, platform().length > 0 ? cpus().length * 2 : 8));
  const progressTimer = setInterval(() => onProgress({ files, dirs: nodes.length, totalBytes }), 300);
  await Promise.all(Array.from({ length: WORKERS }, async () => {
    while (true) {
      const node = await queue.pop();
      if (node === null || finished) return;
      await processDir(node);
    }
  }));
  clearInterval(progressTimer);
  onProgress({ files, dirs: nodes.length, totalBytes, done: true });

  // aggregazione: propaga dimensioni dai figli ai padri (i padri hanno id minore)
  for (let i = nodes.length - 1; i > 0; i--) {
    const n = nodes[i];
    const p = nodes[n.parentId];
    if (p) { p.bytes += n.bytes; p.bytesOnDisk += n.bytesOnDisk; p.files += n.files;
             p.dirs += n.dirs; p.symlinks += n.symlinks; p.errors += n.errors; }
  }

  return { nodes, root, files, symlinks, hardlinkedFiles, totalBytes, totalBytesOnDisk,
           byExtension, errors, topFiles };
}

// ---------------- report ----------------
function renderReport(R, opts, startedAt, durationMs) {
  const co = useColor(opts);
  const out = [];
  const push = (s) => out.push(s);
  const title = (s) => push(c('1;36', s, co));
  const totalErrors = R.nodes.reduce((a, n) => a + n.errors, 0);
  const secs = durationMs / 1000;

  push(c('1;37', `${TOOL} v${TOOL_VERSION} — MantoFreeDisk`, co));
  push(`${c('1', 'Radice', co)}: ${R.root.path}`);
  push(`${c('1', 'Inizio', co)}: ${startedAt.toISOString()}   ${c('1', 'Durata', co)}: ${secs.toFixed(2)}s`);
  push('');

  title('─ RIEPILOGO ' + '─'.repeat(46));
  push(`  Dimensione logica ....... ${c('1;32', humanBytes(R.totalBytes), co)}`);
  push(`  Dimensione su disco ..... ${humanBytes(R.totalBytesOnDisk)}`);
  push(`  File .................... ${R.files.toLocaleString('it-IT')}`);
  push(`  Cartelle ................ ${(R.nodes.length - 1).toLocaleString('it-IT')}`);
  push(`  Symlink/junction ........ ${R.symlinks.toLocaleString('it-IT')}${opts.follow ? ' (seguiti)' : ' (non seguiti)'}`);
  if (R.hardlinkedFiles > 0) push(`  Hardlink aggiuntivi ..... ${R.hardlinkedFiles.toLocaleString('it-IT')} (contati 1 sola volta)`);
  push(`  Errori .................. ${totalErrors.toLocaleString('it-IT')}`);
  push(`  Velocità ................ ${humanBytes(Number(R.totalBytes) / secs)}/s, ${Math.round(R.files / secs)} file/s`);
  if (totalErrors > 0) R.errors.slice(0, 5).forEach((e) => push(`    ⚠ ${e}`));
  push('');

  // albero
  title('─ ALBERO (per dimensione, barra = % del totale) ' + '─'.repeat(6));
  const childrenOf = childrenMap(R);
  const byBytesDesc = (a, b) => (a.bytes < b.bytes ? 1 : a.bytes > b.bytes ? -1 : a.name.localeCompare(b.name));
  for (const list of childrenOf.values()) list.sort(byBytesDesc);

  push(`${c('1;33', R.root.name + sep, co)} 100,0% ${bar(1)} ${humanBytes(R.totalBytes)}  [${R.files.toLocaleString('it-IT')} file]`);
  const renderNode = (n, depth, prefix, isLast) => {
    const share = pct(n.bytes, R.totalBytes);
    push(`${prefix}${isLast ? '└─' : '├─'} ${n.name}${n.errors > 0 ? c('31', ' ⚠' + n.errors, co) : ''}  ${share.toFixed(1).replace('.', ',').padStart(5)}% ${bar(share / 100)} ${humanBytes(n.bytes)}`);
    if (depth >= opts.depth) return;
    const kids = childrenOf.get(n.id) || [];
    const shown = kids.slice(0, opts.top);
    shown.forEach((k, i) => renderNode(k, depth + 1, prefix + (isLast ? '   ' : '│  '), i === shown.length - 1));
    if (kids.length > shown.length) {
      const rest = kids.slice(opts.top).reduce((a, k) => a + k.bytes, 0n);
      push(`${prefix}${isLast ? '   ' : '│  '}   … altre ${kids.length - shown.length} cartelle  ${humanBytes(rest)}`);
    }
  };
  const rootKids = childrenOf.get(R.root.id) || [];
  const rootShown = rootKids.slice(0, opts.top);
  rootShown.forEach((k, i) => renderNode(k, 1, '', i === rootShown.length - 1));
  if (rootKids.length > rootShown.length) {
    const rest = rootKids.slice(opts.top).reduce((a, k) => a + k.bytes, 0n);
    push(`   … altre ${rootKids.length - rootShown.length} cartelle  ${humanBytes(rest)}`);
  }
  push('');

  // top file
  title(`─ TOP ${Math.min(opts.top, R.topFiles.length)} FILE PIÙ GRANDI ` + '─'.repeat(20));
  R.topFiles.slice(0, opts.top).forEach((f, i) => {
    push(`${String(i + 1).padStart(3)}. ${c('1;35', humanBytes(f.bytes), co).padEnd(14)} ${f.path}${f.hardlinks > 1 ? c('33', ` (x${f.hardlinks} hardlink)`, co) : ''}`);
  });
  push('');

  // top estensioni
  const exts = [...R.byExtension.entries()].map(([ext, v]) => ({ ext, ...v })).sort((a, b) => (a.bytes < b.bytes ? 1 : -1));
  title(`─ TOP ${Math.min(opts.top, exts.length)} ESTENSIONI ` + '─'.repeat(25));
  exts.slice(0, opts.top).forEach((e) => {
    push(`  ${extLabel(e.ext).padEnd(18)} ${humanBytes(e.bytes).padStart(12)}  ${bar(pct(e.bytes, R.totalBytes) / 100, 12)}  ${e.files.toLocaleString('it-IT')} file`);
  });
  push('');

  // top cartelle
  const folders = R.nodes.slice(1).sort((a, b) => (a.bytes < b.bytes ? 1 : -1)).slice(0, opts.top);
  title(`─ TOP ${folders.length} CARTELLE (per dimensione totale) ` + '─'.repeat(10));
  folders.forEach((f) => { push(`  ${humanBytes(f.bytes).padStart(12)}  ${f.path}`); });
  push('');
  push(c('2', `Snapshot riutilizzabile: aggiungi --json [file]  •  Piano: docs/PIANO-MantoFreeDisk.md`, co));
  return out.join('\n');
}

// ---------------- snapshot JSON ----------------
function buildSnapshot(R, opts, startedAt, durationMs) {
  return {
    schemaVersion: 1,
    tool: TOOL, toolVersion: TOOL_VERSION,
    root: R.root.path,
    startedAt, durationMs,
    options: { followSymlinks: opts.follow, countHardlinksOnce: true },
    stats: {
      totalBytes: Number(R.totalBytes), totalBytesOnDisk: Number(R.totalBytesOnDisk),
      files: R.files, dirs: R.nodes.length - 1, symlinks: R.symlinks,
      errors: R.nodes.reduce((a, n) => a + n.errors, 0), hardlinkedFiles: R.hardlinkedFiles,
      byExtension: [...R.byExtension.entries()]
        .map(([ext, v]) => ({ ext, bytes: Number(v.bytes), files: v.files }))
        .sort((a, b) => b.bytes - a.bytes)
    },
    folders: R.nodes.map((n) => ({ id: n.id, parentId: n.parentId, name: n.name,
      bytes: Number(n.bytes), bytesOnDisk: Number(n.bytesOnDisk), files: n.files,
      dirs: n.dirs, errors: n.errors })),
    files: R.topFiles
  };
}

// ---------------- modo ls: elenco aggregato one-shot ----------------

// voci visibili di una cartella: sotto-cartelle sempre, file solo con -f;
// applica --min (contando le escluse) e --sort
function lsEntries(node, cmap, opts) {
  const all = [];
  for (const d of cmap.get(node.id) || []) all.push({ type: 'dir', name: d.name, bytes: d.bytes, node: d });
  if (opts.files) for (const f of node.fileEntries) all.push({ type: 'file', name: f.name, bytes: BigInt(f.bytes), file: f });
  let belowMin = 0;
  const entries = all.filter((e) => {
    if (e.bytes >= opts.minBytes) return true;
    belowMin++;
    return false;
  });
  entries.sort(opts.sort === 'name'
    ? (a, b) => a.name.localeCompare(b.name, 'it')
    : (a, b) => (Number(b.bytes) - Number(a.bytes)) || a.name.localeCompare(b.name, 'it'));
  return { entries, belowMin };
}

function renderLs(R, opts, durationMs) {
  const co = useColor(opts);
  const cmap = childrenMap(R);
  const lines = [];
  const push = (s) => lines.push(s);
  const totalErrors = R.nodes.reduce((a, n) => a + n.errors, 0);
  const unlimited = opts.depth === 0;

  push(`${c('1;37', R.root.path, co)} — ${c('1;32', humanBytes(R.totalBytes), co)} · ${R.files.toLocaleString('it-IT')} file · ${(R.nodes.length - 1).toLocaleString('it-IT')} cartelle` +
       (R.symlinks > 0 ? ` · ${R.symlinks.toLocaleString('it-IT')} symlink` : '') +
       (R.hardlinkedFiles > 0 ? ` · ${R.hardlinkedFiles.toLocaleString('it-IT')} hardlink dedup` : '') +
       (totalErrors > 0 ? c('31', ` · ${totalErrors} errori`, co) : ''));
  push(c('2', `ls · profondità ${unlimited ? 'illimitata' : opts.depth} · ${(durationMs / 1000).toFixed(2)}s · % relativa alla cartella che contiene la voce`, co));
  push('');

  const renderFolder = (node, level, prefix) => {
    const { entries, belowMin } = lsEntries(node, cmap, opts);
    const shown = entries.slice(0, opts.top);
    const hasNotes = entries.length > opts.top || belowMin > 0;
    shown.forEach((e, i) => {
      const isLast = i === shown.length - 1 && !hasNotes;
      const share = pct(e.bytes, node.bytes);
      const label = e.type === 'dir' ? c('1;36', e.name + sep, co) : e.name;
      const warn = e.type === 'dir' && e.node.errors > 0 ? c('31', ` ⚠${e.node.errors}`, co) : '';
      push(`${prefix}${isLast ? '└─' : '├─'} ${label}${warn}  ${share.toFixed(1).replace('.', ',').padStart(5)}% ${bar(share / 100)} ${humanBytes(e.bytes)}`);
      if (e.type === 'dir' && (unlimited || level < opts.depth)) renderFolder(e.node, level + 1, prefix + (isLast ? '   ' : '│  '));
    });
    if (hasNotes) {
      const rest = entries.slice(opts.top);
      const parts = [];
      if (rest.length > 0) parts.push(`+${rest.length} voci (${humanBytes(rest.reduce((a, e) => a + e.bytes, 0n))})`);
      if (belowMin > 0) parts.push(`${belowMin} sotto ${humanBytes(opts.minBytes)}`);
      push(`${prefix}   ${c('2', `… ${parts.join(' · ')}`, co)}`);
    }
  };
  renderFolder(R.root, 1, '');

  if (totalErrors > 0) {
    push('');
    R.errors.slice(0, 5).forEach((e) => push(c('31', `⚠ ${e}`, co)));
  }
  return lines.join('\n');
}

function buildLsJson(R, opts, startedAt, durationMs) {
  const cmap = childrenMap(R);
  const unlimited = opts.depth === 0;
  const conv = (node, level) => {
    const { entries, belowMin } = lsEntries(node, cmap, opts);
    const shown = entries.slice(0, opts.top);
    const rest = entries.slice(opts.top);
    const o = { name: node.name, type: 'dir', bytes: Number(node.bytes), files: node.files, dirs: node.dirs, errors: node.errors };
    if (unlimited || level < opts.depth) {
      o.children = shown.map((e) => (e.type === 'dir'
        ? conv(e.node, level + 1)
        : { name: e.name, type: 'file', bytes: Number(e.bytes), hardlinks: e.file.hardlinks }));
    }
    if (rest.length + belowMin > 0) {
      o.omitted = { count: rest.length + belowMin, bytes: Number(rest.reduce((a, e) => a + e.bytes, 0n)) };
    }
    return o;
  };
  return {
    schemaVersion: 1,
    tool: TOOL, toolVersion: TOOL_VERSION,
    root: R.root.path,
    startedAt, durationMs,
    options: { mode: 'ls', depth: opts.depth, files: opts.files, minBytes: Number(opts.minBytes), top: opts.top, sort: opts.sort, followSymlinks: opts.follow, countHardlinksOnce: true },
    stats: {
      totalBytes: Number(R.totalBytes), totalBytesOnDisk: Number(R.totalBytesOnDisk),
      files: R.files, dirs: R.nodes.length - 1, symlinks: R.symlinks,
      errors: R.nodes.reduce((a, n) => a + n.errors, 0), hardlinkedFiles: R.hardlinkedFiles,
    },
    tree: conv(R.root, 0),
  };
}

// ---------------- modo nav: TUI interattiva ----------------

const NAV_FOOTER = '↑↓ muovi · →/Invio entra · ← indietro · q esci';

// traduce un chunk di stdin raw in azione ('up'|'down'|'enter'|'leave'|'home'|'end'|
// 'pageup'|'pagedown'|'quit'|null) — funzione pura, testabile senza TTY
function parseNavKey(b) {
  if (!(b && b.length)) return null;
  if (b[0] === 0x03) return 'quit';                                    // Ctrl+C
  if (b[0] === 0x1b) {
    if (b.length === 1) return 'quit';                                 // Esc
    if (b[1] === 0x5b || b[1] === 0x4f) {                              // CSI / SS3
      const c = b[2];
      if (c === 0x41) return 'up';
      if (c === 0x42) return 'down';
      if (c === 0x43) return 'enter';
      if (c === 0x44) return 'leave';
      if (c === 0x35) return 'pageup';                                 // PgUp  \x1b[5~
      if (c === 0x36) return 'pagedown';                               // PgDn  \x1b[6~
      if (c === 0x48 || (c === 0x31 && b[3] === 0x7e)) return 'home';  // Home
      if (c === 0x46 || (c === 0x34 && b[3] === 0x7e)) return 'end';   // End
      return null;
    }
    return null;
  }
  const ch = b.toString('utf8');
  if (ch === 'q') return 'quit';
  if (ch === '\r' || ch === '\n' || ch === 'l') return 'enter';
  if (ch === '\x7f' || ch === '\b' || ch === 'h') return 'leave';
  if (ch === 'j') return 'down';
  if (ch === 'k') return 'up';
  return null;
}

// elenco cartelle+file di un nodo, ordinato per dimensione desc (con cache per nodo)
function navListing(node, cmap, cache) {
  let l = cache.get(node.id);
  if (l) return l;
  const entries = [];
  for (const d of cmap.get(node.id) || []) entries.push({ type: 'dir', name: d.name, bytes: d.bytes, node: d });
  for (const f of node.fileEntries) entries.push({ type: 'file', name: f.name, bytes: BigInt(f.bytes), file: f });
  entries.sort((a, b) => (Number(b.bytes) - Number(a.bytes)) || a.name.localeCompare(b.name, 'it'));
  cache.set(node.id, entries);
  return entries;
}

// disegna una schermata della TUI — funzione pura (testabile senza TTY)
function renderNavScreen(R, cur, entries, sel, offset, co, width = 80, height = 24) {
  const W = Math.max(40, width);
  const rows = Math.max(1, height - 5);
  const lines = [];
  const crumb = cur.path.length > W - 12 ? '…' + cur.path.slice(-(W - 13)) : cur.path;
  const title = ` mfd nav — ${crumb} `;
  lines.push(co ? '\x1b[7m' + title.slice(0, W).padEnd(W) + '\x1b[0m' : title.slice(0, W));
  const shareRoot = pct(cur.bytes, R.root.bytes);
  const info = ` ${humanBytes(cur.bytes)} · ${cur.files.toLocaleString('it-IT')} file · ${cur.dirs.toLocaleString('it-IT')} cartelle · ${shareRoot.toFixed(1).replace('.', ',')}% della radice${cur.errors > 0 ? ` · ⚠${cur.errors} errori` : ''}`;
  lines.push(info.slice(0, W));
  lines.push('─'.repeat(W));

  for (let i = 0; i < rows; i++) {
    const idx = offset + i;
    if (idx >= entries.length) { lines.push(''); continue; }
    const e = entries[idx];
    const share = pct(e.bytes, cur.bytes);
    const name = e.type === 'dir' ? e.name + sep : e.name;
    const mark = e.type === 'dir' && e.node.errors > 0 ? ' ⚠' + e.node.errors : '';
    const text = ` ${bar(share / 100, 12)} ${humanBytes(e.bytes).padStart(10)} ${share.toFixed(1).replace('.', ',').padStart(5)}% ${name}${mark}`;
    const cut = text.length > W ? text.slice(0, W - 1) + '…' : text;
    if (sel === idx) lines.push(co ? '\x1b[7m' + cut.padEnd(W) + '\x1b[0m' : '>' + cut);
    else lines.push(cut);
  }

  lines.push('─'.repeat(W));
  const pos = entries.length > 0 ? `   ${Math.min(sel + 1, entries.length)}/${entries.length}` : '';
  lines.push((' ' + NAV_FOOTER + pos).slice(0, W));
  return lines.join('\n');
}

// loop interattivo: scan già fatto, si naviga lo snapshot in memoria (istantaneo)
async function runNav(R, opts) {
  if (!process.stdin.isTTY || !process.stdout.isTTY) {
    fail('nav richiede un terminale interattivo (TTY). Per un output non interattivo usa: mfd-scan ls <path>');
  }
  const co = opts.color;
  const cmap = childrenMap(R);
  const cache = new Map();
  let cur = R.root, sel = 0, offset = 0, inAlt = false;

  const height = () => process.stdout.rows || 24;
  const width = () => process.stdout.columns || 80;
  const rows = () => Math.max(1, height() - 5);

  const draw = () => {
    const entries = navListing(cur, cmap, cache);
    sel = Math.max(0, Math.min(sel, Math.max(entries.length - 1, 0)));
    const r = rows();
    if (sel < offset) offset = sel;
    if (sel > offset + r - 1) offset = sel - r + 1;
    offset = Math.max(0, Math.min(offset, Math.max(0, entries.length - r)));
    process.stdout.write('\x1b[H\x1b[2J' + renderNavScreen(R, cur, entries, sel, offset, co, width(), height()));
  };

  return await new Promise((resolve) => {
    const restore = () => {
      process.stdin.removeListener('data', onKey);
      process.stdout.removeListener('resize', draw);
      try { process.stdin.setRawMode(false); } catch { /* stdin già chiuso */ }
      process.stdin.pause();
      if (inAlt) { process.stdout.write('\x1b[?25h\x1b[?1049l'); inAlt = false; }
    };
    const quit = () => { restore(); resolve(); };

    const move = (delta) => { sel += delta; };
    const page = (delta) => { sel += delta * rows(); };
    const enter = () => {
      const e = navListing(cur, cmap, cache)[sel];
      if (e && e.type === 'dir') { cur = e.node; sel = 0; offset = 0; }
    };
    const leave = () => {
      if (cur.parentId !== null) { cur = R.nodes[cur.parentId]; sel = 0; offset = 0; }
    };
    const onKey = (chunk) => {
      switch (parseNavKey(chunk)) {
        case 'quit': return quit();
        case 'up': move(-1); break;
        case 'down': move(1); break;
        case 'enter': enter(); break;
        case 'leave': leave(); break;
        case 'home': sel = 0; break;
        case 'end': sel = Number.MAX_SAFE_INTEGER; break;
        case 'pageup': page(-1); break;
        case 'pagedown': page(1); break;
        default: return;             // tasto non gestito: nessun ridisegno
      }
      draw();
    };

    process.stdin.setRawMode(true);
    process.stdin.resume();
    process.stdout.write('\x1b[?1049h\x1b[?25l');      // schermo alternativo, cursore nascosto
    inAlt = true;
    process.stdin.on('data', onKey);
    process.stdout.on('resize', draw);
    process.on('exit', () => { if (inAlt) process.stdout.write('\x1b[?25h\x1b[?1049l'); });
    draw();
  });
}

// ---------------- main ----------------
async function main() {
  const opts = parseArgs(process.argv.slice(2));
  if (opts.help) { console.log(usage()); return; }
  if (opts.version) { console.log(`${TOOL} ${TOOL_VERSION} (node ${nodeVersion})`); return; }
  const rootPath = resolve(opts.root);
  const startedAt = new Date();
  const onProgress = (p) => {
    if (!process.stderr.isTTY) return;
    if (p.done) { process.stderr.write('\r' + ' '.repeat(72) + '\r'); return; }
    const line = `⏳ ${p.files.toLocaleString('it-IT')} file · ${p.dirs.toLocaleString('it-IT')} cartelle · ${humanBytes(p.totalBytes)}`;
    process.stderr.write('\r' + line.slice(0, 72).padEnd(72));
  };
  const R = await scan(rootPath, opts, onProgress);
  const durationMs = Date.now() - startedAt.getTime();

  if (opts.cmd === 'ls') {
    if (opts.json) {
      const json = JSON.stringify(buildLsJson(R, opts, startedAt.toISOString(), durationMs), null, opts.jsonFile ? 1 : 0);
      if (opts.jsonFile) {
        await writeFile(opts.jsonFile, json, 'utf8');
        console.error(`✓ Albero JSON scritto: ${opts.jsonFile} (${(json.length / 1024).toFixed(1)} KiB)`);
      } else {
        console.log(json);
      }
    } else {
      console.log(renderLs(R, opts, durationMs));
    }
    return;
  }

  if (opts.cmd === 'nav') {
    await runNav(R, opts);
    console.log(`mfd nav — chiuso · ${R.root.path} — ${humanBytes(R.root.bytes)} · ${R.files.toLocaleString('it-IT')} file · scan in ${(durationMs / 1000).toFixed(2)}s`);
    return;
  }

  if (opts.json) {
    const snapshot = buildSnapshot(R, opts, startedAt.toISOString(), durationMs);
    const json = JSON.stringify(snapshot, null, opts.jsonFile ? 1 : 0);
    if (opts.jsonFile) {
      await writeFile(opts.jsonFile, json, 'utf8');
      console.error(`✓ Snapshot scritto: ${opts.jsonFile} (${(json.length / 1024).toFixed(1)} KiB)`);
    } else {
      console.log(json);
    }
  } else {
    console.log(renderReport(R, opts, startedAt, durationMs));
  }
}

// Avvio solo se eseguito direttamente come script (i test importano i moduli senza far partire main)
function invokedAsCli() {
  if (!process.argv[1]) return false;
  const self = pathToFileURL(resolve(process.argv[1])).href;
  return platform() === 'win32'
    ? self.toLowerCase() === import.meta.url.toLowerCase()
    : self === import.meta.url;
}
if (invokedAsCli()) main().catch((e) => { console.error(e); process.exit(1); });

// Esportati per i test (node test-mfd.mjs)
export { parseSize, parseNavKey, childrenMap, lsEntries, renderLs, buildLsJson, renderNavScreen };






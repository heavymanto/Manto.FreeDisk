#!/usr/bin/env node
// walker-node — micro-benchmark MFD (decisione stack fase 1, vedi docs/PIANO-MantoFreeDisk.md §11.6)
// Walker parallelo minimale idiomatico Node. Punto architetturale: readdir non restituisce
// le dimensioni → serve una stat per file (libuv su Windows apre il file per riempire
// anche dev:ino). Logica identica a bench/walker-cs: somma dimensioni logiche, conta
// file/cartelle, salta symlink/junction, conta errori di accesso.
// Uso: node walker-node.mjs <path>   (stopwatch interno, avvio runtime escluso)
import { readdir, stat } from 'node:fs/promises';
import { join } from 'node:path';
import { cpus } from 'node:os';

if (process.argv.length < 3) {
  console.error('uso: node walker-node.mjs <path>');
  process.exit(2);
}
const rootPath = process.argv[2];

const t0 = performance.now();
let bytes = 0, files = 0, dirs = 0, errors = 0;

// coda con waiters (nessun polling) — stesso pattern del motore di app/mfd-scan.mjs
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

const queue = new DirQueue();
queue.push(rootPath);
let inflight = 0, finished = false;

const CHUNK = 64;   // stat in parallelo a blocchi, per non saturare la threadpool (pattern mfd-scan)
async function processDir(dir) {
  inflight++;
  try {
    const entries = await readdir(dir, { withFileTypes: true });
    for (let i = 0; i < entries.length; i += CHUNK) {
      const chunk = entries.slice(i, i + CHUNK);
      await Promise.all(chunk.map(async (ent) => {
        const p = join(dir, ent.name);
        if (ent.isSymbolicLink()) return;             // junction/symlink: skip (parità con reparse skip C#)
        if (ent.isDirectory()) { dirs++; queue.push(p); return; }
        if (!ent.isFile()) return;                    // FIFO/socket/device: ignorati
        const st = await stat(p).catch(() => null);
        if (!st) { errors++; return; }
        bytes += st.size;
        files++;
      }));
    }
  } catch {
    errors++;
  } finally {
    inflight--;
    if (inflight === 0 && queue.q.length === 0) { finished = true; queue.close(); }
  }
}

const WORKERS = Math.max(4, Math.min(32, cpus().length * 2));
await Promise.all(Array.from({ length: WORKERS }, async () => {
  while (true) {
    const dir = await queue.pop();
    if (dir === null || finished) return;
    await processDir(dir);
  }
}));

const ms = performance.now() - t0;
console.log(`RESULT ms=${ms.toFixed(0)} bytes=${bytes} files=${files} dirs=${dirs} errors=${errors}`);

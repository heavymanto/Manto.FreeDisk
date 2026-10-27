#!/usr/bin/env node
// test-mfd.mjs — test zero-dipendenze per mfd-scan (v0.2+)
// Uso: node test-mfd.mjs   (dalla cartella app/)
// Costruisce una fixture in %TEMP% (dimensioni note, hardlink, junction/symlink),
// lancia la CLI in sottoprocessi e verifica output umano + contratti JSON.

import { spawnSync } from 'node:child_process';
import { mkdtempSync, mkdirSync, writeFileSync, rmSync, symlinkSync, linkSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, dirname, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { strict as assert } from 'node:assert';

import { parseSize, parseNavKey, renderNavScreen } from './mfd-scan.mjs';

const HERE = dirname(fileURLToPath(import.meta.url));
const MFD = join(HERE, 'mfd-scan.mjs');
const NODE = process.execPath;

const MiB = 1024 * 1024;
const KiB = 1024;

function run(args) {
  return spawnSync(NODE, [MFD, ...args], { encoding: 'utf8' });
}

// ---------- fixture ----------
const root = mkdtempSync(join(tmpdir(), 'mfd-test-'));
const files = {
  'big.bin': 5 * MiB,
  'small.txt': 100 * KiB,
  'docs/a.pdf': 2 * MiB,
  'docs/b.pdf': 500 * KiB,
  'media/video.mkv': 8 * MiB,
  'media/sub/deep.bin': 1 * MiB,
  'hard1.bin': 1 * MiB,
};
for (const [rel, size] of Object.entries(files)) {
  const p = join(root, rel);
  mkdirSync(dirname(p), { recursive: true });
  writeFileSync(p, Buffer.alloc(size, 1));
}
linkSync(join(root, 'hard1.bin'), join(root, 'hard2.bin'));            // hardlink → dedup
if (process.platform === 'win32') {
  symlinkSync(join(root, 'docs'), join(root, 'junction'), 'junction'); // junction → rilevata, non seguita
} else {
  symlinkSync(join(root, 'docs'), join(root, 'junction'));
}
const TOTAL = Object.values(files).reduce((a, b) => a + b, 0);         // hard2 escluso (dedup)

// ---------- harness ----------
let passed = 0, failed = 0;
const failures = [];
function test(name, fn) {
  try { fn(); passed++; console.log(`  ✓ ${name}`); }
  catch (e) { failed++; failures.push([name, e]); console.error(`  ✗ ${name}\n      ${String(e.message).split('\n')[0]}`); }
}

console.log(`Fixture: ${root} — attesi ${(TOTAL / MiB).toFixed(2)} MiB, 7 file, 3+1 cartelle\n`);

console.log('Unit — parseSize:');
test('unità decimali e binarie (KB/MB/GiB/none/spazi)', () => {
  assert.equal(parseSize('10MB'), 10000000n);
  assert.equal(parseSize('1.5GiB'), 1610612736n);
  assert.equal(parseSize('1,5GiB'), 1610612736n);
  assert.equal(parseSize('500KB'), 500000n);
  assert.equal(parseSize('1048576'), 1048576n);
  assert.equal(parseSize('2 MiB'), 2097152n);
});
test('input non validi → null', () => {
  assert.equal(parseSize('abc'), null);
  assert.equal(parseSize('10XB'), null);
  assert.equal(parseSize('-5MB'), null);
  assert.equal(parseSize(undefined), null);
});

console.log('Unit — parseNavKey:');
test('frecce, paginazione, Home/End, quit, alias vim', () => {
  const K = (s) => Buffer.from(s, 'binary');
  assert.equal(parseNavKey(K('\x1b[A')), 'up');
  assert.equal(parseNavKey(K('\x1b[B')), 'down');
  assert.equal(parseNavKey(K('\x1b[C')), 'enter');
  assert.equal(parseNavKey(K('\x1b[D')), 'leave');
  assert.equal(parseNavKey(K('\x1b[5~')), 'pageup');
  assert.equal(parseNavKey(K('\x1b[6~')), 'pagedown');
  assert.equal(parseNavKey(K('\x1b[H')), 'home');
  assert.equal(parseNavKey(K('\x1b[F')), 'end');
  assert.equal(parseNavKey(K('\x1b')), 'quit');
  assert.equal(parseNavKey(K('\x03')), 'quit');
  assert.equal(parseNavKey(K('q')), 'quit');
  assert.equal(parseNavKey(K('\r')), 'enter');
  assert.equal(parseNavKey(K('\x7f')), 'leave');
  assert.equal(parseNavKey(K('j')), 'down');
  assert.equal(parseNavKey(K('k')), 'up');
  assert.equal(parseNavKey(K('h')), 'leave');
  assert.equal(parseNavKey(K('l')), 'enter');
  assert.equal(parseNavKey(K('x')), null);
});

console.log('Unit — renderNavScreen (funzione pura):');
test('intestazione, righe, percentuali e selezione', () => {
  const R = { root: { bytes: 10000n } };
  const cur = { path: 'X:\\radice\\sub', bytes: 6000n, files: 3, dirs: 1, errors: 0 };
  const entries = [
    { type: 'dir', name: 'cartella', bytes: 5000n, node: {} },
    { type: 'file', name: 'file.txt', bytes: 1000n, file: {} },
  ];
  const s = renderNavScreen(R, cur, entries, 0, 0, false, 60, 10);
  assert.ok(s.includes('mfd nav'), 'manca il titolo');
  assert.ok(s.includes('cartella' + sep), 'manca la cartella');
  assert.ok(s.includes('file.txt'), 'manca il file');
  assert.ok(s.includes('60,0%'), 'manca la % della radice');   // 6000/10000
  assert.ok(s.includes('83,3'), 'manca 5000/6000');
  assert.ok(s.includes('16,7'), 'manca 1000/6000');
  assert.ok(s.split('\n').some((l) => l.startsWith('>')), 'manca il marcatore di selezione (no-color)');
});
test('viewport con scroll: solo le righe visibili', () => {
  const R = { root: { bytes: 10000n } };
  const cur = { path: 'R', bytes: 10000n, files: 5, dirs: 0, errors: 0 };
  const entries = Array.from({ length: 5 }, (_, i) => ({ type: 'file', name: `f${i}.bin`, bytes: 1000n, file: {} }));
  const s = renderNavScreen(R, cur, entries, 4, 2, false, 60, 8);   // rows = 3
  const lines = s.split('\n');
  assert.ok(lines.some((l) => l.includes('f2.bin')), 'offset non rispettato');
  assert.ok(!lines.some((l) => l.includes('f0.bin')), 'f0 fuori viewport non deve apparire');
  assert.ok(lines.some((l) => l.includes('f4.bin')), 'selezionato non visibile');
  assert.ok(lines.some((l) => l.includes('5/5')), 'manca indicatore di posizione');
});

console.log('CLI — report (regressione):');
test('report completo: riepilogo e totale corretto', () => {
  const r = run([root]);
  assert.equal(r.status, 0, r.stderr);
  assert.ok(r.stdout.includes('RIEPILOGO'));
  assert.ok(r.stdout.includes('17,6 MiB'));
});
test('report --json: snapshot v1 invariato', () => {
  const r = run([root, '--json']);
  assert.equal(r.status, 0, r.stderr);
  const snap = JSON.parse(r.stdout);
  assert.equal(snap.schemaVersion, 1);
  assert.equal(snap.stats.totalBytes, TOTAL);
  assert.equal(snap.stats.files, 7);
  assert.equal(snap.stats.hardlinkedFiles, 1);
  assert.ok(snap.stats.symlinks >= 1);
  assert.equal(snap.folders.length, 4);        // root, docs, media, media/sub
});

console.log('CLI — ls:');
test('default: solo cartelle di primo livello, totale in header', () => {
  const r = run(['ls', root]);
  assert.equal(r.status, 0, r.stderr);
  assert.ok(r.stdout.includes('docs' + sep));
  assert.ok(r.stdout.includes('media' + sep));
  assert.ok(!r.stdout.includes('big.bin'), 'senza -f non devono apparire i file');
  assert.ok(!r.stdout.includes('sub' + sep), 'con depth 1 non deve apparire il livello 2');
  assert.ok(r.stdout.includes('17,6 MiB'));
});
test('-f: include i file e dedup degli hardlink', () => {
  const r = run(['ls', root, '-f', '-d', '2']);
  assert.equal(r.status, 0, r.stderr);
  assert.ok(r.stdout.includes('big.bin'));
  assert.ok(r.stdout.includes('video.mkv'));
  assert.ok(r.stdout.includes('a.pdf'));
  assert.ok(r.stdout.includes('hard1.bin') !== r.stdout.includes('hard2.bin'), 'deve apparire uno solo dei due hardlink');
  assert.ok(!r.stdout.includes('junction'), 'lo symlink non seguito non va listato');
});
test('-d 2: scende di due livelli', () => {
  const r = run(['ls', root, '-d', '2']);
  assert.equal(r.status, 0, r.stderr);
  assert.ok(r.stdout.includes('sub' + sep));
});
test('-d 0 -f: profondità illimitata', () => {
  const r = run(['ls', root, '-f', '-d', '0']);
  assert.equal(r.status, 0, r.stderr);
  assert.ok(r.stdout.includes('deep.bin'));
});
test('--min: filtra le voci piccole ma non i totali', () => {
  const r = run(['ls', root, '-f', '--min', '1MiB']);
  assert.equal(r.status, 0, r.stderr);
  assert.ok(r.stdout.includes('17,6 MiB'), 'il totale in header non deve cambiare');
  assert.ok(!r.stdout.includes('small.txt'));
  assert.ok(!r.stdout.includes('b.pdf'));
  assert.ok(r.stdout.includes('big.bin'));
  assert.ok(r.stdout.includes('1 sotto'), 'manca la nota sulle voci escluse');
});
test('--top 1: troncamento per cartella con nota di omissione', () => {
  const r = run(['ls', root, '-f', '--top', '1']);
  assert.equal(r.status, 0, r.stderr);
  assert.ok(r.stdout.includes('…'), 'manca la nota di omissione');
  assert.ok(r.stdout.includes('+4 voci'));
});
test('--sort name: ordinamento alfabetico', () => {
  const r = run(['ls', root, '--sort', 'name']);
  assert.equal(r.status, 0, r.stderr);
  const iDocs = r.stdout.indexOf('docs' + sep);
  const iMedia = r.stdout.indexOf('media' + sep);
  assert.ok(iDocs > -1 && iMedia > -1 && iDocs < iMedia, 'docs deve precedere media');
});

test('--json: contratto annidato e invariante somma figli', () => {
  const r = run(['ls', root, '-f', '-d', '0', '--top', '1000', '--json']);
  assert.equal(r.status, 0, r.stderr);
  const j = JSON.parse(r.stdout);
  assert.equal(j.schemaVersion, 1);
  assert.equal(j.options.mode, 'ls');
  assert.equal(j.stats.totalBytes, TOTAL);
  assert.equal(j.stats.files, 7);
  const check = (node) => {
    if (node.children) {
      const sum = node.children.reduce((a, c) => a + c.bytes, 0);
      assert.equal(sum, node.bytes, `somma figli di ${node.name}`);
      node.children.filter((c) => c.type === 'dir').forEach(check);
    }
  };
  check(j.tree);
  assert.ok(!j.tree.omitted, 'con --top 1000 non deve esserci omissione');
  assert.ok(j.tree.children.some((c) => c.name === 'media' && c.type === 'dir'));
  assert.ok(j.tree.children.some((c) => c.name.startsWith('hard') && c.type === 'file'));
});
test('--json --min: campo omitted popolato', () => {
  const r = run(['ls', root, '-f', '--min', '2MiB', '--json']);
  assert.equal(r.status, 0, r.stderr);
  const j = JSON.parse(r.stdout);
  assert.ok(j.tree.omitted && j.tree.omitted.count > 0);
});

console.log('CLI — nav e varie:');
test('nav senza TTY: errore chiaro e codice 2', () => {
  const r = run(['nav', root]);
  assert.equal(r.status, 2);
  assert.ok(r.stderr.includes('TTY') || r.stderr.includes('ls'));
});
test('--help documenta ls e nav', () => {
  const r = run(['--help']);
  assert.equal(r.status, 0);
  assert.ok(r.stdout.includes('ls'));
  assert.ok(r.stdout.includes('nav'));
});
test('--version riporta 0.2.0', () => {
  const r = run(['--version']);
  assert.ok(r.stdout.includes('0.2.0'));
});
test('opzione fuori modo rifiutata (nav --json)', () => {
  const r = run(['nav', root, '--json']);
  assert.equal(r.status, 2);
});

// ---------- fine ----------
try { rmSync(root, { recursive: true, force: true }); } catch { /* best effort */ }
console.log(`\nRisultato: ${passed} superati, ${failed} falliti su ${passed + failed}`);
if (failed > 0) {
  for (const [n, e] of failures) console.error(`\n--- ${n} ---\n${e.stack}`);
  process.exit(1);
}
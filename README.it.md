<table>
  <tr>
    <td width="75%" valign="middle">
      <h1>Manto.FreeDisk</h1>
      <h3>Dove e' finito lo spazio sul disco?</h3>
      <p><strong><em>Un motore, due frontend da riga di comando — e un esperimento aperto: lo stesso walker parallelo in Rust e C#, misurato uno contro l'altro.</em></strong></p>
      <p>
        <img src="https://img.shields.io/badge/0-STAT%20CALLS-e3b341?style=for-the-badge" alt="0 stat" />
        <img src="https://img.shields.io/badge/WORKERS-ONE%20PER%20CORE-e3b341?style=for-the-badge" alt="un worker per core" />
        <img src="https://img.shields.io/badge/OUTPUT-DUF%20%2B%20ASCII-e3b341?style=for-the-badge" alt="look duf piu' contratto ASCII" />
      </p>
      <p>
        <a href="https://github.com/heavymanto/Manto.FreeDisk/actions/workflows/ci.yml"><img src="https://github.com/heavymanto/Manto.FreeDisk/actions/workflows/ci.yml/badge.svg" alt="CI" /></a>
        <img src="https://img.shields.io/badge/walk-181.513%20file%20in%201%2C4%20s-brightgreen" alt="181.513 file in 1,4 s" />
        <img src="https://img.shields.io/badge/frontends-mfd--cli%200.8.0%20%C2%B7%20mfd--tui%200.2.1-blue" alt="mfd-cli 0.8.0 e mfd-tui 0.2.1" />
        <img src="https://img.shields.io/badge/lingue-7-blue" alt="sette lingue" />
        <img src="https://img.shields.io/badge/licenza-AGPL--3.0-brightgreen" alt="AGPL-3.0" />
      </p>
      <p><sub>📖 <a href="docs/FAQ.md"><b>Il manuale</b></a> · 🔧 <a href="#installazione"><b>Installazione</b></a> · 🇬🇧 <a href="README.md"><b>Read it in English</b></a></sub></p>
    </td>
    <td width="25%" valign="middle" align="center">
      <img src="docs/img/MNT_C1.png" alt="Manto.FreeDisk" width="150" />
    </td>
  </tr>
</table>

---

L'idea e' nata da un fastidio vero. In un momento in cui le memorie
costavano tantissimo — e un disco piu' capiente non me lo potevo
permettere — continuavo a chiedermi perche' fosse sempre pieno, anche
dopo aver fatto spazio con tutti i tool piu' noti. Nessuno pero' mi
dava una risposta in un colpo d'occhio, o con un comando diretto.
Stanco di risultati scarsi, ho deciso di farmelo da solo. Avrei potuto
scriverlo in C# — il mio linguaggio preferito, che conosco da vent'anni
— ma ho voluto cimentarmi in un po' di vibe coding e provare Rust. Mi
ha aiutato GLM 5.3, open weight, on-premise. E ha funzionato: col mio
tool ho trovato subito file enormi dimenticati — backup, macchine
virtuali non piu' usate — e individuato un bel 30% di dati vecchi da
cancellare. L'esperimento qui sotto racconta il resto della storia.

**Manto.FreeDisk** e' un analizzatore di spazio disco costruito attorno a
una domanda sola: *dove sono finiti i miei gigabyte*. Scansiona un albero
con un worker per core, legge le dimensioni dei file direttamente dai dati
di enumerazione delle cartelle (niente `stat` per file, su Windows), e li
trasforma in una griglia colorata, una classifica dei file piu' grossi, un
albero navigabile — o una fotografia JSON che le macchine sanno fare a
diff.

E' nato come esperimento aperto: lo stesso walker parallelo scritto tre
volte — Node (il prototipo della fase 0, oggi congelato), Rust e una proof
of concept in C# — stesse cartelle, stesso protocollo, totali incrociati:

> Rust e C# girano alla stessa velocita' — questa sessione ha il C# JIT
> persino un capello davanti. Node resta ~6 volte dietro. La RAM e'
> dove si dividono: Rust percorre l'albero con 12,5 MB, il walker C#
> con ~59, Node con ~251 — un quinto, un ventesimo. Questo, e un
> prodotto da spedire, e' il perche' Rust e' il motore; il gemello C#
> oggi e' una proof of concept — il walker, misurato, senza la CLI e
> la TUI attorno: la sua promozione e' il prossimo capitolo del
> roadmap. Node resta congelato come prototipo della fase 0. Una
> macchina, una sessione, gli output grezzi in
> [bench/results/](bench/results); il metodo in
> [docs/BENCHMARKS.md](docs/BENCHMARKS.md).

```text
> powershell -ExecutionPolicy Bypass -File bench\run-bench.ps1 -Target C:\Windows -Runs 5
> powershell -ExecutionPolicy Bypass -File bench\mem-bench.ps1 -Target C:\Windows -Runs 5
```

| Implementazione | CPU, mediana (ms) | RAM, picco working set |
| --- | ---: | ---: |
| c# JIT (dimensioni dall'enumerazione, no stat) | 1378 | 59,3 MB |
| c# R2R (ReadyToRun, zero JIT) | 1464 | 59,2 MB |
| **rust (find-data via std::fs, no stat)** | **1462** | **12,5 MB** |
| node (readdir+stat per file, libuv) | 8531 | 251,3 MB |

181.513 file, 102.998 cartelle, totali incrociati `[OK]` su ogni riga: la
CPU e' un pareggio, la RAM no — e quello e' il verdetto dell'esperimento,
i dettagli piu' sotto.

## Scaricalo

Niente da installare: estrai ed esegui — e basta. I due binari
funzionano subito, in modalita' portabile: l'inglese e' incorporato, e
le altre sei lingue vengono trovate in `lang/` accanto ai binari.

[![Windows x64](https://img.shields.io/badge/%E2%AC%87-Windows%20x64-2ea44f?style=for-the-badge)](../../releases/download/v0.8.0/mantofreedisk-0.8.0-win-x64.zip)
[![Linux x64](https://img.shields.io/badge/%E2%AC%87-Linux%20x64-2ea44f?style=for-the-badge)](../../releases/download/v0.8.0/mantofreedisk-0.8.0-linux-x64.tar.gz)
[![macOS arm64](https://img.shields.io/badge/%E2%AC%87-macOS%20arm64-2ea44f?style=for-the-badge)](../../releases/download/v0.8.0-mantofreedisk-0.8.0-macos-aarch64.tar.gz)

**Opzionale**: se vuoi i comandi disponibili ovunque (non solo dalla
cartella estratta), lancia `install.ps1` (Windows) o `install.sh`
(Linux). Un comando, due secondi, fatto: `mfd-cli` e `mfd-tui`
funzionano da qualsiasi terminale, con tutte le lingue. Senza lanciarlo
funziona comunque tutto — ti sposti solo nella cartella prima di
usarli. Lo SHA256 sta accanto a ogni file. macOS arriva dopo.

## Installazione

Serve [Rust](https://rustup.rs) (`cargo --version`) e un clone di questo
repository. Se Rust non c'e' ancora: `winget install Rustlang.Rustup`, poi
aprite un terminale **nuovo** — quelli gia' aperti non vedono il PATH che
rustup ha appena scritto. Tutto il resto sono due comandi:

```powershell
cd rust
cargo build --release     # circa un minuto → mfd-cli.exe e mfd-tui.exe in target\release\
.\target\release\mfd-cli.exe -big 10 C:\Windows
```

Poi fate la domanda per cui questo tool esiste — dove e' finito lo
spazio? — e leggete la risposta: in classifica, full path, ordine fisso di
taglia decrescente. Quello e' Windows visto da dentro: un pacchetto
d'installazione da 348,5 MB in cima, e `msedge.dll` quattro volte, perche'
WinSxS conserva le versioni dei componenti. Aggiungete `-json` e la stessa
fotografia arriva in JSON (`mfd-cli -big 10 -json > big.json`).

<div align="center">
<br />
<img src="docs/img/cli-big.png" alt="I dieci file piu' grandi di C:\Windows, in classifica con full path, come si vestono su un terminale vero" width="100%" />
<sub>🔍 <b>La risposta, in classifica</b> — mfd-cli -big 10 C:\Windows: full path, ordine fisso, e il blocco WARNING che dichiara cosa la scansione non ha potuto leggere. (Foto scattata su Windows, console host.)</sub>
</div>

<details>
<summary><b>Opzionale:</b> pubblicare i comandi (e le sette lingue)</summary>

`.\install.ps1` in `rust\` copia gli exe in `%USERPROFILE%\.local\bin`
(verificate che quella cartella sia nel vostro PATH) e i file delle lingue
in `%APPDATA%\mfd\lang`, il registro che CLI e TUI scoprono a runtime —
l'inglese e' incorporato nei binari, quindi funzionano anche senza quella
cartella. `install.ps1` non tocca mai il `PATH`. Senza installazione,
eseguite direttamente `rust\target\release\mfd-cli.exe`. I binari
precompilati via GitHub Releases sono in programma.

</details>

---

## Due frontend, un motore

| `mfd-cli` — la risposta, da leggere | `mfd-tui` — la risposta, da navigare |
|:---:|:---:|
| ![La griglia su un terminale vero: mfd-cli -top 10](docs/img/cli-grid.png) | ![mfd-tui: i due pannelli a fine scansione](docs/img/tui-start.png) |

| | `mfd-cli` | `mfd-tui` |
| --- | --- | --- |
| **Cosa e'** | la griglia di ripartizione, look duf, a qualsiasi profondita' | un navigatore a due pannelli sulla stessa scansione |
| **Si avvia** | `mfd-cli <cartella>` | `mfd-tui <cartella>` |
| **La lente** | `-f` (glob o `REG(...)`), `-o`, `-top` | la stessa lente: `Shift+S` ordina, `Shift+F` filtra |
| **Le mosse chiave** | `-big N` classifica i piu' grossi, `-json` fotografa | `Shift+X` apre l'albero, `Shift+R` il report, `F1` la tastiera |
| **Il resto** | `-d a` albero intero, 7 lingue, contratto ASCII | 4 temi, stampa JSON con `Shift+P`, `F2` salva, il mouse si veste da tasto |
| **Manuale** | [FAQ](docs/FAQ.md), voci 1-9 e 11-16 · `mfd-cli -h` | [FAQ](docs/FAQ.md#10-is-there-a-version-to-navigate-not-just-to-read), voce 10 · `F1` nell'app |

### `mfd-cli` in azione

La griglia colorata ha aperto questa pagina; ecco il resto della famiglia,
sullo stesso terminale. (Gli output parlano la lingua della sessione; i
messaggi del sistema nei blocchi WARNING, come "Accesso negato", arrivano
tal quali dal Windows italiano su cui girano le scansioni.)

![L'albero completo: mfd-cli -d a -top 12](docs/img/cli-tree.png)

![La griglia parla la lingua della sessione: -lang it](docs/img/cli-lang.png)

![La lente del filtro, dichiarata accanto alla radice: -f REG(...)](docs/img/cli-filter.png)

Riferimento completo: `mfd-cli -h`. Esempi reali spiegati riga per riga:
[docs/FAQ.md](docs/FAQ.md) — la FAQ e' un pilastro del progetto: ogni voce e'
una domanda vera, un comando vero e il suo output vero.

### `mfd-tui` in azione

Una scansione, poi si naviga a frecce. Ogni `Shift+X` apre un ramo in piu' —
l'albero cresce un livello alla volta:

![mfd-tui: rami aperti uno alla volta con Shift+X](docs/img/tui-expand.png)

`Shift+A` apre tutto il sottoalbero, e TAB porta il cursore in lettura nel
dettaglio:

![mfd-tui: l'albero aperto e il cursore in lettura nel dettaglio](docs/img/tui-tree.png)

La finestra report (`Shift+R`/F9) mette in classifica i file piu' grossi
della cartella corrente — il top si cambia con le cifre:

![mfd-tui: la finestra report](docs/img/tui-report.png)

E F1 documenta tutta la tastiera in una finestra sola:

![mfd-tui: l'aiuto F1](docs/img/tui-help.png)

`F2` salva le impostazioni (tema, lente, barra) nel profilo utente — il
flash dice dove:

![mfd-tui: F2 salva le impostazioni](docs/img/tui-save.png)

`Shift+P` / F11 stampa la fotografia JSON di quello che stai guardando —
dalla schermata principale o dalla finestra report, il flash dice il file:

![mfd-tui: la stampa dalla schermata principale](docs/img/tui-print-home.png)

La stessa stampa da dentro la finestra report:

![mfd-tui: la stampa dalla finestra report](docs/img/tui-print-report.png)

Quattro temi, scelti con `Shift+T` / F3 — Notturno blu, Debian installer,
Console (il default), CLI:

![Il menu dei temi della TUI (Shift+T / F3)](docs/img/tui-theme-menu.png)

![Il tema Notturno](docs/img/tui-theme-night.png)

![Il tema Debian](docs/img/tui-theme-debian.png)

Il prompt del filtro (`Shift+F` / F7), il menu delle lingue (`Shift+L` /
F12) e il loader della scansione:

![Il prompt del filtro (Shift+F / F7)](docs/img/tui-filter.png)

![Il menu delle lingue (Shift+L / F12)](docs/img/tui-lang.png)

![Il loader della scansione](docs/img/tui-scan.png)

## Il contratto JSON

Ogni fotografia nasce dallo stesso serializzatore, promosso nel motore
(mfd-core): ASCII puro, ordine deterministico, diffabile byte per byte.

| Fotografia | Cosa fotografa | Come |
| --- | --- | --- |
| la griglia | le righe di livello 1 che stai guardando | `mfd-cli -json <cartella>` |
| l'albero intero | ogni cartella, ogni file — quello che serve a un diff | `mfd-cli -d a -json <cartella>` |
| il report | gli N file piu' grossi, classifica e full path | `mfd-cli -big N -json <cartella>` |
| la vista | entrambi i pannelli della TUI, come li hai lasciati | `mfd-tui`, `Shift+P` |
| la finestra report | il report aperto, top e file grossi | `Shift+R`, poi `Shift+P` |

Una stampa vera — la cartella demo da sette file, rami chiusi, selezione
su `alpha` (`mfd-stampa-demo-tui.json`):

```json
{
  "mfd": "0.2.1",
  "root": "C:\\Users\\Public\\mfd-captures\\demo-tui",
  "filter": null,
  "depth": 4294967295,
  "totals": {"bytes": 3530, "files": 7, "dirs": 5, "warnings": 0, "elapsed_ms": 3.7},
  "warnings": [],
  "tree": {
    "bytes": 3530,
    "files": 7,
    "dirs": 5,
    "children": [
      {"name": "alpha", "bytes": 3000, "files": 2, "dirs": 0, "children": null, "leaves": null},
      {"name": "gamma", "bytes": 310, "files": 2, "dirs": 1, "children": null, "leaves": null},
      {"name": "beta", "bytes": 120, "files": 2, "dirs": 1, "children": null, "leaves": null}
    ],
    "leaves": [
      {"name": "readme.txt", "bytes": 100}
    ]
  },
  "detail": {
    "selected": "C:\\Users\\Public\\mfd-captures\\demo-tui\\alpha",
    "bytes": 3000,
    "files": 2,
    "dirs": 0,
    "rows": [
      {"name": "data.dat", "kind": "file", "bytes": 2000},
      {"name": "lib.dll", "kind": "file", "bytes": 1000}
    ]
  }
}
```

I file nascono nella cartella da cui avete lanciato `mfd-tui` — `mfd-stampa-<nome>.json`
per la vista, `mfd-report-<nome>.json` per il report, numerati in caso di
collisione: niente sovrascritture silenziose, due fotografie valgono un
diff. La ricetta prima/dopo e le stampe della TUI sono nel FAQ (voci 9 e 12).

## L'esperimento: stesso walker, tre linguaggi

Questo repository e' nato come esperimento di scelta dello stack: lo stesso
walker parallelo — Node, C#, Rust — sulle stesse cartelle, riscaldamento
di tutto, round interleaved con ordine ruotato, mediane e totali incrociati
prima di stampare qualsiasi cosa. Tre bench, tre domande
([docs/BENCHMARKS.md](docs/BENCHMARKS.md)): quanto e' veloce la scansione,
quanta RAM usa, quanto costa avviarla. Le sessioni sono incollate per
intero in [bench/results/](bench/results); la tabella in cima a questa
pagina e' quella di oggi. Il verdetto, su tre assi:

### 1 — Prestazioni: CPU e RAM

La CPU e' un pareggio: C# JIT 1378 ms di mediana, C# R2R 1464, Rust 1462
— questa sessione ha persino il C# un capello davanti, ben dentro il
rumore. Su una cartella piccola il C# JIT paga il riscaldamento e sembra
4 volte piu' lento — un artefatto, non un verdetto (FAQ, voce 8).

La RAM e' dove si separano: Rust tocca un picco di 12,5 MB di working
set, il walker C# ~59, Node ~251 — un quinto, un ventesimo. Il motivo e'
il runtime, non il codice: il walker non alloca quasi niente (le
dimensioni arrivano dritte dall'enumerazione), ma il C# si porta dietro
il runtime .NET e il suo garbage collector — heap gestito, strutture del
JIT, generazioni della GC che scambiano RAM con la velocita' di
allocazione. Rust non ha GC: l'ownership libera la memoria a fine ambito,
in modo deterministico, e i 12,5 MB sono quasi solo buffer. Il bench di
avvio racconta la stessa storia da un altro angolo: un exe nativo parte
in ~10 ms, caricare il CLR o V8 ne costa ~60. **Il linguaggio conta meno
del runtime che ci sta attorno.**

### 2 — Robustezza, maturita', sviluppo

Qui il confronto si rigira. C#/.NET e' una piattaforma ventennale: una
libreria di classi immensa, tooling e debugger al vertice, sviluppatori
ovunque — scriverci sopra e' rapido e permissivo. Rust e' piu' giovane
(1.0 nel 2015) e piu' ripido: il borrow checker ti combatte per
settimane prima di cominciare a combattere per te — e quello che compra
e' la robustezza per costruzione: data race, use-after-free e
dereferenziazioni nulle qui sono errori di compilazione, non crash a
runtime. Entrambi i walker sono rocciosi (il bench ne incrocia i totali a
ogni giro); la differenza e' quando paghi quella sicurezza: Rust subito,
nel compilatore; C# dopo, nei test.

### 3 — Portabilita'

Due giochi diversi. .NET ha una capacita' che Rust non ha: la
cross-publish da una macchina sola — da Windows, `dotnet publish -r
linux-x64 --self-contained true` produce l'eseguibile per Linux, da
qualsiasi host verso qualsiasi bersaglio, con un comando (l'SDK scarica
il runtime pack del bersaglio). Rust cross-compila in teoria, ma come il
C++ vuole il toolchain del bersaglio: i target GNU chiedono un
cross-linker che Windows non si porta dietro, quindi in pratica si
compila su due sistemi — o su due runner di CI, o per vie indirette come
musl e zigbuild — pagando ogni volta le frizioni di compatibilita'.
Quello che Rust restituisce e' l'artefatto: `mfd-cli.exe` e' un file
solo da 2 MB, nativo, senza runtime da spedire ne' installare — i binari
self-contained di .NET si portano il runtime in corpo, decine di MB, e
la publish R2R lo vuole installato. Il processo e' il gioco di .NET,
l'artefatto quello di Rust. Anche Node vuole il suo runtime, e porta
l'avvio piu' pesante dei tre.

### La classifica finale

| Asse | Rust | C# | Node |
| --- | :---: | :---: | :---: |
| CPU | 🥇 5 | 🥇 5 | 2 |
| RAM | 🥇 5 | 3 | 1 |
| Maturita' ed ecosistema | 3 | 🥇 5 | 4 |
| Facilita' di sviluppo | 2 | 🥇 5 | 4 |
| Portabilita' | 1 | 4 | 🥇 5 |
| **Totale** | **16/25** | **22/25** | **16/25** |
| **Il vincitore, se…** | sei maniaco delle performance | vuoi la via ponderata | lo usi ogni tanto, da una riga di comando |

Nessuno vince in senso assoluto — ogni asse ha il suo campione: **Rust
prende la corona delle prestazioni** (CPU pari, un quinto della RAM, un
binario solo da 2 MB che parte in ~10 ms), **Node quella della
portabilita'** (una base di codice sola, che gira ovunque ci sia il suo
runtime — ovunque), e **C# la somma migliore: il compromesso giusto** —
parita' in CPU, un quarto della memoria di Node, piattaforma ventennale,
cross-publish di ogni bersaglio da una macchina sola. La somma incorona
il compromesso, non un dominatore: i due puristi — quello delle
prestazioni e quello della portabilita' — pareggiano pure, a 16/25.

Le prime due righe sono misurate (le sessioni in
[bench/results/](bench/results)); le ultime tre sono giudizio
d'ingegneria, dichiarato per tale.

Il prodotto ha scelto il suo campione secondo il proprio mandato: un
analizzatore di disco che deve sembrare istantaneo e spedirsi come un
binario solo — quindi **Rust e' il motore e il prodotto** (mfd-core,
mfd-cli, mfd-tui). **Il gemello C# e' una proof of concept** — il
walker, completo e misurato, senza la CLI e la TUI attorno: la sua
promozione e' il prossimo capitolo del roadmap
([docs/ROADMAP.md](docs/ROADMAP.md)), e il compromesso che incarna e'
esattamente quello per cui si tiene un secondo parere. Node resta
congelato come prototipo della fase 0: la sua portabilita' non ha potuto
salvare i suoi 251 MB.

### La strada da qui

L'esperimento ha deciso la strada della famiglia, ed e' esplicita:

- **Rust parte per primo**: il prodotto viene pubblicato e stabilizzato
  — e' il passo che sblocca tutto il resto.
- **Il gemello C# cresce attraverso la CLI** (fase 3 del roadmap),
  quando il contratto Rust sara' congelato — il suo test di accettazione
  e' gia' scritto: lo stesso ASCII rediretto, byte per byte. **La TUI in
  C# e' deliberatamente fuori percorso**: l'endgame del gemello e' la
  GUI della fase 4, dove .NET e' a casa — la CLI fa crescere l'engine,
  la GUI lo raccoglie.
- **Node resta il patriarca congelato**: walker per il bench, prototipo
  per la storia. Nient'altro e' previsto per lui.

Queste sono le scelte del manutentore, non porte chiuse: **la comunita'
e' benvenuta sulle strade non prese** — una CLI Node, una TUI C#, port
su altre piattaforme. I contratti rendono verificabile ogni contributo:
le righe RESULT del bench e l'output ASCII rediretto sono il giudice, e
il FAQ e' il posto dove una domanda vera diventa una feature
([CONTRIBUTING.md](CONTRIBUTING.md)).

## Limiti noti

- **Testato su una macchina sola, in una sessione sola.** Il bench
  confronta le implementazioni fra loro; non classifica i computer.
- **Windows, Linux, macOS.** Su Windows vale l'ottimizzazione no-`stat`
  (il trucco Win32); su Linux e macOS il walker usa `stat`. L'harness
  del bench e' PowerShell (solo Windows). La CI gira tutti i test su
  tutti e tre i sistemi a ogni push.
- Le cartelle bloccate sono warning, non errori: la griglia le mostra lo
  stesso (0 B), e i totali dicono quanti report mancano.
- Il look duf richiede un terminale ANSI; rediretto, l'output e' la
  griglia ASCII piatta di sempre (`MFD_DUF=1` forza il look anche
  attraverso una pipe, per `less -R`).
- **Solo macOS arm64** nella release: i Mac Intel compilano da fonte (un
  comando `cargo`, circa un minuto).
- La TUI parla 16 colori, per scelta: la palette della console.

## Sviluppo

```powershell
cargo test                                        # i test del workspace
powershell -ExecutionPolicy Bypass -File bench\run-bench.ps1 -Target . -Runs 2
```

Appunti di architettura — motore e frontend, il walker, i contratti:
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Struttura del repository

```text
rust\     famiglia Rust (workspace Cargo): crates\mfd-core (il motore),
          crates\mfd-cli (la CLI), crates\mfd-tui (il TUI interattivo)
net\      famiglia .NET: la proof of concept C# — il walker, verificato dal bench (gemello completo: roadmap)
node\     prototipo Node della fase 0 (congelato: storia, e un implementazione del bench)
bench\    script di benchmark (run / startup / mem) e loro risultati
docs\     ARCHITECTURE - BENCHMARKS - ROADMAP - FAQ
gui\ app\ riservate alla futura GUI e alla distribuzione app
```

## Documentazione

- [docs/FAQ.md](docs/FAQ.md) — domande vere, comandi veri, output veri
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) — motore e pelle, il walker, i contratti
- [docs/BENCHMARKS.md](docs/BENCHMARKS.md) — metodo, script, contratto RESULT
- [docs/ROADMAP.md](docs/ROADMAP.md) — TUI, promozione del gemello C#, GUI, app
- [docs/DESIGN-TUI.md](docs/DESIGN-TUI.md) — il design approvato del TUI: due pannelli, una lente, e dalla 0.7.1 l'albero e il cursore di lettura

## Contribuire

Vedi [CONTRIBUTING.md](CONTRIBUTING.md). In breve: il contratto di bench e'
sacro, naked significa cartella corrente, l'output e' ASCII puro, e ogni
esempio nella documentazione e' vero. Lo sviluppo e' FAQ-driven: se un esempio
risulta scomodo, quella e' una richiesta di feature. Le issue sono aperte —
tre template (bug, domanda, feature) portano il contesto, e le domande vere
diventano voci del FAQ.

## Licenza

Copyright (C) 2026 Francesco Mantovanelli.

Questo programma e' software libero: potete ridistribuirlo e/o modificarlo nei
termini della **GNU Affero General Public License v3.0** pubblicata dalla Free
Software Foundation — vedete [LICENSE](LICENSE). Per prodotti che non possono
essere distribuiti in AGPL e' disponibile, su richiesta, un canale di
licenziamento commerciale.

---

<div align="center">

**Autore** — Francesco Mantovanelli · **Progetto** — Manto.FreeDisk

</div>
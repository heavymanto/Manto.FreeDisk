// mfd-cli — command line interface di MantoFreeDisk (vedi docs/ARCHITECTURE.md e docs/FAQ.md).
// Pelle sul motore mfd-core (stesso workspace): qui vive il parsing degli argomenti,
// la griglia di ripartizione, la riga TOTALE che la chiude, le righe RESULT/ITER dei
// bench e -json (il serializzatore JSON vive in mfd-core, condiviso con la
// stampa Shift+P del TUI). Il walker parallelo, l'aggregazione per profondita', i filtri e i
// warning stanno in mfd-core, condivisi con mfd-tui (fase 2).
// Uso: mfd-cli [opzioni] [<path> [loops]] - senza argomenti scansiona la
// cartella corrente (naked = ., come du); -h aiuto, -V versione.
// Output: con env MFD_BENCH=1 righe machine-readable RESULT/ITER (contratto degli
// script di bench, che la impostano); senza, formato leggibile per uso interattivo
// ("mfd-cli"): tabella di ripartizione chiusa dalla riga TOTALE incolonnata e,
// quando lo scan ha voci non leggibili, dalle righe WARNING che le rendicontano
// (con -d 0 la sola riga di totali; -d a tutta la profondita'); con
// -json la fotografia JSON di quello che la griglia mostra (senza -d il
// livello 1, -d N apre N livelli, -d a l'albero intero dei diff
// prima/dopo); con -big l'elenco dei file piu' grandi con full path — o,
// con -json, la fotografia del report (vedi docs/FAQ.md).
// La griglia ha due look (theme.rs): sul terminale vero quello di duf
// (github.com/muesli/duf — box arrotondato, colori ANSI a soglie; NO_COLOR
// li spegne, MFD_ASCII=1 torna al formato storico, MFD_DUF=1 lo forza anche
// ridiretto); ridiretta in file o pipe resta il formato ASCII contratto: la
// lingua segue -lang/MFD_LANG, il formato no.
// Durante la scansione una riga di avanzamento vive su stderr (il loader:
// solo su terminale interattivo, mai con MFD_BENCH o loops; stdout non
// cambia): spinner braille e barra arcobaleno con percentuale stimata sul
// terminale che li sa mostrare — NO_COLOR la tiene monocroma, MFD_ASCII=1
// (o una console ostile) riporta il loader storico.
// La lingua (fase 2 di docs/DESIGN-I18N.md): -lang/--lang — in tutte le sue
// forme, -lang it, -lang=it, -lang:it e le doppie -- — batte MFD_LANG che
// batte l'inglese incorporato; le altre lingue sono i file del profilo, lo
// stesso registro e la stessa cartella del TUI (%APPDATA%\mfd\lang). Di
// fabbrica si parla inglese: -h, errori, header della griglia, WARNING/NOTE
// e loader seguono la lingua attiva; il contratto macchina non si traduce
// mai — RESULT/ITER e -json restano byte per byte — e i numeri restano col
// separatore di sempre (il §5 del design li rinvia, dichiarati).

use mfd_core::lang::Catalog;
use mfd_core::{Filter, Order, Progress, Row, ScanOptions, Tree, Warning, WarningKind, WarningLog};
use std::collections::HashSet;
use std::env;
use std::io::{IsTerminal, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

mod theme;

/// Formattazione leggibile della dimensione (solo modalita' standalone mfd-cli).
fn fmt_bytes(b: i64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut v = b as f64;
    let mut u = 0usize;
    while v >= 1024.0 && u < UNITS.len() - 1 {
        v /= 1024.0;
        u += 1;
    }
    if u == 0 {
        format!("{} B", b)
    } else {
        format!("{:.1} {}", v, UNITS[u]).replace('.', ",")
    }
}

/// Separatore delle migliaia stile italiano (171710 -> 171.710).
fn group_digits(n: i64) -> String {
    let s = n.to_string();
    let b = s.as_bytes();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for i in 0..b.len() {
        if i > 0 && (b.len() - i) % 3 == 0 {
            out.push('.');
        }
        out.push(b[i] as char);
    }
    out
}

/// Barra ASCII proporzionale alla quota di `bytes` sul totale della scansione:
/// '#' per la parte piena, '.' per il resto. Sta in colonna fissa tra la % e il
/// conteggio file di ogni riga della ripartizione (--depth), allineata su tutti
/// i livelli perche' l'indentazione viene dopo. Solo ASCII volutamente, come il
/// resto dell'output (codepage non UTF-8 nei log).
const BAR_W: usize = 24;

fn bar(bytes: i64, total: i64) -> String {
    let filled = if total > 0 && bytes > 0 {
        (((bytes as f64 / total as f64) * BAR_W as f64).round() as usize).min(BAR_W)
    } else {
        0
    };
    format!("{}{}", "#".repeat(filled), ".".repeat(BAR_W - filled))
}

/// La scansione con il loader: una riga live su stderr (spinner, barra,
/// percentuale stimata e contatori) che si aggiorna mentre il walker lavora
/// e sparisce quando arrivano i risultati — la schermata di scansione del
/// TUI ridotta a una riga, perche' anche la CLI non deve sembrare congelata
/// sulle cartelle grandi. Vive su stderr (stdout resta la risposta pulita),
/// si accende solo se stderr e' un terminale vero e mai con MFD_BENCH o
/// loops: redirect, catture di regressione e contratto RESULT/ITER non
/// vedono nulla. La callback non annulla mai: qui ESC non esiste. Sul
/// terminale che le sa mostrare la riga veste braille e arcobaleno
/// (theme::loader): lo spinner gira per le dodici tinte ANSI, la barra si
/// riempie seguendo lo spettro e la percentuale e' la stima onesta del TUI;
/// NO_COLOR la tiene monocroma, MFD_ASCII=1 — o una console che rifiuta le
/// sequenze ANSI su stderr — riporta il loader storico: forma di sempre,
/// parole della lingua attiva.
fn scan_live(root: &std::path::Path, opts: &ScanOptions, live: bool, cat: &Catalog) -> Tree {
    if !live {
        return mfd_core::scan(root, opts);
    }
    // il vestito si sceglie una volta per corsa, e con lui il budget del
    // percorso: barra e percentuale si prendono il loro spazio
    let fancy = theme::loader::fancy();
    let color = fancy && theme::loader::colored();
    let disp = theme::loader::shorten(
        &root.display().to_string(),
        theme::loader::path_budget(fancy),
    );
    // lo stato del loader e' condiviso con la callback, che gira sui thread
    // del walker (anche piu' di uno alla volta: Mutex)
    let loader = Arc::new(Mutex::new(Loader { last: Instant::now(), frame: 0, width: 0 }));
    let shared = Arc::clone(&loader);
    // il vocabolario del loader entra nella callback per valore: gira sui
    // thread del walker e deve vivere da sola ('static, come shared)
    let tick_cat = cat.clone();
    let tick = move |p: &Progress, l: &mut Loader| {
        let (s, vis) = if fancy {
            theme::loader::line(p, l.frame, &disp, color, &tick_cat)
        } else {
            theme::loader::legacy(p, l.frame, &disp, &tick_cat)
        };
        l.frame += 1;
        l.width = l.width.max(vis);
        // si stende alla larghezza massima gia' stampata, in glifi visibili
        // (i codici colore non contano): la riga si sovrascrive sempre per
        // intero, senza residui
        eprint!("\r{}{}", s, " ".repeat(l.width.saturating_sub(vis)));
        let _ = std::io::stderr().flush();
    };
    // prima riga subito, a contatori zero: il "freeze" da togliere e' quello
    // dei primi istanti, quando la callback non e' ancora scattata
    {
        let mut l = loader.lock().unwrap();
        tick(
            &Progress { files: 0, dirs: 0, bytes: 0, errors: 0, closed: 0, open: 0 },
            &mut l,
        );
    }
    let cb = move |p: Progress| -> bool {
        let mut l = shared.lock().unwrap();
        if l.last.elapsed() >= Duration::from_millis(100) {
            l.last = Instant::now();
            tick(&p, &mut l);
        }
        true // il loader non annulla mai
    };
    let tree = mfd_core::scan_with_progress(root, opts, cb)
        .expect("il loader non chiede mai l'annullamento");
    // la riga sparisce: si copre della larghezza raggiunta e si torna a capo riga
    let w = loader.lock().unwrap().width;
    eprint!("\r{}\r", " ".repeat(w));
    let _ = std::io::stderr().flush();
    tree
}

/// Lo stato del loader della scansione: il throttle (una riga ogni 100 ms),
/// il frame dello spinner (che gira anche sulla ruota delle tinte), la
/// larghezza massima stampata in glifi visibili (per la pulizia: i codici
/// colore non contano).
struct Loader {
    last: Instant,
    frame: usize,
    width: usize,
}

/// I totali che chiudono la griglia: le tre info indispensabili (dimensione,
/// file, cartelle) della radice — le stesse che il dettaglio del TUI mette in
/// cima al pannello destro, cosi' chi passa da un'interfaccia all'altra trova
/// sempre gli stessi numeri, e in fondo alla lista che li dettaglia — piu' il
/// conto delle voci della tabella, il totale della colonna NOME. Con -f
/// attivo sono i numeri della lente, come la riga radice che apre la
/// tabella. Il footer li stampa incolonnati sotto le rispettive colonne
/// (totals_row qui, render_table nel box duf): la lettura e' la classica
/// verticale, col dato cumulativo sotto ogni colonna.
struct Totals {
    bytes: i64,
    files: i64,
    dirs: i64,
    /// Le voci della tabella, radice compresa: tutte quelle che l'albero
    /// stamperebbe, non il taglio di -top — i totali non si tagliano.
    voci: i64,
}

/// Il conto delle voci col plurale onesto ("1 voce", "2 voci"): e' il totale
/// della colonna NOME, e il footer lo mette li', accanto all'etichetta.
fn voci(n: i64, cat: &Catalog) -> String {
    if n == 1 {
        cat.t("cli.voce").to_string()
    } else {
        cat.tf("cli.voci", &[&group_digits(n)])
    }
}

/// La riga che chiude la griglia: i totali incolonnati sotto le rispettive
/// colonne — la dimensione sotto DIMENSIONE, i file sotto FILE, le cartelle
/// sotto CARTELLE — e in colonna NOME l'etichetta col conto delle voci: la
/// lettura e' la classica verticale, col dato cumulativo sotto. % e barra
/// restano vuote: la somma non e' una quota di se' stessa (la radice, in
/// cima, e' gia' il 100%). Le spaziature sono quelle di ascii_row, cella
/// per cella: le colonne coincidono al byte.
fn totals_row(t: &Totals, cat: &Catalog) -> String {
    format!(
        " {:>10}  {:>4}  {:>24}  {:>11}  {:>9}  {}",
        fmt_bytes(t.bytes),
        "",
        "",
        group_digits(t.files),
        group_digits(t.dirs),
        cat.tf("cli.totale", &[&voci(t.voci, cat)])
    )
}

/// Una riga della griglia raccolta prima di stamparla: i dati che il
/// rendering chiede, e il rendering e' doppio — il formato ASCII storico
/// (il contratto d'output) o il box duf del terminale (theme.rs).
/// `files`/`dirs` a None sono la foglia (il "-" della griglia); `name` e'
/// gia' display: prefisso dell'albero, nome e "\\" finale per le cartelle.
struct GridRow {
    bytes: i64,
    files: Option<i64>,
    dirs: Option<i64>,
    name: String,
}

/// La riga della griglia nel formato ASCII storico — il contratto d'output
/// che redirect, log e catture di regressione devono trovare identico: la
/// forma e' quella dell'esempio nell'help, cella per cella.
fn ascii_row(r: &GridRow, total: i64) -> String {
    let pct = if total > 0 { r.bytes * 100 / total } else { 0 };
    let (f, d) = match (r.files, r.dirs) {
        (Some(f), Some(d)) => (group_digits(f), group_digits(d)),
        _ => ("-".to_string(), "-".to_string()),
    };
    format!(
        " {:>10}  {:>3}%  {}  {:>11}  {:>9}  {}",
        fmt_bytes(r.bytes),
        pct,
        bar(r.bytes, total),
        f,
        d,
        r.name
    )
}

/// Tabella di ripartizione stile TreeSize (--depth): per ogni cartella fino al
/// livello N la dimensione ricorsiva del sottoalbero; i fratelli (cartelle e file
/// diretti insieme) sono ordinati secondo -o (default: dimensione decrescente) e
/// il nome porta i connettori ASCII dell'albero (vedi tree_prefix). Dopo
/// intestazione e separatore, la prima riga e' la radice (punto zero: 100%,
/// barra piena); seguono i suoi figli. `top` (-top N) tronca la stampa a N righe
/// (radice compresa): quando il taglio ha effetto, la funzione restituisce la
/// riga NOTE da stampare nel blocco di note in fondo (insieme ai warning). La riga
/// radice e' anche il riepilogo dello scan: con -f attivo (`filter_disp`) il suo
/// nome porta "(filtro: P)", perche' cambia il senso di tutti i numeri. Sotto
/// l'ultima riga la TOTALE chiude la griglia incolonnata sotto le rispettive
/// colonne (dimensione, file, cartelle — e il conto delle voci in colonna
/// NOME): -top non la tocca, perche' i totali non sono tagliati. Se lo scan
/// ha segnalazioni, le righe WARNING seguono la TOTALE dentro la tabella:
/// la prima cella e' l'etichetta, le altre colonne si fondono in un'unica
/// cella di testo, una segnalazione per riga col suo trattino.
/// Con -f attivo le cartelle senza nemmeno un file che matcha non si stampano
/// (sarebbero righe a 0 B e 0 file: rumore): il loro conteggio torna al chiamante
/// per la NOTE nel blocco di note — il filtro non nasconde in silenzio.
/// La griglia ha due look (theme.rs): il box duf sul terminale vero, il
/// formato ASCII storico ridiretto — il contratto non cambia col pipe.
/// Ritorna (NOTE del -top, cartelle nascoste dal filtro).
fn print_breakdown(
    root: &std::path::Path,
    tree: &Tree,
    order: Order,
    top: Option<usize>,
    filter_active: bool,
    filter_disp: Option<&str>,
    cat: &Catalog,
) -> (Option<String>, usize) {
    // il look si decide qui, una volta per corsa: terminale vero -> il box duf
    // di theme.rs; redirect/pipe -> il formato ASCII storico, byte per byte
    let look = theme::Look::detect();
    let total_bytes = tree.stats().bytes;
    let r = tree.metrics(0);
    let root_name = match filter_disp {
        Some(f) => format!("{} {}", root.display(), cat.tf("cli.filtro", &[f])),
        None => root.display().to_string(),
    };
    // la radice e' la prima riga: il punto zero della % e della barra (100%)
    let mut rows = vec![GridRow {
        bytes: total_bytes,
        files: Some(r.files),
        dirs: Some(r.dirs),
        name: root_name,
    }];
    let limit = top.unwrap_or(usize::MAX);
    let mut budget = limit;
    budget -= 1; // la radice appena raccolta conta come prima riga
    let mut hidden = 0usize;
    collect_rows(
        0,
        total_bytes,
        tree,
        order,
        &[],
        &mut budget,
        filter_active,
        &mut hidden,
        &mut rows,
    );
    // i totali che chiudono la griglia: le tre info della radice piu' il
    // conto delle voci — tutte quelle che la tabella avrebbe, non il taglio
    // di -top (i totali non si tagliano: il conto concorda con la NOTE)
    let total_rows = 1 + count_rows(0, tree, filter_active);
    let totals = Totals {
        bytes: total_bytes,
        files: r.files,
        dirs: r.dirs,
        voci: total_rows as i64,
    };
    // le segnalazioni dello scan, rese una volta per entrambi i look: il
    // footer le stampa sotto la TOTALE, dentro la tabella
    let warns = warning_lines(tree.warnings(), cat);
    match look {
        theme::Look::Ascii => {
            println!(
                " {:>10}  {:>4}  {:>24}  {:>11}  {:>9}  {}",
                cat.t("cli.dim"),
                "%",
                "",
                cat.t("cli.file"),
                cat.t("cli.cartelle"),
                cat.t("cli.nome")
            );
            println!("{}", "-".repeat(79));
            for row in &rows {
                println!("{}", ascii_row(row, total_bytes));
            }
            // la TOTALE chiude la griglia incolonnata, dietro un filo di
            // separazione come la riga che chiude una tabella vera: il totale
            // e' il dato che si cerca per primo e quando la tabella e' lunga
            // la radice, in cima, e' gia' fuori dallo schermo. -top non la
            // tocca: le righe sono tagliate, i totali no
            println!("{}", "-".repeat(79));
            println!("{}", totals_row(&totals, cat));
            // e sotto la TOTALE, nel footer, le segnalazioni: prima cella
            // l'etichetta, resto della riga fuso nel testo
            print_warning_rows(&warns, cat);
        }
        theme::Look::Duf { color } => {
            // nel box la TOTALE e' grassa e incolonnata come le altre, gli
            // stessi numeri della griglia ASCII; i warning la seguono nella
            // cella fusa che chiude il box
            for line in theme::render_table(&rows, &totals, &warns, color, cat) {
                println!("{}", line);
            }
        }
    }
    // -top ha troncato davvero? la NOTA non si stampa qui: si restituisce al
    // chiamante, che la mette nel blocco di note in fondo insieme ai warning
    let note = if top.is_some() {
        if total_rows > limit {
            Some(cat.tf(
                "cli.nota_top",
                &[&group_digits(limit as i64), &group_digits(total_rows as i64)],
            ))
        } else {
            None
        }
    } else {
        None
    };
    (note, hidden)
}

/// Righe totali della tabella sotto il nodo `id` (figli cartella + file diretti
/// + ricorsi): serve solo a dire a -top quante righe sono rimaste fuori.
/// Con -f attivo i rami senza match non contano: non verrebbero stampati
/// (stessa condizione di print_rows, perche' NOTE e griglia devono concordare).
fn count_rows(id: u32, tree: &Tree, filter_active: bool) -> usize {
    let mut tot = tree.leaves_count(id);
    for &i in tree.children(id) {
        let m = tree.metrics(i);
        if filter_active && m.files == 0 && m.bytes == 0 {
            continue; // figlia senza match: nascosta anche nel conteggio
        }
        tot += 1 + count_rows(i, tree, filter_active);
    }
    tot
}

/// Prefisso del nome nella vista ad albero: convenzione ASCII del comando `tree`
/// ("tree --charset=ascii"). "|-- " per un figlio con fratelli in coda, "`-- "
/// per l'ultimo; ogni antenato con altri figli in coda tiene la guida verticale
/// "|   " (altrimenti 4 spazi). Le righe escono in ordine di visita, figlio
/// subito sotto il padre: i connettori rispecchiano sempre la struttura reale,
/// e la lunghezza del prefisso (4 x livello) rende il livello immediato.
fn tree_prefix(last: bool, guides: &[bool]) -> String {
    let mut s = String::with_capacity(guides.len() * 4 + 4);
    for &g in guides {
        s.push_str(if g { "|   " } else { "    " });
    }
    s.push_str(if last { "`-- " } else { "|-- " });
    s
}

/// Rende una segnalazione del blocco note nel formato storico della griglia
/// (contratto d'output): il prefisso dice l'operazione fallita, il testo e'
/// quello localizzato gia' privo del suffisso tecnico "(os error N)".
fn render_warning(w: &Warning, cat: &Catalog) -> String {
    let p = w.path.display().to_string();
    let what = match w.kind {
        WarningKind::OpenDir => cat.tf("cli.w_opendir", &[&p]),
        WarningKind::EnumEntry => cat.tf("cli.w_enum", &[&p]),
        WarningKind::EntryType => cat.tf("cli.w_type", &[&p]),
        WarningKind::Metadata => cat.tf("cli.w_meta", &[&p]),
    };
    format!("{}: {}", what, w.message)
}

/// Le segnalazioni rese per il footer: una riga per warning, ognuna col suo
/// trattino, e oltre dieci l'ultima riga rendiconta quante restano fuori —
/// l'elenco resta breve, il conto resta onesto. Funzione pura: la griglia
/// (entrambi i look) e -big la condividono, la forma e' la stessa in ogni
/// risposta.
fn warning_lines(wl: &WarningLog, cat: &Catalog) -> Vec<String> {
    let mut out: Vec<String> = wl
        .iter()
        .take(10)
        .map(|w| format!("- {}", render_warning(w, cat)))
        .collect();
    let altre = wl.len().saturating_sub(10);
    if altre > 0 {
        out.push(if altre == 1 {
            cat.t("cli.w_altre1").to_string()
        } else {
            cat.tf("cli.w_altreN", &[&group_digits(altre as i64)])
        });
    }
    out
}

/// La riga di una segnalazione nel formato della griglia ASCII (il contratto
/// d'output): la prima cella e' l'etichetta — solo sulla prima riga, le
/// altre la lasciano vuota — e tutto il resto della riga e' una cella fusa
/// di testo, che comincia dove comincia la colonna %: piu' righe restano
/// ordinate, i trattini in colonna.
fn warning_row(first: bool, text: &str, cat: &Catalog) -> String {
    format!(
        " {:>10}  {}",
        if first { cat.t("cli.warn") } else { "" },
        text
    )
}

/// Le righe WARNING del footer, le warning_lines rese da warning_row: dentro
/// la griglia (sotto la TOTALE) e in coda a -big — chi impara a leggerle in
/// una risposta le ritrova identiche nell'altra.
fn print_warning_rows(warns: &[String], cat: &Catalog) {
    for (i, l) in warns.iter().enumerate() {
        println!("{}", warning_row(i == 0, l, cat));
    }
}

/// Le righe di -big: i file piu' grandi dell'albero con taglia e full path,
/// in ordine fisso di taglia decrescente (a parita', path crescente).
/// Funzione pura, senza stampa: il test di regressione le confronta riga per
/// riga — la forma e' il contratto d'output, stabile per i diff come --json.
fn big_lines(tree: &Tree, n: usize) -> Vec<String> {
    tree.top_files(n, None)
        .into_iter()
        .map(|(s, p)| format!(" {:>10}  {}", fmt_bytes(s), p.display()))
        .collect()
}

/// La risposta di -big: intestazione, separatore e le N righe di big_lines.
/// La prima colonna e' la DIMENSIONE della griglia, accanto al full path —
/// l'informazione su cui un batch, uno script o un'AI possono agire. Con -f
/// attivo lo scan e' gia' sotto la lente e l'intestazione lo dichiara, come
/// la radice della griglia: i numeri cambiano senso. Nessuna TOTALE: la
/// lista e' la risposta, e i file esclusi dal taglio non sono totali persi.
/// Le segnalazioni chiudono la lista come i warning chiudono la griglia:
/// righe WARNING, prima cella l'etichetta e resto fuso — stessa forma. Il
/// look segue la stessa regola della griglia: terminale vero -> il box duf
/// di theme.rs (la classifica vestita come il resto della famiglia);
/// redirect/pipe -> il formato ASCII storico, byte per byte il contratto.
fn print_big(tree: &Tree, n: usize, filter_disp: Option<&str>, look: &theme::Look, cat: &Catalog) {
    let header = match filter_disp {
        Some(f) => format!("{} {}", cat.t("cli.path"), cat.tf("cli.filtro", &[f])),
        None => cat.t("cli.path").to_string(),
    };
    let warns = warning_lines(tree.warnings(), cat);
    match look {
        theme::Look::Ascii => {
            println!(" {:>10}  {}", cat.t("cli.dim"), header);
            println!("{}", "-".repeat(79));
            for line in big_lines(tree, n) {
                println!("{}", line);
            }
            print_warning_rows(&warns, cat);
        }
        theme::Look::Duf { color } => {
            let entries = tree.top_files(n, None);
            for line in theme::render_big_table(
                &entries,
                cat.t("cli.dim"),
                &header,
                &warns,
                *color,
                cat,
            ) {
                println!("{}", line);
            }
        }
    }
}

/// Raccoglie i figli del nodo `id` nell'ordine scelto con -o (default T.D: la
/// piu' grande in cima) come righe della griglia; `guides` porta le guide
/// verticali degli antenati e `budget` e' il contatore di righe residue del
/// taglio -top (0 = fermarsi subito). `hidden` accumola le cartelle senza
/// match saltate dal filtro (-f), per la NOTE del blocco di note: la
/// condizione e' quella di count_rows.
fn collect_rows(
    id: u32,
    total: i64,
    tree: &Tree,
    order: Order,
    guides: &[bool],
    budget: &mut usize,
    filter_active: bool,
    hidden: &mut usize,
    out: &mut Vec<GridRow>,
) {
    let mut rows = tree.rows_of(id);
    tree.sort_rows(&mut rows, order);
    let count = rows.len();
    for (idx, r) in rows.into_iter().enumerate() {
        if *budget == 0 {
            return; // -top: raggiunto il limite di righe richiesto
        }
        let prefix = tree_prefix(idx + 1 == count, guides);
        match r {
            Row::Dir(i) => {
                let m = tree.metrics(i);
                if filter_active && m.files == 0 && m.bytes == 0 {
                    // -f: nel sottoalbero non c'e' nessun file che matcha: la riga
                    // (e il ramo) esce dalla tabella, il blocco note lo rendiconta
                    *hidden += 1;
                    continue;
                }
                out.push(GridRow {
                    bytes: m.bytes,
                    files: Some(m.files),
                    dirs: Some(m.dirs),
                    name: format!("{}{}\\", prefix, tree.name(i).to_string_lossy()),
                });
                *budget -= 1;
                let mut child_guides = guides.to_vec();
                child_guides.push(idx + 1 != count); // questo ramo ha fratelli in coda
                collect_rows(i, total, tree, order, &child_guides, budget, filter_active, hidden, out);
            }
            Row::Leaf(s, name) => {
                out.push(GridRow {
                    bytes: s,
                    files: None,
                    dirs: None,
                    name: format!("{}{}", prefix, name.to_string_lossy()),
                });
                *budget -= 1;
            }
        }
    }
}

/// La lingua con cui la CLI parla — -h, errori, griglia, loader: -lang della
/// riga di comando, poi MFD_LANG, poi l'inglese incorporato. La CLI non ha
/// un profilo da leggere (e' uno strumento senza stato): la catena e' piu'
/// corta di quella del TUI, ma il registro e' lo stesso — stessa scoperta,
/// stessa cartella del profilo (%APPDATA%\mfd\lang). Ritorna anche il codice
/// chiesto e mancato, per l'avviso: non e' un errore, si parte in inglese.
fn resolve_lang(cli: Option<String>) -> (Catalog, Option<String>) {
    let env_lang = std::env::var("MFD_LANG")
        .ok()
        .filter(|s| !s.trim().is_empty());
    let asked = cli.or(env_lang);
    let (langs, _) = mfd_core::lang::registry();
    match asked {
        Some(code) => match langs.iter().find(|l| l.code == code) {
            Some(l) => (Catalog::with(l.clone()), None),
            None => (Catalog::english(), Some(code)),
        },
        None => (Catalog::english(), None),
    }
}

/// Le forme del flag lingua — `-lang it`, `-lang=it`, `-lang:it` e le
/// doppie `--` — diventano tutte `-lang` + valore: il parsing ne vede una
/// sola, e chi digita come gli viene naturale non sbaglia.
fn split_lang_arg(a: String) -> Vec<String> {
    for p in ["--lang=", "-lang=", "--lang:", "-lang:"] {
        if let Some(v) = a.strip_prefix(p) {
            return vec!["-lang".to_string(), v.to_string()];
        }
    }
    vec![a]
}

/// I rami aperti della fotografia della griglia: la foto `-d N -json`
/// mostra le righe della tabella, e la tabella a N livelli apre i rami
/// fino al livello N-1 — il livello N resta riga di totali ricorsivi
/// ("children": null), come la griglia. Con N illimitato (-d a) apre
/// tutto: ma li', a fotografare il mondo intero, ci pensa `full`.
fn expanded_to_depth(tree: &Tree, depth: usize) -> HashSet<u32> {
    let mut set = HashSet::new();
    let mut level: Vec<u32> = vec![tree.root_id()];
    let mut d = 0usize;
    while d < depth && !level.is_empty() {
        let mut next = Vec::new();
        for &id in &level {
            set.insert(id);
            next.extend(tree.children(id).iter().copied());
        }
        level = next;
        d += 1;
    }
    set
}

/// Aiuto (-h): parla la lingua della sessione (docs/DESIGN-I18N.md). Il
/// vocabolario arriva da lang/ — tagline e blocco intero, una chiave
/// ciascuna — lo stesso movimento del -h del TUI.
fn print_help(cat: &Catalog) {
    println!("{}", cat.tf("cli.tagline", &[env!("CARGO_PKG_VERSION")]));
    print!("{}", cat.t("cli.help"));
}

/// Il vocabolario dei test: l'italiano del crate, come fa il TUI (§6 di
/// DESIGN-I18N: i test asseriscono l'italiano tramite it.json, il
/// meccanismo e' esplicito nel codice, non un default nascosto).
#[cfg(test)]
pub(crate) fn test_cat() -> Catalog {
    let path = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../mfd-core/lang/it.json"));
    mfd_core::lang::Lang::from_file(path)
        .map(Catalog::with)
        .expect("it.json del crate: c'e' e si legge")
}

fn main() {
    // le forme del flag lingua (-lang it, -lang=it, -lang:it e le doppie --)
    // si normalizzano a una sola: -lang + valore, il parsing ne vede una
    let args: Vec<String> = env::args().skip(1).flat_map(split_lang_arg).collect();

    // La lingua si decide prima del parsing, perche' -h, gli errori e la
    // griglia parlano lei: -lang batte MFD_LANG che batte l'inglese
    // incorporato (docs/DESIGN-I18N.md, fase 2). Un codice che non esiste
    // avvisa su stderr e non e' un errore: si parte in inglese
    let mut cli_lang: Option<String> = None;
    {
        let mut i = 0usize;
        while i < args.len() {
            if args[i] == "-lang" || args[i] == "--lang" {
                if i + 1 >= args.len() {
                    // -lang senza valore: lo si dice nella lingua che si
                    // sarebbe usata, e si esce come gli altri errori di uso
                    let (cat0, _) = resolve_lang(None);
                    eprintln!("{}", cat0.t("cli.err_lang"));
                    std::process::exit(2);
                }
                cli_lang = Some(args[i + 1].clone());
                i += 1;
            }
            i += 1;
        }
    }
    let (cat, missing) = resolve_lang(cli_lang);
    if let Some(code) = &missing {
        let (langs, _) = mfd_core::lang::registry();
        let disp = langs
            .iter()
            .map(|l| l.code.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        eprintln!("{}", cat.tf("cli.lang_ko", &[code, &disp]));
    }

    // Parsing: flag riconosciuti ovunque, argomenti posizionali <path> [loops].
    let mut path: Option<&str> = None;
    let mut loops: usize = 1;
    let mut depth: usize = 0;
    let mut depth_explicit = false;
    let mut order_opt: Option<Order> = None;
    let mut top_opt: Option<usize> = None;
    let mut big_opt: Option<usize> = None;
    let mut filter: Option<Filter> = None;
    let mut json = false;
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" | "/?" => {
                print_help(&cat);
                return;
            }
            "-V" | "--version" => {
                println!("mfd-cli {}", env!("CARGO_PKG_VERSION"));
                return;
            }
            "-json" | "--json" => {
                json = true;
            }
            "-d" | "--depth" => {
                i += 1;
                let val = args.get(i).map(String::as_str);
                match val {
                    // -d a / -d all: tutta la profondita' — la stessa
                    // scansione completa di --json, mostrata come albero
                    // (quante righe vedere lo governa -top)
                    Some("a") | Some("all") => {
                        depth = usize::MAX;
                        depth_explicit = true;
                    }
                    _ => match val.and_then(|s| s.parse::<usize>().ok()) {
                        Some(d) => {
                            depth = d;
                            depth_explicit = true;
                        }
                        None => {
                            eprintln!("{}", cat.t("cli.err_d"));
                            std::process::exit(2);
                        }
                    },
                }
            }
            "-o" | "--order" => {
                i += 1;
                match args.get(i).map(String::as_str).and_then(Order::parse) {
                    Some(o) => order_opt = Some(o),
                    None => {
                        eprintln!("{}", cat.t("cli.err_o"));
                        std::process::exit(2);
                    }
                }
            }
            "-f" | "--filter" => {
                i += 1;
                match args.get(i).map(String::as_str).map(Filter::build) {
                    Some(Ok(f)) => filter = Some(f),
                    Some(Err(msg)) => {
                        eprintln!("{}", cat.tf("cli.err_f_ko", &[&msg]));
                        std::process::exit(2);
                    }
                    None => {
                        eprintln!("{}", cat.t("cli.err_f"));
                        std::process::exit(2);
                    }
                }
            }
            "-top" | "--top" => {
                i += 1;
                match args
                    .get(i)
                    .and_then(|s| s.parse::<usize>().ok())
                    .filter(|&n| n >= 1)
                {
                    Some(n) => top_opt = Some(n),
                    None => {
                        eprintln!("{}", cat.t("cli.err_top"));
                        std::process::exit(2);
                    }
                }
            }
            "-big" | "--big" => {
                i += 1;
                match args
                    .get(i)
                    .and_then(|s| s.parse::<usize>().ok())
                    .filter(|&n| n >= 1)
                {
                    Some(n) => big_opt = Some(n),
                    None => {
                        eprintln!("{}", cat.t("cli.err_big"));
                        std::process::exit(2);
                    }
                }
            }
            // la lingua: gia' decisa nella passata che precede il parsing,
            // qui si saltano il flag e il suo valore (non sono path ne' loops)
            "-lang" | "--lang" => {
                i += 1;
            }
            a => {
                if path.is_none() {
                    path = Some(a);
                } else if let Ok(n) = a.parse::<usize>() {
                    loops = n;
                }
            }
        }
        i += 1;
    }

    // Senza path si parte dalla cartella corrente (naked = .): e' quello che
    // fanno du, ncdu e dua, e quello che chi digita "mfd-cli" e INVIO si
    // aspetta. L'aiuto resta una scelta esplicita (-h).
    let root = match path {
        Some(p) => {
            let ppath = PathBuf::from(p);
            if !ppath.is_dir() {
                eprintln!("{}", cat.tf("cli.err_path", &[p]));
                std::process::exit(2);
            }
            ppath
        }
        None => PathBuf::from("."),
    };

    // Formato di output: MFD_BENCH=1 (impostato dagli script di benchmark) produce le
    // righe machine-readable RESULT/ITER che run-bench/startup-bench/mem-bench parsano;
    // senza env var (uso interattivo "mfd-cli") il formato e' leggibile.
    let bench = env::var_os("MFD_BENCH").is_some();
    // -big e' una risposta completa a se': scansiona l'albero intero (come
    // -d a -json), ordina da solo (taglia decrescente) e porta con se' il
    // suo numero di righe — le opzioni di forma non c'entrano. Con -json
    // la risposta e' la fotografia JSON del report (json::report): le
    // stesse N righe con ranking e path interi, in un formato per le
    // macchine
    if big_opt.is_some() {
        if bench || loops > 1 {
            eprintln!("{}", cat.t("cli.err_big_bench"));
            std::process::exit(2);
        }
        if order_opt.is_some() {
            eprintln!("{}", cat.t("cli.err_big_o"));
            std::process::exit(2);
        }
        if top_opt.is_some() {
            eprintln!("{}", cat.t("cli.err_big_top"));
            std::process::exit(2);
        }
        if depth_explicit {
            eprintln!("{}", cat.t("cli.err_big_d"));
            std::process::exit(2);
        }
    }
    if depth > 0 && (bench || loops > 1) {
        eprintln!("{}", cat.t("cli.err_depth_bench"));
        std::process::exit(2);
    }
    // Uso interattivo senza -d: la griglia di livello 1 e' il minimo che l'utente si
    // aspetta ("dove sta lo spazio?"); -d 0 la disattiva esplicitamente. I bench
    // (MFD_BENCH o loops > 1) restano sui soli totali: il contratto RESULT/ITER e la
    // comparabilita' con i walker Node/C# non devono cambiare col default.
    if !bench && loops == 1 && !depth_explicit {
        depth = 1;
    }
    // -json e' la fotografia di quello che la griglia mostra: senza -d
    // esplicito resta il default interattivo (il livello 1) — la radice e
    // le sue righe, non il mondo intero; -d a lo apre tutto (il contratto
    // completo dei diff prima/dopo)
    if json && (bench || loops > 1) {
        eprintln!("{}", cat.t("cli.err_json_bench"));
        std::process::exit(2);
    }
    let order = order_opt.unwrap_or(Order::TD); // default: la piu' grande in cima
    if order_opt.is_some() && depth == 0 {
        eprintln!("{}", cat.t("cli.err_o_d0"));
        std::process::exit(2);
    }
    if top_opt.is_some() && depth == 0 {
        eprintln!("{}", cat.t("cli.err_top_d0"));
        std::process::exit(2);
    }
    // il JSON e' dati completi e deterministici: niente taglio, niente ordine
    // alternativo (due scansioni --json della stessa cartella devono poter
    // diffare solo per cio' che e' cambiato sul disco)
    if json && order_opt.is_some() {
        eprintln!("{}", cat.t("cli.err_json_o"));
        std::process::exit(2);
    }
    if json && top_opt.is_some() {
        eprintln!("{}", cat.t("cli.err_json_top"));
        std::process::exit(2);
    }
    let filter = filter.map(Arc::new);

    // il loader dell'avanzamento (uso interattivo): solo se stderr e' un
    // terminale vero e la corsa e' una sola fuori dal bench — redirect,
    // catture di regressione e contratto RESULT/ITER non devono vederlo
    let live = !bench && loops == 1 && std::io::stderr().is_terminal();

    // la risposta di -big: i file piu' grandi con full path. Come -json con
    // -d a scansiona l'albero completo (il costo e' quello), ma la risposta
    // sono N righe su cui agire, non la mappa. Con -json le N righe
    // diventano la fotografia JSON del report (json::report: il ranking,
    // i path interi, i totali del ramo in testa) — la stessa che il TUI
    // stampa dalla sua finestra del report
    if let Some(n) = big_opt {
        let filter_disp = filter.as_ref().map(|f| f.display().to_string());
        let lens = filter.clone();
        let tree = scan_live(&root, &ScanOptions { depth: usize::MAX, filter }, live, &cat);
        if json {
            print!(
                "{}",
                mfd_core::json::report(
                    &tree,
                    tree.root_id(),
                    n,
                    env!("CARGO_PKG_VERSION"),
                    lens.as_deref()
                )
            );
            return;
        }
        // il look si decide qui come per la griglia, una volta per corsa:
        // terminale vero -> anche la classifica veste il box duf;
        // redirect/pipe -> l'ASCII di sempre, il contratto non cambia
        let look = theme::Look::detect();
        print_big(&tree, n, filter_disp.as_deref(), &look, &cat);
        // le segnalazioni chiudono la lista dentro print_big, come i
        // warning chiudono la griglia: lo scan non sporca la risposta
        return;
    }

    // la fotografia JSON: una sola corsa, l'albero in formato deterministico.
    // Il serializzatore vive in mfd-core, condiviso con la stampa Shift+P
    // del TUI; il limite u32::MAX della profondita' lo dichiara lui
    if json {
        let filter_disp = filter.as_ref().map(|f| f.display().to_string());
        let lens = filter.clone();
        let tree = scan_live(&root, &ScanOptions { depth, filter }, live, &cat);
        let foto = if depth == usize::MAX {
            // -d a: il contratto completo di sempre, quello dei diff
            // prima/dopo una pulizia — il mondo intero, tutto aperto
            mfd_core::json::full(&root, &tree, filter_disp.as_deref(), depth, env!("CARGO_PKG_VERSION"))
        } else if depth == 0 {
            // -d 0: la griglia non ha righe e la foto non ha albero — i
            // totali sono tutta la storia (full con "tree": null; prima
            // di questo era un panic)
            mfd_core::json::full(&root, &tree, filter_disp.as_deref(), 0, env!("CARGO_PKG_VERSION"))
        } else {
            // la griglia a N livelli: la radice aperta, N-1 livelli di rami
            // aperti, il livello N chiuso a righe di totali — le stesse
            // righe della tabella, con la profondita' dichiarata in testa
            let expanded = expanded_to_depth(&tree, depth);
            mfd_core::json::subtree(
                &tree,
                tree.root_id(),
                env!("CARGO_PKG_VERSION"),
                &expanded,
                lens.as_deref(),
                depth,
            )
        };
        print!("{}", foto);
        return;
    }

    for k in 1..=loops {
        let tree = scan_live(&root, &ScanOptions { depth, filter: filter.clone() }, live, &cat);
        let s = tree.stats();
        if loops == 1 && !bench {
            // Con la griglia (depth > 0) la risposta e' la tabella stessa: la riga
            // radice al 100% la apre e la riga TOTALE la chiude incolonnando i
            // totali sotto le rispettive colonne (le stesse info del dettaglio
            // del TUI); le righe WARNING seguono la TOTALE. Con -f, che cambia
            // il senso dei numeri, il filtro appare accanto alla radice.
            let mut notes: Vec<String> = Vec::new();
            if depth > 0 {
                let filter_disp = filter.as_ref().map(|f| f.display().to_string());
                let (top_note, hidden) = print_breakdown(
                    &root,
                    &tree,
                    order,
                    top_opt,
                    filter.is_some(),
                    filter_disp.as_deref(),
                    &cat,
                );
                if let Some(n) = top_note {
                    notes.push(n);
                }
                if hidden > 0 {
                    // -f non nasconde in silenzio: le cartelle senza match escono
                    // dalla griglia ma il blocco note le rendiconta
                    notes.push(if hidden == 1 {
                        cat.t("cli.nota_fil1").to_string()
                    } else {
                        cat.tf("cli.nota_filN", &[&group_digits(hidden as i64)])
                    });
                }
            } else {
                let ms = s.ms;
                let ms_str = if ms < 10.0 {
                    format!("{:.1}", ms).replace('.', ",")
                } else {
                    format!("{:.0}", ms)
                };
                let mut line = cat.tf(
                    "cli.solo_tot",
                    &[
                        &root.display().to_string(),
                        &group_digits(s.files),
                        &group_digits(s.dirs),
                        &fmt_bytes(s.bytes),
                        &ms_str,
                    ],
                );
                if s.errors > 0 {
                    // il plurale onesto, come le voci della TOTALE
                    let suffix = if s.errors == 1 {
                        cat.t("cli.warn_1").to_string()
                    } else {
                        cat.tf("cli.warn_n", &[&s.errors.to_string()])
                    };
                    line.push_str(&suffix);
                }
                if let Some(f) = &filter {
                    line.push(' ');
                    line.push_str(&cat.tf("cli.filtro", &[f.display()]));
                }
                println!("{}", line);
            }
            // blocco di note in fondo, separato dalla griglia da una riga vuota:
            // le NOTE (taglio -top, cartelle nascoste dal filtro) — le
            // segnalazioni invece chiudono gia' la tabella, righe WARNING sotto
            // la TOTALE, per non sporcare la lettura dei risultati
            if !notes.is_empty() {
                println!();
            }
            for note in &notes {
                println!("{}", note);
            }
        } else if loops == 1 {
            println!(
                "RESULT ms={:.0} bytes={} files={} dirs={} errors={}",
                s.ms, s.bytes, s.files, s.dirs, s.errors
            );
        } else {
            println!(
                "ITER {}/{} ms={:.0} bytes={} files={} dirs={} errors={}",
                k, loops, s.ms, s.bytes, s.files, s.dirs, s.errors
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Una fixture minima: una cartella con due file (42 B in tutto).
    fn fixture(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("mfd-cli-test-{}-{}", std::process::id(), tag));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.txt"), vec![0u8; 10]).unwrap();
        std::fs::write(dir.join("b.txt"), vec![0u8; 32]).unwrap();
        dir
    }

    #[test]
    fn la_riga_totale_chiude_la_griglia_incolonnata() {
        // la lettura e' la classica verticale: ogni colonna si chiude col suo
        // cumulativo — la dimensione sotto DIMENSIONE, i file sotto FILE, le
        // cartelle sotto CARTELLE — e in colonna NOME l'etichetta col conto
        // delle voci (virgola decimale, punti in migliaia, plurale onesto);
        // % e barra restano vuote: la somma non e' una quota di se' stessa
        // (la radice, in cima, e' gia' il 100%)
        let t = Totals { bytes: 3000, files: 2, dirs: 3, voci: 6 };
        let cat = test_cat();
        let tot = totals_row(&t, &cat);
        assert!(tot.starts_with("     2,9 KB"));
        assert!(tot.ends_with("TOTALE: 6 voci"));
        assert_eq!(voci(1, &cat), "1 voce");
        assert_eq!(voci(1234, &cat), "1.234 voci");
        // le colonne coincidono con quelle della riga dati, al byte: la
        // dimensione, i file e le cartelle cadono sugli stessi caratteri
        let data = ascii_row(
            &GridRow {
                bytes: 3000,
                files: Some(2),
                dirs: Some(3),
                name: "|-- sub\\".to_string(),
            },
            10000,
        );
        assert_eq!(tot.find("2,9 KB"), data.find("2,9 KB"));
        assert_eq!(tot.rfind('2'), data.rfind('2')); // il 2 dei FILE
        assert_eq!(tot.rfind('3'), data.rfind('3')); // il 3 delle CARTELLE
    }

    #[test]
    fn le_righe_warning_seguono_la_totale_fuse_in_una_cella() {
        // il footer rendiconta anche le segnalazioni: la prima cella e'
        // l'etichetta (solo sulla prima riga), tutto il resto e' una cella
        // fusa di testo che comincia dove comincia la colonna % — piu'
        // righe restano ordinate, i trattini in colonna
        let cat = test_cat();
        let first = warning_row(true, "- apertura cartella C:\\x: Accesso negato.", &cat);
        assert_eq!(first, "    WARNING  - apertura cartella C:\\x: Accesso negato.");
        let second = warning_row(false, "- enumerazione in C:\\y: Accesso negato.", &cat);
        assert_eq!(second, "             - enumerazione in C:\\y: Accesso negato.");
        // il testo fuso comincia al byte dove la riga dati comincia la
        // cella %: le colonne della tabella guidano anche il footer
        let data = ascii_row(
            &GridRow {
                bytes: 3000,
                files: Some(2),
                dirs: Some(3),
                name: "|-- sub\\".to_string(),
            },
            10000,
        );
        assert_eq!(first.find("- apertura"), data.find(" 30"));
        assert_eq!(second.find("- enumerazione"), data.find(" 30"));
    }

    #[test]
    fn warning_lines_una_riga_per_segnalazione_col_trattino() {
        // funzione pura condivisa da griglia e -big: una riga per warning,
        // col trattino che le divide; oltre dieci, l'ultima rendiconta
        // quante restano fuori (singolare e plurale onesti); vuota senza
        // segnalazioni — il footer non compare
        let cat = test_cat();
        let mut wl = WarningLog::default();
        assert!(warning_lines(&wl, &cat).is_empty());
        for i in 0..3 {
            wl.push(Warning {
                path: PathBuf::from(format!("C:\\d{}", i)),
                kind: WarningKind::OpenDir,
                message: "Accesso negato.".to_string(),
            });
        }
        let lines = warning_lines(&wl, &cat);
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], "- apertura cartella C:\\d0: Accesso negato.");
        assert!(lines.iter().all(|l| l.starts_with("- ")));
        // undici segnalazioni: dieci righe e quella che manca
        for i in 3..11 {
            wl.push(Warning {
                path: PathBuf::from(format!("C:\\d{}", i)),
                kind: WarningKind::EnumEntry,
                message: "Accesso negato.".to_string(),
            });
        }
        let lines = warning_lines(&wl, &cat);
        assert_eq!(lines.len(), 11);
        assert_eq!(lines[10], "... e un'altra segnalazione");
        // e al plurale
        wl.push(Warning {
            path: PathBuf::from("C:\\d11"),
            kind: WarningKind::OpenDir,
            message: "Accesso negato.".to_string(),
        });
        let lines = warning_lines(&wl, &cat);
        assert_eq!(lines[10], "... e altre 2 segnalazioni");
    }

    #[test]
    fn scan_live_senza_loader_e_la_solita_scansione() {
        let fx = fixture("quiet");
        let tree = scan_live(&fx, &ScanOptions { depth: 1, filter: None }, false, &test_cat());
        assert_eq!(tree.stats().files, 2);
        assert_eq!(tree.stats().bytes, 42);
        let _ = std::fs::remove_dir_all(&fx);
    }

    #[test]
    fn scan_live_con_loader_scansiona_uguale() {
        // il percorso live: la riga va su stderr (qui catturato dall'harness
        // dei test) e la scansione deve dare gli stessi numeri di quella
        // silenziosa — il loader non tocca i risultati
        let fx = fixture("live");
        let tree = scan_live(&fx, &ScanOptions { depth: 1, filter: None }, true, &test_cat());
        assert_eq!(tree.stats().files, 2);
        assert_eq!(tree.stats().bytes, 42);
        let _ = std::fs::remove_dir_all(&fx);
    }

    /// La fixture di -big: taglie diverse in profondita' e una parita' di
    /// taglia fra due cartelle (il tie-break dell'ordine deterministico)
    fn fixture_big(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("mfd-cli-test-{}-{}", std::process::id(), tag));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("m")).unwrap();
        std::fs::create_dir_all(dir.join("z")).unwrap();
        std::fs::write(dir.join("z/big.dat"), vec![0u8; 100]).unwrap();
        std::fs::write(dir.join("b.txt"), vec![0u8; 30]).unwrap();
        std::fs::write(dir.join("m/eq.bin"), vec![0u8; 30]).unwrap();
        std::fs::write(dir.join("z/eq.bin"), vec![0u8; 30]).unwrap();
        std::fs::write(dir.join("small.txt"), vec![0u8; 5]).unwrap();
        dir
    }

    #[test]
    fn big_le_righe_ordinano_per_taglia_con_full_path() {
        // la forma e' il contratto d'output: la prima colonna e' la DIMENSIONE
        // della griglia, accanto il full path; ordine fisso taglia
        // decrescente, a parita' path crescente (b.txt < m\eq.bin < z\eq.bin)
        let fx = fixture_big("righe");
        let tree = scan_live(&fx, &ScanOptions { depth: usize::MAX, filter: None }, false, &test_cat());
        let lines = big_lines(&tree, 10);
        assert_eq!(lines.len(), 5);
        assert_eq!(
            lines[0],
            format!(" {:>10}  {}", "100 B", fx.join("z\\big.dat").display())
        );
        assert!(lines[1].ends_with("b.txt"));
        assert!(lines[2].ends_with("m\\eq.bin"));
        assert!(lines[3].ends_with("z\\eq.bin"));
        assert!(lines[4].ends_with("small.txt"));
        // il taglio a N tiene i primi N, anche quando i file sono meno
        assert_eq!(big_lines(&tree, 2).len(), 2);
        let _ = std::fs::remove_dir_all(&fx);
    }

    #[test]
    fn big_con_filtro_conta_solo_i_match() {
        // -big e -f: lo scan gira sotto la lente e le righe portano solo i
        // file che matchano (l'intestazione lo dichiara in print_big)
        let fx = fixture_big("filtro");
        let f = Arc::new(Filter::build("*.bin").unwrap());
        let tree = scan_live(&fx, &ScanOptions { depth: usize::MAX, filter: Some(f) }, false, &test_cat());
        let lines = big_lines(&tree, 10);
        assert_eq!(lines.len(), 2);
        assert!(lines[0].ends_with("m\\eq.bin"));
        assert!(lines[1].ends_with("z\\eq.bin"));
        let _ = std::fs::remove_dir_all(&fx);
    }

    #[test]
    fn i_rami_aperti_della_foto_seguono_il_livello_della_griglia() {
        // la fotografia -d N -json apre i rami come la tabella: a -d 1 solo
        // la radice (i figli restano righe di totali), a -d 2 anche il
        // primo livello, a profondita' illimitata tutto l'albero
        let fx = std::env::temp_dir().join(format!("mfd-cli-test-{}-aperti", std::process::id()));
        let _ = std::fs::remove_dir_all(&fx);
        std::fs::create_dir_all(fx.join("uno\\sotto")).unwrap();
        std::fs::write(fx.join("a.txt"), vec![0u8; 10]).unwrap();
        std::fs::write(fx.join("uno\\b.bin"), vec![0u8; 20]).unwrap();
        std::fs::write(fx.join("uno\\sotto\\c.bin"), vec![0u8; 30]).unwrap();
        let tree = scan_live(
            &fx,
            &ScanOptions { depth: usize::MAX, filter: None },
            false,
            &test_cat(),
        );
        let radice = tree.root_id();
        let uno = *tree
            .children(radice)
            .iter()
            .find(|&&c| tree.name(c).to_string_lossy() == "uno")
            .unwrap();
        let sotto = *tree
            .children(uno)
            .iter()
            .find(|&&c| tree.name(c).to_string_lossy() == "sotto")
            .unwrap();
        assert_eq!(expanded_to_depth(&tree, 1), HashSet::from([radice]));
        assert_eq!(expanded_to_depth(&tree, 2), HashSet::from([radice, uno]));
        assert_eq!(
            expanded_to_depth(&tree, usize::MAX),
            HashSet::from([radice, uno, sotto])
        );
        let _ = std::fs::remove_dir_all(&fx);
    }

    #[test]
    fn la_riga_ascii_della_griglia_resta_identica() {
        // il contratto d'output: ridiretta (redirect, log, catture) la riga
        // della griglia non cambia di un byte — ne' celle ne' spaziature
        let dir = GridRow {
            bytes: 3000,
            files: Some(2),
            dirs: Some(3),
            name: "|-- sub\\".to_string(),
        };
        assert_eq!(
            ascii_row(&dir, 10000),
            "     2,9 KB   30%  #######.................            2          3  |-- sub\\"
        );
        // la foglia: trattini al posto dei conteggi, niente backslash finale
        let leaf = GridRow {
            bytes: 500,
            files: None,
            dirs: None,
            name: "`-- x.txt".to_string(),
        };
        assert_eq!(
            ascii_row(&leaf, 10000),
            "      500 B    5%  #.......................            -          -  `-- x.txt"
        );
    }

    #[test]
    fn le_forme_del_flag_lingua_si_normalizzano_a_una_sola() {
        // -lang it, -lang=it, -lang:it e le doppie -- sono la stessa
        // richiesta: il parsing ne vede una sola, chi digita come gli
        // viene naturale non sbaglia
        assert_eq!(
            split_lang_arg("-lang=it".to_string()),
            vec!["-lang".to_string(), "it".to_string()]
        );
        assert_eq!(
            split_lang_arg("-lang:it".to_string()),
            vec!["-lang".to_string(), "it".to_string()]
        );
        assert_eq!(
            split_lang_arg("--lang=it".to_string()),
            vec!["-lang".to_string(), "it".to_string()]
        );
        assert_eq!(
            split_lang_arg("--lang:it".to_string()),
            vec!["-lang".to_string(), "it".to_string()]
        );
        // il flag nudo passa tale e quale (il valore viene dopo), e un
        // argomento che non c'entra non si tocca
        assert_eq!(
            split_lang_arg("-lang".to_string()),
            vec!["-lang".to_string()]
        );
        assert_eq!(split_lang_arg("-big".to_string()), vec!["-big".to_string()]);
        assert_eq!(
            split_lang_arg("D:\\SVILUPPO".to_string()),
            vec!["D:\\SVILUPPO".to_string()]
        );
    }

    #[test]
    fn il_vocabolario_della_cli_esiste_in_inglese_e_italiano() {
        // la faccia della CLI parla il vocabolario: l'inglese incorporato e
        // l'italiano del crate devono avere le stesse chiavi (il gate di
        // parita' di mfd-core lo chiede su tutti i file; qui si tocca con
        // mano che le chiavi che il codice usa ci sono e rendono)
        let en = Catalog::english();
        assert!(en.t("cli.tagline").contains("mfd-cli"));
        assert!(en.t("cli.help").starts_with("USAGE"));
        assert_eq!(en.t("cli.dim"), "SIZE");
        assert_eq!(en.t("cli.warn"), "WARNING");
        let it = test_cat();
        assert_eq!(it.t("cli.dim"), "DIMENSIONE");
        assert_eq!(it.t("cli.cartelle"), "CARTELLE");
        assert!(it.t("cli.help").starts_with("USO"));
        // e la tagline porta l'argomento al posto del segnaposto
        assert!(it.tf("cli.tagline", &["9.9"]).contains("9.9"));
    }
}

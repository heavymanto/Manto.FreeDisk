//! Il formato della fotografia JSON: nato per `--json` della CLI (i diff
//! prima/dopo una pulizia, la domanda in coda dalla nascita del progetto) e
//! vissuto nel crate della CLI finche' la CLI era l'unico consumatore. Con
//! la stampa del TUI (tasto p) il contratto diventa condiviso e sale nel
//! motore: la CLI lo stampa su stdout, il TUI lo scrive su file — lo stesso
//! JSON, il principio 2 del piano ("snapshot come contratto, leggibile da
//! tutti i front-end"). Serializzatore interno, senza dipendenze:
//!
//! Quattro inquadrature dello stesso formato: `full` fotografa tutto (la
//! CLI con `-d a -json`: il diff prima/dopo una pulizia vuole il mondo
//! intero), `subtree` l'albero come la vista lo apre (i rami aperti e la
//! lente dichiarata — e' l'inquadratura della fotografia della griglia
//! della CLI, `-d N -json`: le righe che l'utente vede, niente di piu'),
//! `view` la stampa Shift+P / F11 dei DUE pannelli del TUI (l'albero di
//! sinistra piu' il dettaglio di destra: la riga selezionata e i suoi
//! figli) e `report` la stessa stampa dalla finestra Shift+R / F9, nonche'
//! la risposta di `-big N -json` della CLI (i big col ranking): la foto
//! salva quello che si vede a schermo, non il mondo intero.
//!
//! - **ASCII puro** come tutto l'output mfd (contratto 3): i caratteri
//!   non-ASCII dei nomi escono come escape \uXXXX, il JSON resta valido e
//!   identico in ogni codepage;
//! - **deterministico**: figli e foglie in ordine di taglia decrescente
//!   (T.D, il default della griglia), cosi' due fotografie della stessa
//!   cartella diffano solo se il filesystem e' cambiato;
//! - chiavi in ordine fisso, indentazione di 2 spazi;
//! - il campo `mfd` e' la versione del frontend che scatta la foto: la
//!   dichiara chi chiama (la CLI la sua, il TUI la sua), perche' sapere
//!   chi ha fotografato fa parte del dato.

use crate::filter::Filter;
use crate::tree::{NodeMetrics, Tree};
use crate::view::{Order, Row};
use crate::warning::{Warning, WarningLog};
use std::collections::HashSet;
use std::path::Path;

/// Escape JSON di una stringa, ASCII puro. Condiviso con lang.rs (il
/// round-trip del paio: questo scrive, il lettore delle lingue rilegge).
pub(crate) fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 || (c as u32) > 0x7e => {
                let n = c as u32;
                if n > 0xffff {
                    // fuori dal BMP: la coppia di surrogate, come vuole
                    // JSON (un \uXXXXXX a sei cifre non e' un escape
                    // valido — il round-trip con lang.rs l'ha portato alla
                    // luce: i nomi-file emoji ora escono come JSON vero)
                    let hi = 0xd800 + ((n - 0x10000) >> 10);
                    let lo = 0xdc00 + ((n - 0x10000) & 0x3ff);
                    out.push_str(&format!("\\u{:04x}\\u{:04x}", hi, lo));
                } else {
                    out.push_str(&format!("\\u{:04x}", n));
                }
            }
            c => out.push(c),
        }
    }
    out
}

/// Stringa JSON fra doppi apici, con escape.
fn q(s: &str) -> String {
    format!("\"{}\"", esc(s))
}

/// Come la fotografia inquadra l'albero. `Full` e' il contratto completo
/// di `--json`: tutto aperto, metriche strutturali — la lente, se c'era,
/// l'ha gia' applicata la scansione stessa (-f). `View` e' la stampa del
/// TUI: l'albero come lo vede l'utente — i rami che ha aperto si aprono
/// anche nella foto, i chiusi restano righe di totali, e la lente vale
/// con le stesse regole della lista e dell'anteprima.
enum Framing<'a> {
    Full,
    View {
        expanded: &'a HashSet<u32>,
        filter: Option<&'a Filter>,
    },
}

/// Un nodo dell'albero: nome (la testa della fotografia non ce l'ha: il suo
/// path sta nell'intestazione), totali ricorsivi, figli-cartella e foglie,
/// ciascun gruppo in ordine T.D. Nella vista un ramo chiuso non ha ne'
/// figli ne' foglie nella foto: solo i totali ("children": null).
fn node_json(tree: &Tree, id: u32, ind: usize, head: bool, fr: &Framing) -> String {
    let pad = "  ".repeat(ind);
    let pad_in = "  ".repeat(ind + 1);
    // la lente della vista: nessuna nella foto completa (la scansione
    // l'ha gia' applicata, se c'era, e i numeri sono gia' filtrati)
    let lens = match fr {
        Framing::Full => None,
        Framing::View { filter, .. } => *filter,
    };
    // i totali del nodo: attraverso la lente se c'e' (i numeri che la
    // riga mostra a schermo), strutturali altrimenti. Le cartelle non
    // passano dalla lente nemmeno con -f (si attraversano comunque
    // tutte): il loro conteggio resta strutturale, come nella CLI
    let (bytes, files) = match lens {
        Some(f) => tree.subtree_match(id, f),
        None => {
            let m = tree.metrics(id);
            (m.bytes, m.files)
        }
    };
    let dirs_count = tree.metrics(id).dirs;
    let mut s = format!("{}{{\n", pad);
    if !head {
        s.push_str(&format!(
            "{}\"name\": {},\n",
            pad_in,
            q(&tree.name(id).to_string_lossy())
        ));
    }
    s.push_str(&format!("{}\"bytes\": {},\n", pad_in, bytes));
    s.push_str(&format!("{}\"files\": {},\n", pad_in, files));
    s.push_str(&format!("{}\"dirs\": {},\n", pad_in, dirs_count));
    // ramo aperto? La radice della foto sempre (la lista la mostra);
    // nella vista anche i rami che l'utente ha aperto con x / a. Un
    // ramo chiuso fotografa solo i suoi totali: "children": null non
    // e' una cartella vuota, e' un ramo che la foto lascia com'era a
    // schermo
    let open = match fr {
        Framing::Full => true,
        Framing::View { expanded, .. } => head || expanded.contains(&id),
    };
    if !open {
        s.push_str(&format!("{}\"children\": null,\n", pad_in));
        s.push_str(&format!("{}\"leaves\": null\n", pad_in));
        s.push_str(&format!("{}}}", pad));
        return s;
    }
    let mut dirs: Vec<Row> = tree.children(id).iter().map(|&c| Row::Dir(c)).collect();
    // nella vista con lente i rami senza match non si vedono (la regola
    // della lista dalla 0.7.0): non entrano nella foto
    if let Some(f) = lens {
        dirs.retain(|r| match r {
            Row::Dir(c) => {
                let (b, n) = tree.subtree_match(*c, f);
                !(n == 0 && b == 0)
            }
            Row::Leaf(..) => true,
        });
    }
    tree.sort_rows(&mut dirs, Order::TD);
    let mut leaves: Vec<Row> = tree
        .leaves(id)
        .into_iter()
        .map(|l| Row::Leaf(l.size, l.name))
        .collect();
    // le foglie non matching sono invisibili alla lente, come
    // nell'anteprima
    if let Some(f) = lens {
        leaves.retain(|r| match r {
            Row::Leaf(_, name) => f.is_match(name),
            Row::Dir(..) => true,
        });
    }
    tree.sort_rows(&mut leaves, Order::TD);
    s.push_str(&format!("{}\"children\": [", pad_in));
    if dirs.is_empty() {
        s.push_str("],\n");
    } else {
        s.push('\n');
        let parts: Vec<String> = dirs
            .iter()
            .map(|r| match r {
                Row::Dir(c) => node_json(tree, *c, ind + 2, false, fr),
                Row::Leaf(..) => unreachable!("solo figli cartella"),
            })
            .collect();
        s.push_str(&parts.join(",\n"));
        s.push_str(&format!("\n{}],\n", pad_in));
    }
    s.push_str(&format!("{}\"leaves\": [", pad_in));
    if leaves.is_empty() {
        s.push_str("]\n");
    } else {
        s.push('\n');
        let parts: Vec<String> = leaves
            .iter()
            .map(|r| match r {
                Row::Leaf(sz, name) => format!(
                    "{}{{\"name\": {}, \"bytes\": {}}}",
                    "  ".repeat(ind + 2),
                    q(&name.to_string_lossy()),
                    sz
                ),
                Row::Dir(..) => unreachable!("solo foglie"),
            })
            .collect();
        s.push_str(&parts.join(",\n"));
        s.push_str(&format!("\n{}]\n", pad_in));
    }
    s.push_str(&format!("{}}}", pad));
    s
}

/// L'intestazione della fotografia: meta (versione, radice, filtro,
/// profondita', totali, warning) — la parte che il diff legge per prima.
fn header(
    out: &mut String,
    version: &str,
    root: &Path,
    filter: Option<&str>,
    depth: usize,
    m: &NodeMetrics,
    wl: &[&Warning],
    ms: f64,
) {
    out.push_str("{\n");
    out.push_str(&format!("  \"mfd\": {},\n", q(version)));
    out.push_str(&format!("  \"root\": {},\n", q(&root.display().to_string())));
    match filter {
        Some(f) => out.push_str(&format!("  \"filter\": {},\n", q(f))),
        None => out.push_str("  \"filter\": null,\n"),
    }
    out.push_str(&format!("  \"depth\": {},\n", depth));
    out.push_str(&format!(
        "  \"totals\": {{\"bytes\": {}, \"files\": {}, \"dirs\": {}, \"warnings\": {}, \"elapsed_ms\": {:.1}}},\n",
        m.bytes, m.files, m.dirs, wl.len(), ms
    ));
    if wl.is_empty() {
        out.push_str("  \"warnings\": [],\n");
    } else {
        out.push_str("  \"warnings\": [\n");
        let parts: Vec<String> = wl
            .iter()
            .map(|w| {
                format!(
                    "    {{\"path\": {}, \"message\": {}}}",
                    q(&w.path.display().to_string()),
                    q(&w.message)
                )
            })
            .collect();
        out.push_str(&parts.join(",\n"));
        out.push_str("\n  ],\n");
    }
}

/// La fotografia completa dello scan: quella di `--json` della CLI, byte
/// per byte. `root` e' il path come chiesto all'utente, `depth` la
/// profondita' di aggregazione (illimitata di norma), `filter` la lente
/// dichiarata nell'intestazione (-f). Esce come Stringa: a stamparla e'
/// il frontend.
pub fn full(root: &Path, tree: &Tree, filter_disp: Option<&str>, depth: usize, version: &str) -> String {
    let s = tree.stats();
    let m = NodeMetrics {
        bytes: s.bytes,
        files: s.files,
        dirs: s.dirs,
    };
    let wl: Vec<&Warning> = tree.warnings().iter().collect();
    let mut out = String::new();
    // il limite reale dell'aggregazione e' u32::MAX (il livello e' un u32):
    // usize::MAX uscirebbe come intero oltre i limiti sicuri del JSON
    header(
        &mut out,
        version,
        root,
        filter_disp,
        depth.min(u32::MAX as usize),
        &m,
        &wl,
        s.ms,
    );
    // -d 0 (o un albero senza nemmeno la radice): nessuna riga da
    // fotografare, i totali sono tutta la storia. "tree": null non e'
    // una cartella vuota: e' una foto senza righe (prima di questo era
    // un panic: l'indice della radice su un albero di zero nodi)
    if tree.node_count() == 0 {
        out.push_str("  \"tree\": null\n}\n");
        return out;
    }
    out.push_str("  \"tree\": ");
    // il nodo radice parte attaccato al "tree": (si toglie il pad della
    // prima riga); i suoi campi restano un livello sotto, come i fratelli
    let node = node_json(tree, tree.root_id(), 1, true, &Framing::Full);
    out.push_str(node.trim_start_matches(' '));
    out.push_str("\n}\n");
    out
}

/// La fotografia del sottoalbero di `id` — l'albero come la vista lo
/// apre: la cartella dove si sta diventa la testa della foto, come il
/// report Shift+R / F9 e' -big della cartella corrente. L'inquadratura
/// e' la vista: `expanded` sono i rami aperti e nella foto si aprono,
/// gli altri restano righe di totali ricorsivi ("children": null);
/// `filter` e' la lente, che vale come nella lista e nell'anteprima e si
/// dichiara nell'intestazione — la foto salva quello che si vede a
/// schermo, non il mondo intero (quello e' il lavoro di `full`). La CLI
/// la usa per la fotografia della sua griglia (`-d N -json`): `expanded`
/// i rami fino al livello N-1, il livello N chiuso a righe di totali —
/// le righe della tabella — e `depth` la profondita' dichiarata in
/// testa. Stesso contratto per il resto: warning solo quelli del ramo
/// (la regola di count_under), path assoluti (la radice dello scan
/// davanti, perche' la foto si legge anche da sola). E' l'albero di
/// sinistra da solo: la stampa del TUI (Shift+P / F11) e' `view`, che a
/// questa testa attacca anche il dettaglio di destra.
pub fn subtree(
    tree: &Tree,
    id: u32,
    version: &str,
    expanded: &HashSet<u32>,
    filter: Option<&Filter>,
    depth: usize,
) -> String {
    let mut out = String::new();
    photo_head(tree, id, version, expanded, filter, depth, &mut out);
    out.push_str("\n}\n");
    out
}

/// La testa della fotografia di vista: intestazione (la radice e' il
/// sottoalbero di `id`, i totali passano dalla lente, i warning sono
/// quelli del ramo) e l'albero con l'inquadratura della vista. Senza la
/// chiusura: la mette chi chiama, magari dopo aver attaccato altre
/// sezioni (il dettaglio di destra). La condividono subtree e view: le
/// due foto differiscono solo per la coda. `depth` e' la profondita'
/// dichiarata in testa, quella di aggregazione della foto: per la vista
/// del TUI, che scansiona sempre tutto, e' illimitata; per la griglia
/// della CLI e' il suo -d.
fn photo_head(
    tree: &Tree,
    id: u32,
    version: &str,
    expanded: &HashSet<u32>,
    filter: Option<&Filter>,
    depth: usize,
    out: &mut String,
) {
    let base = tree.path_of(id);
    let m = lens_metrics(tree, id, filter);
    let wl = warnings_under(tree.warnings(), &base);
    let fr = Framing::View { expanded, filter };
    header(
        out,
        version,
        &base,
        filter.map(|f| f.display()),
        depth.min(u32::MAX as usize),
        &m,
        &wl,
        tree.stats().ms,
    );
    out.push_str("  \"tree\": ");
    let node = node_json(tree, id, 1, true, &fr);
    out.push_str(node.trim_start_matches(' '));
}

/// I totali di un nodo attraverso la lente: i byte contano (i numeri che
/// il titolo mostra quando c'e' un filtro), i file pure, le cartelle
/// restano strutturali (si attraversano comunque tutte, come nella CLI
/// con -f).
fn lens_metrics(tree: &Tree, id: u32, filter: Option<&Filter>) -> NodeMetrics {
    let (bytes, files) = match filter {
        Some(f) => tree.subtree_match(id, f),
        None => {
            let m = tree.metrics(id);
            (m.bytes, m.files)
        }
    };
    NodeMetrics {
        bytes,
        files,
        dirs: tree.metrics(id).dirs,
    }
}

/// La fotografia della VISTA del TUI: la stampa p / F11 dalla schermata
/// principale, ENTRAMBI i pannelli. La testa e' l'albero di sinistra
/// (il sottoalbero di `id` coi rami aperti, come `subtree`); in coda la
/// sezione `detail` fotografa il pannello di destra: `selected` e' la
/// riga selezionata della lista e `rows` i suoi figli — perche' il
/// dettaglio mostra cio' che l'albero tiene chiuso: il ramo selezionato
/// resta una riga di totali ricorsivi nella sezione tree, ma i suoi
/// figli stanno nella sezione detail. La foto dice cosa si sta guardando
/// da entrambe le parti. `selected` None (lista vuota) vale "detail":
/// null. Stesse regole dell'anteprista: la lente vale (i rami senza
/// match e le foglie non matching non compaiono) e l'ordine delle righe
/// e' quello canonico della foto (T.D, come l'albero), non quello scelto
/// col tasto s: due fotografie della stessa vista diffano solo se il
/// filesystem e' cambiato, non per l'ordinamento del momento.
pub fn view(
    tree: &Tree,
    id: u32,
    selected: Option<u32>,
    version: &str,
    expanded: &HashSet<u32>,
    filter: Option<&Filter>,
) -> String {
    let mut out = String::new();
    // la vista del TUI scansiona sempre l'albero intero: la profondita'
    // dichiarata e' illimitata (a chiudere i rami e' la vista, non un -d)
    photo_head(tree, id, version, expanded, filter, usize::MAX, &mut out);
    out.push_str(",\n");
    out.push_str(&detail_json(tree, selected, filter));
    out.push_str("\n}\n");
    out
}

/// La fotografia del REPORT del TUI: la stampa p / F11 dalla finestra
/// r / F9, quello che la finestra mostra. `top` e' il taglio chiesto in
/// finestra (le cifre): "top": 10 con sette file dentro e' il report di
/// chi ha chiesto dieci e ne ha trovati sette — la stessa onesta' del
/// titolo. Le righe sono la risposta di top_files_under (-big della
/// cartella `id`, radice compresa) con il ranking davanti e i path
/// ASSOLUTI, perche' la foto si legge anche da sola. La testa e' quella
/// delle altre foto: la cartella fotografata, i totali del ramo
/// attraverso la lente, i warning del ramo — e la lente si dichiara,
/// come fa la finestra col suo titolo.
pub fn report(tree: &Tree, id: u32, top: usize, version: &str, filter: Option<&Filter>) -> String {
    let base = tree.path_of(id);
    let m = lens_metrics(tree, id, filter);
    let wl = warnings_under(tree.warnings(), &base);
    let mut out = String::new();
    header(
        &mut out,
        version,
        &base,
        filter.map(|f| f.display()),
        u32::MAX as usize,
        &m,
        &wl,
        tree.stats().ms,
    );
    let big = tree.top_files_under(id, top, filter);
    out.push_str(&format!("  \"report\": {{\n    \"top\": {},\n", top));
    if big.is_empty() {
        out.push_str("    \"files\": []\n  }\n");
    } else {
        out.push_str("    \"files\": [\n");
        let parts: Vec<String> = big
            .iter()
            .enumerate()
            .map(|(i, (s, p))| {
                format!(
                    "      {{\"rank\": {}, \"bytes\": {}, \"path\": {}}}",
                    i + 1,
                    s,
                    q(&p.display().to_string())
                )
            })
            .collect();
        out.push_str(&parts.join(",\n"));
        out.push_str("\n    ]\n  }\n");
    }
    out.push_str("}\n");
    out
}

/// La sezione del pannello di destra: la riga selezionata (`selected`,
/// path assoluto), i suoi totali con la stessa regola del titolo
/// dell'anteprima (byte attraverso la lente, file e cartelle
/// strutturali) e le sue righe — figli-cartella e foglie insieme, come
/// le mostra l'anteprista: stessa pipeline (rows_of + sort_rows in
/// T.D), stessa lente (i rami senza match e le foglie non matching non
/// compaiono). Le righe-cartella portano kind "dir" con i totali
/// ricorsivi, le foglie kind "file" con la loro taglia.
fn detail_json(tree: &Tree, selected: Option<u32>, filter: Option<&Filter>) -> String {
    let Some(sid) = selected else {
        return "  \"detail\": null".to_string();
    };
    let base = tree.path_of(sid);
    let m = lens_metrics(tree, sid, filter);
    let mut s = String::new();
    s.push_str("  \"detail\": {\n");
    s.push_str(&format!(
        "    \"selected\": {},\n",
        q(&base.display().to_string())
    ));
    s.push_str(&format!("    \"bytes\": {},\n", m.bytes));
    s.push_str(&format!("    \"files\": {},\n", m.files));
    s.push_str(&format!("    \"dirs\": {},\n", m.dirs));
    let mut rows = tree.rows_of(sid);
    if let Some(f) = filter {
        rows.retain(|r| match r {
            Row::Dir(c) => {
                let (b, n) = tree.subtree_match(*c, f);
                !(n == 0 && b == 0)
            }
            Row::Leaf(_, name) => f.is_match(name),
        });
    }
    tree.sort_rows(&mut rows, Order::TD);
    if rows.is_empty() {
        s.push_str("    \"rows\": []\n  }");
        return s;
    }
    s.push_str("    \"rows\": [\n");
    let parts: Vec<String> = rows
        .iter()
        .map(|r| match r {
            Row::Dir(c) => {
                let b = match filter {
                    Some(f) => tree.subtree_match(*c, f).0,
                    None => tree.metrics(*c).bytes,
                };
                let cm = tree.metrics(*c);
                format!(
                    "      {{\"name\": {}, \"kind\": \"dir\", \"bytes\": {}, \"files\": {}, \"dirs\": {}}}",
                    q(&tree.name(*c).to_string_lossy()),
                    b,
                    cm.files,
                    cm.dirs
                )
            }
            Row::Leaf(sz, name) => format!(
                "      {{\"name\": {}, \"kind\": \"file\", \"bytes\": {}}}",
                q(&name.to_string_lossy()),
                sz
            ),
        })
        .collect();
    s.push_str(&parts.join(",\n"));
    s.push_str("\n    ]\n  }");
    s
}

/// Le segnalazioni che cadono nel sottoalbero di `base`: la stessa regola
/// di WarningLog::count_under (confronto per componenti del path, non per
/// prefisso di stringa).
fn warnings_under<'a>(wl: &'a WarningLog, base: &Path) -> Vec<&'a Warning> {
    wl.iter().filter(|w| w.path.starts_with(base)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scan::ScanStats;
    use crate::tree::{AggNode, Leaf};
    use crate::warning::WarningKind;
    use std::ffi::OsString;
    use std::path::PathBuf;
    use std::sync::atomic::AtomicI64;
    use std::sync::{Arc, Mutex};

    /// Un nodo come li costruisce il walker: la radice non ha nome e ha
    /// parent u32::MAX (non compare fra i figli di nessuno).
    fn nodo(
        name: &str,
        parent: u32,
        bytes: i64,
        files: i64,
        dirs: i64,
        leaves: &[(&str, i64)],
    ) -> Arc<AggNode> {
        Arc::new(AggNode {
            name: OsString::from(name),
            parent,
            bytes: AtomicI64::new(bytes),
            files: AtomicI64::new(files),
            dirs: AtomicI64::new(dirs),
            leaves: Mutex::new(
                leaves
                    .iter()
                    .map(|(n, s)| Leaf {
                        size: *s,
                        name: OsString::from(*n),
                    })
                    .collect(),
            ),
        })
    }

    /// Un path come appare nel JSON: i backslash raddoppiati dall'escape
    /// (la fixture e' ASCII, quindi e' l'unico caso).
    fn jpath(p: &Path) -> String {
        p.display().to_string().replace('\\', "\\\\")
    }

    /// L'albero dei warning: radice + ramo + altro, con una segnalazione
    /// dentro il ramo, una nell'altro ramo e una sulla radice stessa (che
    /// nella foto del ramo non c'e': la radice non e' sotto se stessa).
    fn albero(root: &Path) -> Tree {
        let nodes = vec![
            nodo("", u32::MAX, 300, 3, 2, &[]), // 0: la radice
            nodo("ramo", 0, 200, 2, 0, &[("a.bin", 120), ("b.bin", 80)]), // 1
            nodo("altro", 0, 100, 1, 0, &[("c.bin", 100)]),               // 2
        ];
        let mut wl = WarningLog::default();
        wl.push(Warning {
            path: root.join("ramo").join("a.bin"),
            kind: WarningKind::Metadata,
            message: "metadata fallito".to_string(),
        });
        wl.push(Warning {
            path: root.join("altro").join("c.bin"),
            kind: WarningKind::EnumEntry,
            message: "voce illeggibile".to_string(),
        });
        wl.push(Warning {
            path: root.to_path_buf(),
            kind: WarningKind::OpenDir,
            message: "apertura negata".to_string(),
        });
        Tree::from_parts(
            root.to_path_buf(),
            nodes,
            ScanStats {
                ms: 1.0,
                bytes: 300,
                files: 3,
                dirs: 2,
                errors: 3,
            },
            wl,
        )
    }

    #[test]
    fn il_sottoalbero_rendiconta_solo_i_warning_del_ramo() {
        let root = PathBuf::from("/fx");
        let tree = albero(&root);
        let sotto = subtree(&tree, 1, "0.0.0", &HashSet::new(), None, usize::MAX);
        // una segnalazione nel ramo, due fuori (l'altro ramo e la radice
        // stessa): la stessa regola di count_under, quella del conto
        // "N warning nel ramo" dell'anteprima del TUI
        assert_eq!(tree.warnings().count_under(&root.join("ramo")), 1);
        assert!(sotto.contains(&format!(r#""root": "{}""#, jpath(&root.join("ramo")))));
        assert!(sotto.contains(
            r#""totals": {"bytes": 200, "files": 2, "dirs": 0, "warnings": 1, "elapsed_ms": 1.0},"#
        ));
        assert!(sotto.contains(&format!(
            r#""path": "{}", "message": "metadata fallito""#,
            jpath(&root.join("ramo").join("a.bin"))
        )));
        assert!(!sotto.contains("voce illeggibile"));
        assert!(!sotto.contains("apertura negata"));
        assert!(sotto.contains(r#""filter": null"#));
        // la foto intera li rendiconta tutti e tre, radice compresa
        let tutto = full(&root, &tree, None, usize::MAX, "0.0.0");
        assert!(tutto.contains(r#""warnings": 3,"#));
        assert!(tutto.contains("voce illeggibile"));
        assert!(tutto.contains("apertura negata"));
    }

    /// L'albero della vista: la radice con una foglia sua, un ramo con
    /// sotto-ramo e un ramo fratello senza match per *.bin — per provare
    /// che la foto segue l'apertura dell'utente e la lente.
    fn albero_vista() -> Tree {
        let nodes = vec![
            nodo("", u32::MAX, 400, 4, 3, &[("radice.bin", 20)]), // 0
            nodo("ramo", 0, 200, 2, 1, &[("a.bin", 120)]),        // 1
            nodo("soto", 1, 80, 1, 0, &[("b.bin", 80)]),          // 2
            nodo("altro", 0, 180, 1, 0, &[("c.txt", 180)]),       // 3
        ];
        Tree::from_parts(
            PathBuf::from("/fx"),
            nodes,
            ScanStats {
                ms: 1.0,
                bytes: 400,
                files: 4,
                dirs: 3,
                errors: 0,
            },
            WarningLog::default(),
        )
    }

    #[test]
    fn la_foto_della_vista_segue_i_rami_aperti() {
        let tree = albero_vista();
        // niente aperto: la radice si vede (la lista la mostra) con le
        // sue foglie, i figli restano righe di totali — "children":
        // null, che non e' una cartella vuota: e' un ramo chiuso
        let nessuno = HashSet::new();
        let foto = subtree(&tree, 0, "0.0.0", &nessuno, None, usize::MAX);
        assert!(foto.contains(r#""children": null"#));
        assert!(foto.contains("radice.bin"));
        assert!(!foto.contains(r#""name": "soto""#)); // dentro ramo, chiuso
        assert!(!foto.contains("b.bin"));
        // l'utente apre ramo (x): il ramo si apre nella foto e soto
        // compare — chiuso a sua volta, coi suoi totali
        let mut aperti = HashSet::new();
        aperti.insert(1);
        let foto = subtree(&tree, 0, "0.0.0", &aperti, None, usize::MAX);
        assert!(foto.contains(r#""name": "soto""#));
        assert!(!foto.contains("b.bin")); // soto e' ancora chiuso
        // apre anche soto: la foglia in fondo entra nella foto
        aperti.insert(2);
        let foto = subtree(&tree, 0, "0.0.0", &aperti, None, usize::MAX);
        assert!(foto.contains("b.bin"));
    }

    #[test]
    fn la_foto_della_vista_passa_dalla_lente() {
        let tree = albero_vista();
        let f = Filter::build("*.bin").unwrap();
        let mut aperti = HashSet::new();
        aperti.insert(1);
        aperti.insert(2);
        let foto = subtree(&tree, 0, "0.0.0", &aperti, Some(&f), usize::MAX);
        // la lente si dichiara in testa, come -f nella foto completa
        assert!(foto.contains(r#""filter": "*.bin""#));
        // le foglie non matching non si vedono, i rami senza match
        // nemmeno (altro ha solo c.txt)
        assert!(foto.contains("b.bin"));
        assert!(!foto.contains("c.txt"));
        assert!(!foto.contains(r#""name": "altro""#));
        // i totali passano dalla lente: 20 + 120 + 80, tre file; le
        // cartelle restano strutturali (3), come i contatori di -f
        assert!(foto.contains(
            r#""totals": {"bytes": 220, "files": 3, "dirs": 3, "warnings": 0, "elapsed_ms": 1.0},"#
        ));
    }

    #[test]
    fn la_foto_della_griglia_dichiara_la_sua_profondita() {
        // la fotografia della griglia della CLI (`-d N -json`) apre i rami
        // come la tabella e dichiara in testa la profondita' di
        // aggregazione; la vista del TUI, che scansiona sempre tutto,
        // continua a dichiarare il tetto
        let tree = albero_vista();
        let mut radice = HashSet::new();
        radice.insert(0);
        let foto = subtree(&tree, 0, "0.0.0", &radice, None, 1);
        assert!(foto.contains(r#""depth": 1"#));
        // il primo livello resta riga di totali, come la griglia -d 1
        assert!(foto.contains(r#""children": null"#));
        assert!(foto.contains(r#""name": "ramo""#));
        let vista = view(&tree, 0, None, "0.0.0", &radice, None);
        assert!(vista.contains(r#""depth": 4294967295"#));
    }

    #[test]
    fn full_con_albero_vuoto_fotografa_i_soli_totali() {
        // -d 0: nessun nodo, nemmeno la radice — la foto dei soli totali
        // ("tree": null non e' una cartella vuota: e' una foto senza
        // righe). Prima di questo era un panic: l'indice della radice su
        // un albero di zero nodi
        let tree = Tree::from_parts(
            PathBuf::from("/fx"),
            Vec::new(),
            ScanStats {
                ms: 1.0,
                bytes: 100,
                files: 2,
                dirs: 0,
                errors: 0,
            },
            WarningLog::default(),
        );
        let out = full(&Path::new("/fx"), &tree, None, 0, "0.0.0");
        assert!(out.contains(r#""depth": 0"#));
        assert!(out.contains(
            r#""totals": {"bytes": 100, "files": 2, "dirs": 0, "warnings": 0, "elapsed_ms": 1.0},"#
        ));
        assert!(out.contains(r#""tree": null"#));
    }
}
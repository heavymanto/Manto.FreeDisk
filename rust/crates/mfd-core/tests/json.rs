//! Le prove del formato della fotografia JSON: il contratto di `--json`
//! della CLI resta byte per byte attraverso lo spostamento del
//! serializzatore in mfd-core, e la stampa del TUI (la foto del
//! sottoalbero) parla lo stesso linguaggio.

use mfd_core::{json, scan, Filter, ScanOptions, Tree};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// La fixture classica di regressione (la stessa dei test di scan): 7 file,
/// 5 cartelle, 3530 byte — alpha con lib.dll 1000 e data.dat 2000, beta con
/// note.txt e sub\\altro.log, gamma con z.tmp e x\\y.dll.
fn fixture(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("mfd-json-test-{}-{}", std::process::id(), tag));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("alpha")).unwrap();
    fs::create_dir_all(dir.join("beta/sub")).unwrap();
    fs::create_dir_all(dir.join("gamma/x")).unwrap();
    fs::write(dir.join("readme.txt"), vec![0u8; 100]).unwrap();
    fs::write(dir.join("alpha/lib.dll"), vec![0u8; 1000]).unwrap();
    fs::write(dir.join("alpha/data.dat"), vec![0u8; 2000]).unwrap();
    fs::write(dir.join("beta/note.txt"), vec![0u8; 50]).unwrap();
    fs::write(dir.join("beta/sub/altro.log"), vec![0u8; 70]).unwrap();
    fs::write(dir.join("gamma/x/y.dll"), vec![0u8; 300]).unwrap();
    fs::write(dir.join("gamma/z.tmp"), vec![0u8; 10]).unwrap();
    dir
}

fn child_named(tree: &Tree, id: u32, name: &str) -> u32 {
    *tree
        .children(id)
        .iter()
        .find(|&&i| tree.name(i).to_string_lossy() == name)
        .unwrap_or_else(|| panic!("figlio '{}' non trovato", name))
}

/// Un path come appare nel JSON: i backslash raddoppiati dall'escape (la
/// fixture e' ASCII, quindi e' l'unico caso).
fn jpath(p: &Path) -> String {
    p.display().to_string().replace('\\', "\\\\")
}

#[test]
fn il_json_pieno_resta_byte_per_byte_quello_della_cli() {
    let fx = fixture("pieno");
    let tree = scan(&fx, &ScanOptions { depth: usize::MAX, filter: None });
    let out = json::full(&fx, &tree, None, usize::MAX, "9.9.9");
    // l'unica riga che cambia a ogni scansione e' elapsed_ms (FAQ 9): si
    // scrive nell'attesa quello che lo scan ha misurato
    let ms = format!("{:.1}", tree.stats().ms);
    let lines: Vec<String> = vec![
        r#"{"#.into(),
        r#"  "mfd": "9.9.9","#.into(),
        format!(r#"  "root": "{}","#, jpath(&fx)),
        r#"  "filter": null,"#.into(),
        r#"  "depth": 4294967295,"#.into(),
        format!(
            r#"  "totals": {{"bytes": 3530, "files": 7, "dirs": 5, "warnings": 0, "elapsed_ms": {}}},"#,
            ms
        ),
        r#"  "warnings": [],"#.into(),
        r#"  "tree": {"#.into(),
        r#"    "bytes": 3530,"#.into(),
        r#"    "files": 7,"#.into(),
        r#"    "dirs": 5,"#.into(),
        r#"    "children": ["#.into(),
        r#"      {"#.into(),
        r#"        "name": "alpha","#.into(),
        r#"        "bytes": 3000,"#.into(),
        r#"        "files": 2,"#.into(),
        r#"        "dirs": 0,"#.into(),
        r#"        "children": [],"#.into(),
        r#"        "leaves": ["#.into(),
        r#"          {"name": "data.dat", "bytes": 2000},"#.into(),
        r#"          {"name": "lib.dll", "bytes": 1000}"#.into(),
        r#"        ]"#.into(),
        r#"      },"#.into(),
        r#"      {"#.into(),
        r#"        "name": "gamma","#.into(),
        r#"        "bytes": 310,"#.into(),
        r#"        "files": 2,"#.into(),
        r#"        "dirs": 1,"#.into(),
        r#"        "children": ["#.into(),
        r#"          {"#.into(),
        r#"            "name": "x","#.into(),
        r#"            "bytes": 300,"#.into(),
        r#"            "files": 1,"#.into(),
        r#"            "dirs": 0,"#.into(),
        r#"            "children": [],"#.into(),
        r#"            "leaves": ["#.into(),
        r#"              {"name": "y.dll", "bytes": 300}"#.into(),
        r#"            ]"#.into(),
        r#"          }"#.into(),
        r#"        ],"#.into(),
        r#"        "leaves": ["#.into(),
        r#"          {"name": "z.tmp", "bytes": 10}"#.into(),
        r#"        ]"#.into(),
        r#"      },"#.into(),
        r#"      {"#.into(),
        r#"        "name": "beta","#.into(),
        r#"        "bytes": 120,"#.into(),
        r#"        "files": 2,"#.into(),
        r#"        "dirs": 1,"#.into(),
        r#"        "children": ["#.into(),
        r#"          {"#.into(),
        r#"            "name": "sub","#.into(),
        r#"            "bytes": 70,"#.into(),
        r#"            "files": 1,"#.into(),
        r#"            "dirs": 0,"#.into(),
        r#"            "children": [],"#.into(),
        r#"            "leaves": ["#.into(),
        r#"              {"name": "altro.log", "bytes": 70}"#.into(),
        r#"            ]"#.into(),
        r#"          }"#.into(),
        r#"        ],"#.into(),
        r#"        "leaves": ["#.into(),
        r#"          {"name": "note.txt", "bytes": 50}"#.into(),
        r#"        ]"#.into(),
        r#"      }"#.into(),
        r#"    ],"#.into(),
        r#"    "leaves": ["#.into(),
        r#"      {"name": "readme.txt", "bytes": 100}"#.into(),
        r#"    ]"#.into(),
        r#"  }"#.into(),
        r#"}"#.into(),
    ];
    assert_eq!(out, lines.join("\n") + "\n");
    let _ = fs::remove_dir_all(&fx);
}

#[test]
fn il_json_della_stampa_ha_la_cartella_corrente_in_testa() {
    // la foto del sottoalbero, come la scatta p nel TUI: tre file sparsi su
    // due rami, la foto parte da "grandi" e il ramo fratello non esiste
    let dir = std::env::temp_dir().join(format!("mfd-json-test-{}-sotto", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("grandi")).unwrap();
    fs::create_dir_all(dir.join("piccoli")).unwrap();
    fs::write(dir.join("grandi/uno.bin"), vec![0u8; 1000]).unwrap();
    fs::write(dir.join("grandi/due.bin"), vec![0u8; 500]).unwrap();
    fs::write(dir.join("piccoli/tre.bin"), vec![0u8; 200]).unwrap();
    let tree = scan(&dir, &ScanOptions { depth: usize::MAX, filter: None });
    let grandi = child_named(&tree, tree.root_id(), "grandi");
    let out = json::subtree(&tree, grandi, "9.9.9", &HashSet::new(), None, usize::MAX);
    let ms = format!("{:.1}", tree.stats().ms);
    let lines: Vec<String> = vec![
        r#"{"#.into(),
        r#"  "mfd": "9.9.9","#.into(),
        format!(r#"  "root": "{}","#, jpath(&dir.join("grandi"))),
        r#"  "filter": null,"#.into(),
        r#"  "depth": 4294967295,"#.into(),
        format!(
            r#"  "totals": {{"bytes": 1500, "files": 2, "dirs": 0, "warnings": 0, "elapsed_ms": {}}},"#,
            ms
        ),
        r#"  "warnings": [],"#.into(),
        r#"  "tree": {"#.into(),
        r#"    "bytes": 1500,"#.into(),
        r#"    "files": 2,"#.into(),
        r#"    "dirs": 0,"#.into(),
        r#"    "children": [],"#.into(),
        r#"    "leaves": ["#.into(),
        r#"      {"name": "uno.bin", "bytes": 1000},"#.into(),
        r#"      {"name": "due.bin", "bytes": 500}"#.into(),
        r#"    ]"#.into(),
        r#"  }"#.into(),
        r#"}"#.into(),
    ];
    assert_eq!(out, lines.join("\n") + "\n");
    assert!(!out.contains("tre.bin")); // il ramo fratello non esiste nella foto
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn il_filtro_e_la_profondita_si_dichiarano_in_testa() {
    let fx = fixture("testa");
    let tree = scan(&fx, &ScanOptions { depth: usize::MAX, filter: None });
    let out = json::full(&fx, &tree, Some("*.dll"), 2, "9.9.9");
    assert!(out.contains("\n  \"filter\": \"*.dll\",\n"));
    assert!(out.contains("\n  \"depth\": 2,\n"));
    let _ = fs::remove_dir_all(&fx);
}

#[test]
fn il_json_della_stampa_inquadra_la_vista_del_tui() {
    // la foto del TUI segue l'albero che l'utente ha aperto: un ramo
    // chiuso resta una riga di totali ("children": null), aperto si
    // apre; la lente filtra le foglie e si dichiara in testa, come -f
    // nella foto completa. Il tag e' "vistatui": "vista" e' gia' il tag
    // di fixture("vista") (il test del dettaglio di destra), e due test
    // paralleli sulla stessa cartella si cancellano i file a vicenda —
    // race emersa nei run dell'intero workspace
    let dir = std::env::temp_dir().join(format!("mfd-json-test-{}-vistatui", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("grandi/soto")).unwrap();
    fs::create_dir_all(dir.join("piccoli")).unwrap();
    fs::write(dir.join("grandi/uno.bin"), vec![0u8; 1000]).unwrap();
    fs::write(dir.join("grandi/soto/due.bin"), vec![0u8; 500]).unwrap();
    fs::write(dir.join("piccoli/tre.bin"), vec![0u8; 200]).unwrap();
    let tree = scan(&dir, &ScanOptions { depth: usize::MAX, filter: None });
    let grandi = child_named(&tree, tree.root_id(), "grandi");
    let soto = child_named(&tree, grandi, "soto");
    // tutto chiuso: soto e' una riga di totali, due.bin non esiste
    let vuoto = HashSet::new();
    let foto = json::subtree(&tree, grandi, "9.9.9", &vuoto, None, usize::MAX);
    assert!(foto.contains(r#""children": null"#));
    assert!(!foto.contains("due.bin"));
    // soto aperto: il dentro entra nella foto
    let mut aperti = HashSet::new();
    aperti.insert(soto);
    let foto = json::subtree(&tree, grandi, "9.9.9", &aperti, None, usize::MAX);
    assert!(foto.contains("due.bin"));
    // la lente si dichiara in testa e tiene le foglie matching
    let f = Filter::build("*.bin").unwrap();
    let foto = json::subtree(&tree, grandi, "9.9.9", &aperti, Some(&f), usize::MAX);
    assert!(foto.contains(r#""filter": "*.bin""#));
    assert!(foto.contains("due.bin"));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn i_nomi_non_ascii_escono_come_escape_unicode() {
    let dir = std::env::temp_dir().join(format!("mfd-json-test-{}-escape", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("caff\u{e8}.bin"), vec![0u8; 10]).unwrap();
    let tree = scan(&dir, &ScanOptions { depth: usize::MAX, filter: None });
    let out = json::full(&dir, &tree, None, usize::MAX, "9.9.9");
    // la e' accentata esce come \u00e8: il JSON e' ASCII puro, valido in
    // ogni codepage (contratto 3)
    assert!(out.contains(r#"{"name": "caff\u00e8.bin", "bytes": 10}"#));
    assert!(out.is_ascii());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn la_foto_della_vista_porta_anche_il_dettaglio_di_destra() {
    // la stampa p / F11 del TUI fotografa ENTRAMBI i pannelli: alla foto
    // dell'albero (subtree) si attacca la sezione detail — la riga
    // selezionata della lista e i suoi figli, che l'albero puo' tenere
    // chiusi. Come la scatta il TUI sulla fixture classica: selezione su
    // alpha (il ramo piu' grande, chiuso a sinistra)
    let fx = fixture("vista");
    let tree = scan(&fx, &ScanOptions { depth: usize::MAX, filter: None });
    let alpha = child_named(&tree, tree.root_id(), "alpha");
    let vuoto = HashSet::new();
    let out = json::view(&tree, tree.root_id(), Some(alpha), "9.9.9", &vuoto, None);
    // il dettaglio dichiara la riga selezionata e i suoi figli in ordine
    // di taglia: quello che il pannello di destra mostra anche col ramo
    // chiuso nell'albero
    assert!(out.contains(r#""detail": {"#));
    assert!(out.contains(&format!(r#""selected": "{}""#, jpath(&fx.join("alpha")))));
    assert!(out.contains(r#"{"name": "data.dat", "kind": "file", "bytes": 2000}"#));
    assert!(out.contains(r#"{"name": "lib.dll", "kind": "file", "bytes": 1000}"#));
    // senza selezione (lista vuota) la sezione resta, null: la foto dice
    // anche cosa NON si sta guardando
    let out = json::view(&tree, tree.root_id(), None, "9.9.9", &vuoto, None);
    assert!(out.contains(r#""detail": null"#));
    // un dettaglio con figli-cartella li porta col kind e i totali
    // ricorsivi, in riga con l'anteprista (cartelle e file insieme)
    let gamma = child_named(&tree, tree.root_id(), "gamma");
    let out = json::view(&tree, tree.root_id(), Some(gamma), "9.9.9", &vuoto, None);
    assert!(out.contains(r#"{"name": "x", "kind": "dir", "bytes": 300, "files": 1, "dirs": 0}"#));
    assert!(out.contains(r#"{"name": "z.tmp", "kind": "file", "bytes": 10}"#));
    // la lente vale anche nel dettaglio: le foglie non matching non
    // compaiono, come nell'anteprima
    let f = Filter::build("*.dll").unwrap();
    let out = json::view(&tree, tree.root_id(), Some(alpha), "9.9.9", &vuoto, Some(&f));
    assert!(out.contains(r#"{"name": "lib.dll", "kind": "file", "bytes": 1000}"#));
    assert!(!out.contains("data.dat"));
    let _ = fs::remove_dir_all(&fx);
}

#[test]
fn la_foto_del_report_riporta_i_big_con_il_rank() {
    // la stampa p / F11 dalla finestra r / F9 del TUI: il taglio chiesto
    // in finestra, i file in ordine col ranking davanti e i path
    // assoluti — la foto si legge anche da sola, senza la lista accanto
    let fx = fixture("bigfoto");
    let tree = scan(&fx, &ScanOptions { depth: usize::MAX, filter: None });
    let out = json::report(&tree, tree.root_id(), 3, "9.9.9", None);
    let ms = format!("{:.1}", tree.stats().ms);
    // la testa e' quella delle altre foto: la cartella fotografata, i
    // totali del ramo, la lente dichiarata
    assert!(out.contains(&format!(r#""root": "{}""#, jpath(&fx))));
    assert!(out.contains(&format!(
        r#""totals": {{"bytes": 3530, "files": 7, "dirs": 5, "warnings": 0, "elapsed_ms": {}}},"#,
        ms
    )));
    // il taglio chiesto e la risposta in ordine, dal rank 1
    assert!(out.contains(r#""report": {"#));
    assert!(out.contains(r#""top": 3,"#));
    assert!(out.contains(&format!(
        r#"{{"rank": 1, "bytes": 2000, "path": "{}"}}"#,
        jpath(&fx.join("alpha").join("data.dat"))
    )));
    assert!(out.contains(&format!(
        r#"{{"rank": 3, "bytes": 300, "path": "{}"}}"#,
        jpath(&fx.join("gamma").join("x").join("y.dll"))
    )));
    // oltre il taglio non si va: il quarto big non esiste nella foto
    assert!(!out.contains("readme.txt"));
    // e nemmeno l'albero: la foto e' del report
    assert!(!out.contains(r#""tree":"#));
    // con la lente i big sono solo i matching e il filtro si dichiara
    let f = Filter::build("*.dll").unwrap();
    let out = json::report(&tree, tree.root_id(), 5, "9.9.9", Some(&f));
    assert!(out.contains(r#""filter": "*.dll""#));
    assert!(out.contains(r#"{"rank": 1, "bytes": 1000, "path": ""#)); // lib.dll
    assert!(out.contains(r#"{"rank": 2, "bytes": 300, "path": ""#)); // y.dll
    assert!(!out.contains("data.dat"));
    let _ = fs::remove_dir_all(&fx);
}
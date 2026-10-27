//! Prove d'integrazione del motore su una fixture reale creata al volo in
//! %TEMP%: totali, filtri, profondita', struttura e viste dell'albero.

use mfd_core::{scan, Filter, Order, Row, ScanOptions, Tree};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

/// La fixture classica di regressione: 7 file, 5 cartelle, 3530 byte in tutto
/// (alpha con lib.dll 1000 e data.dat 2000; beta senza .dll; gamma con y.dll
/// 300 in gamma\x). Ogni test la ricrea con un nome unico (i test girano in
/// parallelo nello stesso processo).
fn fixture(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("mfd-core-test-{}-{}", std::process::id(), tag));
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

#[test]
fn totali_e_struttura() {
    let fx = fixture("totali");
    let tree = scan(&fx, &ScanOptions { depth: usize::MAX, filter: None });
    let s = tree.stats();
    assert_eq!(s.bytes, 3530);
    assert_eq!(s.files, 7);
    assert_eq!(s.dirs, 5); // alpha, beta, beta\sub, gamma, gamma\x (la radice non si conta)
    assert_eq!(s.errors, 0);
    assert!(tree.warnings().is_empty());
    assert_eq!(tree.node_count(), 6); // radice + 5 cartelle
    assert_eq!(tree.children(tree.root_id()).len(), 3);
    assert_eq!(tree.leaves(tree.root_id()).len(), 1); // readme.txt
    let _ = fs::remove_dir_all(&fx);
}

#[test]
fn profondita_zero_solo_totali() {
    let fx = fixture("d0");
    let tree = scan(&fx, &ScanOptions { depth: 0, filter: None });
    assert_eq!(tree.stats().bytes, 3530);
    assert_eq!(tree.stats().files, 7);
    assert_eq!(tree.node_count(), 0);
    let _ = fs::remove_dir_all(&fx);
}

#[test]
fn profondita_uno_aggrega_solo_il_primo_livello() {
    let fx = fixture("d1");
    let tree = scan(&fx, &ScanOptions { depth: 1, filter: None });
    assert_eq!(tree.node_count(), 4); // radice + alpha + beta + gamma
    let alpha = child_named(&tree, tree.root_id(), "alpha");
    let m = tree.metrics(alpha);
    assert_eq!((m.bytes, m.files, m.dirs), (3000, 2, 0));
    // le foglie si registrano per i file a level+1 <= depth: con -d 1 solo i
    // file diretti della radice (come explorer.exe nella griglia di C:\Windows),
    // i file dentro alpha compaiono da -d 2
    assert_eq!(tree.leaves(tree.root_id()).len(), 1); // readme.txt
    assert_eq!(tree.leaves(alpha).len(), 0);
    // con -d 2 alpha ha i suoi due file come foglie
    let tree2 = scan(&fx, &ScanOptions { depth: 2, filter: None });
    let alpha2 = child_named(&tree2, tree2.root_id(), "alpha");
    assert_eq!(tree2.leaves(alpha2).len(), 2); // lib.dll, data.dat
    assert_eq!(tree2.node_count(), 6);
    let _ = fs::remove_dir_all(&fx);
}

#[test]
fn filtro_nei_totali_e_nelle_foglie() {
    let fx = fixture("filtro");
    let f = Arc::new(Filter::build("*.dll").unwrap());
    let tree = scan(&fx, &ScanOptions { depth: usize::MAX, filter: Some(f) });
    let s = tree.stats();
    assert_eq!(s.bytes, 1300); // lib.dll + y.dll
    assert_eq!(s.files, 2);
    assert_eq!(s.dirs, 5); // le cartelle si attraversano comunque tutte
    let _ = fs::remove_dir_all(&fx);
}

#[test]
fn subtree_match_stessa_semantica_del_filtro() {
    // senza filtro in scansione, la lente a posteriori deve dare gli stessi
    // numeri di una scansione filtrata: e' la promessa dell'anteprima TUI
    let fx = fixture("subtree");
    let tree = scan(&fx, &ScanOptions { depth: usize::MAX, filter: None });
    let f = Filter::build("*.dll").unwrap();
    assert_eq!(tree.subtree_match(tree.root_id(), &f), (1300, 2));
    assert_eq!(tree.subtree_match(child_named(&tree, tree.root_id(), "alpha"), &f), (1000, 1));
    assert_eq!(tree.subtree_match(child_named(&tree, tree.root_id(), "beta"), &f), (0, 0));
    assert_eq!(tree.subtree_match(child_named(&tree, tree.root_id(), "gamma"), &f), (300, 1));
    let _ = fs::remove_dir_all(&fx);
}

#[test]
fn ordinamento_e_percorsi() {
    let fx = fixture("ordine");
    let tree = scan(&fx, &ScanOptions { depth: usize::MAX, filter: None });
    let mut rows = tree.rows_of(tree.root_id());
    tree.sort_rows(&mut rows, Order::TD);
    let names: Vec<String> = rows
        .iter()
        .map(|r| match r {
            Row::Dir(i) => tree.name(*i).to_string_lossy().into_owned(),
            Row::Leaf(_, n) => n.to_string_lossy().into_owned(),
        })
        .collect();
    assert_eq!(names, vec!["alpha", "gamma", "beta", "readme.txt"]);
    tree.sort_rows(&mut rows, Order::AC);
    let names: Vec<String> = rows
        .iter()
        .map(|r| match r {
            Row::Dir(i) => tree.name(*i).to_string_lossy().into_owned(),
            Row::Leaf(_, n) => n.to_string_lossy().into_owned(),
        })
        .collect();
    assert_eq!(names, vec!["alpha", "beta", "gamma", "readme.txt"]);
    // path_of ricostruisce il percorso completo
    assert_eq!(tree.path_of(child_named(&tree, tree.root_id(), "alpha")), fx.join("alpha"));
    let sub = child_named(&tree, child_named(&tree, tree.root_id(), "beta"), "sub");
    assert_eq!(tree.path_of(sub), fx.join("beta").join("sub"));
    let _ = fs::remove_dir_all(&fx);
}

#[test]
fn top_files_i_piu_grandi_con_path_intero() {
    let fx = fixture("top");
    let tree = scan(&fx, &ScanOptions { depth: usize::MAX, filter: None });
    let top = tree.top_files(10, None);
    // data.dat 2000, lib.dll 1000, y.dll 300, readme.txt 100, altro.log 70,
    // note.txt 50, z.tmp 10 — taglia decrescente
    let sizes: Vec<i64> = top.iter().map(|(s, _)| *s).collect();
    assert_eq!(sizes, vec![2000, 1000, 300, 100, 70, 50, 10]);
    // il path e' quello vero, radice compresa: la riga su cui agire
    assert_eq!(top[0].1, fx.join("alpha").join("data.dat"));
    assert_eq!(top[2].1, fx.join("gamma").join("x").join("y.dll"));
    // il taglio tiene i primi n; oltre il totale restano tutti; n=0 non buca
    assert_eq!(tree.top_files(2, None).len(), 2);
    assert_eq!(tree.top_files(99, None).len(), 7);
    assert!(tree.top_files(0, None).is_empty());
    let _ = fs::remove_dir_all(&fx);
}

#[test]
fn top_files_a_parita_di_taglia_vince_il_path_minore() {
    // il determinismo chiede un tie-break fisso: a parita' di taglia vince il
    // path minore — due scansioni danno le stesse righe nello stesso ordine
    let dir = std::env::temp_dir().join(format!("mfd-core-test-{}-tie", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("m")).unwrap();
    fs::create_dir_all(dir.join("z")).unwrap();
    fs::write(dir.join("m/eq.bin"), vec![0u8; 40]).unwrap();
    fs::write(dir.join("z/eq.bin"), vec![0u8; 40]).unwrap();
    fs::write(dir.join("z/small"), vec![0u8; 1]).unwrap();
    let tree = scan(&dir, &ScanOptions { depth: usize::MAX, filter: None });
    let top = tree.top_files(3, None);
    assert_eq!(top[0].1, dir.join("m").join("eq.bin"));
    assert_eq!(top[1].1, dir.join("z").join("eq.bin"));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn top_files_con_filtro_a_posteriori() {
    // la lente del TUI: il filtro entra dopo lo scan, senza riscansionare
    let fx = fixture("topfil");
    let tree = scan(&fx, &ScanOptions { depth: usize::MAX, filter: None });
    let f = Filter::build("*.dll").unwrap();
    let top = tree.top_files(10, Some(&f));
    let sizes: Vec<i64> = top.iter().map(|(s, _)| *s).collect();
    assert_eq!(sizes, vec![1000, 300]); // alpha\lib.dll, gamma\x\y.dll
    let _ = fs::remove_dir_all(&fx);
}

#[test]
fn top_files_under_parte_dal_sottoalbero_di_una_cartella() {
    // il report del TUI e' della cartella dove si sta: stessa scansione,
    // radice diversa — i file fuori dal sottoalbero non esistono, e i path
    // restano assoluti (il report si legge anche da solo)
    let dir = std::env::temp_dir().join(format!("mfd-core-test-{}-topunder", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("grandi")).unwrap();
    fs::create_dir_all(dir.join("piccoli")).unwrap();
    fs::write(dir.join("grandi/uno.bin"), vec![0u8; 1000]).unwrap();
    fs::write(dir.join("grandi/due.bin"), vec![0u8; 500]).unwrap();
    fs::write(dir.join("piccoli/tre.bin"), vec![0u8; 200]).unwrap();
    let tree = scan(&dir, &ScanOptions { depth: usize::MAX, filter: None });
    let grandi = child_named(&tree, tree.root_id(), "grandi");
    let top = tree.top_files_under(grandi, 10, None);
    let names: Vec<String> = top
        .iter()
        .map(|(_, p)| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, vec!["uno.bin", "due.bin"]); // tre.bin e' fuori dal sottoalbero
    assert_eq!(top[0].1, dir.join("grandi").join("uno.bin")); // path assoluto
    // la radice intera li vede tutti: top_files e' top_files_under(0)
    assert_eq!(tree.top_files(10, None).len(), 3);
    let _ = fs::remove_dir_all(&dir);
}

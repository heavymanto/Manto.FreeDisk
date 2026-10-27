//! L'albero di ripartizione: la struttura dati che il walker riempie e i
//! frontend leggono. I nodi vivono in un Vec indicizzato per id (il nodo 0 e'
//! la radice); la mappa dei figli si costruisce a fine scan nell'ordine di
//! scoperta — lo stesso ordine della griglia storica.

use crate::filter::Filter;
use crate::scan::ScanStats;
use crate::warning::WarningLog;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

/// Nodo di aggregazione per la ripartizione: una cartella a livello <= depth.
/// bytes/files/dirs sono i totali RICORSIVI del sottoalbero; leaves i file
/// diretti mostrabili (i file non hanno un nodo proprio: compaiono come foglie
/// del nodo della loro cartella). Gli atomici sono il lascito del walker
/// parallelo che li accreditava senza lock: a scan finito sono stabili.
pub(crate) struct AggNode {
    pub(crate) name: OsString,
    pub(crate) parent: u32, // id del nodo padre; il nodo 0 e' la radice (riga al 100% della tabella)
    pub(crate) bytes: std::sync::atomic::AtomicI64,
    pub(crate) files: std::sync::atomic::AtomicI64,
    pub(crate) dirs: std::sync::atomic::AtomicI64,
    pub(crate) leaves: Mutex<Vec<Leaf>>,
}

/// File diretto di una cartella: taglia e nome (la foglia del nodo).
#[derive(Clone)]
pub struct Leaf {
    pub size: i64,
    pub name: OsString,
}

/// Totali ricorsivi di un nodo: i numeri della riga, senza forma.
pub struct NodeMetrics {
    pub bytes: i64,
    pub files: i64,
    pub dirs: i64,
}

/// L'albero di uno scan completato: radice, nodi, mappa dei figli, totali e
/// warning. Tutto cio' che i frontend ci fanno (griglia, JSON, pannelli TUI)
/// e' una vista su questi dati: niente riscansioni.
pub struct Tree {
    root_path: PathBuf,
    nodes: Vec<Arc<AggNode>>,
    children: HashMap<u32, Vec<u32>>,
    stats: ScanStats,
    warnings: WarningLog,
}

impl Tree {
    /// Assembla l'albero a fine scan e costruisce la mappa dei figli
    /// nell'ordine di scoperta dei nodi (ordine stabile = griglia stabile).
    pub(crate) fn from_parts(
        root_path: PathBuf,
        nodes: Vec<Arc<AggNode>>,
        stats: ScanStats,
        warnings: WarningLog,
    ) -> Tree {
        let mut children: HashMap<u32, Vec<u32>> = HashMap::new();
        for (i, n) in nodes.iter().enumerate() {
            children.entry(n.parent).or_default().push(i as u32);
        }
        Tree {
            root_path,
            nodes,
            children,
            stats,
            warnings,
        }
    }

    /// Il nodo radice (id 0, la riga al 100% della griglia). Con depth 0
    /// l'albero e' vuoto: nessun front-end deve chiederlo.
    pub fn root_id(&self) -> u32 {
        0
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// La cartella scansionata, come chiesta dall'utente.
    pub fn root_path(&self) -> &Path {
        &self.root_path
    }

    /// Totali dello scan (il tempo e' informativo, non confrontabile).
    pub fn stats(&self) -> &ScanStats {
        &self.stats
    }

    /// Il blocco note delle segnalazioni I/O.
    pub fn warnings(&self) -> &WarningLog {
        &self.warnings
    }

    /// Totali ricorsivi del nodo `id`.
    pub fn metrics(&self, id: u32) -> NodeMetrics {
        let n = &self.nodes[id as usize];
        NodeMetrics {
            bytes: n.bytes.load(Ordering::Relaxed),
            files: n.files.load(Ordering::Relaxed),
            dirs: n.dirs.load(Ordering::Relaxed),
        }
    }

    /// Nome del nodo (vuoto per la radice: il path sta in root_path).
    pub fn name(&self, id: u32) -> &OsStr {
        &self.nodes[id as usize].name
    }

    /// Id del nodo padre (None per la radice).
    pub fn parent(&self, id: u32) -> Option<u32> {
        let p = self.nodes[id as usize].parent;
        if id == 0 || p == u32::MAX {
            None
        } else {
            Some(p)
        }
    }

    /// Figli-cartella del nodo `id`, in ordine di scoperta.
    pub fn children(&self, id: u32) -> &[u32] {
        self.children.get(&id).map_or(&[], Vec::as_slice)
    }

    /// File diretti del nodo `id` (clonati: le viste li possono tenere).
    pub fn leaves(&self, id: u32) -> Vec<Leaf> {
        self.nodes[id as usize].leaves.lock().unwrap().clone()
    }

    /// Quanti file diretti ha il nodo `id` (senza clonare: conteggi -top).
    pub fn leaves_count(&self, id: u32) -> usize {
        self.nodes[id as usize].leaves.lock().unwrap().len()
    }

    /// Path completo del nodo `id` (radice compresa), per titoli e ricerche
    /// nei warning: il confronto per componenti e' esatto, non per prefisso
    /// di stringa.
    pub fn path_of(&self, id: u32) -> PathBuf {
        let mut names: Vec<&OsString> = Vec::new();
        let mut cur = id;
        while cur != 0 {
            let n = &self.nodes[cur as usize];
            names.push(&n.name);
            if n.parent == u32::MAX {
                break; // difensivo: solo la radice dovrebbe stare qui
            }
            cur = n.parent;
        }
        let mut p = self.root_path.clone();
        for n in names.iter().rev() {
            p.push(n);
        }
        p
    }

    /// (bytes, file) dei SOLI file che matchano `f` nel sottoalbero di `id`:
    /// la stessa semantica di -f della CLI, calcolata a posteriori sull'albero
    /// in memoria — la lente del TUI, senza riscansionare.
    pub fn subtree_match(&self, id: u32, f: &Filter) -> (i64, i64) {
        let n = &self.nodes[id as usize];
        let mut bytes = 0i64;
        let mut files = 0i64;
        for l in n.leaves.lock().unwrap().iter() {
            if f.is_match(&l.name) {
                bytes += l.size;
                files += 1;
            }
        }
        if let Some(ch) = self.children.get(&id) {
            for &c in ch {
                let (b, fl) = self.subtree_match(c, f);
                bytes += b;
                files += fl;
            }
        }
        (bytes, files)
    }

    /// I (taglia, path) degli al massimo `n` file piu' grandi dell'albero
    /// intero: la risposta a "quali file mangiano il disco?" (-big della CLI),
    /// calcolata a posteriori sull'albero in memoria, senza riscansionare.
    /// Ordine fisso e deterministico: taglia decrescente e, a parita', path
    /// crescente — due scansioni della stessa cartella danno le stesse righe
    /// nello stesso ordine (il principio dei diff di --json). Con `filter`
    /// contano solo i file che matchano: la stessa semantica di -f, come la
    /// lente a posteriori di subtree_match.
    pub fn top_files(&self, n: usize, filter: Option<&Filter>) -> Vec<(i64, PathBuf)> {
        self.top_files_under(0, n, filter)
    }

    /// I (taglia, path) degli al massimo `n` file piu' grandi del sottoalbero
    /// di `id`, radice compresa: il report del TUI e' della cartella dove si
    /// sta, non solo della radice dello scan. Semantica, taglio e ordine di
    /// top_files; i path restano ASSOLUTI (la radice dello scan davanti),
    /// perche' il report si legge anche da solo, senza la lista accanto.
    pub fn top_files_under(
        &self,
        id: u32,
        n: usize,
        filter: Option<&Filter>,
    ) -> Vec<(i64, PathBuf)> {
        if n == 0 {
            return Vec::new(); // la CLI lo rifiuta gia'; il core non ha mai buchi
        }
        // un heap limitato a n elementi: il massimo e' sempre il peggiore dei
        // candidati (taglia minima, o a parita' path maggiore) — cio' che il
        // prossimo file piu' grande caccera'. O(file totali) in tempo e O(n)
        // in memoria: anche un milione di file si setaccia senza mai tenere
        // l'elenco intero
        let mut heap: BinaryHeap<(Reverse<i64>, PathBuf)> = BinaryHeap::new();
        self.collect_top_files(id, &self.path_of(id), n, filter, &mut heap);
        let mut out: Vec<(i64, PathBuf)> =
            heap.into_iter().map(|(Reverse(s), p)| (s, p)).collect();
        out.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
        out
    }

    /// La visita di top_files: le foglie di `id` e la ricorsione sui figli,
    /// portandosi dietro il path corrente (non lo si ricostruisce da zero per
    /// ogni file). Il confronto d'ingresso evita perfino di costruire il path
    /// dei file troppo piccoli per entrare fra i primi n: il costo pieno si
    /// paga solo per i candidati veri.
    fn collect_top_files(
        &self,
        id: u32,
        prefix: &Path,
        n: usize,
        filter: Option<&Filter>,
        heap: &mut BinaryHeap<(Reverse<i64>, PathBuf)>,
    ) {
        for l in self.nodes[id as usize].leaves.lock().unwrap().iter() {
            if let Some(f) = filter {
                if !f.is_match(&l.name) {
                    continue; // fuori lente: non e' un candidato
                }
            }
            // pieno e troppo piccolo: si scarta prima ancora di costruire il
            // path (il caso comune su un disco pieno di file piccoli)
            if heap.len() == n {
                if let Some((Reverse(min), _)) = heap.peek() {
                    if l.size < *min {
                        continue;
                    }
                }
            }
            let key = (Reverse(l.size), prefix.join(&l.name));
            if heap.len() < n {
                heap.push(key);
            } else if key < *heap.peek().unwrap() {
                heap.pop(); // il peggiore dei n se ne va, entra il nuovo
                heap.push(key);
            }
        }
        if let Some(ch) = self.children.get(&id) {
            for &c in ch {
                let child = prefix.join(&self.nodes[c as usize].name);
                self.collect_top_files(c, &child, n, filter, heap);
            }
        }
    }
}

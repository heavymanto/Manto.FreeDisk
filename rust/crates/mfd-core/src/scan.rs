//! Il walker parallelo: un'unica coda di cartelle condivisa (`DirQueue`) e un
//! worker per core logico. Stessa logica dei walker Node e C# (il confronto
//! della fase 1): i totali coincidono.
//!
//! - **push/pop con Condvar**: i worker dormono a coda vuota e si svegliano su
//!   notify; il protocollo `pending` (contatore atomico delle cartelle aperte)
//!   chiude lo scan quando torna a zero.
//! - **Niente stat per file**: su Windows `DirEntry::metadata()` legge i dati
//!   che FindFirstFile/FindNextFile hanno gia' riportato durante
//!   l'enumerazione. E' il percorso veloce in userspace, prima di scendere al
//!   motore $MFT.
//! - **Reparse point saltati**: junction e symlink non vengono attraversati
//!   (niente doppi conteggi, niente cicli).
//! - **Aggregazione a crediti**: ogni file accredita dimensione e conteggio
//!   alla catena delle sue cartelle-antenato entro la profondita' richiesta.
//!   Le cartelle oltre depth riusano la catena del padre: zero allocazioni.

use crate::filter::Filter;
use crate::tree::{AggNode, Leaf, Tree};
use crate::warning::{Warning, WarningKind, WarningLog};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

/// Opzioni di scansione.
pub struct ScanOptions {
    /// Profondita' di aggregazione (--depth): 0 = solo totali, nessun nodo;
    /// usize::MAX (il TUI) = albero completo, un nodo per ogni cartella e
    /// ogni file come foglia del proprio nodo.
    pub depth: usize,
    /// Filtro sui nomi dei file (-f): i non-matchanti sono invisibili ai
    /// totali (le cartelle si attraversano comunque tutte).
    pub filter: Option<Arc<Filter>>,
}

/// Totali di uno scan: i numeri, non la forma.
pub struct ScanStats {
    pub ms: f64,
    pub bytes: i64,
    pub files: i64,
    pub dirs: i64,
    pub errors: i64,
}

/// Fotografia live dello scan, per l'avanzamento del TUI: contatori cumulativi
/// e stima di percorso. Il totale delle cartelle non e' noto a priori, quindi
/// la percentuale e' una stima onesta (chiuse su chiuse+ancora-aperte), non
/// un fatto.
pub struct Progress {
    pub files: i64,
    pub dirs: i64,
    pub bytes: i64,
    pub errors: i64,
    /// cartelle completate finora
    pub closed: i64,
    /// cartelle in coda o in lavorazione
    pub open: i64,
}

/// Messaggio utente di un errore I/O: il Display di std::io::Error appende il
/// suffisso tecnico "(os error N)" al testo localizzato (es. "Accesso negato.
/// (os error 5)"); per l'utente finale basta il testo, quindi il suffisso si taglia.
pub(crate) fn io_msg(e: &std::io::Error) -> String {
    let s = e.to_string();
    const TAG: &str = " (os error ";
    if let Some(i) = s.find(TAG) {
        if s[i + TAG.len()..].ends_with(')') {
            return s[..i].to_string();
        }
    }
    s
}

/// Voce della coda di lavoro: path, livello relativo alla radice, id del
/// proprio nodo di aggregazione (se esiste) e catena dei crediti per i file
/// contenuti.
struct QItem {
    path: PathBuf,
    level: u32,
    own_id: Option<u32>,
    credits: Option<Arc<Credit>>,
}

/// Catena immutabile dei nodi da accreditare per i file contenuti in una
/// cartella: la cartella stessa (se a livello <= depth) e i suoi antenati
/// entro depth. Le cartelle oltre depth riusano la catena del padre: zero
/// allocazioni per dir.
struct Credit {
    node: Arc<AggNode>,
    next: Option<Arc<Credit>>,
}

/// Coda di cartelle condivisa: pending = cartelle push-ate e non ancora
/// completate. Quando pending torna a 0 lo scan e' finito (stesso protocollo
/// dei walker Node/C#).
struct DirQueue {
    items: Mutex<Vec<QItem>>,
    cv: Condvar,
    pending: AtomicI64,
    closed: AtomicI64,
    done: AtomicBool,
}

impl DirQueue {
    fn push(&self, p: QItem) {
        self.pending.fetch_add(1, Ordering::SeqCst);
        self.items.lock().unwrap().push(p);
        self.cv.notify_one();
    }
    fn pop(&self) -> Option<QItem> {
        let mut guard = self.items.lock().unwrap();
        loop {
            if let Some(p) = guard.pop() {
                return Some(p);
            }
            if self.done.load(Ordering::SeqCst) {
                return None;
            }
            guard = self.cv.wait(guard).unwrap();
        }
    }
    /// chiamata a fine elaborazione di ogni cartella estratta
    fn finish(&self) {
        self.closed.fetch_add(1, Ordering::Relaxed);
        if self.pending.fetch_sub(1, Ordering::SeqCst) == 1 {
            // done si aggiorna col mutex della coda in mano: un worker fermo tra il
            // controllo di done e la cv.wait vedra' il flag al risveglio invece di
            // dormire per sempre (missed wakeup: notify_all mentre nessuno e' ancora
            // in wait). Costa un lock una sola volta per walk, quando pending va a 0.
            let _guard = self.items.lock().unwrap();
            self.done.store(true, Ordering::SeqCst);
            self.cv.notify_all();
        }
    }
    /// annullamento (ESC del TUI): stessa procedura di chiusura del walk,
    /// senza aspettare che pending scenda a 0 per la via naturale. I worker
    /// svuotano la coda come None ed escono; gli item in lavorazione finiscono
    /// la loro cartella e poi si fermano.
    fn abort(&self) {
        let _guard = self.items.lock().unwrap();
        self.done.store(true, Ordering::SeqCst);
        self.cv.notify_all();
    }
}

/// Controllo dello scan visto dai worker: la callback di progresso del
/// frontend e' anche l'unico punto dove l'esterno puo' chiedere l'annullamento
/// (basta ritornare false).
struct Ctl {
    cb: Box<dyn Fn(Progress) -> bool + Send + Sync>,
    cancelled: AtomicBool,
    /// l'orologio dell'avanzamento: quando e' partita l'ultima fotografia
    /// spedita al frontend. Condiviso fra i worker (Mutex) perche' la
    /// cadenza e' del walker intero, non di chi chiude la cartella che
    /// capita.
    last: Mutex<Instant>,
}

/// Un walk completo (coda + worker + attesa). Ritorna None se la callback di
/// progresso ha chiesto l'annullamento: niente risultati parziali, o tutto o
/// niente. filter = criterio sui nomi dei file: i non-matchanti sono invisibili
/// ai totali (le cartelle si attraversano comunque tutte).
fn run_walk(root: &Path, opts: &ScanOptions, ctl: Option<Arc<Ctl>>) -> Option<Tree> {
    let depth = opts.depth;
    let q = Arc::new(DirQueue {
        items: Mutex::new(Vec::new()),
        cv: Condvar::new(),
        pending: AtomicI64::new(0),
        closed: AtomicI64::new(0),
        done: AtomicBool::new(false),
    });
    // Nodo radice della ripartizione (id 0, prima riga della tabella al 100%):
    // raccoglie i file direttamente nella radice e fa da padre dei nodi di livello 1.
    let nodes: Arc<Mutex<Vec<Arc<AggNode>>>> = Arc::new(Mutex::new(Vec::new()));
    let root_item = if depth > 0 {
        let node = Arc::new(AggNode {
            name: OsString::new(),
            parent: u32::MAX,
            bytes: AtomicI64::new(0),
            files: AtomicI64::new(0),
            dirs: AtomicI64::new(0),
            leaves: Mutex::new(Vec::new()),
        });
        nodes.lock().unwrap().push(Arc::clone(&node));
        QItem {
            path: root.to_path_buf(),
            level: 0,
            own_id: Some(0),
            credits: Some(Arc::new(Credit { node, next: None })),
        }
    } else {
        QItem {
            path: root.to_path_buf(),
            level: 0,
            own_id: None,
            credits: None,
        }
    };
    q.push(root_item);

    let bytes = Arc::new(AtomicI64::new(0));
    let files = Arc::new(AtomicI64::new(0));
    let dirs = Arc::new(AtomicI64::new(0));
    let errors = Arc::new(AtomicI64::new(0));
    let warnings: Arc<Mutex<WarningLog>> = Arc::new(Mutex::new(WarningLog::default()));

    let start = Instant::now();

    let n_workers = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(8);
    let handles: Vec<_> = (0..n_workers)
        .map(|_| {
            let q = Arc::clone(&q);
            let bytes = Arc::clone(&bytes);
            let files = Arc::clone(&files);
            let dirs = Arc::clone(&dirs);
            let errors = Arc::clone(&errors);
            let warnings = Arc::clone(&warnings);
            let nodes = Arc::clone(&nodes);
            let filter = opts.filter.clone();
            let ctl = ctl.clone();
            std::thread::spawn(move || loop {
                let Some(it) = q.pop() else { break };
                let QItem {
                    path: dir,
                    level,
                    own_id,
                    credits,
                } = it;
                match fs::read_dir(&dir) {
                    Ok(rd) => {
                        for ent in rd {
                            let ent = match ent {
                                Ok(e) => e,
                                Err(e) => {
                                    errors.fetch_add(1, Ordering::Relaxed);
                                    warnings.lock().unwrap().push(Warning {
                                        path: dir.clone(),
                                        kind: WarningKind::EnumEntry,
                                        message: io_msg(&e),
                                    });
                                    continue;
                                }
                            };
                            let ft = match ent.file_type() {
                                Ok(ft) => ft,
                                Err(e) => {
                                    errors.fetch_add(1, Ordering::Relaxed);
                                    warnings.lock().unwrap().push(Warning {
                                        path: ent.path(),
                                        kind: WarningKind::EntryType,
                                        message: io_msg(&e),
                                    });
                                    continue;
                                }
                            };
                            if ft.is_symlink() {
                                continue; // junction/symlink: skip (parita' con Node/C#)
                            } else if ft.is_dir() {
                                dirs.fetch_add(1, Ordering::Relaxed);
                                // la sotto-cartella entra nel sottoalbero di tutti i
                                // nodi della catena: +1 sulla colonna CARTELLE di
                                // ciascuno (stesso schema dell'accredito dei file)
                                if let Some(cr) = &credits {
                                    let mut cur = Some(Arc::clone(cr));
                                    while let Some(c) = cur {
                                        c.node.dirs.fetch_add(1, Ordering::Relaxed);
                                        cur = c.next.clone();
                                    }
                                }
                                let child_level = level + 1;
                                if depth > 0 && child_level <= depth as u32 {
                                    // dentro la profondita' richiesta: la sotto-cartella
                                    // ha un nodo proprio e allunga la catena dei crediti
                                    let node = Arc::new(AggNode {
                                        name: ent.file_name(),
                                        parent: own_id.unwrap_or(u32::MAX),
                                        bytes: AtomicI64::new(0),
                                        files: AtomicI64::new(0),
                                        dirs: AtomicI64::new(0),
                                        leaves: Mutex::new(Vec::new()),
                                    });
                                    let id = {
                                        let mut nv = nodes.lock().unwrap();
                                        nv.push(Arc::clone(&node));
                                        nv.len() as u32 - 1
                                    };
                                    q.push(QItem {
                                        path: ent.path(),
                                        level: child_level,
                                        own_id: Some(id),
                                        credits: Some(Arc::new(Credit {
                                            node,
                                            next: credits.clone(),
                                        })),
                                    });
                                } else {
                                    // oltre depth: stessa catena di crediti del padre
                                    q.push(QItem {
                                        path: ent.path(),
                                        level: child_level,
                                        own_id: None,
                                        credits: credits.clone(),
                                    });
                                }
                            } else if ft.is_file() {
                                // filtro -f: il file che non matcha e' invisibile ai
                                // totali (byte, contatori, foglie, errori di metadati)
                                if let Some(f) = filter.as_ref() {
                                    if !f.is_match(&ent.file_name()) {
                                        continue;
                                    }
                                }
                                match ent.metadata() {
                                    // su Windows i metadati arrivano dall'enumerazione: gratis
                                    Ok(md) => {
                                        let sz = md.len() as i64;
                                        bytes.fetch_add(sz, Ordering::Relaxed);
                                        files.fetch_add(1, Ordering::Relaxed);
                                        // ripartizione: accredita il file su tutta la
                                        // catena degli antenati entro depth (massimo
                                        // depth+1 accrediti atomici, nessun lock globale)
                                        if let Some(cr) = &credits {
                                            let mut cur = Some(Arc::clone(cr));
                                            while let Some(c) = cur {
                                                c.node.bytes.fetch_add(sz, Ordering::Relaxed);
                                                c.node.files.fetch_add(1, Ordering::Relaxed);
                                                cur = c.next.clone();
                                            }
                                            // foglia mostrabile: file a level+1 <= depth,
                                            // registrato tra le foglie del nodo della dir
                                            if level < depth as u32 {
                                                cr.node.leaves.lock().unwrap().push(Leaf {
                                                    size: sz,
                                                    name: ent.file_name(),
                                                });
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        errors.fetch_add(1, Ordering::Relaxed);
                                        warnings.lock().unwrap().push(Warning {
                                            path: ent.path(),
                                            kind: WarningKind::Metadata,
                                            message: io_msg(&e),
                                        });
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        errors.fetch_add(1, Ordering::Relaxed);
                        warnings.lock().unwrap().push(Warning {
                            path: dir.clone(),
                            kind: WarningKind::OpenDir,
                            message: io_msg(&e),
                        });
                    }
                }
                q.finish();
                // Avanzamento (solo se il frontend ha dato una callback): una
                // fotografia ogni 100 ms, a tempo e non a cartelle chiuse —
                // il vecchio ogni-128 saltava i multipli per la corsa dei
                // worker (chi chiude trova il contatore gia' oltre) e sui
                // dischi lenti lasciava l'avanzamento congelato per secondi —
                // e comunque quando la coda sta per svuotarsi, per l'ultimo
                // frame onesto. Se la callback ritorna false, annullamento.
                if let Some(c) = &ctl {
                    let closed = q.closed.load(Ordering::Relaxed);
                    let due = {
                        let mut last = c.last.lock().unwrap();
                        let due = last.elapsed() >= Duration::from_millis(100)
                            || q.pending.load(Ordering::Relaxed) == 0;
                        if due {
                            *last = Instant::now();
                        }
                        due
                    };
                    if due {
                        let p = Progress {
                            files: files.load(Ordering::Relaxed),
                            dirs: dirs.load(Ordering::Relaxed),
                            bytes: bytes.load(Ordering::Relaxed),
                            errors: errors.load(Ordering::Relaxed),
                            closed,
                            open: q.pending.load(Ordering::Relaxed),
                        };
                        if !(c.cb)(p) {
                            c.cancelled.store(true, Ordering::SeqCst);
                            q.abort();
                        }
                    }
                }
            })
        })
        .collect();
    for h in handles {
        let _ = h.join();
    }

    let cancelled = ctl.map_or(false, |c| c.cancelled.load(Ordering::SeqCst));
    let ms = start.elapsed().as_secs_f64() * 1000.0;
    let warnings = std::mem::take(&mut *warnings.lock().unwrap());
    let nodes = std::mem::take(&mut *nodes.lock().unwrap());
    if cancelled {
        return None; // ESC: niente risultati parziali
    }
    let stats = ScanStats {
        ms,
        bytes: bytes.load(Ordering::Relaxed),
        files: files.load(Ordering::Relaxed),
        dirs: dirs.load(Ordering::Relaxed),
        errors: errors.load(Ordering::Relaxed),
    };
    Some(Tree::from_parts(root.to_path_buf(), nodes, stats, warnings))
}

/// Scansione senza avanzamento (la CLI: la sua risposta e' l'output finale).
pub fn scan(root: &Path, opts: &ScanOptions) -> Tree {
    run_walk(root, opts, None).expect("senza callback non ci sono annullamenti")
}

/// Scansione con avanzamento (il TUI): `cb` riceve la fotografia live dello
/// scan; se ritorna false chiede l'annullamento e il risultato e' None
/// (niente risultati parziali: o tutto, o niente).
pub fn scan_with_progress<F>(root: &Path, opts: &ScanOptions, cb: F) -> Option<Tree>
where
    F: Fn(Progress) -> bool + Send + Sync + 'static,
{
    let ctl = Ctl {
        cb: Box::new(cb),
        cancelled: AtomicBool::new(false),
        last: Mutex::new(Instant::now()),
    };
    run_walk(root, opts, Some(Arc::new(ctl)))
}


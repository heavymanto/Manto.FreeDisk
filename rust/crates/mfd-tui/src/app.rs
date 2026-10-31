//! Lo stato dell'applicazione e la macchina a stati della tastiera.
//! Tutto quello che qui decide e' pura logica (nessun I/O, nessun terminale):
//! i test la coprono costruendo l'albero con una scansione vera su fixture.
//! Il disegno sta in ui.rs, i colori in theme.rs.

use crate::settings::Settings;
use crate::theme::{Theme, ThemeId};
use mfd_core::lang::{Catalog, Lang};
use mfd_core::{json, Filter, Order, Progress, Row, ScanOptions, Tree};
use ratatui::crossterm::event::{
    KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::layout::Rect;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Le otto combinazioni di -o, nell'ordine del ciclo del tasto s.
pub const ORDERS: [Order; 8] = [
    Order::TD,
    Order::TC,
    Order::AC,
    Order::AD,
    Order::FD,
    Order::FC,
    Order::CD,
    Order::CC,
];

/// La chiave del vocabolario per l'etichetta di un ordinamento (il lessico
/// di -o si traduce; i codici T.D e compagni restano ASCII, contratto).
pub fn ord_key(o: Order) -> &'static str {
    match o {
        Order::TD => "ord.td",
        Order::TC => "ord.tc",
        Order::AD => "ord.ad",
        Order::AC => "ord.ac",
        Order::FD => "ord.fd",
        Order::FC => "ord.fc",
        Order::CD => "ord.cd",
        Order::CC => "ord.cc",
    }
}

/// Lo stato della lente (unica): ordinamento, filtro e barra valgono per
/// lista e dettaglio insieme — S e F, col SHIFT, agiscono su entrambi i
/// pannelli, senza scegliere il bersaglio. Il lessico e' quello di -o e -f
/// della CLI (decisioni 1 e 2 del design approvato).
pub struct ViewState {
    pub order: Order,
    pub filter: Option<Filter>,
    pub show_bar: bool,
    /// la classifica in fondo (F9): il report della CLI, sempre visibile —
    /// o nascosto, quando serve tutto lo schermo per navigare
    pub show_report: bool,
}

impl Default for ViewState {
    fn default() -> Self {
        ViewState {
            order: Order::TD, // come la griglia: la piu' grande in cima
            filter: None,
            show_bar: true,
            show_report: true, // il quadro e' il default: mappa e colpevoli insieme
        }
    }
}

/// La modalita' corrente: scansione, schermata principale, finestre di dialogo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Scanning,
    Main,
    Help,
    Warnings,
    SortMenu,
    ThemeMenu,
    FilterPrompt,
    LangMenu,
}

/// Dove sta il cursore: l'albero (la lista, a sinistra), il dettaglio (la
/// lente, a destra) o la classifica in fondo (il report vivo). TAB gira
/// per i tre posti, albero aperto o chiuso (il cambio pannello di TC), e
/// porta il cursore in LETTURA: chi naviga sta nell'albero (INVIO entra,
/// <- risale), chi legge sta nel dettaglio o nella classifica.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    List,
    Detail,
    Report,
}

/// Il top del report (Shift+R / F9) quando si apre: dieci, come li chiede la
/// domanda "quali sono i 10 big?" — un numero tondo che si legge in un
/// colpo. Non e' un tetto: in finestra le cifre riscrivono il top al
/// volo (2, 25, 100...), + e - lo spostano di dieci.
const REPORT_DEFAULT: usize = 10;

/// Il tetto del top riscritto a cifre: oltre non si chiede — la finestra
/// scorre, e mille righe non sono piu' un report ma un elenco.
const REPORT_MAX: usize = 1000;

/// Le righe della classifica in fondo: il pannello e' fatto di bordi,
/// intestazione, separatore e queste righe — il top della domanda
/// ("quali sono i 10 piu' grandi?") e la finestra che lo contiene
/// misurano uguale. Il cursore di TAB le percorre; lo scroll segue la
/// stessa regola dell'albero, pronto a qualunque lunghezza.
pub const PANE_ROWS: usize = 10;

/// Le lettere dei comandi, in minuscolo: Q esce, S ordina, T il tema, F il
/// filtro, G la barra, W i warning, R il report, P la stampa, L la lingua,
/// X apre/chiude il ramo, A il sottoalbero. Una lettera nuda non comanda:
/// i comandi a lettera vogliono lo SHIFT addosso, i tasti funzione restano
/// diretti (sono fatti per quello). La regola e' nata sul campo: una q col
/// dito vicino chiudeva l'app, con la scansione pagata che non si rivede.
const CMD_LETTERS: [char; 11] = ['q', 's', 't', 'f', 'g', 'w', 'r', 'p', 'l', 'x', 'a'];

/// Una riga della lista di sinistra: il nodo, la taglia da mostrare (quella
/// della lente: filtrata se c'e' un filtro, strutturale altrimenti), quanto
/// e' indentata nell'albero e se ha sotto-cartelle (solo quelle hanno il
/// + / - e rispondono a x).
pub struct ListRow {
    pub id: u32,
    pub size: i64,
    pub depth: usize,
    pub has_subdirs: bool,
    /// la riga radice: la cartella corrente, puntata in cima all'albero —
    /// la maniglia del report (selezionarla e' chiedere "i piu' grossi di
    /// dove sono") e la scala dei totali. Non e' un figlio e non corre con
    /// loro: % e barra le restano vuote, INVIO e x non fanno niente
    pub is_root: bool,
}

/// Una riga dell'anteprima di destra: cartella o file-foglia con la sua taglia
/// (attraverso la lente dell'anteprima).
pub struct PrevRow {
    pub row: Row,
    pub size: i64,
}

/// Quanto dura la memoria del doppio click: mezzo secondo, come lo sentono
/// tutti (crossterm non lo sintetizza — e' un concetto del terminale, non
/// dell'input — quindi il secondo click entro quest'aria, sulla stessa
/// riga, e' un doppio).
const DOUBLE_CLICK: Duration = Duration::from_millis(500);

/// Una zona cliccabile dell'ultimo frame disegnato: il rettangolo, l'indice
/// della prima riga visibile (le tabelle scrollano) e quante righe di
/// schermo occupa una riga di dati (due nel menu dei temi: nome + blurb).
#[derive(Debug, Clone, Copy)]
pub struct HitRows {
    pub rect: Rect,
    pub top: usize,
    pub stride: usize,
}

/// Le zone cliccabili dell'ultimo frame: le scrive ui.rs a ogni draw, le
/// legge il mouse. L'event loop e' draw-then-poll, quindi coincidono sempre
/// con quello che l'utente sta guardando; dove il frame non disegna un
/// pannello la zona resta None e il click non fa nulla.
#[derive(Default, Debug, Clone, Copy)]
pub struct Hits {
    /// il pannello lista, bordi esclusi: click = cursore all'albero
    pub list_pane: Option<Rect>,
    /// le righe della lista, header escluso: click = selezione
    pub list: Option<HitRows>,
    /// il pannello dettaglio, bordi compresi: click = TAB (lettura)
    pub detail_pane: Option<Rect>,
    /// le righe del dettaglio: click = cursore sulla riga puntata
    pub detail: Option<HitRows>,
    /// il pannello della classifica, bordi compresi: click/rotella = TAB
    pub report_pane: Option<Rect>,
    /// le righe della classifica: click = cursore sulla riga puntata
    pub report: Option<HitRows>,
    /// le voci del menu aperto (ordinamento, tema)
    pub menu: Option<HitRows>,
}

/// Un punto sta nel rettangolo? Il click arriva come colonna/riga del
/// terminale: lo stesso sistema di coordinate dei Rect di ratatui.
fn in_rect(r: Rect, x: u16, y: u16) -> bool {
    x >= r.x
        && y >= r.y
        && x < r.x.saturating_add(r.width)
        && y < r.y.saturating_add(r.height)
}

/// La riga di dati sotto il click, se c'e': le righe oltre la fine (il
/// fondo del pannello, l'header della tabella) non sono righe. Ritorna
/// l'indice nella lista che la zona descrive — `top` compreso: la tabella
/// poteva essere scrollata — e divide per `stride` quando una voce occupa
/// piu' righe di schermo (il menu dei temi: nome e blurb).
fn hit_row(z: &HitRows, x: u16, y: u16) -> Option<usize> {
    if !in_rect(z.rect, x, y) {
        return None;
    }
    let dy = (y - z.rect.y) as usize;
    Some(z.top + dy / z.stride.max(1))
}

/// L'offset che tiene la selezione visibile in `visible` righe: quello che
/// ratatui farebbe da solo con uno stato fresco a ogni frame, ma qui
/// posseduto — il click deve fare lo stesso conto del render o sbaglierebbe
/// riga, e il render lo riceve esplicito: quello che si vede e' quello che
/// le zone dichiarano.
pub fn scroll_top(sel: usize, visible: usize) -> usize {
    if visible == 0 {
        0
    } else {
        sel.saturating_add(1).saturating_sub(visible)
    }
}

/// La lingua dei test: l'italiano del crate, caricato dal file vero
/// (../mfd-core/lang/it.json) — i cento e piu' test esistenti asseriscono
/// l'italiano e l'italiano resta. Fuori dai test il TUI nasce inglese di
/// fabbrica: il meccanismo e' qui, esplicito, non un default nascosto
/// (docs/DESIGN-I18N.md, sezione 6).
#[cfg(test)]
fn lingua_di_test() -> Catalog {
    let path = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../mfd-core/lang/it.json"));
    let it = Lang::from_file(path).expect("it.json del crate: il gate di parita' lo tiene completo");
    Catalog::with(it)
}

#[cfg(not(test))]
fn lingua_di_test() -> Catalog {
    Catalog::english()
}

pub struct App {
    pub root: PathBuf,
    pub mode: Mode,
    /// l'albero dell'ultima scansione completata (None solo durante la prima)
    pub tree: Option<Arc<Tree>>,
    /// il nodo elencato nella lista di sinistra (i suoi figli-cartella)
    pub current: u32,
    /// l'indice selezionato nella lista di sinistra
    pub sel: usize,
    /// i rami aperti nell'albero della lista (x / F4): la vista resta
    /// piatta, ma i discendenti dei nodi qui dentro compaiono indentati
    /// sotto il padre. E' uno stato di vista, non di dati: drill-down e
    /// filtro non lo toccano, una riscansione lo azzera (albero nuovo)
    pub expanded: HashSet<u32>,
    /// dove sta il cursore: l'albero naviga, il dettaglio legge (TAB,
    /// albero aperto o chiuso)
    pub focus: Focus,
    /// il cursore del dettaglio quando ha il focus
    pub dsel: usize,
    /// la lente unica: ordinamento, filtro e barra di lista e dettaglio
    pub view: ViewState,
    /// le viste calcolate: si ricalcolano solo quando cambia qualcosa
    /// (ordinare e filtrare sono viste gratis, ma non si pagano due volte)
    pub list: Vec<ListRow>,
    pub preview: Vec<PrevRow>,
    /// il denominatore di % e barre dell'anteprima (taglia della selezione
    /// attraverso la sua lente)
    pub preview_total: i64,
    /// l'input mentre si scrive al prompt del filtro
    pub filter_input: String,
    /// l'errore di compilazione del filtro mentre la finestra e' aperta:
    /// resta in vista finche' l'utente non corregge (la finestra non si chiude)
    pub prompt_error: Option<String>,
    /// messaggio transitorio per la riga di stato (scade da solo)
    pub status: Option<(String, Instant)>,
    /// ultima fotografia di progresso ricevuta (schermata di scansione)
    pub last_progress: Option<Progress>,
    /// quando e' partita la scansione corrente: il disegno ne ricava il
    /// tempo trascorso e il frame dello spinner — l'animazione della
    /// schermata di scansione vive di orologio, non di eventi di avanzamento
    /// (la stima onesta della percentuale satura al 99% per quasi tutta la
    /// scansione: qualcosa deve muoversi comunque)
    pub scan_start: Instant,
    /// annullamento richiesto (ESC durante la scansione)
    pub cancelling: bool,
    /// scroll della finestra dei warning
    pub warn_top: usize,
    /// la classifica in fondo (F9): i 10 piu' grandi del ramo selezionato,
    /// ricorsivi — la stessa risposta del report, viva: si rinnova a ogni
    /// movimento della selezione, refresh la fotografa
    pub pane: Vec<(i64, PathBuf)>,
    /// il cursore della classifica (TAB la raggiunge): la riga del report
    /// vivo che si sta leggendo — le frecce la muovono e la status ne
    /// dice il path per intero (nel pannello si accorcia)
    pub pane_sel: usize,
    /// lo scroll della classifica: quante righe stanno sopra la finestra.
    /// Col top a dieci e dieci righe visibili resta fermo — ma la regola
    /// e' quella dell'albero (il cursore resta visibile), pronta a
    /// qualunque lunghezza il top dovesse prendere
    pub pane_top: usize,
    /// lo scroll orizzontale della classifica: da quale carattere del path
    /// si inizia a leggere. Le frecce SX/DX (col cursore TAB) lo muovono:
    /// un nome lungo troncato coi puntini si legge tutto, scorrrendolo
    pub pane_hscroll: usize,
    /// lo scroll orizzontale del dettaglio: le frecce SX/DX (col cursore
    /// TAB) scorrono i nomi lunghi anche nel pannello di destra
    pub detail_hscroll: usize,
    /// il top della classifica: parte da dieci e le cifre lo riscrivono al
    /// volo, in qualunque momento della navigazione. E' della sessione,
    /// come dove sei: F2 non lo salva
    pub report_n: usize,
    /// true tra una cifra e l'altra: le cifre si accodano (25 = 2, poi 5);
    /// un altro tasto chiude la scrittura e la prossima cifra riparte
    pub report_typing: bool,
    /// selezione corrente nei menu (ordinamento, tema)
    pub menu_sel: usize,
    /// il tema corrente (t / F3): identita' scelta e stili pronti per ui.rs
    pub theme_id: ThemeId,
    pub theme: Theme,
    /// il vocabolario attivo: la lingua scelta (o l'inglese di fabbrica)
    /// col fallback inglese incorporato — ogni stringa del TUI passa da qui
    pub cat: Catalog,
    /// le lingue trovate: l'inglese incorporato piu' quelle scoperte nella
    /// cartella del profilo (il menu l / F12 le elenca)
    pub langs: Vec<Lang>,
    /// i file di lingua scoperti ma non leggibili: si saltano (mai un
    /// crash), la finestra w li dichiara
    pub lang_errors: Vec<String>,
    /// dove F2 scrive le impostazioni (None: ambiente senza profilo utente).
    /// E' un campo perche' i test lo puntano su una cartella temporanea.
    pub settings_path: Option<PathBuf>,
    /// dove p scrive la stampa: la cartella di lancio (None: ambiente senza
    /// cartella corrente). Come settings_path, e' un campo perche' i test
    /// lo puntano su una cartella temporanea.
    pub stampa_dir: Option<PathBuf>,
    /// le zone cliccabili dell'ultimo frame disegnato: le scrive ui.rs, le
    /// legge il mouse (che cos'e' cliccabile lo sa solo il disegno)
    pub hits: Hits,
    /// l'ultimo click sinistro — quando e dove — per riconoscere il doppio
    last_click: Option<(Instant, u16, u16)>,
    // il plumbing della scansione in background
    progress_rx: Option<Receiver<Progress>>,
    result_rx: Option<Receiver<Option<Arc<Tree>>>>,
    cancel_flag: Option<Arc<AtomicBool>>,
    quit: bool,
    cancelled_first: bool,
}

impl App {
    fn base(root: PathBuf) -> App {
        App {
            root,
            mode: Mode::Scanning,
            tree: None,
            current: 0,
            sel: 0,
            expanded: HashSet::new(),
            focus: Focus::List,
            dsel: 0,
            view: ViewState::default(),
            list: Vec::new(),
            preview: Vec::new(),
            preview_total: 0,
            filter_input: String::new(),
            prompt_error: None,
            status: None,
            last_progress: None,
            scan_start: Instant::now(),
            cancelling: false,
            warn_top: 0,
            pane: Vec::new(),
            pane_sel: 0,
            pane_top: 0,
            pane_hscroll: 0,
            detail_hscroll: 0,
            report_n: REPORT_DEFAULT,
            report_typing: false,
            menu_sel: 0,
            theme_id: ThemeId::DEFAULT,
            theme: Theme::for_id(ThemeId::DEFAULT),
            cat: lingua_di_test(),
            langs: Vec::new(),
            lang_errors: Vec::new(),
            settings_path: Settings::default_path(),
            stampa_dir: std::env::current_dir().ok(),
            hits: Hits::default(),
            last_click: None,
            progress_rx: None,
            result_rx: None,
            cancel_flag: None,
            quit: false,
            cancelled_first: false,
        }
    }

    /// App nuova: le lingue (l'inglese incorporato piu' quelle scoperte nel
    /// profilo), le impostazioni salvate con F2 (se ci sono), la lingua
    /// forzata dalla riga di comando (--lang / MFD_LANG, che batte tutto) e
    /// la scansione completa in background.
    pub fn new(root: PathBuf, lang: Option<String>) -> App {
        let mut app = App::base(root);
        let (langs, errors) = mfd_core::lang::registry();
        app.langs = langs;
        app.lang_errors = errors;
        // il profilo puo' sopravanzare anche l'inglese incorporato: quando
        // si parte dal fabbrica, il fabbrica e' quello scoperto
        if app.cat.active.code == "en" {
            app.adopt_lang("en");
        }
        app.load_saved();
        if let Some(code) = lang {
            app.adopt_lang(&code); // main ha gia' avvisato se non c'e'
        }
        app.start_scan();
        app
    }

    /// Adotta una lingua tra quelle trovate, in silenzio (avvio, F2):
    /// None se il codice non c'e' — chi chiama decide se dirlo.
    pub fn adopt_lang(&mut self, code: &str) -> Option<()> {
        let l = self.langs.iter().find(|l| l.code == code)?.clone();
        self.cat = Catalog::with(l);
        Some(())
    }

    /// l / F12: la lingua scelta si applica alla sessione e il flash lo
    /// dice; a salvarla per i prossimi avvii ci pensa F2 (la riga lingua=
    /// del profilo), come per il tema.
    fn set_lang(&mut self, code: &str) {
        if let Some(l) = self.langs.iter().find(|l| l.code == code).cloned() {
            let nome = l.name.clone();
            self.cat = Catalog::with(l);
            self.flash_tf("flash.lingua", &[&nome]);
        }
    }

    /// Una stringa del vocabolario attivo: la lingua, poi l'inglese
    /// incorporato, poi la chiave stessa (un gap si vede subito).
    fn t<'a>(&'a self, key: &'a str) -> &'a str {
        self.cat.t(key)
    }

    /// La stringa con gli argomenti nei segnaposto {} (in ordine).
    fn tf(&self, key: &str, args: &[&str]) -> String {
        self.cat.tf(key, args)
    }

    /// F2: fotografa lo stato corrente (tema, lenti, barra) e lo scrive nel
    /// profilo utente. Il flash dice dove — e se qualcosa va storto, perche'.
    fn save_settings(&mut self) {
        let Some(path) = self.settings_path.clone() else {
            self.flash("salvataggio impossibile: profilo utente non trovato");
            return;
        };
        let s = Settings::of(self);
        match s.save(&path) {
            Ok(()) => {
                let dove = path.display().to_string();
                self.flash_tf("flash.f2", &[&dove]);
            }
            Err(e) => self.flash(&format!("salvataggio fallito: {}", e)),
        }
    }

    /// Il contrario di F2, all'avvio: se c'e' un file di impostazioni lo si
    /// riprende (tema, lenti, barra). Chi non ha mai salvato non nota niente.
    pub fn load_saved(&mut self) {
        let Some(path) = self.settings_path.clone() else {
            return;
        };
        if let Some(s) = Settings::load(&path) {
            s.apply(self);
        }
    }

    /// p / F11: la stampa — la fotografia JSON di quello che si sta
    /// guardando, COME LO SI VEDE (mfd-core::json, il contratto di
    /// --json). Dalla schermata principale sono i DUE pannelli: l'albero
    /// di sinistra (i rami aperti con x / a si aprono anche nella foto,
    /// i chiusi restano righe di totali) e il dettaglio di destra (la
    /// riga selezionata e i suoi figli, che l'albero tiene chiusi: la
    /// foto dice cosa c'e' da entrambe le parti). Con la finestra del
    /// report aperta (r / F9) la foto e' del report: i big col ranking,
    /// quello che la finestra mostra — l'inquadratura segue la modalita'.
    /// La lente vale sempre e si dichiara in testa. I file vivono nella
    /// cartella di lancio, non in quella scansionata (mfd misura, non
    /// muta): mfd-stampa-<nome>.json per la vista, mfd-report-<nome>.json
    /// per il report — il nome dice cosa c'e' dentro; se esiste gia' si
    /// prende il primo libero (niente sovrascritture silenziose: due
    /// fotografie valgono un diff). Il flash dice dove — e se qualcosa
    /// va storto, perche'.
    fn stampa(&mut self) {
        let Some(tree) = self.tree.clone() else { return };
        let Some(dir) = self.stampa_dir.clone() else {
            self.flash_t("flash.stampa_nodir");
            return;
        };
        let selected = self.list.get(self.sel).map(|r| r.id);
        let foto = json::view(
            &tree,
            self.current,
            selected,
            env!("CARGO_PKG_VERSION"),
            &self.expanded,
            self.view.filter.as_ref(),
        );
        // il nome dice cosa c'e' dentro: la cartella fotografata (64
        // caratteri bastano a riconoscerla, i conflitti li risolve il
        // progressivo) e la famiglia del file (stampa o report). La
        // radice dello scan puo' non avere un nome leggibile (mfd-tui
        // lanciato su .): si chiama "radice"
        let base = tree.path_of(self.current);
        let nome = base
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.t("stampa.radice").to_string());
        let nome: String = nome.chars().take(64).collect();
        let famiglia = "mfd-stampa";
        let mut path = dir.join(format!("{}-{}.json", famiglia, nome));
        let mut k = 2;
        while path.exists() {
            path = dir.join(format!("{}-{}-{}.json", famiglia, nome, k));
            k += 1;
        }
        match std::fs::write(&path, &foto) {
            Ok(()) => {
                let dove = path.display().to_string();
                self.flash_tf("flash.stampa_ok", &[&dove]);
            }
            Err(e) => {
                let perche = e.to_string();
                self.flash_tf("flash.stampa_ko", &[&perche]);
            }
        }
    }

    /// Stato principale su un albero gia' pronto: e' il costruttore dei test
    /// (la logica si prova senza terminale).
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn with_tree(root: PathBuf, tree: Arc<Tree>) -> App {
        let mut app = App::base(root);
        app.tree = Some(tree);
        app.mode = Mode::Main;
        app.refresh();
        app
    }

    /// Lancia la scansione in background: albero COMPLETO (depth illimitata —
    /// e' la premessa del TUI: tutto in memoria, le viste sono gratis), senza
    /// filtro in scansione (i filtri sono lenti a posteriori), progresso via
    /// canale. La callback ritorna false quando l'utente preme ESC: il core
    /// annulla e non ci sono risultati parziali (o tutto, o niente).
    fn start_scan(&mut self) {
        let (ptx, prx) = mpsc::channel();
        let (rtx, rrx) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        self.cancel_flag = Some(Arc::clone(&cancel));
        self.progress_rx = Some(prx);
        self.result_rx = Some(rrx);
        self.last_progress = None;
        self.scan_start = Instant::now();
        self.cancelling = false;
        self.mode = Mode::Scanning;
        let root = self.root.clone();
        // il Sender non e' Sync: lo si chiude in un Mutex per condividerlo
        // con la callback (che gira sui thread del walker)
        let mtx = Mutex::new(ptx);
        std::thread::spawn(move || {
            let cb = move |p: Progress| -> bool {
                let _ = mtx.lock().unwrap().send(p);
                !cancel.load(Ordering::SeqCst)
            };
            let opts = ScanOptions {
                depth: usize::MAX,
                filter: None,
            };
            let res = mfd_core::scan_with_progress(&root, &opts, cb);
            let _ = rtx.send(res.map(Arc::new));
        });
    }

    /// Drena il canale di progresso e incassa l'esito della scansione.
    /// Chiamato a ogni giro dell'event loop: e' qui che il TUI "vede" il
    /// futuro senza mai bloccarsi sul walker.
    pub fn pump(&mut self) {
        if let Some(rx) = &self.progress_rx {
            while let Ok(p) = rx.try_recv() {
                self.last_progress = Some(p);
            }
        }
        if let Some(rx) = &self.result_rx {
            if let Ok(outcome) = rx.try_recv() {
                self.progress_rx = None;
                self.result_rx = None;
                self.cancel_flag = None;
                match outcome {
                    Some(tree) => {
                        self.tree = Some(tree);
                        self.current = 0;
                        self.sel = 0;
                        self.dsel = 0;
                        self.expanded.clear(); // albero nuovo: i rami aperti erano del vecchio
                        self.focus = Focus::List;
                        self.mode = Mode::Main;
                        self.refresh();
                    }
                    None => {
                        if self.tree.is_some() {
                            // riscansione annullata: resta l'albero di prima
                            self.mode = Mode::Main;
                            self.flash_t("flash.riscansione_annullata");
                        } else {
                            // prima scansione annullata: si esce puliti
                            self.cancelled_first = true;
                            self.quit = true;
                        }
                    }
                }
            }
        }
    }

    pub fn should_quit(&self) -> bool {
        self.quit
    }

    pub fn cancelled_first(&self) -> bool {
        self.cancelled_first
    }

    /// Un messaggio per la riga di stato (scade da solo dopo 3 secondi).
    fn flash(&mut self, msg: &str) {
        self.status = Some((msg.to_string(), Instant::now()));
    }

    /// Il messaggio di stato, se non ancora scaduto.
    pub fn status_now(&self) -> Option<&str> {
        match &self.status {
            Some((m, t)) if t.elapsed().as_millis() < 3000 => Some(m),
            _ => None,
        }
    }

    /// Un flash dal vocabolario (la scorciatoia di t + flash).
    fn flash_t(&mut self, key: &str) {
        let m = self.t(key).to_string();
        self.flash(&m);
    }

    /// Un flash dal vocabolario, con gli argomenti nei segnaposto {}.
    fn flash_tf(&mut self, key: &str, args: &[&str]) {
        let m = self.tf(key, args);
        self.flash(&m);
    }

    fn cancel_scan(&mut self) {
        if let Some(f) = &self.cancel_flag {
            f.store(true, Ordering::SeqCst);
            self.cancelling = true;
        }
    }

}

impl App {
    /// Ricalcola le due viste sull'albero corrente. La lista e' L'ALBERO dei
    /// figli-cartella del nodo corrente: i rami aperti (x / F4) si vedono
    /// indentati sotto il padre, un livello per volta (a li apre tutti —
    /// la stessa profondita' illimitata di -d a, un ramo per volta); la
    /// lente vale a ogni livello, come nella griglia a profondita' libera
    /// della CLI. L'anteprima mostra i figli della selezione, cartelle e
    /// file-foglia insieme, con la STESSA lente (una sola: lista e dettaglio
    /// cambiano insieme). Con un filtro attivo le cartelle senza match sono
    /// nascoste (stessa regola della griglia -f dalla 0.7.0) e le foglie non
    /// matching non compaiono: la semantica e' quella di -f, la lente e' una
    /// fotografia a posteriori, non una riscansione.
    pub fn refresh(&mut self) {
        let Some(tree) = self.tree.clone() else {
            self.list.clear();
            self.preview.clear();
            self.preview_total = 0;
            return;
        };
        // --- lista di sinistra: l'albero del nodo corrente, coi rami aperti
        let mut list = Vec::new();
        self.collect_list(&tree, self.current, 0, &mut list);
        // --- la riga radice in cima: la cartella corrente, puntata — la
        // maniglia del report (selezionarla e' chiedere "i piu' grossi di
        // dove sono": alla radice dello scan, il globale) e la scala dei
        // totali. Non e' un figlio e non corre con loro: % e barra le
        // restano vuote — la grammatica della TOTALE della CLI, fatta
        // selezionabile
        let root_size = match &self.view.filter {
            Some(f) => tree.subtree_match(self.current, f).0,
            None => tree.metrics(self.current).bytes,
        };
        list.insert(
            0,
            ListRow {
                id: self.current,
                size: root_size,
                depth: 0,
                has_subdirs: false,
                is_root: true,
            },
        );
        if self.sel >= list.len() {
            self.sel = list.len().saturating_sub(1);
        }
        self.list = list;
        // --- anteprima di destra: i figli della selezione, stessa lente
        let sel_id = self.list.get(self.sel).map(|r| r.id);
        let mut preview = Vec::new();
        let mut preview_total = 0i64;
        if let Some(sid) = sel_id {
            preview_total = match &self.view.filter {
                Some(f) => tree.subtree_match(sid, f).0,
                None => tree.metrics(sid).bytes,
            };
            let mut rows = tree.rows_of(sid);
            tree.sort_rows(&mut rows, self.view.order);
            for r in rows {
                match &r {
                    Row::Dir(i) => {
                        let size = match &self.view.filter {
                            Some(f) => {
                                let (b, n_file) = tree.subtree_match(*i, f);
                                if n_file == 0 && b == 0 {
                                    continue; // ramo senza match
                                }
                                b
                            }
                            None => tree.metrics(*i).bytes,
                        };
                        preview.push(PrevRow { row: r.clone(), size });
                    }
                    Row::Leaf(_, name) => {
                        if let Some(f) = &self.view.filter {
                            if !f.is_match(name) {
                                continue; // foglia non matching: invisibile
                            }
                        }
                        preview.push(PrevRow { row: r.clone(), size: leaf_size(&r) });
                    }
                }
            }
        }
        self.preview = preview;
        self.preview_total = preview_total;
        // --- la classifica in fondo: i 10 piu' grandi del ramo selezionato,
        // ricorsivi — il report della CLI, vivo: scorri l'albero e la
        // risposta ti segue. Si calcola solo quando il pannello e' in
        // vista (F9 lo apre e lo chiude): niente lavoro per nulla
        if self.view.show_report {
            let base = sel_id.unwrap_or(self.current);
            self.pane = tree.top_files_under(base, self.report_n, self.view.filter.as_ref());
        } else {
            self.pane.clear();
        }
        // il ramo e' un'altra fotografia: il cursore della classifica
        // riparte dalla riga piu' grande, la finestra dall'alto — e anche
        // lo scroll orizzontale torna a zero (un path nuovo si legge da capo)
        self.pane_sel = 0;
        self.pane_top = 0;
        self.pane_hscroll = 0;
        self.detail_hscroll = 0;
    }

    /// Un livello dell'albero della lista: i figli-cartella di `node` in
    /// ordine di lente, ognuno con la sua riga; se il ramo e' aperto si
    /// scende subito sotto (depth + 1). Il filtro vale a ogni livello: un
    /// ramo senza match non si vede ne' si attraversa, come nella griglia.
    fn collect_list(&self, tree: &Tree, node: u32, depth: usize, out: &mut Vec<ListRow>) {
        let mut rows: Vec<Row> = tree.children(node).iter().map(|&i| Row::Dir(i)).collect();
        tree.sort_rows(&mut rows, self.view.order);
        for r in rows {
            if let Row::Dir(i) = r {
                let size = match &self.view.filter {
                    Some(f) => {
                        let (b, n_file) = tree.subtree_match(i, f);
                        if n_file == 0 && b == 0 {
                            continue; // senza match: nascosta, come la CLI
                        }
                        b
                    }
                    None => tree.metrics(i).bytes,
                };
                let has_subdirs = !tree.children(i).is_empty();
                out.push(ListRow {
                    id: i,
                    size,
                    depth,
                    has_subdirs,
                    is_root: false,
                });
                if self.expanded.contains(&i) {
                    self.collect_list(tree, i, depth + 1, out);
                }
            }
        }
    }

    /// La selezione corrente come id di nodo (None: lista vuota).
    pub fn selected_id(&self) -> Option<u32> {
        self.list.get(self.sel).map(|r| r.id)
    }
}

/// La taglia di una riga-foglia.
fn leaf_size(r: &Row) -> i64 {
    match r {
        Row::Leaf(s, _) => *s,
        Row::Dir(_) => 0,
    }
}

/// Apre ricorsivamente tutte le sotto-cartelle di `id` (il lavoro di a):
/// ogni discendente-cartella finisce nello stato aperto, quindi la vista
/// lo mostra indentato al suo posto. Ricorsione profonda quanto l'albero.
fn open_all(tree: &Tree, id: u32, open: &mut HashSet<u32>, count: &mut usize) {
    for &c in tree.children(id) {
        open.insert(c);
        *count += 1;
        open_all(tree, c, open, count);
    }
}

impl App {
    /// La macchina a stati della tastiera (la tabella dei tasti e' la
    /// sezione 5 di docs/DESIGN-TUI.md).
    pub fn on_key(&mut self, k: KeyEvent) {
        // Ctrl+C ovunque: si esce subito
        if k.code == KeyCode::Char('c') && k.modifiers.contains(KeyModifiers::CONTROL) {
            self.quit = true;
            return;
        }
        // Una lettera nuda non comanda: i comandi a lettera vogliono lo
        // SHIFT (Shift+Q, Shift+P...), i tasti funzione restano diretti.
        // Lo sbaglio lo dice il flash al momento giusto, col maiuscolo
        // che manca. Nel prompt del filtro no: la' le lettere si scrivono
        // (e durante la scansione non si comanda ancora).
        if !matches!(self.mode, Mode::FilterPrompt | Mode::Scanning) {
            if let KeyCode::Char(c) = k.code {
                if c.is_ascii_lowercase() && CMD_LETTERS.contains(&c) {
                    let maiuscola = c.to_ascii_uppercase().to_string();
                    self.flash_tf("flash.maiuscola", &[&maiuscola]);
                    return;
                }
            }
        }

        match self.mode {
            Mode::Scanning => {
                if k.code == KeyCode::Esc {
                    self.cancel_scan();
                }
            }
            Mode::Main => self.on_key_main(k),
            Mode::Help => match k.code {
                KeyCode::Esc | KeyCode::F(1) | KeyCode::Char('?') | KeyCode::Char('Q') | KeyCode::F(10) => {
                    self.mode = Mode::Main;
                }
                _ => {}
            },
            Mode::Warnings => self.on_key_warnings(k),
            Mode::SortMenu => self.on_key_menu(k),
            Mode::ThemeMenu => self.on_key_theme(k),
            Mode::FilterPrompt => self.on_key_prompt(k),
            Mode::LangMenu => self.on_key_lang(k),
        }
    }

    /// La rotella del mouse: un detent = una freccia, giu' o su'. La si
    /// gira dove le frecce funzionano (la lista o il dettaglio, il report
    /// e i warning che scorrono, i menu) e non fa nulla dove non
    /// funzionano: e' lei a vestirsi da freccia, non i tasti a vestirsi
    /// di rotella (per selezionare il testo del terminale c'e' Shift).
    pub fn on_scroll(&mut self, down: bool) {
        let code = if down { KeyCode::Down } else { KeyCode::Up };
        self.on_key(KeyEvent::new(code, KeyModifiers::NONE));
    }

    /// La rotella orizzontale (il tilt della rotella, o le due dita del
    /// touchpad): sinistra e' <-, destra e' ->. Vale la regola della
    /// verticale: e' lei a vestirsi di freccia, non i tasti a vestirsi di
    /// rotella — dall'albero naviga, dal dettaglio non si naviga (si torna
    /// con TAB), nel prompt del filtro muove il cursore del testo. I
    /// bottoni laterali del mouse (Indietro/Avanti, quelli del browser)
    /// non si possono vestire: non attraversano il terminale, crossterm
    /// non li vede.
    pub fn on_scroll_h(&mut self, right: bool) {
        let code = if right { KeyCode::Right } else { KeyCode::Left };
        self.on_key(KeyEvent::new(code, KeyModifiers::NONE));
    }

    /// La rotella e' sulla classifica dell'ultimo frame? (Solo in Main:
    /// sopra una finestra aperta vince la finestra.)
    fn wheel_on_report(&self, x: u16, y: u16) -> bool {
        self.mode == Mode::Main && self.hits.report_pane.is_some_and(|r| in_rect(r, x, y))
    }

    /// Il mouse: la rotella si veste da freccia (on_scroll per il verticale,
    /// on_scroll_h per il tilt orizzontale — sinistra risale, destra entra),
    /// i click da tasti — il click seleziona la riga della lista (o porta il
    /// cursore nel dettaglio, come TAB), il doppio click e' INVIO, il destro
    /// e' x, il click di menu e' la voce. Ogni gesto ha il suo tasto: il
    /// mouse e' una scorciatoia, mai una via d'uscita — il design resta
    /// "tutto da tastiera". Il drag non interessa: per selezionare il testo
    /// del terminale c'e' Shift. E i bottoni laterali (Indietro/Avanti,
    /// quelli del browser) non ci sono: non attraversano il terminale,
    /// crossterm non li vede.
    pub fn on_mouse(&mut self, m: MouseEvent) {
        match m.kind {
            // la rotella sulla classifica muove il suo cursore (e ci porta
            // il focus, come il click): gira dove le frecce funzionano
            MouseEventKind::ScrollDown => {
                if self.wheel_on_report(m.column, m.row) {
                    self.focus = Focus::Report;
                    self.move_pane_sel(1);
                } else {
                    self.on_scroll(true);
                }
            }
            MouseEventKind::ScrollUp => {
                if self.wheel_on_report(m.column, m.row) {
                    self.focus = Focus::Report;
                    self.move_pane_sel(-1);
                } else {
                    self.on_scroll(false);
                }
            }
            // il tilt: <- e -> vestite da rotella
            MouseEventKind::ScrollLeft => self.on_scroll_h(false),
            MouseEventKind::ScrollRight => self.on_scroll_h(true),
            MouseEventKind::Down(MouseButton::Left) => self.on_left_click(m.column, m.row),
            MouseEventKind::Down(MouseButton::Right) => self.on_right_click(m.column, m.row),
            _ => {} // rilascio e drag: restano al terminale
        }
    }

    /// Il click sinistro. Nei menu seleziona la voce e la applica (il click
    /// di una voce e' frecce + INVIO); nei pannelli seleziona la riga della
    /// lista puntata — o porta il cursore in lettura nel dettaglio, come
    /// TAB — e se la riga era gia' selezionata da un click di adesso e' un
    /// doppio click: INVIO, si entra (come ogni file manager che si rispetti).
    fn on_left_click(&mut self, x: u16, y: u16) {
        let hits = self.hits; // Copy: le zone dell'ultimo frame disegnato
        // il menu aperto copre i pannelli: le sue voci hanno la precedenza
        if matches!(self.mode, Mode::SortMenu | Mode::ThemeMenu | Mode::LangMenu) {
            if let Some(voice) = hits.menu.as_ref().and_then(|z| hit_row(z, x, y)) {
                let n = if matches!(self.mode, Mode::ThemeMenu) {
                    ThemeId::ALL.len()
                } else if matches!(self.mode, Mode::LangMenu) {
                    self.langs.len()
                } else {
                    ORDERS.len()
                };
                if voice < n {
                    self.menu_sel = voice;
                    // il click si veste da INVIO: applica cio' che i tasti
                    // applicherebbero
                    self.on_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
                }
            }
            return;
        }
        // le altre finestre non si cliccano: ESC e INVIO restano tasti
        if self.mode != Mode::Main {
            return;
        }
        if let Some(pane) = hits.list_pane {
            if in_rect(pane, x, y) {
                if let Some(idx) = hits.list.as_ref().and_then(|z| hit_row(z, x, y)) {
                    if idx < self.list.len() {
                        // il doppio click: la stessa riga, cliccata adesso
                        // (crossterm non lo sintetizza: la memoria e' nostra)
                        let double = self.last_click.is_some_and(|(t, cx, cy)| {
                            t.elapsed() <= DOUBLE_CLICK && cy == y && cx.abs_diff(x) <= 2
                        });
                        self.sel = idx;
                        self.dsel = 0; // come le frecce: il cursore riparte
                        self.focus = Focus::List;
                        if double {
                            self.last_click = None;
                            self.enter(); // INVIO: si entra
                        } else {
                            self.last_click = Some((Instant::now(), x, y));
                            self.refresh(); // l'anteprima segue la selezione
                        }
                        return;
                    }
                }
                // header o fondo del pannello: solo il cursore all'albero
                self.focus = Focus::List;
                return;
            }
        }
        if let Some(pane) = hits.detail_pane {
            if in_rect(pane, x, y) {
                self.focus = Focus::Detail; // il pannello si legge: come TAB
                if let Some(idx) = hits.detail.as_ref().and_then(|z| hit_row(z, x, y)) {
                    if idx < self.preview.len() {
                        self.dsel = idx; // e il cursore va sulla riga puntata
                    }
                }
                return;
            }
        }
        if let Some(pane) = hits.report_pane {
            if in_rect(pane, x, y) {
                // anche la classifica si legge: click = TAB, la riga puntata
                self.focus = Focus::Report;
                if let Some(idx) = hits.report.as_ref().and_then(|z| hit_row(z, x, y)) {
                    if idx < self.pane.len() {
                        self.pane_sel = idx;
                        self.pane_top = scroll_top(self.pane_sel, PANE_ROWS);
                    }
                }
            }
        }
    }

    /// Il tasto destro sulla riga della lista: e' Shift+X / F4 del ramo puntato —
    /// prima il cursore va sulla riga, poi il ramo si apre o chiude (un
    /// gesto unico, come insegnano i file manager da sempre).
    fn on_right_click(&mut self, x: u16, y: u16) {
        if self.mode != Mode::Main {
            return;
        }
        let hits = self.hits;
        if let Some(idx) = hits.list.as_ref().and_then(|z| hit_row(z, x, y)) {
            if idx < self.list.len() {
                self.sel = idx;
                self.dsel = 0;
                self.focus = Focus::List;
                self.refresh();
                self.toggle_expand(); // x / F4: il ramo si apre o chiude
            }
        }
    }

    fn on_key_main(&mut self, k: KeyEvent) {
        // il cursore della classifica non resta mai senza casa: se il
        // pannello non c'e' (F9, o uno schermo troppo basso), si torna
        // all'albero prima di ascoltare qualunque tasto
        if self.focus == Focus::Report && self.hits.report_pane.is_none() {
            self.focus = Focus::List;
        }
        // le cifre scrivono il top della classifica: ogni altro tasto
        // chiude la scrittura (la prossima cifra riparte da se')
        if !matches!(k.code, KeyCode::Char(c) if c.is_ascii_digit()) {
            self.report_typing = false;
        }
        match k.code {
            // Shift+Q o F10: si esce. La q nuda non esce piu' — le
            // lettere dei comandi vogliono lo SHIFT, i tasti F son diretti
            KeyCode::Char('Q') | KeyCode::F(10) => self.quit = true,
            KeyCode::F(1) | KeyCode::Char('?') => self.mode = Mode::Help,
            // F2: il classico salva — tema, lenti e barra nel profilo utente
            KeyCode::F(2) => self.save_settings(),
            // F5: il classico refresh (il tasto che F2 ha liberato)
            KeyCode::F(5) => self.start_scan(), // riscansiona
            // S (Shift): il ciclo rapido del design (T/A/F/C x D/C, come
            // -o), uguale per lista e dettaglio
            KeyCode::Char('S') => {
                let next = self.view.order.next();
                self.view.order = next;
                let label = self.t(ord_key(next)).to_string();
                self.flash_tf("flash.ord", &[&label]);
                self.refresh();
            }
            // F6: il menu delle otto combinazioni, con i codici di -o
            KeyCode::F(6) => {
                self.menu_sel = ORDERS
                    .iter()
                    .position(|o| *o == self.view.order)
                    .unwrap_or(0);
                self.mode = Mode::SortMenu;
            }
            // T (Shift): il menu dei quattro temi (Notturno, Debian, Console, CLI)
            KeyCode::Char('T') | KeyCode::F(3) => {
                self.menu_sel = self.theme_id as usize;
                self.mode = Mode::ThemeMenu;
            }
            KeyCode::F(7) | KeyCode::Char('F') => {
                // la finestra del filtro: campo, istruzioni ed esempi in vista
                self.filter_input = self
                    .view
                    .filter
                    .as_ref()
                    .map(|f| f.display().to_string())
                    .unwrap_or_default();
                self.prompt_error = None; // si riparte senza errori in vista
                self.mode = Mode::FilterPrompt;
            }
            KeyCode::Char('G') => {
                // la barra e' un marchio: o c'e' o non c'e', in entrambi i pannelli
                let on = !self.view.show_bar;
                self.view.show_bar = on;
                if on {
                    self.flash_t("flash.barra_on");
                } else {
                    self.flash_t("flash.barra_off");
                }
            }
            KeyCode::Char('W') | KeyCode::F(8) => {
                self.warn_top = 0;
                self.mode = Mode::Warnings;
            }
            // F9: la classifica in fondo si mostra e si nasconde — quando
            // serve tutto lo schermo per navigare si chiude, quando serve
            // la risposta intera si apre. La scelta si ricorda (F2); il
            // pannello che appare e sparisce e' gia' tutto il feedback
            KeyCode::F(9) => {
                self.view.show_report = !self.view.show_report;
                // il cursore non resta mai senza casa: la classifica sparsa
                // rimanda all'albero
                if !self.view.show_report && self.focus == Focus::Report {
                    self.focus = Focus::List;
                }
                self.refresh();
            }
            // Shift+P / F11: la stampa — la fotografia JSON della cartella dove
            // si sta, come la si vede (i rami aperti, la lente), su file
            // (il contratto di --json, mfd-core): la reportistica senza
            // il terminale, senza ridigitare la CLI col redirect
            KeyCode::Char('P') | KeyCode::F(11) => self.stampa(),
            // Shift+L / F12: il menu delle lingue trovate — inglese incorporato
            // piu' quelle scoperte nel profilo. F12 era l'ultimo gemello
            // F libero della tastiera
            KeyCode::Char('L') | KeyCode::F(12) => {
                self.menu_sel = self
                    .langs
                    .iter()
                    .position(|l| l.code == self.cat.active.code)
                    .unwrap_or(0);
                self.mode = Mode::LangMenu;
            }
            KeyCode::Up => match self.focus {
                Focus::List => self.move_sel(-1),
                Focus::Detail => self.move_dsel(-1),
                Focus::Report => self.move_pane_sel(-1),
            },
            KeyCode::Down => match self.focus {
                Focus::List => self.move_sel(1),
                Focus::Detail => self.move_dsel(1),
                Focus::Report => self.move_pane_sel(1),
            },
            KeyCode::PageUp => match self.focus {
                Focus::List => self.move_sel(-10),
                Focus::Detail => self.move_dsel(-10),
                Focus::Report => self.move_pane_sel(-(PANE_ROWS as i64)),
            },
            KeyCode::PageDown => match self.focus {
                Focus::List => self.move_sel(10),
                Focus::Detail => self.move_dsel(10),
                Focus::Report => self.move_pane_sel(PANE_ROWS as i64),
            },
            KeyCode::Home => match self.focus {
                Focus::List => {
                    self.sel = 0;
                    self.dsel = 0;
                    self.refresh();
                }
                Focus::Detail => self.dsel = 0,
                Focus::Report => {
                    self.pane_sel = 0;
                    self.pane_top = 0;
                }
            },
            KeyCode::End => match self.focus {
                Focus::List => {
                    self.sel = self.list.len().saturating_sub(1);
                    self.dsel = 0;
                    self.refresh();
                }
                Focus::Detail => self.dsel = self.preview.len().saturating_sub(1),
                Focus::Report => {
                    self.pane_sel = self.pane.len().saturating_sub(1);
                    self.pane_top = scroll_top(self.pane_sel, PANE_ROWS);
                }
            },
            // INVIO: drill-down dall'ALBERO. Col cursore nel dettaglio o
            // nella classifica non fa nulla: si legge, non si naviga
            // (due strade per entrare non si capirebbero)
            KeyCode::Enter => {
                if self.focus == Focus::List {
                    self.enter();
                }
            }
            // <- o Backspace: risalgono dall'albero; dal dettaglio e dalla
            // classifica non si naviga — si LEGGE, e le frecce orizzontali
            // fanno lo scroll del testo: un nome lungo troncato coi
            // puntini si legge tutto, scorrendolo (la classifica; nel
            // dettaglio restano mute per ora)
            KeyCode::Left | KeyCode::Backspace => {
                match self.focus {
                    Focus::List => self.go_parent(),
                    Focus::Report => self.pane_hscroll = self.pane_hscroll.saturating_sub(5),
                    Focus::Detail => self.detail_hscroll = self.detail_hscroll.saturating_sub(5),
                }
            }
            // ->: entra dall'albero; dalla classifica e dal dettaglio scrolla
            KeyCode::Right => {
                match self.focus {
                    Focus::List => self.enter(),
                    Focus::Report => self.pane_hscroll += 5,
                    Focus::Detail => self.detail_hscroll += 5,
                }
            }
            // Shift+X / F4: apre o chiude il ramo sotto il cursore dell'albero
            // (chiudere mette via tutto il ramo, qualunque profondita'
            // avesse raggiunto)
            KeyCode::Char('X') | KeyCode::F(4) => self.toggle_expand(),
            // Shift+A: apre TUTTO il sottoalbero del ramo sotto il
            // cursore — l'equivalente interattivo di -d a, un ramo per
            // volta
            KeyCode::Char('A') => self.expand_subtree(),
            // TAB: il giro dei tre posti — albero, dettaglio, classifica —
            // albero aperto o chiuso (il cambio pannello alla Total
            // Commander: leggere non dipende dall'albero, e il bordo dice
            // sempre dove si sta). Shift+TAB gira al contrario. La
            // classifica entra nel giro solo quando c'e' (F9 la toglie)
            KeyCode::Tab => self.cycle_focus(true),
            KeyCode::BackTab => self.cycle_focus(false),
            // le cifre riscrivono il top della classifica in fondo (25 =
            // due, poi cinque; il tetto e' REPORT_MAX): il mestiere che
            // faceva la finestra del report, prima della riga radice —
            // la classifica si allarga sotto gli occhi, TAB la scorre
            KeyCode::Char(c) if c.is_ascii_digit() => {
                let d = (c as u8 - b'0') as usize;
                let typed = if self.report_typing {
                    self.report_n * 10 + d
                } else {
                    d
                };
                self.report_n = typed.clamp(1, REPORT_MAX);
                self.report_typing = true;
                self.refresh(); // la classifica segue il nuovo taglio
            }
            _ => {}
        }
    }

    fn move_sel(&mut self, delta: i64) {
        if self.list.is_empty() {
            self.sel = 0;
            return;
        }
        let n = self.list.len() as i64;
        let mut s = self.sel as i64 + delta;
        if s < 0 {
            s = 0;
        }
        if s >= n {
            s = n - 1;
        }
        self.sel = s as usize;
        self.dsel = 0; // il cursore del dettaglio riparte col contenuto nuovo
        self.refresh(); // l'anteprima segue la selezione
    }

    /// Il cursore del dettaglio: muove senza ricalcolare (il dettaglio non
    /// cambia perche' il cursore si sposta — cambia quando cambia la lente
    /// o la selezione della lista).
    fn move_dsel(&mut self, delta: i64) {
        if self.preview.is_empty() {
            self.dsel = 0;
            return;
        }
        let n = self.preview.len() as i64;
        let mut s = self.dsel as i64 + delta;
        if s < 0 {
            s = 0;
        }
        if s >= n {
            s = n - 1;
        }
        self.dsel = s as usize;
    }

    /// Il giro di TAB: albero -> dettaglio -> classifica -> albero. La
    /// classifica sta nel giro solo quando il suo pannello e' disegnato
    /// (lo sa l'ultimo frame, cioe' hits): senza, il giro resta in due.
    fn cycle_focus(&mut self, forward: bool) {
        let has_pane = self.hits.report_pane.is_some();
        self.focus = match (self.focus, forward, has_pane) {
            (Focus::List, true, _) => Focus::Detail,
            (Focus::Detail, true, true) => Focus::Report,
            (Focus::Detail, true, false) => Focus::List,
            (Focus::Report, true, _) => Focus::List,
            (Focus::List, false, true) => Focus::Report,
            (Focus::List, false, false) => Focus::Detail,
            (Focus::Detail, false, _) => Focus::List,
            (Focus::Report, false, _) => Focus::Detail,
        };
        let dove = match self.focus {
            Focus::List => "flash.cursore_albero",
            Focus::Detail => "flash.cursore_dettaglio",
            Focus::Report => "flash.cursore_classifica",
        };
        self.flash_t(dove);
    }

    /// Il cursore della classifica: muove senza ricalcolare (il report del
    /// ramo non cambia perche' il cursore si sposta — cambia quando cambia
    /// la selezione) e la finestra lo segue con la stessa regola
    /// dell'albero: il cursore resta visibile.
    fn move_pane_sel(&mut self, delta: i64) {
        if self.pane.is_empty() {
            self.pane_sel = 0;
            self.pane_top = 0;
            return;
        }
        let n = self.pane.len() as i64;
        let mut s = self.pane_sel as i64 + delta;
        if s < 0 {
            s = 0;
        }
        if s >= n {
            s = n - 1;
        }
        self.pane_sel = s as usize;
        self.pane_top = scroll_top(self.pane_sel, PANE_ROWS);
    }

    /// INVIO o -> (albero): drill-down netto nella cartella selezionata
    /// (anche vuota: si vede che e' vuota, con <- si risale). I rami aperti
    /// restano aperti: sono uno stato di vista, e chi risale li ritrova.
    fn enter(&mut self) {
        if let Some(r) = self.list.get(self.sel) {
            if r.is_root {
                // la riga radice e' dove si sta gia': il report lo dice,
                // l'invio non va da nessuna parte
                self.flash_t("flash.gia_qui");
                return;
            }
            self.current = r.id;
            self.sel = 0;
            self.dsel = 0;
            self.refresh();
        }
    }

    /// <- o Backspace: si sale al padre e la selezione torna sul ramo da cui
    /// si e' saliti (se la lente lo mostra ancora).
    fn go_parent(&mut self) {
        let Some(tree) = self.tree.clone() else { return };
        if let Some(p) = tree.parent(self.current) {
            let target = self.current;
            self.current = p;
            self.sel = 0;
            self.dsel = 0;
            self.refresh();
            if let Some(i) = self.list.iter().position(|r| r.id == target) {
                self.sel = i;
            }
        }
    }

    /// Shift+X / F4: apre o chiude il ramo sotto il cursore dell'albero. Aprire
    /// fa comparire i figli-cartella indentati; chiudere mette via tutto
    /// il ramo, qualunque profondita' avesse raggiunto (basta togliere il
    /// padre: la vista si ricostruisce dall'alto).
    fn toggle_expand(&mut self) {
        let Some(r) = self.list.get(self.sel) else { return };
        if r.is_root {
            // i figli della radice sono gia' le righe sotto: niente da
            // aprire, si e' gia' qui
            self.flash_t("flash.gia_qui");
            return;
        }
        if !r.has_subdirs {
            self.flash_t("flash.nessuna_sub");
            return;
        }
        if self.expanded.remove(&r.id) {
            // il cursore in lettura, se sta nel dettaglio, ci resta: TAB
            // riporta sempre all'albero, non resta mai senza casa
            self.flash_t("flash.ramo_chiuso");
        } else {
            self.expanded.insert(r.id);
            self.flash_t("flash.ramo_aperto");
        }
        // il nodo resta al suo posto: i figli vengono dopo di lui
        self.refresh();
    }

    /// Shift+A: apre TUTTO il sottoalbero del ramo sotto il cursore — la stessa
    /// profondita' illimitata di -d a, scelta per un ramo per volta. Il ramo
    /// stesso si apre (altrimenti resterebbe chiuso coi figli pronti), poi
    /// ogni discendente. Con un filtro attivo i rami senza match restano
    /// nascosti (lo stato e' strutturale, la lente decide cosa si vede).
    fn expand_subtree(&mut self) {
        let Some(r) = self.list.get(self.sel) else { return };
        // la riga radice non ha il marchio delle sotto-cartelle (i suoi
        // figli sono gia' le righe sotto) ma il suo sottoalbero si apre
        // come quello di chiunque: A spalanca tutto l'albero della
        // cartella corrente
        if !r.has_subdirs && !r.is_root {
            self.flash_t("flash.nessuna_sub");
            return;
        }
        let Some(tree) = self.tree.clone() else { return };
        self.expanded.insert(r.id);
        let mut n = 1usize; // il ramo sotto il cursore
        open_all(&tree, r.id, &mut self.expanded, &mut n);
        self.refresh();
        self.flash_tf("flash.sottoalbero", &[&n.to_string()]);
    }

    fn on_key_warnings(&mut self, k: KeyEvent) {
        let n = self.tree.as_ref().map(|t| t.warnings().len()).unwrap_or(0);
        match k.code {
            KeyCode::Esc | KeyCode::Char('W') | KeyCode::F(8) => self.mode = Mode::Main,
            KeyCode::Up => self.warn_top = self.warn_top.saturating_sub(1),
            KeyCode::Down => {
                if self.warn_top + 1 < n {
                    self.warn_top += 1;
                }
            }
            KeyCode::PageUp => self.warn_top = self.warn_top.saturating_sub(10),
            KeyCode::PageDown => self.warn_top = (self.warn_top + 10).min(n.saturating_sub(1)),
            _ => {}
        }
    }

    fn on_key_menu(&mut self, k: KeyEvent) {
        match k.code {
            KeyCode::Esc => self.mode = Mode::Main,
            KeyCode::Up => self.menu_sel = self.menu_sel.saturating_sub(1),
            KeyCode::Down => self.menu_sel = (self.menu_sel + 1).min(ORDERS.len() - 1),
            KeyCode::Enter => {
                let o = ORDERS[self.menu_sel];
                self.view.order = o;
                let label = self.t(ord_key(o)).to_string();
                self.flash_tf("flash.ord", &[&label]);
                self.mode = Mode::Main;
                self.refresh();
            }
            _ => {}
        }
    }

    /// Il menu dei temi (t o F3): quattro voci, INVIO applica, ESC annulla.
    /// Con quattro voci il giro e' ciclico, a differenza dell'ordinamento
    /// (otto).
    fn on_key_theme(&mut self, k: KeyEvent) {
        let n = ThemeId::ALL.len();
        match k.code {
            KeyCode::Esc => self.mode = Mode::Main,
            KeyCode::Up => self.menu_sel = (self.menu_sel + n - 1) % n,
            KeyCode::Down => self.menu_sel = (self.menu_sel + 1) % n,
            KeyCode::Enter => {
                let id = ThemeId::ALL[self.menu_sel];
                self.theme_id = id;
                self.theme = Theme::for_id(id);
                let nome = self.t(id.key()).to_string();
                self.flash_tf("flash.tema", &[&nome]);
                self.mode = Mode::Main;
            }
            _ => {}
        }
    }

    /// La finestra del filtro (f o F7): il testo si compone tasto per tasto;
    /// INVIO compila e applica, ESC annulla. Un filtro che non compila non
    /// chiude la finestra: l'errore resta in vista finche' non si corregge.
    fn on_key_prompt(&mut self, k: KeyEvent) {
        match k.code {
            KeyCode::Esc => {
                self.prompt_error = None;
                self.mode = Mode::Main;
            }
            KeyCode::Enter => {
                let spec = self.filter_input.trim().to_string();
                if spec.is_empty() {
                    self.view.filter = None;
                    self.flash_t("flash.filtro_rimosso");
                } else {
                    match Filter::build(&spec) {
                        Ok(f) => {
                            self.view.filter = Some(f);
                            self.flash_tf("flash.fil", &[&spec]);
                        }
                        // non compila: si corregge sul posto, con l'errore
                        // scritto nella finestra stessa (non un flash che
                        // scompare mentre la finestra si chiude)
                        Err(e) => {
                            let msg = self.tf("prompt.filtro_ko", &[&e.to_string()]);
                            self.prompt_error = Some(msg);
                            return;
                        }
                    }
                }
                self.prompt_error = None;
                self.mode = Mode::Main;
                self.sel = 0; // la lista puo' essere cambiata: si riparte da capo
                self.dsel = 0; // e il cursore del dettaglio con lei
                self.refresh();
            }
            KeyCode::Backspace => {
                self.filter_input.pop();
            }
            KeyCode::Char(c) => self.filter_input.push(c),
            _ => {}
        }
    }

    /// Il menu delle lingue (Shift+L o F12): INVIO applica (flash di
    /// conferma, la sessione cambia lingua subito), ESC annulla, Shift+L
    /// e F12 chiudono pure — come Shift+W nei warning. F2 rende la
    /// scelta permanente (la riga lingua= del profilo).
    fn on_key_lang(&mut self, k: KeyEvent) {
        let n = self.langs.len();
        match k.code {
            KeyCode::Esc | KeyCode::Char('L') | KeyCode::F(12) => self.mode = Mode::Main,
            KeyCode::Up => self.menu_sel = (self.menu_sel + n.saturating_sub(1)) % n.max(1),
            KeyCode::Down => self.menu_sel = (self.menu_sel + 1) % n.max(1),
            KeyCode::Enter => {
                if let Some(l) = self.langs.get(self.menu_sel) {
                    let code = l.code.clone();
                    self.set_lang(&code);
                }
                self.mode = Mode::Main;
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mfd_core::scan;

    /// La stessa fixture dei test di mfd-core: 7 file, 5 cartelle, 3530 B.
    fn fixture(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("mfd-tui-test-{}-{}", std::process::id(), tag));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("alpha")).unwrap();
        std::fs::create_dir_all(dir.join("beta/sub")).unwrap();
        std::fs::create_dir_all(dir.join("gamma/x")).unwrap();
        std::fs::write(dir.join("readme.txt"), vec![0u8; 100]).unwrap();
        std::fs::write(dir.join("alpha/lib.dll"), vec![0u8; 1000]).unwrap();
        std::fs::write(dir.join("alpha/data.dat"), vec![0u8; 2000]).unwrap();
        std::fs::write(dir.join("beta/note.txt"), vec![0u8; 50]).unwrap();
        std::fs::write(dir.join("beta/sub/altro.log"), vec![0u8; 70]).unwrap();
        std::fs::write(dir.join("gamma/x/y.dll"), vec![0u8; 300]).unwrap();
        std::fs::write(dir.join("gamma/z.tmp"), vec![0u8; 10]).unwrap();
        dir
    }

    /// Una fixture a piramide per provare la profondita' di a: quattro
    /// cartelle in colonna, un file in fondo.
    fn deep_fixture(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("mfd-tui-deep-{}-{}", std::process::id(), tag));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("d1/d2/d3/d4")).unwrap();
        std::fs::write(dir.join("d1/d2/d3/d4/leaf.txt"), vec![0u8; 40]).unwrap();
        std::fs::write(dir.join("d1/other.txt"), vec![0u8; 10]).unwrap();
        dir
    }

    fn app_on_fixture(tag: &str) -> App {
        let fx = fixture(tag);
        let tree = Arc::new(scan(&fx, &ScanOptions { depth: usize::MAX, filter: None }));
        App::with_tree(fx, tree)
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn list_names(app: &App) -> Vec<String> {
        let t = app.tree.as_ref().unwrap();
        app.list
            .iter()
            .map(|r| {
                // la riga radice si chiama come la chiama il disegno: il
                // punto della shell, non il nome della cartella
                if r.is_root {
                    ".".to_string()
                } else {
                    t.name(r.id).to_string_lossy().into_owned()
                }
            })
            .collect()
    }

    fn preview_names(app: &App) -> Vec<String> {
        let t = app.tree.as_ref().unwrap();
        app.preview
            .iter()
            .map(|p| match &p.row {
                Row::Dir(i) => format!("{}\\", t.name(*i).to_string_lossy()),
                Row::Leaf(_, n) => n.to_string_lossy().into_owned(),
            })
            .collect()
    }

    #[test]
    fn lista_solo_cartelle_per_taglia() {
        let app = app_on_fixture("lista");
        // la riga radice in cima, poi le cartelle in taglia decrescente
        assert_eq!(list_names(&app), vec![".", "alpha", "gamma", "beta"]);
        assert!(app.list[0].is_root);
        assert_eq!(app.list[1].size, 3000);
        assert_eq!(app.list[2].size, 310);
        assert_eq!(app.list[3].size, 120);
    }

    #[test]
    fn invio_entra_e_backspace_risale_riselezionando() {
        let mut app = app_on_fixture("nav");
        // sulla riga radice l'invio non va da nessuna parte (si e' gia'
        // lì): lo dice il flash
        app.on_key(key(KeyCode::Enter));
        assert_eq!(app.current, 0);
        assert!(app.status_now().unwrap().contains("gia"));
        // su alpha, invece, si entra
        app.on_key(key(KeyCode::Down)); // alpha
        let alpha = app.list[app.sel].id;
        app.on_key(key(KeyCode::Enter));
        assert_eq!(app.current, alpha);
        assert_eq!(list_names(&app), vec!["."]); // alpha non ha sotto-cartelle: resta la radice
        app.on_key(key(KeyCode::Backspace));
        assert_eq!(app.current, 0);
        assert_eq!(app.list[app.sel].id, alpha); // la selezione torna su alpha
    }

    #[test]
    fn l_anteprima_mostra_i_figli_della_selezione() {
        let app = app_on_fixture("prev");
        // selezionata la riga radice: l'anteprima sono i figli della
        // cartella corrente, cartelle e file insieme (T.D)
        assert_eq!(
            preview_names(&app),
            vec!["alpha\\", "gamma\\", "beta\\", "readme.txt"]
        );
        assert_eq!(app.preview[0].size, 3000);
        assert_eq!(app.preview_total, 3530);
    }

    #[test]
    fn s_cicla_l_ordinamento_della_lente_unica() {
        let mut app = app_on_fixture("ciclo");
        assert_eq!(app.view.order, Order::TD);
        app.on_key(key(KeyCode::Char('S')));
        assert_eq!(app.view.order, Order::TC);
        assert_eq!(list_names(&app), vec![".", "beta", "gamma", "alpha"]); // taglia crescente
        // il dettaglio cambia insieme: e' la stessa lente. Con T.C la riga
        // selezionata e' la radice: il suo dettaglio in taglia crescente
        assert_eq!(
            preview_names(&app),
            vec!["readme.txt", "beta\\", "gamma\\", "alpha\\"]
        );
        // e scendendo su alpha (3.000 B) il dettaglio segue, sempre in T.C
        app.on_key(key(KeyCode::Down));
        app.on_key(key(KeyCode::Down));
        app.on_key(key(KeyCode::Down));
        assert_eq!(preview_names(&app), vec!["lib.dll", "data.dat"]);
    }

    #[test]
    fn filtro_della_lista_nasconde_le_cartelle_senza_match() {
        let mut app = app_on_fixture("fil-lista");
        app.on_key(key(KeyCode::Char('F')));
        for c in "*.dll".chars() {
            app.on_key(key(KeyCode::Char(c)));
        }
        app.on_key(key(KeyCode::Enter));
        assert_eq!(list_names(&app), vec![".", "alpha", "gamma"]); // beta senza .dll
        assert_eq!(app.list[1].size, 1000); // semantica -f: taglia filtrata
        assert_eq!(app.list[2].size, 300);
        // filtro rimosso: il prompt arriva precompilato col filtro attivo
        // (si puo' ritoccare); si svuota con Backspace e INVIO
        app.on_key(key(KeyCode::Char('F')));
        for _ in 0.."*.dll".len() {
            app.on_key(key(KeyCode::Backspace));
        }
        app.on_key(key(KeyCode::Enter));
        assert_eq!(list_names(&app), vec![".", "alpha", "gamma", "beta"]);
    }

    #[test]
    fn il_filtro_vale_per_lista_e_dettaglio_insieme() {
        let mut app = app_on_fixture("fil");
        app.on_key(key(KeyCode::Char('F')));
        for c in "*.dll".chars() {
            app.on_key(key(KeyCode::Char(c)));
        }
        app.on_key(key(KeyCode::Enter));
        // la lista attraverso la lente *.dll: beta sparisce (senza .dll)
        assert_eq!(list_names(&app), vec![".", "alpha", "gamma"]);
        assert_eq!(app.list[1].size, 1000); // semantica -f: taglia filtrata
        // il dettaglio della selezione (la radice) con la STESSA lente: le
        // cartelle che hanno un .dll dentro, le altre no
        assert_eq!(preview_names(&app), vec!["alpha\\", "gamma\\"]);
        assert_eq!(app.preview_total, 1300); // lib.dll 1000 + y.dll 300
    }

    #[test]
    fn filtro_non_valido_si_dice_in_finestra_e_non_si_applica() {
        let mut app = app_on_fixture("fil-ko");
        app.on_key(key(KeyCode::Char('F')));
        for c in "REG([)".chars() {
            app.on_key(key(KeyCode::Char(c)));
        }
        app.on_key(key(KeyCode::Enter));
        // la finestra resta aperta con l'errore in vista: si corregge sul posto
        assert!(matches!(app.mode, Mode::FilterPrompt));
        assert!(app.view.filter.is_none());
        assert!(app.prompt_error.as_deref().unwrap().contains("non valido"));
        // ESC chiude senza applicare nulla
        app.on_key(key(KeyCode::Esc));
        assert!(matches!(app.mode, Mode::Main));
        assert!(app.view.filter.is_none());
        assert!(app.prompt_error.is_none());
    }

    #[test]
    fn g_toggle_la_barra_ovunque() {
        let mut app = app_on_fixture("barra");
        assert!(app.view.show_bar);
        app.on_key(key(KeyCode::Char('G')));
        assert!(!app.view.show_bar);
        app.on_key(key(KeyCode::Char('G')));
        assert!(app.view.show_bar);
    }

    #[test]
    fn shift_q_esce_e_le_finestre_si_chiudono_con_esc() {
        let mut app = app_on_fixture("esci");
        app.on_key(key(KeyCode::Char('W')));
        app.on_key(key(KeyCode::Esc)); // chiude i warning, non esce
        assert!(!app.should_quit());
        app.on_key(key(KeyCode::Char('Q')));
        assert!(app.should_quit());
    }

    #[test]
    fn la_lettera_nuda_non_comanda_vuole_lo_shift() {
        let mut app = app_on_fixture("shift");
        // la q col dito vicino non chiude piu' l'app: la lettera nuda non
        // fa nulla, e il flash insegna il maiuscolo che manca
        app.on_key(key(KeyCode::Char('q')));
        assert!(!app.should_quit());
        assert!(app.status_now().unwrap().contains("Shift+Q"));
        // col SHIFT addosso comanda: si esce
        app.on_key(key(KeyCode::Char('Q')));
        assert!(app.should_quit());
    }

    #[test]
    fn la_lettera_nuda_non_stampa() {
        let fx = fixture("shift-rep");
        let tree = Arc::new(scan(&fx, &ScanOptions { depth: usize::MAX, filter: None }));
        let mut app = App::with_tree(fx.clone(), tree);
        let dir = std::env::temp_dir().join(format!("mfd-shiftrep-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        app.stampa_dir = Some(dir.clone());
        // la p nuda non stampa: il flash insegna lo Shift, e si resta dove
        // si sta (la foto e' un comando, non un reflesso)
        app.on_key(key(KeyCode::Char('p')));
        assert!(matches!(app.mode, Mode::Main));
        assert!(app.status_now().unwrap().contains("Shift+P"));
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 0); // nessuna foto
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&fx);
    }

    #[test]
    fn f8_apre_e_richiude_la_finestra_dei_warning() {
        // regressione: F8 apriva nulla — la finestra si apriva solo con w,
        // mentre F8 era gestito soltanto come tasto di chiusura
        let mut app = app_on_fixture("f8");
        app.on_key(key(KeyCode::F(8)));
        assert!(matches!(app.mode, Mode::Warnings));
        app.on_key(key(KeyCode::F(8))); // la richiude
        assert!(matches!(app.mode, Mode::Main));
    }

    #[test]
    fn f9_muove_la_classifica_e_le_cifre_tagliano_il_top() {
        let mut app = app_on_fixture("report");
        // la selezione parte dalla riga radice: la classifica e' quella
        // dell'intera cartella corrente — il globale, alla radice dello
        // scan. I 7 file della fixture, data.dat 2000 in testa
        assert!(app.list[app.sel].is_root);
        assert_eq!(app.pane.len(), 7);
        assert_eq!(app.pane[0].0, 2000);
        assert!(app.pane[0].1.ends_with("data.dat"));
        assert_eq!(app.pane[6].0, 10); // z.tmp in coda
        // F9 nasconde la classifica (e niente lavoro per nulla)
        app.on_key(key(KeyCode::F(9)));
        assert!(!app.view.show_report);
        assert!(app.pane.is_empty());
        app.on_key(key(KeyCode::F(9)));
        assert_eq!(app.pane.len(), 7); // ricalcolata al ritorno
        // le cifre riscrivono il top, senza finestre: 2 tiene i primi 2
        app.on_key(key(KeyCode::Char('2')));
        assert_eq!(app.report_n, 2);
        assert_eq!(app.pane.len(), 2);
        // 25: la fixture ha solo 7 file, il taglio non le inventa
        app.on_key(key(KeyCode::Char('5')));
        assert_eq!(app.report_n, 25);
        assert_eq!(app.pane.len(), 7);
        // un altro tasto chiude la scrittura: la prossima cifra riparte
        app.on_key(key(KeyCode::Down));
        app.on_key(key(KeyCode::Char('3')));
        assert_eq!(app.report_n, 3);
    }

    #[test]
    fn la_classifica_in_fondo_segue_la_selezione() {
        // il report della CLI, vivo: i 10 piu' grandi del ramo selezionato,
        // ricorsivi — scorri l'albero e la risposta ti segue. F9 la mostra
        // e la nasconde, senza mai aprire finestre
        let mut app = app_on_fixture("report-rel");
        // si parte dalla riga radice: la classifica e' dell'intera
        // cartella — tutti i file della fixture
        assert!(app.list[app.sel].is_root);
        assert_eq!(app.pane.len(), 7);
        // giu' su alpha: la classifica cambia insieme alla selezione —
        // i suoi due file, il piu' grande in testa
        app.on_key(key(KeyCode::Down)); // alpha
        assert_eq!(app.pane.len(), 2);
        assert_eq!(app.pane[0].0, 2000);
        assert!(app.pane[0].1.ends_with("data.dat"));
        // ancora giu' su gamma: la classifica cambia di nuovo
        app.on_key(key(KeyCode::Down));
        assert_eq!(app.pane.len(), 2); // y.dll 300, z.tmp 10
        assert_eq!(app.pane[0].0, 300);
        assert!(app.pane[0].1.ends_with("y.dll"));
        // F9: si nasconde (e niente lavoro per nulla), si ripresenta
        app.on_key(key(KeyCode::F(9)));
        assert!(!app.view.show_report);
        assert!(app.pane.is_empty());
        assert!(matches!(app.mode, Mode::Main)); // mai una finestra
        app.on_key(key(KeyCode::F(9)));
        assert!(app.view.show_report);
        assert_eq!(app.pane.len(), 2); // ricalcolata al ritorno
    }

    #[test]
    fn la_classifica_passa_dalla_lente_come_tutto_il_resto() {
        let mut app = app_on_fixture("report-fil");
        // lente *.dll: nella classifica della radice finiscono solo i file
        // che matchano
        app.view.filter = Some(Filter::build("*.dll").unwrap());
        app.refresh();
        let names: Vec<String> = app
            .pane
            .iter()
            .map(|(_, p)| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, vec!["lib.dll", "y.dll"]); // 1000, 300
        // tolta la lente, la classifica torna a vedere tutti i file
        app.view.filter = None;
        app.refresh();
        assert_eq!(app.pane.len(), 7);
    }

    #[test]
    fn la_classifica_segue_il_drill_down_con_la_riga_radice() {
        // la riga radice in cima e' la cartella corrente: selezionata, la
        // classifica e' dell'intera cartella (il globale); entrati in
        // gamma, la riga radice diventa gamma e la classifica la segue.
        // Tag a se' stante: il fixture vive su disco e i test girano in
        // parallelo
        let mut app = app_on_fixture("report-drill");
        assert_eq!(app.pane.len(), 7); // alla radice: tutta la fixture
        app.on_key(key(KeyCode::Down)); // alpha (alpha 3000, gamma 310)
        app.on_key(key(KeyCode::Down)); // gamma
        app.on_key(key(KeyCode::Enter)); // drill-down dentro gamma
        // la riga radice adesso e' gamma: la classifica e' la sua
        assert!(app.list[app.sel].is_root);
        assert_eq!(app.list[app.sel].id, app.current);
        assert_eq!(app.pane.len(), 2); // y.dll 300, z.tmp 10
        assert_eq!(app.pane[0].0, 300);
        assert!(app.pane[0].1.ends_with("y.dll"));
        // il big della radice non c'entra: e' fuori dal sottoalbero
        assert!(!app.pane.iter().any(|(_, p)| p.ends_with("data.dat")));
        // e risalendo la riga radice torna quella di prima
        app.on_key(key(KeyCode::Left));
        assert_eq!(app.pane.len(), 7);
    }

    #[test]
    fn il_top_della_classifica_si_cambia_a_cifre() {
        // la classifica parte da 10 e le cifre la riscrivono al volo,
        // senza finestre: si allarga sotto gli occhi, TAB la scorre
        let mut app = app_on_fixture("report-top");
        assert_eq!(app.report_n, 10);
        assert_eq!(app.pane.len(), 7); // la fixture ne ha 7
        app.on_key(key(KeyCode::Char('2'))); // la prima cifra riparte da zero
        assert_eq!(app.report_n, 2);
        assert_eq!(app.pane.len(), 2);
        assert_eq!(app.pane[1].0, 1000); // il taglio tiene i primi 2
        app.on_key(key(KeyCode::Char('5'))); // la seconda si accoda: 25
        assert_eq!(app.report_n, 25);
        assert_eq!(app.pane.len(), 7); // ma i file sono quelli che sono
        // un tasto qualunque chiude la scrittura: la prossima cifra riparte
        app.on_key(key(KeyCode::Enter)); // sulla radice: flash, non drill
        app.on_key(key(KeyCode::Char('3')));
        assert_eq!(app.report_n, 3);
        assert_eq!(app.pane.len(), 3);
        // il top scelto e' della sessione: scorre l'albero, e resta
        app.on_key(key(KeyCode::Down));
        assert_eq!(app.report_n, 3);
    }

    #[test]
    fn menu_ordinamento_applica_la_scelta() {
        let mut app = app_on_fixture("menu");
        app.on_key(key(KeyCode::F(6))); // menu dell'ordinamento
        // ORDERS = [T.D, T.C, A.C, ...]: due Discese portano su A.C
        app.on_key(key(KeyCode::Down));
        app.on_key(key(KeyCode::Down));
        app.on_key(key(KeyCode::Enter));
        assert_eq!(app.view.order, Order::AC);
        assert_eq!(list_names(&app), vec![".", "alpha", "beta", "gamma"]);
    }

    #[test]
    fn menu_tema_applica_la_scelta_e_esc_annulla() {
        let mut app = app_on_fixture("tema");
        assert_eq!(app.theme_id, ThemeId::Console);
        app.on_key(key(KeyCode::Char('T'))); // menu dei temi: parte dalla console (default)
        assert!(matches!(app.mode, Mode::ThemeMenu));
        app.on_key(key(KeyCode::Down)); // CLI (il menu e' ciclico)
        app.on_key(key(KeyCode::Down)); // Notturno
        app.on_key(key(KeyCode::Enter));
        assert_eq!(app.theme_id, ThemeId::Notturno);
        assert!(matches!(app.mode, Mode::Main));
        assert!(app.status_now().unwrap().contains("tema"));
        // ESC chiude senza cambiare il tema in uso
        app.on_key(key(KeyCode::Char('T')));
        app.on_key(key(KeyCode::Down)); // Debian
        app.on_key(key(KeyCode::Esc));
        assert_eq!(app.theme_id, ThemeId::Notturno);
        assert!(matches!(app.mode, Mode::Main));
    }

    #[test]
    fn f3_apre_il_menu_dei_temi() {
        let mut app = app_on_fixture("tema-f3");
        app.on_key(key(KeyCode::F(3)));
        assert!(matches!(app.mode, Mode::ThemeMenu));
    }

    #[test]
    fn dettaglio_totali_in_cima_taglia_file_cartelle() {
        let app = app_on_fixture("dettaglio");
        let t = app.tree.as_ref().unwrap();
        // senza filtro: i totali della selezione di default — la riga
        // radice, cioe' l'intera cartella (7 file, 5 cartelle, 3.430 B)
        assert_eq!(
            crate::ui::detail_totals(t, app.current, None, &app.cat),
            " 3,4 KB - 7 file - 5 cartelle"
        );
        // col filtro: la lente conta solo cio' che lascia passare (nella
        // radice: lib.dll e y.dll)
        let fl = Filter::build("*.dll").unwrap();
        assert_eq!(
            crate::ui::detail_totals(t, app.current, Some(&fl), &app.cat),
            " 1,3 KB - 2 file dentro *.dll"
        );
    }

    #[test]
    fn f2_salva_le_impostazioni_e_il_prossimo_avvio_le_riprende() {
        let fx = fixture("f2");
        let tmp = std::env::temp_dir().join(format!("mfd-f2-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let path = tmp.join("settings.txt");
        let tree = Arc::new(scan(&fx, &ScanOptions { depth: usize::MAX, filter: None }));
        let mut app = App::with_tree(fx.clone(), Arc::clone(&tree));
        app.settings_path = Some(path.clone());
        // uno stato che merita di sopravvivere: tema, lente e barra
        app.theme_id = ThemeId::Debian;
        app.view.order = Order::AC;
        app.view.filter = Some(Filter::build("*.dll").unwrap());
        app.view.show_bar = false;
        app.on_key(key(KeyCode::F(2)));
        assert!(app.status_now().unwrap().contains("salvate"));
        assert!(path.exists());
        // il prossimo avvio riprende tutto quello che F2 aveva fotografato
        let mut app2 = App::with_tree(fx.clone(), tree);
        app2.settings_path = Some(path);
        app2.load_saved();
        assert_eq!(app2.theme_id, ThemeId::Debian);
        assert_eq!(app2.view.order, Order::AC);
        assert_eq!(app2.view.filter.as_ref().unwrap().display(), "*.dll");
        assert!(!app2.view.show_bar);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn f5_riscansiona() {
        let mut app = app_on_fixture("f5");
        assert!(matches!(app.mode, Mode::Main));
        app.on_key(key(KeyCode::F(5)));
        assert!(matches!(app.mode, Mode::Scanning));
    }

    #[test]
    fn p_stampa_la_fotografia_della_cartella_corrente() {
        let fx = fixture("stampa");
        let tree = Arc::new(scan(&fx, &ScanOptions { depth: usize::MAX, filter: None }));
        let mut app = App::with_tree(fx.clone(), tree);
        // si entra in alpha: la foto e' del suo sottoalbero, non di tutta la
        // scansione — la radice della stampa segue il drill-down, come il
        // resto della navigazione
        app.on_key(key(KeyCode::Down)); // alpha
        app.on_key(key(KeyCode::Enter));
        let dir = std::env::temp_dir().join(format!("mfd-stampa-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        app.stampa_dir = Some(dir.clone());
        app.on_key(key(KeyCode::Char('P')));
        let foto = dir.join("mfd-stampa-alpha.json");
        assert!(foto.exists());
        let testo = std::fs::read_to_string(&foto).unwrap();
        // la testa dichiara la cartella fotografata (nel JSON i backslash si
        // raddoppiano, come tutti gli escape) e i totali sono quelli
        // ricorsivi del ramo
        let alpha = fx.join("alpha").display().to_string().replace('\\', "\\\\");
        assert!(testo.contains(&format!(r#""root": "{}""#, alpha)));
        assert!(testo.contains(r#""bytes": 3000,"#));
        assert!(testo.contains(r#"{"name": "data.dat", "bytes": 2000}"#));
        assert!(!testo.contains("readme.txt")); // fuori dal ramo
        assert!(app.status_now().unwrap().contains("stampa salvata"));
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&fx);
    }

    #[test]
    fn p_di_nuovo_non_sovrascrive_la_stampa_precedente() {
        let fx = fixture("stampa2");
        let tree = Arc::new(scan(&fx, &ScanOptions { depth: usize::MAX, filter: None }));
        let mut app = App::with_tree(fx.clone(), tree);
        // fermi sulla radice: la foto e' dell'intera scansione e il nome del
        // file e' la cartella scansionata (la testata della foto)
        let dir = std::env::temp_dir().join(format!("mfd-stampa2-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        app.stampa_dir = Some(dir.clone());
        app.on_key(key(KeyCode::Char('P')));
        app.on_key(key(KeyCode::Char('P')));
        // due fotografie valgono un diff: la seconda non si mette sopra la
        // prima, prende il primo nome libero
        let nome = fx.file_name().unwrap().to_string_lossy().into_owned();
        assert!(dir.join(format!("mfd-stampa-{}.json", nome)).exists());
        let seconda = dir.join(format!("mfd-stampa-{}-2.json", nome));
        assert!(seconda.exists());
        let testo = std::fs::read_to_string(&seconda).unwrap();
        assert!(testo.contains("readme.txt")); // i file diretti della radice si vedono
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&fx);
    }

    #[test]
    fn p_senza_cartella_dove_scrivere_lo_dice_nel_flash() {
        let mut app = app_on_fixture("stampa3");
        app.stampa_dir = None;
        app.on_key(key(KeyCode::Char('P')));
        assert!(app.status_now().unwrap().contains("stampa impossibile"));
    }

    #[test]
    fn p_fotografa_l_albero_come_lo_si_vede() {
        // la foto segue l'apertura dell'utente: a rami chiusi la foto ha
        // righe di totali ("children": null) e il dentro non c'e'; aperto
        // il sottoalbero di gamma con a, il dentro entra — la stampa salva
        // quello che si vede, non il mondo intero
        let fx = fixture("stampa-vista");
        let tree = Arc::new(scan(&fx, &ScanOptions { depth: usize::MAX, filter: None }));
        let mut app = App::with_tree(fx.clone(), tree);
        let dir = std::env::temp_dir().join(format!("mfd-stampa-vista-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        app.stampa_dir = Some(dir.clone());
        app.on_key(key(KeyCode::Char('P'))); // tutto chiuso
        let nome = fx.file_name().unwrap().to_string_lossy().into_owned();
        let testo =
            std::fs::read_to_string(dir.join(format!("mfd-stampa-{}.json", nome))).unwrap();
        assert!(testo.contains("readme.txt")); // i file diretti della radice si vedono
        assert!(testo.contains(r#""children": null"#)); // i rami chiusi: righe di totali
        assert!(!testo.contains("y.dll")); // dentro gamma/x: chiuso, invisibile
        assert!(!testo.contains("note.txt")); // dentro beta: chiuso, invisibile
        // si apre tutto il sottoalbero di gamma (a) e si ristampa
        app.on_key(key(KeyCode::Down));
        app.on_key(key(KeyCode::Down)); // gamma
        app.on_key(key(KeyCode::Char('A')));
        app.on_key(key(KeyCode::Char('P')));
        let testo =
            std::fs::read_to_string(dir.join(format!("mfd-stampa-{}-2.json", nome))).unwrap();
        assert!(testo.contains("y.dll")); // adesso il dentro c'e'
        assert!(testo.contains("z.tmp")); // le foglie dirette di gamma pure
        assert!(!testo.contains("note.txt")); // beta resta chiuso
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&fx);
    }

    #[test]
    fn p_fotografa_attraverso_la_lente() {
        // col filtro attivo la foto passa dalla lente: le foglie non
        // matching non ci sono, i rami senza match nemmeno, i totali sono
        // quelli filtrati e la lente si dichiara in testa
        let fx = fixture("stampa-lente");
        let tree = Arc::new(scan(&fx, &ScanOptions { depth: usize::MAX, filter: None }));
        let mut app = App::with_tree(fx.clone(), tree);
        app.view.filter = Some(Filter::build("*.dll").unwrap());
        app.refresh();
        let dir = std::env::temp_dir().join(format!("mfd-stampa-lente-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        app.stampa_dir = Some(dir.clone());
        // si apre il sottoalbero di gamma (a): y.dll diventa visibile;
        // beta resta nascosto (senza match, come nella lista)
        app.on_key(key(KeyCode::Down));
        app.on_key(key(KeyCode::Down)); // gamma
        app.on_key(key(KeyCode::Char('A')));
        app.on_key(key(KeyCode::Char('P')));
        let nome = fx.file_name().unwrap().to_string_lossy().into_owned();
        let testo =
            std::fs::read_to_string(dir.join(format!("mfd-stampa-{}.json", nome))).unwrap();
        assert!(testo.contains(r#""filter": "*.dll""#)); // la lente si dichiara
        assert!(testo.contains("y.dll")); // matching, nel ramo aperto
        assert!(!testo.contains("z.tmp")); // foglia non matching: invisibile
        assert!(!testo.contains("readme.txt")); // nemmeno alla radice
        assert!(!testo.contains(r#""name": "beta""#)); // ramo senza match
        assert!(testo.contains(r#""name": "alpha""#)); // chiuso ma visibile
        assert!(testo.contains(r#""children": null"#)); // come riga di totali
        // i totali passano dalla lente (lib.dll + y.dll); le cartelle
        // restano strutturali, come i contatori di -f
        assert!(testo.contains(r#""totals": {"bytes": 1300, "files": 2, "dirs": 5,"#));
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&fx);
    }

    #[test]
    fn f11_e_l_alias_di_p() {
        // ogni feature del TUI ha il suo tasto funzione: la stampa e'
        // p / F11, come t / F3 il tema e r / F9 il report
        let mut app = app_on_fixture("f11");
        let dir = std::env::temp_dir().join(format!("mfd-f11-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        app.stampa_dir = Some(dir.clone());
        app.on_key(key(KeyCode::F(11)));
        assert!(app.status_now().unwrap().contains("stampa salvata"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn p_fotografa_anche_il_pannello_di_destra() {
        // la stampa dice cosa c'e' su ENTRAMBI i pannelli: l'albero di
        // sinistra (alpha chiuso = riga di totali) e il dettaglio di
        // destra (la riga selezionata con i suoi figli, che l'albero
        // tiene chiusi)
        let fx = fixture("dettaglio");
        let tree = Arc::new(scan(&fx, &ScanOptions { depth: usize::MAX, filter: None }));
        let mut app = App::with_tree(fx.clone(), tree);
        // la selezione parte dalla riga radice: si scende su alpha (3000 B)
        // per fotografare anche il suo dettaglio
        assert_eq!(list_names(&app), vec![".", "alpha", "gamma", "beta"]);
        app.on_key(key(KeyCode::Down)); // alpha
        let dir = std::env::temp_dir().join(format!("mfd-dettaglio-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        app.stampa_dir = Some(dir.clone());
        app.on_key(key(KeyCode::Char('P')));
        let nome = fx.file_name().unwrap().to_string_lossy().into_owned();
        let testo = std::fs::read_to_string(dir.join(format!("mfd-stampa-{}.json", nome))).unwrap();
        // l'albero di sinistra: alpha resta una riga di totali ricorsivi
        assert!(testo.contains(r#""children": null"#));
        // il dettaglio di destra: la riga selezionata dichiarata e i suoi
        // due file in ordine di taglia, come li mostra l'anteprista
        let alpha = fx.join("alpha").display().to_string().replace('\\', "\\\\");
        assert!(testo.contains(&format!(r#""selected": "{}""#, alpha)));
        assert!(testo.contains(r#"{"name": "data.dat", "kind": "file", "bytes": 2000}"#));
        assert!(testo.contains(r#"{"name": "lib.dll", "kind": "file", "bytes": 1000}"#));
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&fx);
    }

    #[test]
    fn x_apre_e_chiude_il_ramo_sotto_il_cursore() {
        let mut app = app_on_fixture("x");
        // la lista parte piatta: la radice, poi alpha, gamma, beta (la taglia)
        assert_eq!(list_names(&app), vec![".", "alpha", "gamma", "beta"]);
        assert_eq!(app.list.iter().map(|r| r.depth).collect::<Vec<_>>(), vec![0, 0, 0, 0]);
        app.on_key(key(KeyCode::Down));
        app.on_key(key(KeyCode::Down)); // gamma: la prima con sotto-cartelle
        app.on_key(key(KeyCode::Char('X')));
        // il ramo aperto mostra i figli indentati sotto il padre, al posto
        // giusto nell'ordine di lente (gamma 310 B, x 300 B, beta 120 B)
        assert_eq!(list_names(&app), vec![".", "alpha", "gamma", "x", "beta"]);
        assert_eq!(app.list[3].depth, 1);
        assert!(app.expanded.contains(&app.list[2].id));
        // x di nuovo: il ramo si chiude e tutto cio' che stava sotto sparisce
        app.on_key(key(KeyCode::Char('X')));
        assert_eq!(list_names(&app), vec![".", "alpha", "gamma", "beta"]);
        assert!(app.expanded.is_empty());
    }

    #[test]
    fn f4_e_l_alias_di_x() {
        let mut app = app_on_fixture("f4");
        app.on_key(key(KeyCode::Down));
        app.on_key(key(KeyCode::Down)); // gamma
        app.on_key(key(KeyCode::F(4)));
        assert_eq!(list_names(&app), vec![".", "alpha", "gamma", "x", "beta"]);
    }

    #[test]
    fn a_apre_tutto_il_sottoalbero() {
        let fx = deep_fixture("a");
        let tree = Arc::new(scan(&fx, &ScanOptions { depth: usize::MAX, filter: None }));
        let mut app = App::with_tree(fx.clone(), tree);
        assert_eq!(list_names(&app), vec![".", "d1"]);
        // A sulla riga radice apre TUTTO l'albero della cartella corrente:
        // quattro livelli in colonna, la profondita' illimitata di -d a
        app.on_key(key(KeyCode::Char('A')));
        assert_eq!(list_names(&app), vec![".", "d1", "d2", "d3", "d4"]);
        assert_eq!(
            app.list.iter().map(|r| r.depth).collect::<Vec<_>>(),
            vec![0, 0, 1, 2, 3]
        );
        let _ = std::fs::remove_dir_all(&fx);
    }

    #[test]
    fn il_filtro_nasconde_anche_i_rami_aperti() {
        let mut app = app_on_fixture("albero-fil");
        // *.log vive solo in beta/sub: la lente lascia vedere solo beta
        app.view.filter = Some(Filter::build("*.log").unwrap());
        app.refresh();
        assert_eq!(list_names(&app), vec![".", "beta"]);
        app.on_key(key(KeyCode::Down)); // beta
        app.on_key(key(KeyCode::Char('X'))); // apre beta: sub ha un match
        assert_eq!(list_names(&app), vec![".", "beta", "sub"]);
        assert_eq!(app.list[2].depth, 1);
        // i rami senza match non si vedono ne' si attraversano, anche se
        // alpha e gamma sono piu' grandi: la lente vale a ogni livello
    }

    #[test]
    fn tab_porta_il_cursore_in_lettura_anche_con_lalbero_chiuso() {
        let mut app = app_on_fixture("tab");
        assert_eq!(app.focus, Focus::List);
        // albero chiuso: TAB funziona lo stesso — il cambio pannello alla
        // Total Commander non ha condizioni, il dettaglio si legge sempre
        app.on_key(key(KeyCode::Down));
        app.on_key(key(KeyCode::Down)); // gamma: dettaglio di 2 righe
        app.on_key(key(KeyCode::Tab));
        assert_eq!(app.focus, Focus::Detail);
        assert!(app.status_now().unwrap().contains("dettaglio"));
        assert!(app.expanded.is_empty()); // l'albero resta chiuso
        app.on_key(key(KeyCode::Down)); // muove il cursore del dettaglio
        assert_eq!(app.dsel, 1);
        assert_eq!(app.sel, 2); // gamma: la selezione dell'albero non si muove
        app.on_key(key(KeyCode::Tab)); // il cursore torna all'albero
        assert_eq!(app.focus, Focus::List);
        app.on_key(key(KeyCode::Down)); // adesso si muove la selezione (beta)
        assert_eq!(app.sel, 3);
        assert_eq!(app.dsel, 0); // e il cursore del dettaglio riparte
    }

    #[test]
    fn invio_e_sinistra_dal_dettaglio_non_navigano() {
        let mut app = app_on_fixture("invio-dettaglio");
        app.on_key(key(KeyCode::Down));
        app.on_key(key(KeyCode::Down)); // gamma: il dettaglio e' [x\, z.tmp]
        app.on_key(key(KeyCode::Tab)); // il cursore passa al dettaglio
        assert_eq!(preview_names(&app), vec!["x\\", "z.tmp"]);
        let before = app.current;
        // INVIO: il dettaglio si legge, non si naviga — entrare e' roba
        // dell'albero (due strade per entrare non si capirebbero)
        app.on_key(key(KeyCode::Enter));
        assert_eq!(app.current, before);
        assert_eq!(app.focus, Focus::Detail);
        // <- e Backspace: risalire pure, dall'albero
        app.on_key(key(KeyCode::Left));
        assert_eq!(app.current, before);
        app.on_key(key(KeyCode::Backspace));
        assert_eq!(app.current, before);
    }

    #[test]
    fn chiudere_lultimo_ramo_lascia_il_cursore_nel_dettaglio() {
        let mut app = app_on_fixture("ultimo-ramo");
        app.on_key(key(KeyCode::Down));
        app.on_key(key(KeyCode::Down)); // gamma
        app.on_key(key(KeyCode::Char('X'))); // apre gamma
        app.on_key(key(KeyCode::Tab)); // cursore in lettura nel dettaglio
        assert_eq!(app.focus, Focus::Detail);
        app.on_key(key(KeyCode::Char('X'))); // chiude gamma, anche da qui
        // la vista estesa e' finita ma il cursore in lettura ci resta:
        // TAB riporta sempre all'albero, non c'e' motivo di cacciarlo
        assert_eq!(app.focus, Focus::Detail);
        assert!(app.expanded.is_empty());
        assert_eq!(list_names(&app), vec![".", "alpha", "gamma", "beta"]);
    }

    #[test]
    fn i_rami_aperti_sopravvivono_al_drill_down() {
        let mut app = app_on_fixture("drill-rami");
        app.on_key(key(KeyCode::Down));
        app.on_key(key(KeyCode::Down)); // gamma
        app.on_key(key(KeyCode::Char('X'))); // apre gamma
        let gamma = app.list[app.sel].id;
        app.on_key(key(KeyCode::Enter)); // drill-down dentro gamma
        assert_eq!(app.current, gamma);
        assert_eq!(list_names(&app), vec![".", "x"]);
        app.on_key(key(KeyCode::Backspace)); // risale
        // il ramo resta aperto: chi risale ritrova la vista che aveva
        assert_eq!(list_names(&app), vec![".", "alpha", "gamma", "x", "beta"]);
        assert!(app.expanded.contains(&gamma));
    }

    #[test]
    fn la_rotella_muove_il_cursore_dove_sta() {
        let mut app = app_on_fixture("rotella");
        app.on_scroll(true); // un detent giu': come la freccia
        assert_eq!(app.sel, 1);
        assert_eq!(list_names(&app)[1], "alpha");
        app.on_scroll(false); // e uno su': torna
        assert_eq!(app.sel, 0);
        app.on_scroll(false); // sopra la prima riga si resta, come la freccia
        assert_eq!(app.sel, 0);
        // col cursore in lettura gira il dettaglio, non la lista
        app.on_key(key(KeyCode::Tab));
        app.on_scroll(true);
        assert_eq!(app.dsel, 1); // il dettaglio della radice: la riga dopo alpha
        assert_eq!(app.sel, 0); // la selezione dell'albero non si muove
    }

    #[test]
    fn la_rotella_sulla_classifica_muove_il_suo_cursore() {
        let mut app = app_on_fixture("rotella-report");
        app.hits = fake_hits(); // il pannello della classifica c'e' (y 24-37)
        app.on_mouse(mouse(MouseEventKind::ScrollDown, 40, 30));
        assert_eq!(app.focus, Focus::Report); // la rotella porta li' il cursore
        assert_eq!(app.pane_sel, 1);
        assert_eq!(app.sel, 0); // l'albero non si tocca
    }

    #[test]
    fn la_rotella_nei_menu_muove_la_voce() {
        let mut app = app_on_fixture("rotella-menu");
        // menu dei temi: quattro voci, il giro e' ciclico — come le frecce
        app.on_key(key(KeyCode::F(3)));
        assert_eq!(app.menu_sel, 2); // parte dal tema corrente (Console)
        app.on_scroll(true); // 2 -> 3
        app.on_scroll(true); // 3 -> 0: il giro si chiude e riparte
        assert_eq!(app.menu_sel, 0);
        app.on_scroll(false); // 0 -> 3
        app.on_scroll(false); // 3 -> 2
        assert_eq!(app.menu_sel, 2);
        app.on_key(key(KeyCode::Esc)); // annulla e chiude
        // menu dell'ordinamento: otto voci, ai bordi si resta (non ciclico)
        app.on_key(key(KeyCode::F(6)));
        app.on_scroll(false);
        assert_eq!(app.menu_sel, 0);
    }

    #[test]
    fn la_rotella_orizzontale_risale_e_entra_come_le_frecce() {
        let mut app = app_on_fixture("tilt");
        app.on_key(key(KeyCode::Down)); // gamma
        let gamma = app.list[app.sel].id;
        let root = app.current;
        app.on_key(key(KeyCode::Enter)); // drill-down dentro gamma
        assert_eq!(app.current, gamma);
        // tilt a sinistra: <- vestita da rotella — risale e riseleziona
        app.on_mouse(mouse(MouseEventKind::ScrollLeft, 1, 1));
        assert_eq!(app.current, root);
        assert_eq!(app.list[app.sel].id, gamma);
        // tilt a destra: -> vestita da rotella — rientra dove si era
        app.on_mouse(mouse(MouseEventKind::ScrollRight, 1, 1));
        assert_eq!(app.current, gamma);
        // e di nuovo a sinistra si risale: il giro si puo' rifare
        app.on_mouse(mouse(MouseEventKind::ScrollLeft, 1, 1));
        assert_eq!(app.current, root);
        // dal dettaglio non si naviga, nemmeno col tilt — come la freccia
        app.on_key(key(KeyCode::Tab)); // cursore in lettura nel dettaglio
        app.on_mouse(mouse(MouseEventKind::ScrollLeft, 1, 1));
        assert_eq!(app.current, root);
        assert_eq!(app.focus, Focus::Detail);
    }

    /// Le zone di un frame finto ma coerente: pannello lista a sinistra
    /// (righe dati da y=2), dettaglio a destra (righe da y=5), come le
    /// disegnerebbe ui.rs. I test di logica non passano dal terminale: il
    /// contratto render/zone e' del test di ui.rs.
    fn fake_hits() -> Hits {
        Hits {
            list_pane: Some(Rect::new(1, 1, 59, 22)),
            list: Some(HitRows {
                rect: Rect::new(1, 2, 59, 21),
                top: 0,
                stride: 1,
            }),
            detail_pane: Some(Rect::new(61, 1, 18, 22)),
            detail: Some(HitRows {
                rect: Rect::new(61, 5, 18, 15),
                top: 0,
                stride: 1,
            }),
            report_pane: Some(Rect::new(1, 24, 78, 14)),
            report: Some(HitRows {
                rect: Rect::new(2, 27, 76, 10),
                top: 0,
                stride: 1,
            }),
            menu: None,
        }
    }

    fn mouse(kind: MouseEventKind, x: u16, y: u16) -> MouseEvent {
        MouseEvent {
            kind,
            column: x,
            row: y,
            modifiers: KeyModifiers::NONE,
        }
    }

    #[test]
    fn il_click_seleziona_la_riga_puntata() {
        let mut app = app_on_fixture("click-sel");
        app.hits = fake_hits();
        // la quarta riga (beta) sta a y=5: la radice ha preso il posto d'onore
        app.on_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 10, 5));
        assert_eq!(app.sel, 3);
        assert_eq!(app.focus, Focus::List);
        // l'anteprima segue la selezione, come con le frecce: il dettaglio
        // racconta beta, non alpha
        assert_eq!(preview_names(&app), vec!["sub\\", "note.txt"]);
    }

    #[test]
    fn il_doppio_click_entra_come_invio() {
        let mut app = app_on_fixture("click-doppio");
        app.hits = fake_hits();
        let beta = app.list[3].id;
        // il primo click seleziona (e se ne prende nota)...
        app.on_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 10, 5));
        assert_eq!(app.sel, 3);
        assert_eq!(app.current, 0); // ...ma non entra
        // ...il secondo, subito e sulla stessa riga (due colonne di
        // tolleranza): INVIO
        app.on_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 11, 5));
        assert_eq!(app.current, beta); // si entra in beta
        assert_eq!(app.sel, 0); // e la lista riparte da capo, come da tastiera
    }

    #[test]
    fn il_doppio_click_stanco_resta_una_selezione() {
        let mut app = app_on_fixture("click-lento");
        app.hits = fake_hits();
        app.on_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 10, 5));
        // il click di un'ora fa non conta piu': il doppio ha una scadenza
        app.last_click = Some((Instant::now() - Duration::from_secs(3600), 10, 5));
        app.on_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 10, 5));
        assert_eq!(app.current, 0); // non si entra: e' solo un'altra selezione
        assert_eq!(app.sel, 3);
    }

    #[test]
    fn il_click_sul_dettaglio_porta_la_lettura() {
        let mut app = app_on_fixture("click-dettaglio");
        app.hits = fake_hits();
        // il dettaglio di alpha ha due righe: data.dat (y=5) e lib.dll (y=6)
        app.on_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 70, 6));
        assert_eq!(app.focus, Focus::Detail); // come TAB
        assert_eq!(app.dsel, 1); // il cursore va sulla riga puntata
        // un click sul pannello ma fuori dalle righe (i totali in cima):
        // solo il cursore in lettura, la riga resta dov'era
        app.on_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 70, 1));
        assert_eq!(app.focus, Focus::Detail);
        assert_eq!(app.dsel, 1);
    }

    #[test]
    fn il_tasto_destro_apre_il_ramo_cliccato() {
        let mut app = app_on_fixture("click-destro");
        app.hits = fake_hits();
        let gamma = app.list[2].id;
        // il destro sulla terza riga (gamma): cursore lì e ramo aperto
        app.on_mouse(mouse(MouseEventKind::Down(MouseButton::Right), 10, 4));
        assert_eq!(app.sel, 2);
        assert!(app.expanded.contains(&gamma)); // x / F4, senza tastiera
        // di nuovo: il ramo si chiude (e' un toggle, come x)
        app.on_mouse(mouse(MouseEventKind::Down(MouseButton::Right), 10, 4));
        assert!(!app.expanded.contains(&gamma));
    }

    #[test]
    fn il_click_sul_menu_ordina_e_applica() {
        let mut app = app_on_fixture("click-menu");
        app.on_key(key(KeyCode::F(6))); // menu dell'ordinamento
        // la finestra del menu, con le voci da y=8 (una riga l'una)
        app.hits.menu = Some(HitRows {
            rect: Rect::new(20, 8, 40, 12),
            top: 0,
            stride: 1,
        });
        // la terza voce delle otto di -o: si clicca e si applica
        app.on_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 30, 10));
        assert_eq!(app.view.order, ORDERS[2]);
        assert_eq!(app.mode, Mode::Main); // INVIO, non solo selezione
    }

    #[test]
    fn il_click_sul_menu_tema_capisce_il_blurb() {
        let mut app = app_on_fixture("click-menu-tema");
        app.on_key(key(KeyCode::Char('T')));
        // nel menu dei temi ogni voce sono DUE righe (nome e blurb):
        // cliccare il blurb e' cliccare la voce
        app.hits.menu = Some(HitRows {
            rect: Rect::new(20, 6, 40, 12),
            top: 0,
            stride: 2,
        });
        app.on_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 30, 9)); // blurb
        assert_eq!(app.theme_id, ThemeId::ALL[1]);
        assert_eq!(app.mode, Mode::Main);
    }

    #[test]
    fn la_rotella_arriva_anche_da_on_mouse() {
        let mut app = app_on_fixture("mouse-rotella");
        // tutto il mouse passa da on_mouse: anche la rotella di prima
        app.on_mouse(mouse(MouseEventKind::ScrollDown, 10, 10));
        assert_eq!(app.sel, 1); // un detent = una freccia
    }

    #[test]
    fn il_click_nelle_finestre_non_fa_nulla() {
        let mut app = app_on_fixture("click-finestra");
        app.on_key(key(KeyCode::F(1))); // l'aiuto copre i pannelli
        app.hits = fake_hits(); // ...e le zone dei pannelli ci sono comunque
        app.on_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 10, 4));
        app.on_mouse(mouse(MouseEventKind::Down(MouseButton::Right), 10, 4));
        assert!(matches!(app.mode, Mode::Help)); // niente selezioni ne' invii:
        assert_eq!(app.sel, 0); // ESC e INVIO restano tasti
    }

    /// L'italiano del crate, quello vero (lo stesso file che lingua_di_test
    /// carica): i test del menu parlano di lingue vere, non di JSON inventati.
    fn lingua_it_del_crate() -> Lang {
        let path = std::path::Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../mfd-core/lang/it.json"
        ));
        Lang::from_file(path).unwrap()
    }

    #[test]
    fn l_apre_il_menu_delle_lingue_e_f12_e_il_gemello() {
        let mut app = app_on_fixture("lang-menu");
        app.langs = vec![Lang::english(), lingua_it_del_crate()];
        app.on_key(key(KeyCode::Char('L')));
        assert!(matches!(app.mode, Mode::LangMenu));
        // la selezione parte dalla lingua in uso (il test e' italiano)
        assert_eq!(app.menu_sel, 1);
        app.on_key(key(KeyCode::Esc));
        assert!(matches!(app.mode, Mode::Main));
        app.on_key(key(KeyCode::F(12))); // il gemello F, come p / F11
        assert!(matches!(app.mode, Mode::LangMenu));
        // e l chiude, come w nella finestra dei warning
        app.on_key(key(KeyCode::Char('L')));
        assert!(matches!(app.mode, Mode::Main));
    }

    #[test]
    fn la_lingua_scelta_si_applica_e_il_flash_parla_la_lingua_nuova() {
        let mut app = app_on_fixture("lang-scelta");
        app.langs = vec![Lang::english(), lingua_it_del_crate()];
        // si sceglie l'inglese (il giro del menu e' ciclico, come i temi)
        app.on_key(key(KeyCode::Char('L')));
        app.on_key(key(KeyCode::Up)); // 1 -> 0: l'inglese
        app.on_key(key(KeyCode::Enter));
        assert!(matches!(app.mode, Mode::Main));
        assert_eq!(app.cat.active.code, "en");
        // il flash parla la lingua NUOVA: "language: English"
        assert!(app.status_now().unwrap().contains("language"));
        // e il vocabolario e' cambiato davvero
        assert_eq!(app.cat.t("footer.f1"), "Help");
        // si torna italiani: il flash torna italiano
        app.on_key(key(KeyCode::Char('L')));
        app.on_key(key(KeyCode::Down)); // 0 -> 1: l'italiano
        app.on_key(key(KeyCode::Enter));
        assert_eq!(app.cat.active.code, "it");
        assert!(app.status_now().unwrap().contains("lingua"));
        assert_eq!(app.cat.t("footer.f1"), "Aiuto");
    }

    #[test]
    fn f2_rende_permanente_la_lingua_e_il_prossimo_avvio_la_riprende() {
        let fx = fixture("f2-lang");
        let tmp = std::env::temp_dir().join(format!("mfd-f2-lang-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let path = tmp.join("settings.txt");
        let tree = Arc::new(scan(&fx, &ScanOptions { depth: usize::MAX, filter: None }));
        let mut app = App::with_tree(fx.clone(), Arc::clone(&tree));
        app.langs = vec![Lang::english(), lingua_it_del_crate()];
        app.settings_path = Some(path.clone());
        // il test e' italiano: F2 lo scrive nel profilo (il fabbrica e'
        // l'inglese, una lingua diversa merita di sopravvivere)
        app.on_key(key(KeyCode::F(2)));
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("lingua=it"), "nel file: {}", text);
        // un avvio di fabbrica (inglese) che carica il profilo la riprende
        let mut app2 = App::with_tree(fx.clone(), tree);
        app2.langs = vec![Lang::english(), lingua_it_del_crate()];
        app2.adopt_lang("en"); // il fabbrica, come partirebbe davvero
        app2.settings_path = Some(path);
        app2.load_saved();
        assert_eq!(app2.cat.active.code, "it");
        let _ = std::fs::remove_dir_all(&tmp);
        let _ = std::fs::remove_dir_all(&fx);
    }
}



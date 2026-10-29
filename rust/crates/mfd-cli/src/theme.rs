//! Il tema della CLI: il look di duf (github.com/muesli/duf) per la griglia
//! vista da un terminale vero. duf e' il df a colori che ha ispirato la
//! grafica (riga 9 di infot.txt): box unicode arrotondato con gli incroci
//! delle colonne, la TOTALE che chiude il box incolonnata sotto le
//! intestazioni col conto delle voci in colonna NOME, le righe WARNING che
//! la seguono (prima cella l'etichetta, le altre fuse in una sola), barra e
//! % colorate alle soglie del suo --usage-threshold "0.5,0.9"
//! (verde sotto meta', giallo da meta', rosso dal 90% della quota sul
//! totale), nomi blu, conteggi grigi e bordi del box in giallo — il colore
//! d'accento che la console nera del TUI da' ai suoi bordi. 16 colori ANSI,
//! la stessa famiglia del TUI (docs/DESIGN-TUI.md).
//! Licenze prese (ognuna spiegata al posto giusto nel codice): i fondi che in
//! duf velano la parte vuota della barra qui non entrano — nei suoi temi
//! truecolor sono fondi quasi neri, ma nei 16 colori del tema "ansi" urlano,
//! e i punti nella stessa tinta dei # rendono gia' l'idea; i bordi del box
//! prendono il giallo 93, l'accento condiviso col TUI (il bianco 37 dello
//! screenshot di duf, sui terminali chiari, si perdeva nel testo);
//! e i nomi non si tranciano sul filo del terminale — il nome e' il dato,
//! il box e' largo quanto serve come la griglia ASCII.
//! Solo quando stdout e' un terminale: ridiretta in file o pipe la griglia
//! resta il formato ASCII storico, perche' quello e' il contratto d'output di
//! log, catture e consumatori di dati (docs/FAQ.md). NO_COLOR spegne i colori
//! ma tiene il box; MFD_ASCII=1 e' il paracadute che riporta il formato
//! storico anche sul terminale; MFD_DUF=1 forza il look duf anche ridiretto
//! (per chi lo vuole dentro less -R).

use crate::{GridRow, Totals, BAR_W, bar, fmt_bytes, group_digits, voci};
use mfd_core::lang::Catalog;
use std::env;
use std::io::IsTerminal;
use std::iter::once;
use std::path::PathBuf;

// -- ANSI, 16 colori (il tema "ansi" di duf) --

/// Il blu dei nomi: duf colora di blu il "mounted on".
const BLUE: &str = "\x1b[94m";
/// Il grigio dei conteggi secondari: duf grigio per "type" e "filesystem".
const GRAY: &str = "\x1b[37m";
/// Il colore dei bordi del box: giallo (93), lo stesso accento della console
/// nera del TUI. Il grigio chiaro (37) era il look duf originale.
const BORDER: &str = "\x1b[93m";
/// Il grasso di titolo, intestazioni e TOTALE.
const BOLD: &str = "\x1b[1m";
/// La scala d'uso di duf: verde (10), giallo (11), rosso (9) — tinge la barra
/// intera (# e punti) e la percentuale.
const GREEN: &str = "\x1b[92m";
const YELLOW: &str = "\x1b[93m";
const RED: &str = "\x1b[91m";
const RESET: &str = "\x1b[0m";

/// Come si presenta la griglia: il formato storico ASCII (il contratto) o il
/// look duf (il terminale interattivo).
pub enum Look {
    /// Il formato ASCII storico: righe piatte, barra `#`/`.`, niente colori —
    /// quello che log, redirect e catture di regressione devono trovare.
    Ascii,
    /// Il box arrotondato di duf; `color` dice se i colori ANSI sono accesi
    /// (NO_COLOR li spegne: il box resta).
    Duf { color: bool },
}

/// I colori sono ammessi? La convenzione NO_COLOR (no-color.org): presente e
/// non vuoto li spegne, qualunque sia il valore.
fn colors_allowed() -> bool {
    match env::var_os("NO_COLOR") {
        Some(v) => v.is_empty(),
        None => true,
    }
}

impl Look {
    /// La scelta del look, una volta per corsa: terminale vero -> duf;
    /// redirect/pipe -> ASCII (il contratto non cambia col pipe). MFD_ASCII=1
    /// e MFD_DUF=1 forzano la mano nelle due direzioni.
    pub fn detect() -> Look {
        if env::var_os("MFD_ASCII").is_some() {
            return Look::Ascii;
        }
        if env::var_os("MFD_DUF").is_some() {
            return Look::Duf { color: colors_allowed() };
        }
        if !std::io::stdout().is_terminal() {
            return Look::Ascii;
        }
        #[cfg(windows)]
        {
            // i colori ANSI su Windows chiedono ENABLE_VIRTUAL_TERMINAL_
            // PROCESSING; dove non si puo' accenderlo (conhost ostile) anche
            // i glifi del box sarebbero a rischio: si resta sul formato ASCII
            if !win::enable_vt() {
                return Look::Ascii;
            }
        }
        Look::Duf { color: colors_allowed() }
    }
}

/// Il minimo Windows per il look: accendere le sequenze ANSI sulla console
/// (ENABLE_VIRTUAL_TERMINAL_PROCESSING). Una coppia di chiamate kernel32 e
/// niente dipendenze: la CLI resta mfd-core e basta.
#[cfg(windows)]
mod win {
    use std::ffi::c_void;

    const STD_OUTPUT_HANDLE: i32 = -11;
    const STD_ERROR_HANDLE: i32 = -12;
    const ENABLE_VIRTUAL_TERMINAL_PROCESSING: u32 = 0x0004;

    #[link(name = "kernel32")]
    extern "system" {
        fn GetStdHandle(n_std_handle: i32) -> *mut c_void;
        fn GetConsoleMode(console_handle: *mut c_void, mode: *mut u32) -> i32;
        fn SetConsoleMode(console_handle: *mut c_void, mode: u32) -> i32;
        fn GetConsoleScreenBufferInfo(
            console_handle: *mut c_void,
            info: *mut ScreenBufferInfo,
        ) -> i32;
    }

    #[repr(C)]
    struct Coord {
        x: i16,
        y: i16,
    }

    #[repr(C)]
    struct SmallRect {
        left: i16,
        top: i16,
        right: i16,
        bottom: i16,
    }

    #[repr(C)]
    struct ScreenBufferInfo {
        size: Coord,
        cursor: Coord,
        attributes: u16,
        window: SmallRect,
        maximum: Coord,
    }

    /// L'handle richiesto, NULL se non e' la console (pipe, file, niente).
    fn handle(n_std: i32) -> *mut c_void {
        unsafe {
            let h = GetStdHandle(n_std);
            if h.is_null() || h == (-1isize) as *mut c_void {
                std::ptr::null_mut()
            } else {
                h
            }
        }
    }

    /// L'handle di stdout, NULL se non e' la console (pipe, file, niente).
    fn stdout_handle() -> *mut c_void {
        handle(STD_OUTPUT_HANDLE)
    }

    /// L'handle di stderr, lo stesso discorso (il loader vive lì).
    fn stderr_handle() -> *mut c_void {
        handle(STD_ERROR_HANDLE)
    }

    /// Accende l'elaborazione ANSI su conhost; true se la console le sa
    /// (Windows Terminal le ha sempre avute, e' conhost il caso borderline).
    pub fn enable_vt() -> bool {
        vt_on(stdout_handle())
    }

    /// La stessa domanda sull'handle di stderr: la modalita' e' per handle,
    /// accenderla su stdout non basta — e il loader scrive su stderr.
    pub fn enable_vt_stderr() -> bool {
        vt_on(stderr_handle())
    }

    /// La domanda grezza: l'handle sa le sequenze ANSI?
    fn vt_on(h: *mut c_void) -> bool {
        if h.is_null() {
            return false;
        }
        unsafe {
            let mut mode: u32 = 0;
            if GetConsoleMode(h, &mut mode) == 0 {
                return false;
            }
            SetConsoleMode(h, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING) != 0
        }
    }

    /// Le colonne della finestra di console di stderr (quelle visibili, non
    /// le piu' larghe del buffer): il loader le usa per non mandare la sua
    /// riga a capo — il ritorno carrello che la sovrascrive pulisce una riga
    /// sola. None se stderr non e' una console.
    pub fn console_width() -> Option<usize> {
        let h = stderr_handle();
        if h.is_null() {
            return None;
        }
        unsafe {
            let mut info = ScreenBufferInfo {
                size: Coord { x: 0, y: 0 },
                cursor: Coord { x: 0, y: 0 },
                attributes: 0,
                window: SmallRect { left: 0, top: 0, right: 0, bottom: 0 },
                maximum: Coord { x: 0, y: 0 },
            };
            if GetConsoleScreenBufferInfo(h, &mut info) == 0 {
                return None;
            }
            Some((info.window.right - info.window.left + 1).max(1) as usize)
        }
    }
}

/// Avvolge `s` nel codice ANSI se i colori sono accesi; altrimenti e' se'.
fn wrap(on: bool, code: &str, s: &str) -> String {
    if on {
        format!("{}{}{}", code, s, RESET)
    } else {
        s.to_string()
    }
}

/// La scala d'uso di duf (il suo --usage-threshold "0.5,0.9"): la quota della
/// riga sul totale decide il colore — verde sotto meta', giallo da meta',
/// rosso dal 90%. La riga che si porta via il disco si vede da lontano.
fn usage(share: f64) -> &'static str {
    if share >= 0.9 {
        RED
    } else if share >= 0.5 {
        YELLOW
    } else {
        GREEN
    }
}

/// La cella barra: `[` + la barra di `bar()` + `]`, tutta nella tinta d'uso —
/// `#` pieni e `.` vuoti della stessa tinta, niente fondi: nei 16 colori un
/// fondo pieno accende lo schermo, e i punti da soli rendono gia' l'idea
/// (il riempimento di duf e' un fondo quasi nero, roba da temi truecolor).
/// Senza colori resta [#.....].
fn bar_cell(bytes: i64, total: i64, color: bool) -> String {
    let b = bar(bytes, total);
    if !color {
        return format!("[{}]", b);
    }
    let share = if total > 0 { bytes as f64 / total as f64 } else { 0.0 };
    format!("[{}]", wrap(true, usage(share), &b))
}

/// Una riga del box: le celle gia' larghe `w_i` (i codici colore non contano),
/// con lo spazio di respiro e i separatori verticali alla go-pretty/duf. I
/// separatori sono del colore BORDER (37): il bianco coincide col testo
/// di default, come la cornice dello screenshot di duf.
fn row_line(cells: &[String], color: bool) -> String {
    let v = wrap(color, BORDER, "│");
    let mut s = v.clone();
    for c in cells {
        s.push(' ');
        s.push_str(c);
        s.push(' ');
        s.push_str(&v);
    }
    s
}

/// Un separatore orizzontale del box (sotto il titolo, sotto le intestazioni,
/// sopra la TOTALE): `l`/`m`/`r` sono angoli e incroci.
fn sep(l: &str, m: &str, r: &str, widths: &[usize], color: bool) -> String {
    let mut s = String::new();
    s.push_str(&wrap(color, BORDER, l));
    for (i, w) in widths.iter().enumerate() {
        if i > 0 {
            s.push_str(&wrap(color, BORDER, m));
        }
        s.push_str(&wrap(color, BORDER, &"─".repeat(w + 2)));
    }
    s.push_str(&wrap(color, BORDER, r));
    s
}

fn right(s: &str, w: usize) -> String {
    format!("{:>w$}", s, w = w)
}

fn left(s: &str, w: usize) -> String {
    format!("{:<w$}", s, w = w)
}

fn center(s: &str, w: usize) -> String {
    let pad = w.saturating_sub(s.chars().count());
    let l = pad / 2;
    format!("{}{}{}", " ".repeat(l), s, " ".repeat(pad - l))
}

/// La griglia vestita da duf: bordo arrotondato con gli incroci delle
/// colonne, intestazioni, righe e TOTALE di chiusura — incolonnata come le
/// righe di dati, col conto delle voci in colonna NOME: la lettura verticale
/// della griglia ASCII, col cumulativo sotto ogni colonna. `warnings` sono
/// le segnalazioni gia' rese (warning_lines in main.rs): quando non sono
/// vuote chiudono il box sotto la TOTALE in una sezione a parte — la prima
/// colonna resta, le altre si fondono in un'unica cella, il footer della
/// griglia ASCII rivestito. Le larghezze sono
/// il massimo fra etichetta e celle (il computeMaxContentWidths di duf). I
/// nomi non si tranciano: come nella griglia ASCII il box e' largo quanto
/// serve — se il terminale e' piu' stretto e' lui a mandare a capo, il dato
/// resta intero.
fn grid_open(
    rows: &[GridRow],
    t: &Totals,
    title: &str,
    warnings: &[String],
    files: Option<(&[(i64, PathBuf)], &str)>,
    color: bool,
    cat: &Catalog,
) -> (Vec<String>, [usize; 6], usize, usize) {
    let total = t.bytes;
    let w_size = rows.iter().map(|r| fmt_bytes(r.bytes).len()).chain(once(10)).max().unwrap();
    let w_pct = 4; // "100%"
    let w_bar = BAR_W + 2; // la barra di main.rs tra le sue quadre
    let w_files = rows
        .iter()
        .map(|r| match r.files {
            Some(f) => group_digits(f).len(),
            None => 1, // il "-" della foglia
        })
        .chain(once(cat.t("cli.file").chars().count()))
        .max()
        .unwrap();
    let w_dirs = rows
        .iter()
        .map(|r| match r.dirs {
            Some(d) => group_digits(d).len(),
            None => 1,
        })
        .chain(once(cat.t("cli.cartelle").chars().count()))
        .max()
        .unwrap();
    // il nome non ha tetto: e' il dato della griglia, quello che dice dove
    // sta lo spazio — il box e' largo quanto serve, come la griglia ASCII.
    // L'etichetta della TOTALE vive nella stessa colonna: se e' lei la piu'
    // lunga e' il box ad allargarsi — nemmeno lei si trancia
    let label = cat.tf("cli.totale", &[&voci(t.voci, cat)]);
    let mut w_name = rows
        .iter()
        .map(|r| r.name.chars().count())
        .chain(once(4))
        .chain(once(label.chars().count()))
        .max()
        .unwrap();
    // il titolo attraversa tutto il box: la cella fusa piu' larga, il
    // percorso della scansione sopra le intestazioni. Se e' piu' lungo
    // della somma delle colonne e' il box ad allargarsi — l'extra sulla
    // colonna NOME, come i nomi e i warning: nemmeno il titolo si trancia
    let w_title = w_size + w_pct + w_bar + w_files + w_dirs + w_name + 15;
    if title.chars().count() > w_title {
        w_name += title.chars().count() - w_title;
    }
    // il quadro aggiunge la classifica: le sue righe vivono nel fuso delle
    // colonne dopo SIZE — il path largo quanto quella zona di sopra, e
    // nemmeno lui si trancia: se serve piu' spazio e' il box ad allargarsi
    // (sulla colonna NOME, la regola di sempre). Li' viaggia anche il
    // titolo della sezione, nel divisore. Questo vincolo viene PRIMA di
    // quello dei warning: la sezione warning vive nello stesso fuso del
    // divisore, e la sua larghezza va calcolata sull'albero gia' allargato
    // dalla classifica — cosi' le pareti combaciano, dritte dall'alto in basso
    if let Some((entries, section)) = files {
        let base = w_pct + w_bar + w_files + w_dirs + w_name + 12;
        let need = entries
            .iter()
            .map(|(_, p)| p.display().to_string().chars().count())
            .chain(once(section.chars().count() + 4))
            .max()
            .unwrap_or(0);
        if need > base {
            w_name += need - base;
        }
    }
    // la cella fusa dei warning assorbe tutte le colonne dopo la prima
    // (spazi e incroci compresi); se una segnalazione e' piu' lunga, e' il
    // box ad allargarsi — l'extra va sulla colonna NOME, che di coda ne ha:
    // nemmeno le segnalazioni si tranciano, come i nomi. Calcolata dopo la
    // classifica perche' warning e divisore condividono il fuso: stessa
    // larghezza, parete dritta anche qui
    let w_warn = if warnings.is_empty() {
        0
    } else {
        let base = w_pct + w_bar + w_files + w_dirs + w_name + 12;
        let longest = warnings.iter().map(|w| w.chars().count()).max().unwrap_or(0);
        if longest > base {
            w_name += longest - base;
            longest
        } else {
            base
        }
    };
    let widths = [w_size, w_pct, w_bar, w_files, w_dirs, w_name];
    // la larghezza della cella fusa del titolo, ricalcolata dopo gli
    // allargamenti: il bordo alto e' liscio perche' sopra le colonne c'e'
    // il titolo, non i dati
    let fused = w_size + w_pct + w_bar + w_files + w_dirs + w_name + 15;

    let mut out = Vec::with_capacity(rows.len() + 8);
    out.push(sep("╭", "─", "╮", &[fused], color));
    // la fascia del titolo: il percorso in blu, come i nomi — la riga che
    // era della radice, promossa a intestazione del box
    out.push(row_line(&[wrap(color, BLUE, &left(title, fused))], color));
    // da qui si aprono le colonne: la forma e' quella di sempre
    out.push(sep("├", "┬", "┤", &widths, color));
    // le intestazioni (go-pretty le mette in maiuscolo: le nostre gia' lo sono)
    out.push(row_line(&[
        wrap(color, BORDER, &right(cat.t("cli.dim"), w_size)),
        wrap(color, BORDER, &right("%", w_pct)),
        left("", w_bar),
        wrap(color, BORDER, &right(cat.t("cli.file"), w_files)),
        wrap(color, BORDER, &right(cat.t("cli.cartelle"), w_dirs)),
        wrap(color, BORDER, &left(cat.t("cli.nome"), w_name)),
    ], color));
    out.push(sep("├", "┼", "┤", &widths, color));
    for r in rows {
        let pct = if total > 0 { r.bytes * 100 / total } else { 0 };
        let share = if total > 0 { r.bytes as f64 / total as f64 } else { 0.0 };
        let fg = usage(share);
        let (f, d) = match (r.files, r.dirs) {
            (Some(f), Some(d)) => (group_digits(f), group_digits(d)),
            _ => ("-".to_string(), "-".to_string()),
        };
        out.push(row_line(&[
            right(&fmt_bytes(r.bytes), w_size),
            wrap(color, fg, &right(&format!("{}%", pct), w_pct)),
            bar_cell(r.bytes, total, color),
            wrap(color, GRAY, &right(&f, w_files)),
            wrap(color, GRAY, &right(&d, w_dirs)),
            wrap(color, BLUE, &left(&r.name, w_name)),
        ], color));
    }
    // la TOTALE chiude i dati come l'ultima riga di una tabella vera: ogni
    // colonna col suo cumulativo — la dimensione sotto DIMENSIONE, i file
    // sotto FILE, le cartelle sotto CARTELLE — % e barra vuote (la somma non
    // e' una quota di se' stessa: la radice, in cima, e' gia' il 100%) e in
    // colonna NOME l'etichetta col conto delle voci, il totale dei nomi
    // elencati. Tutta in grasso, tenuta in colonna dagli stessi incroci che
    // separano l'intestazione dalle righe
    // la TOTALE chiude i dati come l'ultima riga di una tabella vera: ogni
    // colonna col suo cumulativo — la dimensione sotto DIMENSIONE, i file
    // sotto FILE, le cartelle sotto CARTELLE — % e barra vuote: la somma
    // non e' una quota di se' stessa (le righe sopra, con la divisione
    // intera, chiudono a quasi tutto: i resti non si fingono). E in colonna
    // NOME l'etichetta col conto delle voci, il totale dei nomi elencati.
    // Tutta in grasso, tenuta in colonna dagli stessi incroci che separano
    // l'intestazione dalle righe
    out.push(sep("├", "┼", "┤", &widths, color));
    out.push(row_line(&[
        wrap(color, BOLD, &right(&fmt_bytes(t.bytes), w_size)),
        left("", w_pct),
        left("", w_bar),
        wrap(color, BOLD, &right(&group_digits(t.files), w_files)),
        wrap(color, BOLD, &right(&group_digits(t.dirs), w_dirs)),
        wrap(color, BOLD, &left(&label, w_name)),
    ], color));
    // quando ci sono, i warning chiudono il box in una sezione tutta loro:
    // un incrocio solo, dove finisce la prima colonna — le altre si fondono
    // in una cella sola. L'etichetta sta in colonna DIMENSIONE sulla prima
    // riga (gialla: il colore d'avviso della famiglia), le seguenti la
    // lasciano vuota — i trattini restano in colonna; il fondo segue la
    // sezione fusa, incrocio uno anche lui
    // il fuso delle colonne dopo SIZE: dove vivono i warning e dove la
    // classifica del quadro prende casa
    let w_fused = w_pct + w_bar + w_files + w_dirs + w_name + 12;
    (out, widths, w_warn, w_fused)
}

/// La sezione dei warning: le segnalazioni rese come righe della tabella —
/// l'etichetta gialla incolonnata su SIZE, il testo fuso accanto. Apre col
/// suo incrocio e non chiude: sta a chi chiama dirle dove — la griglia da
/// sola le vuole sotto la TOTALE, il quadro le vuole in fondo, dopo la
/// classifica: note e segnalazioni, contestuali a entrambe le parti,
/// secondarie a entrambe
fn warn_rows(warnings: &[String], w_size: usize, w_fused: usize, color: bool, cat: &Catalog) -> Vec<String> {
    let mut out = Vec::with_capacity(warnings.len() + 1);
    out.push(sep("├", "┼", "┤", &[w_size, w_fused], color));
    for (i, w) in warnings.iter().enumerate() {
        let etichetta = if i == 0 {
            right(cat.t("cli.warn"), w_size)
        } else {
            left("", w_size)
        };
        out.push(row_line(&[wrap(color, YELLOW, &etichetta), left(w, w_fused)], color));
    }
    out
}

/// La griglia vestita da duf, tutta sua: il corpo di grid_open chiuso dal
/// fondo — incroci sulle colonne, o sulla sezione fusa dei warning.
pub fn render_table(
    rows: &[GridRow],
    t: &Totals,
    title: &str,
    warnings: &[String],
    color: bool,
    cat: &Catalog,
) -> Vec<String> {
    let (mut out, widths, w_warn, _) = grid_open(rows, t, title, warnings, None, color, cat);
    let w_size = widths[0];
    if warnings.is_empty() {
        out.push(sep("╰", "┴", "╯", &widths, color));
    } else {
        // la griglia da sola: le segnalazioni chiudono sotto la TOTALE,
        // la forma di sempre
        out.extend(warn_rows(warnings, w_size, w_warn, color, cat));
        out.push(sep("╰", "┴", "╯", &[w_size, w_warn], color));
    }
    out
}

/// Il quadro vestito da duf: un box solo in due parti. Sopra la griglia —
/// fascia del percorso, intestazioni, righe, TOTALE; sotto la classifica
/// dei file piu' grandi: la colonna SIZE e' quella di sopra, il path vive
/// nel fuso delle altre colonne. In mezzo il divisore di sezione: la riga
/// che scioglie le colonne e porta il titolo della classifica fra i
/// trattini. In fondo, dopo i dati, le segnalazioni: note contestuali a
/// entrambe le parti, secondarie a entrambe. Un solo oggetto, due risposte.
pub fn render_report(
    rows: &[GridRow],
    t: &Totals,
    title: &str,
    warnings: &[String],
    entries: &[(i64, PathBuf)],
    section: &str,
    color: bool,
    cat: &Catalog,
) -> Vec<String> {
    let (mut out, widths, _, w_fused) =
        grid_open(rows, t, title, warnings, Some((entries, section)), color, cat);
    let w_size = widths[0];
    // il divisore di sezione: la colonna DIMENSIONE resta colonna e si
    // ri-dichiara — la sezione inizia dopo la TOTALE, e la sua colonna
    // SIZE merita l'etichetta di nuovo, gialla come le intestazioni. Nel
    // fuso il titolo della sezione: centrato, maiuscolo, giallo anche lui.
    // Una riga sopra e una sotto; poi le righe della classifica
    out.push(sep("├", "┼", "┤", &[w_size, w_fused], color));
    out.push(row_line(
        &[
            wrap(color, BORDER, &right(cat.t("cli.dim"), w_size)),
            wrap(color, BORDER, &center(section, w_fused)),
        ],
        color,
    ));
    out.push(sep("├", "┼", "┤", &[w_size, w_fused], color));
    // le righe della classifica: SIZE in colonna con la griglia, il path
    // nel fuso — in blu, come tutti i path della famiglia
    for (s, p) in entries {
        out.push(row_line(
            &[
                right(&fmt_bytes(*s), w_size),
                wrap(color, BLUE, &left(&p.display().to_string(), w_fused)),
            ],
            color,
        ));
    }
    // le segnalazioni chiudono il quadro, dopo i dati: note contestuali a
    // entrambe le parti, secondarie a entrambe — prima la mappa e i
    // colpevoli, poi cio' che lo scan non ha potuto leggere
    if !warnings.is_empty() {
        out.extend(warn_rows(warnings, w_size, w_fused, color, cat));
    }
    out.push(sep("╰", "┴", "╯", &[w_size, w_fused], color));
    out
}

/// La classifica di -big vestita da duf: lo stesso box arrotondato della
/// griglia, le stesse regole — bordi e intestazioni gialli, i full path in
/// blu come i nomi, le segnalazioni che chiudono in una sezione fusa col
/// fondo a incrocio uno. Due colonne sole: DIMENSIONE e PATH (che col filtro
/// attivo lo dichiara, come la radice della griglia). Niente TOTALE: la
/// lista e' la risposta, e i file esclusi dal taglio non sono totali persi
/// (docs/FAQ.md, voce 11). Anche qui niente si trancia: i path sono il dato,
/// il box e' largo quanto serve — e se una segnalazione e' piu' lunga e' il
/// box ad allargarsi, con l'extra sulla colonna PATH: la stessa regola che
/// i nomi della griglia seguono da sempre.
/// La classifica di -big vestita da duf: lo stesso box arrotondato della
/// griglia, le stesse regole — bordi e intestazioni gialli, i full path in
/// blu come i nomi, le segnalazioni che chiudono in una sezione fusa col
/// fondo a incrocio uno. Due colonne sole — DIMENSIONE e PATH — senza
/// intestazioni ridichiate: la fascia del titolo dice cosa sono, e la
/// lente, quando c'e', viaggia nel titolo. Niente TOTALE: la
/// lista e' la risposta, e i file esclusi dal taglio non sono totali persi
/// (docs/FAQ.md, voce 11). Anche qui niente si trancia: i path sono il dato,
/// il box e' largo quanto serve — e se una segnalazione e' piu' lunga e' il
/// box ad allargarsi, con l'extra sulla colonna PATH: la stessa regola che
/// i nomi della griglia seguono da sempre.
pub fn render_big_table(
    entries: &[(i64, PathBuf)],
    title: &str,
    warnings: &[String],
    color: bool,
    cat: &Catalog,
) -> Vec<String> {
    let w_size = entries
        .iter()
        .map(|(s, _)| fmt_bytes(*s).len())
        .chain(once(10))
        .max()
        .unwrap();
    let mut w_path = entries
        .iter()
        .map(|(_, p)| p.display().to_string().chars().count())
        .chain(warnings.iter().map(|w| w.chars().count()))
        .max()
        .unwrap();
    // il titolo attraversa il box: la cella fusa sopra le righe, la stessa
    // fascia della griglia — la classifica dichiara cosa si guarda
    let w_title = w_size + w_path + 3;
    if title.chars().count() > w_title {
        w_path += title.chars().count() - w_title;
    }
    let widths = [w_size, w_path];
    let fused = w_size + w_path + 3;
    let mut out = Vec::with_capacity(entries.len() + 6);
    out.push(sep("╭", "─", "╮", &[fused], color));
    // la fascia: il titolo della sezione, centrato e giallo come le
    // intestazioni — la stessa riga che nel quadro divide griglia e
    // classifica, qui e' la testa del box
    out.push(row_line(&[wrap(color, BORDER, &center(title, fused))], color));
    out.push(sep("├", "┬", "┤", &widths, color));
    for (s, p) in entries {
        out.push(row_line(
            &[
                right(&fmt_bytes(*s), w_size),
                wrap(color, BLUE, &left(&p.display().to_string(), w_path)),
            ],
            color,
        ));
    }
    // le segnalazioni chiudono il box come chiudono la griglia: prima cella
    // l'etichetta (gialla, il colore d'avviso della famiglia), resto della
    // riga fuso in una cella sola — chi impara a leggerle in una risposta
    // le ritrova identiche nell'altra
    if warnings.is_empty() {
        out.push(sep("╰", "┴", "╯", &widths, color));
    } else {
        out.push(sep("├", "┼", "┤", &widths, color));
        for (i, w) in warnings.iter().enumerate() {
            let etichetta = if i == 0 {
                right(cat.t("cli.warn"), w_size)
            } else {
                left("", w_size)
            };
            out.push(row_line(
                &[wrap(color, YELLOW, &etichetta), left(w, w_path)],
                color,
            ));
        }
        out.push(sep("╰", "┴", "╯", &widths, color));
    }
    out
}

/// Il loader della scansione: l'aspetto della riga live su stderr che
/// scan_live (main.rs) stampa mentre il walker lavora. Due vestiti: quello
/// nuovo — spinner braille la cui tinta ruota per le dodici tinte ANSI,
/// barra d'avanzamento che si riempie seguendo lo spettro, percentuale
/// stimata onesta — e quello storico, forma di sempre con le parole della
/// lingua attiva, che `MFD_ASCII=1` e le console ostili devono ritrovare.
/// Tutto puro: la riga la costruisce chi
/// la stampa, e i test la leggono glifo per glifo.
pub mod loader {
    use super::*;
    use mfd_core::Progress;

    /// I dieci frame dello spinner braille: piu' fluido dell'ASCII storico
    /// `|/-\` e non costa niente — il loader vive solo su stderr
    /// interattivo, mai nei log.
    const SPIN: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

    /// Le tinte che il tema della griglia non ha e la ruota chiede: ciano e
    /// magenta accesi (chiudono il cerchio delle sei famiglie) e il grigio
    /// scuro del vuoto della barra.
    const CYAN: &str = "\x1b[96m";
    const MAGENTA: &str = "\x1b[95m";
    const DARK: &str = "\x1b[90m";

    /// La ruota dello spinner: dodici passi attorno alle sei tinte ANSI,
    /// ciascuna prima accesa e poi nella variante spenta — la tinta scorre
    /// (un passo per riga, 100 ms) e respira. Un ciclo pieno e' poco piu'
    /// di un secondo: lento abbastanza da non stordire.
    const WHEEL: [&str; 12] = [
        BLUE, "\x1b[34m", CYAN, "\x1b[36m", GREEN, "\x1b[32m", YELLOW, "\x1b[33m", RED,
        "\x1b[31m", MAGENTA, "\x1b[35m",
    ];

    /// Lo spettro della barra: le sei tinte accese in ordine d'arcobaleno.
    /// La parte piena le percorre cella per cella (tre celle per tinta): la
    /// barra si accende dal rosso e arriva al magenta man mano che la
    /// scansione chiude cartelle — il gradiente e' spaziale, non un
    /// lampeggio.
    const SPECTRUM: [&str; 6] = [RED, YELLOW, GREEN, CYAN, BLUE, MAGENTA];

    /// La larghezza della barra del loader: piu' corta di quella della
    /// griglia (24) perche' qui la riga porta anche spinner, percentuale e
    /// contatori, e deve stare in una riga di terminale.
    const BAR: usize = 18;

    /// Il budget del percorso nel vestito nuovo; in quello storico e' 40,
    /// il formato di sempre non si tocca.
    const PATH: usize = 28;
    const PATH_LEGACY: usize = 40;

    /// Il resto della riga nel caso tipico (spinner, etichetta, barra,
    /// percentuale, contatori): nota la larghezza della console, il budget
    /// del percorso si adatta per tenere tutto su una riga.
    const FIXED: usize = 75;

    /// Il vestito nuovo si puo' indossare? `MFD_ASCII=1` e' il paracadute
    /// che lo spegne (come per il box), e su Windows serve una console che
    /// sappia le sequenze ANSI anche su stderr, dove il loader vive —
    /// altrimenti i codici uscirebbero come sporcizia. Sul terminale vero
    /// di oggi la risposta e' quasi sempre si'.
    pub fn fancy() -> bool {
        if env::var_os("MFD_ASCII").is_some() {
            return false;
        }
        #[cfg(windows)]
        {
            if !win::enable_vt_stderr() {
                return false;
            }
        }
        true
    }

    /// I colori sono ammessi? La convenzione del resto del look (NO_COLOR):
    /// il vestito nuovo resta, monocromo.
    pub fn colored() -> bool {
        colors_allowed()
    }

    /// La larghezza della console di stderr quando si riesce a leggerla.
    #[cfg(windows)]
    fn console_width() -> Option<usize> {
        win::console_width()
    }
    #[cfg(not(windows))]
    fn console_width() -> Option<usize> {
        None
    }

    /// Il budget del percorso: col vestito nuovo barra e percentuale si
    /// prendono il loro spazio, quindi il percorso ha meno glifi; e se si
    /// conosce la larghezza della console il budget scende finche' la riga
    /// non rischia di andare a capo (il ritorno carrello puo' pulire una
    /// riga sola). Pavimento basso: un pezzetto di percorso meglio di niente.
    pub fn path_budget(fancy: bool) -> usize {
        if !fancy {
            return PATH_LEGACY;
        }
        match console_width() {
            Some(w) => w.saturating_sub(FIXED).clamp(8, PATH),
            None => PATH,
        }
    }

    /// Il percorso mostrato, accorciato se supera il budget: puntini in
    /// testa e coda integra, com'e' sempre stato.
    pub fn shorten(path: &str, budget: usize) -> String {
        let n = path.chars().count();
        if n <= budget {
            return path.to_string();
        }
        let tail: String = path.chars().skip(n + 3 - budget).collect();
        format!("...{}", tail)
    }

    /// La percentuale stimata: cartelle chiuse sul totale visto (chiuse +
    /// ancora aperte) — la stima onesta della schermata di scansione del
    /// TUI. Non e' un fatto (il totale non e' noto a priori) e puo' anche
    /// scendere quando il walker scopre cartelle nuove: e' giusto cosi'.
    fn pct(p: &Progress) -> i64 {
        let tot = p.closed + p.open;
        if tot > 0 {
            p.closed * 100 / tot
        } else {
            0
        }
    }

    /// La riga del vestito nuovo: spinner, etichetta e percorso, barra,
    /// percentuale e contatori. Restituisce la riga e la sua larghezza
    /// visibile — i codici colore non contano, e chi stampa deve saperlo
    /// per stendere e pulire la riga senza residui.
    pub fn line(p: &Progress, frame: usize, path: &str, color: bool, cat: &Catalog) -> (String, usize) {
        let pc = pct(p);
        let filled = ((pc as usize) * BAR / 100).min(BAR);
        // la riga si costruisce due volte in parallelo: coi soli glifi
        // (per la larghezza visibile) e vestita di codici quando il colore e' on
        let mut plain = String::new();
        let mut s = String::new();
        let mut put = |t: &str, c: Option<&str>| {
            plain.push_str(t);
            match c {
                Some(c) if color => {
                    s.push_str(c);
                    s.push_str(t);
                    s.push_str(RESET);
                }
                _ => s.push_str(t),
            }
        };
        let spin = SPIN[frame % SPIN.len()];
        put(&spin.to_string(), Some(WHEEL[frame % WHEEL.len()]));
        put(cat.t("cli.load_verb"), None);
        put(path, Some(BLUE));
        put("  ", None);
        for i in 0..BAR {
            let (cell, c) = if i < filled {
                ("#", SPECTRUM[i * SPECTRUM.len() / BAR])
            } else {
                (".", DARK)
            };
            put(cell, Some(c));
        }
        put("  ", None);
        put(&format!("{:>3}%", pc), Some(BOLD));
        put(
            &format!(
                "  {}",
                cat.tf(
                    "cli.load_conti",
                    &[&group_digits(p.files), &group_digits(p.dirs), &fmt_bytes(p.bytes)]
                )
            ),
            None,
        );
        (s, plain.chars().count())
    }

    /// Il loader storico: il formato di sempre, che `MFD_ASCII=1` e le
    /// console ostili devono ritrovare (le parole parlano la lingua attiva,
    /// la forma no: stesso spinner, stessi due punti, stessi contatori).
    pub fn legacy(p: &Progress, frame: usize, path: &str, cat: &Catalog) -> (String, usize) {
        const SPIN: [char; 4] = ['|', '/', '-', '\\'];
        let s = format!(
            "{}{}{}: {}",
            SPIN[frame % SPIN.len()],
            cat.t("cli.load_verb"),
            path,
            cat.tf(
                "cli.load_conti",
                &[&group_digits(p.files), &group_digits(p.dirs), &fmt_bytes(p.bytes)]
            )
        );
        let width = s.chars().count();
        (s, width)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::title_band;

    /// Il vocabolario dei test: l'italiano del crate (come main.rs).
    fn cat() -> mfd_core::lang::Catalog {
        crate::test_cat()
    }

    fn row(bytes: i64, name: &str) -> GridRow {
        GridRow { bytes, files: Some(1), dirs: Some(0), name: name.to_string() }
    }

    #[test]
    fn il_box_duf_di_big_ha_bordo_fascia_dati_e_fondo() {
        let entries = vec![
            (2000, PathBuf::from("C:\\demo\\alpha\\data.dat")),
            (1000, PathBuf::from("C:\\demo\\alpha\\lib.dll")),
        ];
        let lines = render_big_table(&entries, "LA CLASSIFICA", &[], false, &cat());
        // bordo alto e basso del box, come la griglia
        assert!(lines[0].starts_with('╭') && lines[0].ends_with('╮'));
        let bottom = lines.last().unwrap();
        assert!(bottom.starts_with('╰') && bottom.ends_with('╯'));
        // tutte le righe sono lunghe uguali: il box non si rompe
        let w = lines[0].chars().count();
        assert!(lines.iter().all(|l| l.chars().count() == w));
        // la fascia del titolo, il separatore, le due righe di dati, fondo
        assert!(lines[1].contains("LA CLASSIFICA"));
        assert!(lines[3].contains("data.dat") && lines[3].contains("2,0 KB"));
        assert!(lines[4].contains("lib.dll"));
        assert_eq!(lines.len(), 6);
    }

    #[test]
    fn il_box_duf_di_big_colora_i_path_e_chiude_con_i_warning() {
        let entries = vec![(2000, PathBuf::from("a\\data.dat"))];
        let warns = vec!["- apertura cartella C:\\x: Accesso negato.".to_string()];
        let lines = render_big_table(&entries, "LA CLASSIFICA", &warns, true, &cat());
        // il path e' blu, come i nomi della griglia (la riga dati e' la
        // quarta: bordo, fascia, separatore, dati)
        assert!(lines[3].contains(BLUE));
        // i warning chiudono il box: etichetta in colonna SIZE, fondo a
        // incrocio uno come la sezione fusa della griglia — coi colori il
        // fondo comincia col codice ANSI del bordo, si cerca dentro
        assert!(lines.iter().any(|l| l.contains("WARNING")));
        let bottom = lines.last().unwrap();
        assert!(bottom.contains('╰') && bottom.matches('┴').count() == 1);
        // senza colori non c'e' nessun codice ANSI in giro
        let plain = render_big_table(&entries, "LA CLASSIFICA", &warns, false, &cat());
        assert!(plain.iter().all(|l| !l.contains('\x1b')));
    }

    #[test]
    fn il_box_duf_e_chiuso_con_titolo_intestazione_e_totale() {
        let rows = vec![row(100, "radice"), row(60, "|-- a\\"), row(40, "`-- b")];
        let t = Totals { bytes: 100, files: 1, dirs: 0, voci: 3 };
        let lines = render_table(&rows, &t, "C:\\demo\\radice", &[], false, &cat());
        // bordo alto e basso del box
        assert!(lines[0].starts_with('╭') && lines[0].ends_with('╮'));
        let bottom = lines.last().unwrap();
        assert!(bottom.starts_with('╰') && bottom.ends_with('╯'));
        // tutte le righe sono lunghe uguali: il box non si rompe
        let w = lines[0].chars().count();
        assert!(lines.iter().all(|l| l.chars().count() == w), "righe di larghezza diversa");
        // la fascia del percorso sopra le intestazioni
        assert!(lines[1].contains("C:\\demo\\radice"));
        // l'intestazione nomina le colonne
        assert!(lines[3].contains("DIMENSIONE"));
        assert!(lines[3].contains("CARTELLE"));
        assert!(lines[3].contains("NOME"));
        // gli incroci: il bordo alto e' liscio (sopra c'e' il titolo, non
        // i dati), le colonne si aprono sotto la fascia
        assert!(!lines[0].contains('┬'));
        assert!(lines[2].contains('┬'));
        assert!(lines[4].contains('┼'));
        // la TOTALE e' l'ultima riga del box: incolonnata come le righe di
        // dati (i divisori │ ci sono tutti) e col conto delle voci in colonna
        // NOME; il separatore sopra le tiene, come quello dell'intestazione
        let tot = &lines[lines.len() - 2];
        assert!(tot.contains("TOTALE: 3 voci"));
        assert!(tot.contains("100 B"));
        assert!(tot.contains('│'));
        assert!(lines[lines.len() - 3].contains('┼'));
        assert!(bottom.contains('┴'));
        // e le colonne coincidono con quelle delle righe di dati: la
        // dimensione e l'etichetta (nella colonna dei nomi) cadono sugli
        // stessi caratteri della prima riga
        let data = &lines[5];
        assert_eq!(data.find("100 B"), tot.find("100 B"));
        assert_eq!(data.find("radice"), tot.find("TOTALE"));
    }

    #[test]
    fn l_intestazione_inglese_non_sfora_la_colonna_dei_file() {
        // FILES in inglese (5) e' piu' lungo del minimo storico della colonna
        // (4, il FILE italiano): con pochi file l'intestazione sforava la
        // cella e tutto cio' che seguiva slittava di uno. La larghezza
        // minima ora segue la lingua attiva, perche' ogni lingua chiama le
        // colonne a modo suo (ARCHIVOS, ARQUIVOS, DATEIEN...) e nessuna
        // intestazione puo' uscire dal suo box.
        let cat = mfd_core::lang::Catalog::english();
        let rows = vec![row(100, "root"), row(60, "|-- a")];
        let t = Totals { bytes: 100, files: 7, dirs: 0, voci: 2 };
        let lines = render_table(&rows, &t, "root folder", &[], false, &cat);
        // il box resta intero: tutte le righe della stessa larghezza, con
        // l'intestazione inglese dentro la sua cella
        let w = lines[0].chars().count();
        assert!(lines.iter().all(|l| l.chars().count() == w), "righe di larghezza diversa");
        assert!(lines[3].contains("FILES"));
        assert!(lines[3].contains("FOLDERS"));
    }

    #[test]
    fn le_soglie_di_duf_colorano_barra_e_percentuale() {
        let rows = vec![row(100, "r"), row(95, "rosso"), row(60, "giallo"), row(10, "verde")];
        let t = Totals { bytes: 100, files: 4, dirs: 0, voci: 4 };
        let lines = render_table(&rows, &t, "C:\\demo", &[], true, &cat());
        let find = |n: &str| lines.iter().find(|l| l.contains(n)).unwrap().clone();
        // 95% e 100% rosse, 60% gialla, 10% verde: le soglie 0.5/0.9 di duf
        assert!(find("rosso").contains(RED));
        assert!(find("r").contains(RED)); // la radice al 100%
        assert!(find("giallo").contains(YELLOW));
        assert!(find("verde").contains(GREEN));
        // la barra e' tutta della tinta d'uso, punti compresi: nessun codice
        // di sfondo ([4x) in giro — i fondi pieni accenderebbero lo schermo
        assert!(lines.iter().all(|l| !l.contains("\x1b[4")));
        // e bordi, intestazioni e TOTALE hanno i colori del tema
        assert!(lines[0].contains(BORDER));
        assert!(lines[2].contains(BORDER));
        assert!(find("DIMENSIONE").contains(BORDER));
        assert!(find("voci").contains(BOLD));
        // senza colori non c'e' nessun codice ANSI in giro
        let plain = render_table(&rows, &t, "radice", &[], false, &cat());
        assert!(plain.iter().all(|l| !l.contains('\x1b')));
    }

    #[test]
    fn i_nomi_lunghi_non_si_tranciano_il_box_si_allarga() {
        let rows = vec![row(100, "nome-lunghissimo-che-non-ci-sta-proprio")];
        let t = Totals { bytes: 100, files: 1, dirs: 0, voci: 1 };
        let lines = render_table(&rows, &t, "radice", &[], false, &cat());
        // il nome resta intero: niente "..." da taglio — il nome e' il dato,
        // il box si allarga quanto serve, come la griglia ASCII
        assert!(lines[5].contains("nome-lunghissimo-che-non-ci-sta-proprio"));
        assert!(!lines[5].contains("..."));
        // e resta un box: tutte le righe della stessa larghezza
        let w = lines[0].chars().count();
        assert!(lines.iter().all(|l| l.chars().count() == w));
        // una riga sola: il conto delle voci al singolare, nella TOTALE
        assert!(lines[lines.len() - 2].contains("TOTALE: 1 voce"));
    }

    #[test]
    fn i_warning_chiudono_il_box_fusi_dopo_la_totale() {
        let rows = vec![row(100, "radice")];
        let t = Totals { bytes: 100, files: 1, dirs: 0, voci: 1 };
        let warns = vec![
            "- apertura cartella C:\\x: Accesso negato.".to_string(),
            "- enumerazione in C:\\y: Accesso negato.".to_string(),
        ];
        let lines = render_table(&rows, &t, "radice", &warns, false, &cat());
        // top, fascia, colonne, intestazione, separatore, dato, separatore,
        // TOTALE, separatore, due righe di warning, fondo
        assert_eq!(lines.len(), 12);
        // il box non si rompe: tutte le righe della stessa larghezza
        let w = lines[0].chars().count();
        assert!(lines.iter().all(|l| l.chars().count() == w), "righe di larghezza diversa");
        // la sezione warning ha incrocio uno: dove finisce la prima cella —
        // le altre colonne si fondono in una
        let sep_w = &lines[8];
        assert!(sep_w.starts_with('├') && sep_w.ends_with('┤'));
        assert_eq!(sep_w.matches('┼').count(), 1);
        // la prima riga porta l'etichetta in colonna DIMENSIONE e il testo
        // accanto: due celle, tre separatori — e la prima cella finisce
        // dove finisce nelle righe di dati
        let warn1 = &lines[9];
        assert!(warn1.contains("WARNING"));
        assert!(warn1.contains("- apertura cartella"));
        assert_eq!(warn1.matches('│').count(), 3);
        assert_eq!(
            warn1.match_indices('│').nth(1),
            lines[5].match_indices('│').nth(1)
        );
        // le successive lasciano la prima cella vuota: i trattini restano in
        // colonna, la lettura resta ordinata
        let warn2 = &lines[10];
        assert!(warn2.contains("- enumerazione"));
        assert!(warn2.starts_with(&format!("│{}│", " ".repeat(12))));
        assert_eq!(warn1.find("- "), warn2.find("- "));
        // il fondo segue la sezione fusa: incrocio uno
        let bottom = lines.last().unwrap();
        assert!(bottom.starts_with('╰') && bottom.ends_with('╯'));
        assert_eq!(bottom.matches('┴').count(), 1);
        // coi colori l'etichetta e' gialla — il colore d'avviso — e il
        // testo resta della tinta del terminale
        let colored = render_table(&rows, &t, "radice", &warns, true, &cat());
        assert!(colored[9].contains(&format!("{}   WARNING{}", YELLOW, RESET)));
    }

    #[test]
    fn i_warning_lunghi_allargano_il_box_senza_troncare() {
        let rows = vec![row(100, "radice")];
        let t = Totals { bytes: 100, files: 1, dirs: 0, voci: 1 };
        let corto = render_table(&rows, &t, "radice", &["- corto: Accesso negato.".to_string()], false, &cat());
        let testo = format!(
            "- apertura cartella C:\\{}: Accesso negato.",
            "molto-lungo-".repeat(20)
        );
        let lungo = render_table(&rows, &t, "radice", &[testo.clone()], false, &cat());
        // la segnalazione resta intera: e' il box ad allargarsi (l'extra va
        // sulla colonna NOME), e le righe restano tutte della stessa
        // larghezza — piu' larga di prima
        assert!(lungo[9].contains(&testo));
        let w = lungo[0].chars().count();
        assert!(lungo.iter().all(|l| l.chars().count() == w));
        assert!(w > corto[0].chars().count());
    }

    #[test]
    fn la_fascia_della_griglia_ascii_e_il_trattino_di_ncdu() {
        // la fascia del titolo: il percorso fra i trattini, riempita a 79
        // — la convenzione di ncdu, il posto dove la radice vive adesso
        // che riga non e' piu'
        let corta = title_band("C:\\demo");
        assert_eq!(corta.chars().count(), 79);
        assert!(corta.starts_with("--- C:\\demo ---"));
        assert!(corta.ends_with('-'));
        // lunga: niente riempimento, la fascia resta dichiarata
        let lunga = title_band(&"x".repeat(100));
        assert_eq!(lunga, format!("--- {} ---", "x".repeat(100)));
    }

    #[test]
    fn il_quadro_e_un_box_solo_in_due_parti() {
        // il quadro: la griglia sopra e la classifica sotto, un box solo.
        // La sezione si dichiara con la sua intestazione — la colonna
        // DIMENSIONE che riparte e il titolo centrato nel fuso — e la SIZE
        // della classifica resta in colonna con la griglia di sopra
        let rows = vec![row(100, "|-- a"), row(60, "`-- b")];
        let t = Totals { bytes: 100, files: 2, dirs: 0, voci: 2 };
        let entries = vec![(100, PathBuf::from("C:\\demo\\big.dat"))];
        let lines = render_report(
            &rows, &t, "C:\\demo", &[], &entries, "I 2 FILE PIU' GRANDI", false, &cat(),
        );
        // un box solo: bordo liscio in alto, fondo unico, righe tutte uguali
        assert!(lines[0].starts_with('╭') && !lines[0].contains('┬'));
        assert!(lines.last().unwrap().starts_with('╰'));
        let w = lines[0].chars().count();
        assert!(lines.iter().all(|l| l.chars().count() == w), "righe di larghezza diversa");
        // bordo, fascia, colonne, intestazione, separatore, due dati,
        // separatore, TOTALE, separatore, intestazione di sezione,
        // separatore, la riga della classifica, fondo
        assert_eq!(lines.len(), 14);
        assert!(lines[8].contains("TOTALE: 2 voci"));
        // l'intestazione della sezione: DIMENSIONE a sinistra, il titolo
        // al centro nel fuso
        let sez = &lines[10];
        assert!(sez.contains("DIMENSIONE"));
        assert!(sez.contains("I 2 FILE PIU' GRANDI"));
        assert_eq!(sez.matches('│').count(), 3);
        // la riga della classifica: SIZE in colonna con la griglia
        let file = &lines[12];
        assert!(file.contains("big.dat"));
        assert_eq!(lines[5].find("100 B"), file.find("100 B"));
    }

    // -- il loader (theme::loader) --

    /// Un avanzamento per il loader: i numeri che la riga mostra.
    fn prog(files: i64, dirs: i64, bytes: i64, closed: i64, open: i64) -> mfd_core::Progress {
        mfd_core::Progress { files, dirs, bytes, errors: 0, closed, open }
    }

    #[test]
    fn il_loader_storico_resta_di_sempre_e_parla_la_lingua() {
        // MFD_ASCII=1 e le console ostili devono ritrovare il loader di
        // sempre: la forma non si tocca, le parole seguono la lingua attiva
        let (s, w) = loader::legacy(&prog(3, 1, 500, 3, 1), 2, "radice", &cat());
        assert_eq!(s, "- scansione di radice: 3 file, 1 cartelle, 500 B");
        assert_eq!(w, s.chars().count());
        // e in inglese la forma e' identica, solo le parole cambiano: il
        // contratto della console ostile e' la forma, non la lingua
        let en = mfd_core::lang::Catalog::english();
        let (s2, _) = loader::legacy(&prog(3, 1, 500, 3, 1), 2, "radice", &en);
        assert_eq!(s2, "- scanning radice: 3 files, 1 folders, 500 B");
    }

    #[test]
    fn il_loader_nuovo_e_spinner_etichetta_barra_percentuale_contatori() {
        // la riga del vestito nuovo: spinner braille, etichetta e percorso,
        // barra di 18, percentuale incolonnata a destra e contatori in coda
        let (s, w) = loader::line(&prog(3, 1, 500, 1, 1), 0, "radice", false, &cat());
        assert!(s.starts_with("⠋ scansione di radice  "));
        assert!(s.contains("#########.........")); // 9 su 18 al 50%
        assert!(s.ends_with("   50%  3 file, 1 cartelle, 500 B"));
        // la larghezza dichiarata e' quella dei glifi: senza colori coincide
        assert_eq!(w, s.chars().count());
    }

    #[test]
    fn la_percentuale_del_loader_e_la_stima_onesta_del_tui() {
        // chiuse su chiuse + aperte, come la schermata di scansione del TUI;
        // a scansione appena nata non si sa niente: zero, con barra vuota
        let (s, _) = loader::line(&prog(0, 0, 0, 3, 1), 0, "r", false, &cat());
        assert!(s.contains("  75%"));
        let (s, _) = loader::line(&prog(0, 0, 0, 0, 0), 0, "r", false, &cat());
        assert!(s.contains("  0%"));
        assert!(s.contains(".................."));
    }

    #[test]
    fn la_barra_si_riempie_in_proporzione() {
        let (vuota, _) = loader::line(&prog(0, 0, 0, 0, 0), 0, "r", false, &cat());
        let (piena, _) = loader::line(&prog(0, 0, 0, 100, 0), 0, "r", false, &cat());
        assert!(vuota.contains(".................."));
        assert!(piena.contains("##################"));
    }

    #[test]
    fn i_codici_colore_non_contano_nella_larghezza_visibile() {
        // vestita o monocroma, la riga occupa gli stessi glifi: chi la
        // stampa stende e pulisce su questa larghezza, i codici non devono
        // allungare niente
        let p = prog(3, 1, 500, 1, 1);
        let (mono, w) = loader::line(&p, 0, "radice", false, &cat());
        let (colorata, wc) = loader::line(&p, 0, "radice", true, &cat());
        assert_eq!(w, wc);
        assert!(!mono.contains('\x1b'));
        assert!(colorata.chars().count() > wc);
    }

    #[test]
    fn lo_spinner_gira_e_la_tinta_ruota_con_i_frame() {
        let p = prog(0, 0, 0, 0, 0);
        let (f0, _) = loader::line(&p, 0, "r", true, &cat());
        let (f1, _) = loader::line(&p, 1, "r", true, &cat());
        // il braille avanza di un frame, la ruota con lui (tinta accesa,
        // poi spenta della stessa famiglia)...
        assert!(f0.starts_with("\x1b[94m⠋\x1b[0m"));
        assert!(f1.starts_with("\x1b[34m⠙\x1b[0m"));
        // ...e al giro della ruota (dodici passi) il braille e' gia' avanti
        // perche' i suoi frame sono dieci
        let (f12, _) = loader::line(&p, 12, "r", true, &cat());
        assert!(f12.starts_with("\x1b[94m⠹\x1b[0m"));
    }

    #[test]
    fn la_barra_arcobaleno_segue_lo_spettro() {
        // ogni cella piena prende la tinta della sua posizione: al 50% le
        // nove piene coprono rosso, giallo e verde e il ciano resta di la'
        // da venire; le vuote sono grigio scuro, la percentuale e' grassa e
        // il percorso blu come i nomi della griglia
        let (s, _) = loader::line(&prog(3, 1, 500, 1, 1), 0, "radice", true, &cat());
        assert!(s.contains(&format!("{}#", RED)));
        assert!(s.contains(&format!("{}#", YELLOW)));
        assert!(s.contains(&format!("{}#", GREEN)));
        assert!(!s.contains("\x1b[96m#"));
        assert!(s.contains("\x1b[90m."));
        assert!(s.contains("\x1b[1m 50%"));
        assert!(s.contains("\x1b[94mradice"));
    }

    #[test]
    fn il_percorso_si_accorcia_al_budget_con_i_puntini() {
        let lungo = "a".repeat(40);
        let s = loader::shorten(&lungo, 28);
        assert_eq!(s.chars().count(), 28);
        assert!(s.starts_with("..."));
        assert!(s.ends_with(&"a".repeat(25)));
        assert_eq!(loader::shorten("corto", 28), "corto");
    }

    #[test]
    fn il_budget_del_percorso_segue_il_vestito() {
        // il loader storico ha il suo 40 di sempre; il vestito nuovo parte
        // da 28 e puo' solo scendere se la console e' stretta (pavimento 8)
        assert_eq!(loader::path_budget(false), 40);
        assert!((8..=28).contains(&loader::path_budget(true)));
    }
}